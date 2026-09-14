//! Windows implementation via Toolhelp + psapi (TASK-006 M3).
//!
//! Ham `extern "system"` FFI, yeni crate yok (`macos.rs` libproc emsali):
//! - `list_children`: `CreateToolhelp32Snapshot` + `Process32First/Next`
//! - `rss_mb`: `GetProcessMemoryInfo` (`WorkingSetSize`)
//! - CPU ham zaman: `GetProcessTimes` (user+kernel, 100ns) → centisecond;
//!   `CpuTracker` (100Hz varsayımı) aynen beslenir (`proc::cpu_time`)
//! - `cmdline`: `QueryFullProcessImageName` (best-effort exe yolu —
//!   macOS "path, not argv" emsali)
//! - `fds_open`: `GetProcessHandleCount` (best-effort)
//! - `io_*`: `GetProcessIoCounters` (gerçek sayaç — stall dedektörünün
//!   IO bacağı Windows'ta da çalışır)
//! - `net_tcp`: `GetExtendedTcpTable` (iphlpapi) ile sahip-pid eşleşen
//!   TCP satır sayısı; `net_udp` = 0 (sahip-eşlemeli UDP tablosu yok,
//!   belgeli eksik)
//! - `is_alive`: `GetExitCodeProcess != STILL_ACTIVE`
//!
//! Her FFI dönüşü kontrol edilir; tek başarısız pid tüm ağacı
//! devirmez (Linux best-effort ilkesi). Handle sızıntısı yok:
//! her `OpenProcess`/snapshot `CloseHandle` ile kapanır.

#[cfg(target_os = "windows")]
mod inner {
    use super::super::{Metrics, ProcessInspector, TreeNode};
    use std::collections::HashMap;

    use crate::platform::winffi;

    pub struct WindowsInspector;
    impl Default for WindowsInspector {
        fn default() -> Self {
            Self::new()
        }
    }
    impl WindowsInspector {
        pub fn new() -> Self {
            Self
        }
    }

    const QUERY: u32 = winffi::PROCESS_QUERY_LIMITED_INFORMATION;
    const QUERY_VM: u32 = winffi::PROCESS_QUERY_LIMITED_INFORMATION | winffi::PROCESS_VM_READ;

    fn open(pid: u32, access: u32) -> Option<winffi::HANDLE> {
        let h = unsafe { winffi::OpenProcess(access, 0, pid) };
        if winffi::valid(h) {
            Some(h)
        } else {
            None
        }
    }

    /// Ham CPU zamanı, centisecond (100ns / 100_000).
    /// `CpuTracker` birimiyle birebir (100Hz jiffies eşdeğeri).
    pub fn cpu_centis(pid: u32) -> Option<u64> {
        let h = open(pid, QUERY)?;
        let mut creation = winffi::FileTime { low: 0, high: 0 };
        let mut exit = winffi::FileTime { low: 0, high: 0 };
        let mut kernel = winffi::FileTime { low: 0, high: 0 };
        let mut user = winffi::FileTime { low: 0, high: 0 };
        let ok =
            unsafe { winffi::GetProcessTimes(h, &mut creation, &mut exit, &mut kernel, &mut user) };
        unsafe {
            winffi::CloseHandle(h);
        }
        if ok == 0 {
            return None;
        }
        Some((kernel.as_u64().saturating_add(user.as_u64())) / 100_000)
    }

