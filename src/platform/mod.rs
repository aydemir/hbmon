pub mod linux;
pub mod macos;

use crate::proc::linux::LinuxInspector;
use crate::proc::macos::MacosInspector;
use crate::proc::ProcessInspector;

/// cfg-dispatched inspector (RFC Section 10).
pub fn inspector() -> Box<dyn ProcessInspector> {
    #[cfg(target_os = "macos")]
    {
        Box::new(MacosInspector::new())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = MacosInspector::new(); // keep symbol referenced cross-OS
        Box::new(LinuxInspector::new())
    }
}
