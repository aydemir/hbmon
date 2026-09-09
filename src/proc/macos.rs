//! macOS implementation via libproc FFI (v1: functional but best-effort).
//! Compiled only on macOS; on Linux this module is a stub so the
//! crate still builds cross-platform.

#[cfg(target_os = "macos")]
mod inner {
    use super::super::{Metrics, ProcessInspector, TreeNode};
    use std::collections::HashMap;
    use std::ffi::CStr;

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
    }

    const PROC_ALL_PIDS: u32 = 1;
    const PROC_PIDTASKINFO: i32 = 4;

    #[repr(C)]
    struct ProcTaskInfo {
        _pad: [u8; 128],
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
                let pid = u32::from_ne_bytes([buf[i * 4], buf[i * 4 + 1], buf[i * 4 + 2], buf[i * 4 + 3]]);
                if pid != 0 {
                    out.push(pid);
                }
            }
            out
        }
    }

    fn ppid_of(pid: u32) -> Option<u32> {
        // fallback via ps would be slow; use sysctl-less heuristic:
        // libproc has no direct ppid flavor here, use PROC_PIDTBSDINFO (flavor 3)
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
                3,
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
                let mut buf = vec![0i8; 4096];
                // PROC_PIDPATHINFO flavor=11
                let r = proc_pidinfo(
                    pid as i32,
                    11,
                    0,
                    buf.as_mut_ptr() as *mut _,
                    buf.len() as i32,
                );
                if r > 0 {
                    let c = CStr::from_ptr(buf.as_ptr());
                    return Ok(c.to_string_lossy().to_string());
                }
            }
            Err(format!("no cmdline for {}", pid))
        }
        fn metrics(&self, pid: u32) -> Result<Metrics, String> {
            // Best-effort: resident size via task info would need Mach APIs;
            // v1 returns zeros except alive-ness so monitoring still works.
            if !self.is_alive(pid) {
                return Err(format!("pid {} gone", pid));
            }
            Ok(Metrics::default())
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
