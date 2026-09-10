//! Windows re-export (gerçekleşim `crate::proc::windows`).
//!
//! TASK-006 ile tam izleme: Toolhelp süreç ağacı + RSS/IO/handle,
//! Job Object ile grup kill, named-pipe transport (`\\.\pipe\hbmon-<uuid>`).
//! `net` best-effort kalır, cmdline = exe yolu.

#[cfg(target_os = "windows")]
pub use crate::proc::windows::WindowsInspector;
