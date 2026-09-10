pub mod pidfile;
pub mod signals;

// daemon::daemon path is internal-only; re-exports below keep the
// public path flat (daemon::run_daemon), so allow the inception lint.
#[allow(clippy::module_inception)]
pub mod daemon;
pub use daemon::{run_daemon, spawn_watch, MonitorConfig};
