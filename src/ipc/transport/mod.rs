//! IPC transport dispatcher: unix UDS, windows named pipe.
//!
//! `serve` / `send_request` / `can_connect` imzaları platformda aynı;
//! üst katman (`daemon`, `cli`) `#[cfg]` içermez.

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use unix::{can_connect, send_request, serve};

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::{can_connect, send_request, serve};