    /// Bu pid'in sahip olduğu TCP bağlantı sayısı (TASK-043).
    /// Best-effort: tablo okunamazsa 0.
    pub fn net_tcp_for(pid: u32) -> u32 {
        unsafe {
            let mut size: u32 = 0;
            // Boy öğrenme turu (122 = yetersiz tampon, beklenen).
            winffi::GetExtendedTcpTable(
                std::ptr::null_mut(),
                &mut size,
                0,
                winffi::AF_INET,
                winffi::TCP_TABLE_OWNER_PID_ALL,
                0,
            );
            if size == 0 || size > 64 * 1024 * 1024 {
                return 0;
            }
            let mut buf = vec![0u8; size as usize];
            let mut rc = winffi::GetExtendedTcpTable(
                buf.as_mut_ptr() as *mut std::ffi::c_void,
                &mut size,
                0,
                winffi::AF_INET,
                winffi::TCP_TABLE_OWNER_PID_ALL,
                0,
            );
            if rc == winffi::ERROR_INSUFFICIENT_BUFFER {
                // Tablo arada büyüdü: bir kez yeni boyla dene.
                buf.resize(size as usize, 0);
                rc = winffi::GetExtendedTcpTable(
                    buf.as_mut_ptr() as *mut std::ffi::c_void,
                    &mut size,
                    0,
                    winffi::AF_INET,
                    winffi::TCP_TABLE_OWNER_PID_ALL,
                    0,
                );
            }
            if rc != winffi::NO_ERROR {
                return 0;
            }
            let n = *(buf.as_ptr() as *const u32) as usize;
            let row_sz = std::mem::size_of::<winffi::TcpRowOwnerPid>();
            let max = (size as usize).saturating_sub(4) / row_sz;
            let rows = buf.as_ptr().add(4) as *const winffi::TcpRowOwnerPid;
            (0..n.min(max))
                .filter(|&i| (*rows.add(i)).owning_pid == pid)
                .count() as u32
        }
    }

    /// Job Object yoksa fallback: kök + tüm torunlara `TerminateProcess`
    /// (yaprak-önce). Dönüş: kök öldürülebildi mi?
    pub fn kill_tree(root: u32) -> bool {
        let insp = WindowsInspector;
        let mut pids = crate::proc::collect_descendants(&insp, root, 1000);
        // Yapraklar önce ölsün: BFS sırasını ters çevir (kök en sonda).
        pids.reverse();
        let mut root_ok = false;
        for pid in pids {
            let h = unsafe { winffi::OpenProcess(winffi::PROCESS_TERMINATE, 0, pid) };
            if !winffi::valid(h) {
                continue;
            }
            let ok = unsafe { winffi::TerminateProcess(h, 1) } != 0;
            unsafe {
                winffi::CloseHandle(h);
            }
            if pid == root {
                root_ok = ok;
            }
        }
        root_ok
    }

    impl ProcessInspector for WindowsInspector {
        fn list_children(&self, pid: u32) -> Result<Vec<u32>, String> {
            let snap = unsafe { winffi::CreateToolhelp32Snapshot(winffi::TH32CS_SNAPPROCESS, 0) };
            if !winffi::valid(snap) {
                return Err("snapshot failed".to_string());
            }
            let mut kids = vec![];
            let mut e = winffi::ProcessEntry {
                size: std::mem::size_of::<winffi::ProcessEntry>() as u32,
                usage: 0,
                pid: 0,
                default_heap: 0,
                module_id: 0,
                threads: 0,
                ppid: 0,
                pri_base: 0,
                flags: 0,
                exe: [0; 260],
            };
            let mut rc = unsafe { winffi::Process32FirstW(snap, &mut e) };
            while rc != 0 {
                if e.ppid == pid && e.pid != pid {
                    kids.push(e.pid);
                }
                rc = unsafe { winffi::Process32NextW(snap, &mut e) };
            }
            unsafe {
                winffi::CloseHandle(snap);
            }
            Ok(kids)
        }

