//! Windows re-export (gerçekleşim `crate::proc::windows`).
//!
//! M1: stub — derlenir, sorgular hata döner. Windows'ta detach M3'e
//! kadar kapalı olduğu için stub canlıda yoklanmaz.

#[cfg(target_os = "windows")]
pub use crate::proc::windows::WindowsInspector;
