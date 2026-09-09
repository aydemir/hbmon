pub mod detach;
pub mod linux;
pub mod macos;
pub mod paths;
pub mod perm;
pub mod signal;
pub mod windows;
#[cfg(windows)]
pub mod winffi;

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
use crate::proc::linux::LinuxInspector;
#[cfg(not(target_os = "windows"))]
use crate::proc::macos::MacosInspector;
#[cfg(target_os = "windows")]
use crate::proc::windows::WindowsInspector;
use crate::proc::ProcessInspector;

/// cfg-dispatched inspector (RFC Section 10).
pub fn inspector() -> Box<dyn ProcessInspector> {
    #[cfg(target_os = "macos")]
    {
        Box::new(MacosInspector::new())
    }
    #[cfg(target_os = "windows")]
    {
        Box::new(WindowsInspector::new())
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = MacosInspector::new(); // keep symbol referenced cross-OS
        Box::new(LinuxInspector::new())
    }
}