        fn cmdline(&self, pid: u32) -> Result<String, String> {
            let h = open(pid, QUERY).ok_or_else(|| format!("open {}", pid))?;
            let mut buf = [0u16; 4096];
            let mut n: u32 = buf.len() as u32;
            let ok = unsafe { winffi::QueryFullProcessImageNameW(h, 0, buf.as_mut_ptr(), &mut n) };
            unsafe {
                winffi::CloseHandle(h);
            }
            if ok == 0 {
                return Err(format!("image name {}", pid));
            }
            let s = String::from_utf16_lossy(&buf[..(n as usize).min(buf.len())]);
            // `\??\C:\…` öneki ayıklanır (kozmetik).
            Ok(s.strip_prefix(r"\??\").unwrap_or(&s).to_string())
        }

        fn metrics(&self, pid: u32) -> Result<Metrics, String> {
            let h = open(pid, QUERY_VM).ok_or_else(|| format!("open {}", pid))?;
            let mut m = Metrics::default();
            unsafe {
                let mut mem: winffi::ProcessMemoryCounters = std::mem::zeroed();
                mem.cb = std::mem::size_of::<winffi::ProcessMemoryCounters>() as u32;
                if winffi::GetProcessMemoryInfo(
                    h,
                    &mut mem,
                    std::mem::size_of::<winffi::ProcessMemoryCounters>() as u32,
                ) != 0
                {
                    m.rss_mb = (mem.working_set / (1024 * 1024)) as u32;
                }
                let mut io: winffi::IoCounters = std::mem::zeroed();
                if winffi::GetProcessIoCounters(h, &mut io) != 0 {
                    m.io_read_bytes = io.read_bytes;
                    m.io_write_bytes = io.write_bytes;
                }
                let mut nfds: u32 = 0;
                if winffi::GetProcessHandleCount(h, &mut nfds) != 0 {
                    m.fds_open = nfds;
                }
                winffi::CloseHandle(h);
            }
            m.net_tcp = net_tcp_for(pid);
            Ok(m)
        }

        fn tree(&self, pid: u32) -> Result<TreeNode, String> {
            super::super::tree::build_tree(self, pid, 8)
        }

        fn is_alive(&self, pid: u32) -> bool {
            let h = match open(pid, QUERY) {
                Some(h) => h,
                None => return false,
            };
            let mut code: u32 = 0;
            let ok = unsafe { winffi::GetExitCodeProcess(h, &mut code) };
            unsafe {
                winffi::CloseHandle(h);
            }
            ok != 0 && code == winffi::STILL_ACTIVE
        }

        fn bulk_metrics(&self, pids: &[u32]) -> Result<HashMap<u32, Metrics>, String> {
            let mut m = HashMap::new();
            for &p in pids {
                if let Ok(met) = self.metrics(p) {
                    m.insert(p, met);
                }
            }
            Ok(m)
        }
    }
}

#[cfg(target_os = "windows")]
pub use inner::{cpu_centis, kill_tree, WindowsInspector};

#[cfg(not(target_os = "windows"))]
mod stub {
    use super::super::{Metrics, ProcessInspector, TreeNode};
    pub struct WindowsInspector;
    impl Default for WindowsInspector {
        fn default() -> Self {
            Self::new()
        }
    }
    impl WindowsInspector {
        pub fn new() -> Self {
            Self
        }
    }
    impl ProcessInspector for WindowsInspector {
        fn list_children(&self, _pid: u32) -> Result<Vec<u32>, String> {
            Err("windows only".to_string())
        }
        fn metrics(&self, _pid: u32) -> Result<Metrics, String> {
            Err("windows only".to_string())
        }
        fn cmdline(&self, _pid: u32) -> Result<String, String> {
            Err("windows only".to_string())
        }
        fn tree(&self, _pid: u32) -> Result<TreeNode, String> {
            Err("windows only".to_string())
        }
        fn is_alive(&self, _pid: u32) -> bool {
            false
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub use stub::WindowsInspector;

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::inner::net_tcp_for;

    #[test]
    fn loopback_tcp_counted_for_self() {
        let me = std::process::id();
        let before = net_tcp_for(me);
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().expect("addr").port();
        let connector = std::thread::spawn(move || {
            std::net::TcpStream::connect(("127.0.0.1", port)).expect("connect")
        });
        let (srv, cli) = (
            listener.accept().expect("accept"),
            connector.join().expect("join"),
        );
        let after = net_tcp_for(me);
        // Listener + accepted + connected: en az 3 yeni satır bizim pid'e.
        assert!(after >= before + 3, "before={} after={}", before, after);
        let _ = (srv, cli);
        // Olmayan pid: panik yok, 0 satır.
        assert_eq!(net_tcp_for(u32::MAX), 0);
    }
}
