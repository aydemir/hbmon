//! macOS implementation via libproc (v1.1: RSS + executable path + fd
//! count + process tree; CPU% still best-effort/0).
//!
//! Only safe, well-established entry points are used:
//! - `proc_pidpath` (executable path, no flavor guessing)
//! - `PROC_PIDTASKINFO` (resident size = first 16 bytes; time fields
//!   deliberately ignored — units undocumented, garbage CPU% would be
//!   worse than honest 0.0 for the stall detector)
//! - `proc_listpids(PROC_PIDLISTFDS, pid, NULL, 0)` (fd count trick)
//! Every query falls back gracefully so a wrong struct size on a future
//! macOS can never crash the daemon — it just reports zeros again.

#[cfg(target_os = "macos")]
mod inner {
    use super::super::{Metrics, ProcessInspector, TreeNode};
    use std::collections::HashMap;

    pub struct MacosInspector;
    impl MacosInspector {
        pub fn new() -> Self {
            Self
        }
    }

    extern "C" {
        fn proc_listpids(t: u32, typeinfo: u32, buffer: *mut u8, buffersize: i32) -> i32;
        fn proc_pidinfo(
            pid: i32,
            flavor: i32,
            arg: u64,
            buffer: *mut std::ffi::c_void,
            buffersize: i32,
        ) -> i32;
        fn proc_pidpath(pid: i32, buffer: *mut u8, buffersize: u32) -> i32;
    }

    const PROC_ALL_PIDS: u32 = 1;
    const PROC_PIDLISTFDS: u32 = 1;
    const PROC_PIDTASKINFO: i32 = 4;
    const PROC_PIDTBSDINFO: i32 = 3;

    /// Full proc_taskinfo layout (96 bytes). If a future macOS changes
    /// it, proc_pidinfo errors out and we fall back to zeros — never panic.
    #[repr(C)]
    struct ProcTaskInfo {
        virtual_size: u64,
        resident_size: u64,
        _total_user: u64,
        _total_system: u64,
        _threads_user: u64,
        _threads_system: u64,
        _rest: [i32; 12],
    }

    fn all_pids() -> Vec<u32> {
        unsafe {
            let mut buf = vec![0u8; 4096 * 4];
            let n = proc_listpids(PROC_ALL_PIDS, 0, buf.as_mut_ptr(), buf.len() as i32);
            if n <= 0 {
                return vec![];
            }
            let count = n as usize / 4;
            let mut out = vec![];
            for i in 0..count {
                let pid = u32::from_ne_bytes([
                    buf[i * 4],
                    buf[i * 4 + 1],
                    buf[i * 4 + 2],
                    buf[i * 4 + 3],
                ]);
                if pid != 0 {
                    out.push(pid);
                }
            }
            out
        }
    }

    fn ppid_of(pid: u32) -> Option<u32> {
        unsafe {
            #[repr(C)]
            struct BsdInfo {
                _pad1: [u8; 16],
                ppid: u32,
                _rest: [u8; 512],
            }
            let mut info: BsdInfo = std::mem::zeroed();
            let r = proc_pidinfo(
                pid as i32,
                PROC_PIDTBSDINFO,
                0,
                &mut info as *mut _ as *mut _,
                std::mem::size_of::<BsdInfo>() as i32,
            );
            if r > 0 {
                Some(info.ppid)
            } else {
                None
            }
        }
    }

    impl ProcessInspector for MacosInspector {
        fn list_children(&self, pid: u32) -> Result<Vec<u32>, String> {
            Ok(all_pids()
                .into_iter()
                .filter(|&c| ppid_of(c) == Some(pid))
                .collect())
        }

        fn cmdline(&self, pid: u32) -> Result<String, String> {
            unsafe {
                let mut buf = vec![0u8; 4096];
                let r = proc_pidpath(pid as i32, buf.as_mut_ptr(), buf.len() as u32);
                if r > 0 {
                    let n = (r as usize).min(buf.len());
                    let s = String::from_utf8_lossy(&buf[..n]);
                    let s = s.trim_matches('\0').trim();
                    if !s.is_empty() {
                        // NOTE: executable path, not full argv (best-effort).
                        return Ok(s.to_string());
                    }
                }
            }
            Err(format!("no path for {}", pid))
        }

        fn metrics(&self, pid: u32) -> Result<Metrics, String> {
            if !self.is_alive(pid) {
                return Err(format!("pid {} gone", pid));
            }
            let mut m = Metrics::default();
            unsafe {
                let mut info: ProcTaskInfo = std::mem::zeroed();
                let r = proc_pidinfo(
                    pid as i32,
                    PROC_PIDTASKINFO,
                    0,
                    &mut info as *mut _ as *mut _,
                    std::mem::size_of::<ProcTaskInfo>() as i32,
                );
                // Only trust resident_size if at least the first 16
                // bytes were filled.
                if r >= 16 {
                    m.rss_mb = (info.resident_size / (1024 * 1024)) as u32;
                }
                // fd count without any buffer: returns count directly.
                let nfds = proc_listpids(
                    PROC_PIDLISTFDS,
                    pid,
                    std::ptr::null_mut(),
                    0,
                );
                if nfds > 0 {
                    m.fds_open = nfds as u32;
                }
            }
            Ok(m)
        }

        fn tree(&self, pid: u32) -> Result<TreeNode, String> {
            super::super::tree::build_tree(self, pid, 8)
        }

        fn is_alive(&self, pid: u32) -> bool {
            unsafe { libc::kill(pid as i32, 0) == 0 }
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

#[cfg(target_os = "macos")]
pub use inner::MacosInspector;

#[cfg(not(target_os = "macos"))]
mod stub {
    use super::super::{Metrics, ProcessInspector, TreeNode};
    pub struct MacosInspector;
    impl MacosInspector {
        pub fn new() -> Self {
            Self
        }
    }
    impl ProcessInspector for MacosInspector {
        fn list_children(&self, _pid: u32) -> Result<Vec<u32>, String> {
            Err("macos only".to_string())
        }
        fn metrics(&self, _pid: u32) -> Result<Metrics, String> {
            Err("macos only".to_string())
        }
        fn cmdline(&self, _pid: u32) -> Result<String, String> {
            Err("macos only".to_string())
        }
        fn tree(&self, _pid: u32) -> Result<TreeNode, String> {
            Err("macos only".to_string())
        }
        fn is_alive(&self, _pid: u32) -> bool {
            false
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub use stub::MacosInspector;
