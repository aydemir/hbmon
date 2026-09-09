pub mod exec;
pub mod kill;
pub mod shutdown;
pub mod status;
pub mod wait;
pub mod watch;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// hbmon — harness-independent build monitor (RFC v0.1)
#[derive(Debug, Parser)]
#[command(name = "hbmon", version, about = "Harness-independent build monitor")]
pub struct Cli {
    #[command(subcommand)]
    pub cmd: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Spawn a monitored build (daemonizes with --detach)
    Watch(watch::WatchArgs),
    /// Query monitor status (UDS)
    Status(status::StatusArgs),
    /// Block until build finishes (UDS wait)
    Wait(wait::WaitArgs),
    /// Run build in foreground with handshake on stdout line 1
    Exec(exec::ExecArgs),
    /// Kill the build process group (UDS or pid)
    Kill(kill::KillArgs),
    /// Shut the daemon down
    Shutdown(shutdown::ShutdownArgs),
    /// Remove stale pid/sock files
    Cleanup(CleanupArgs),
}

#[derive(Debug, Parser)]
pub struct CleanupArgs {
    /// Remove files older than N seconds (default 86400)
    #[arg(long, default_value = "86400")]
    pub older_than: u64,
}

pub fn dispatch(cli: Cli) -> Result<i32, String> {
    match cli.cmd {
        Commands::Watch(a) => watch::run(a),
        Commands::Status(a) => status::run(a),
        Commands::Wait(a) => wait::run(a),
        Commands::Exec(a) => exec::run(a),
        Commands::Kill(a) => kill::run(a),
        Commands::Shutdown(a) => shutdown::run(a),
        Commands::Cleanup(a) => run_cleanup(a),
    }
}

/// Resolve --sock > $HBMON_SOCK > newest /tmp/hbmon-*.sock (RFC Katman 3).
pub fn resolve_sock(explicit: Option<PathBuf>) -> Result<PathBuf, String> {
    if let Some(p) = explicit {
        return Ok(p);
    }
    if let Ok(e) = std::env::var("HBMON_SOCK") {
        if !e.is_empty() {
            return Ok(PathBuf::from(e));
        }
    }
    // convention scan: newest sock
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    if let Ok(dir) = std::fs::read_dir("/tmp") {
        for e in dir.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with("hbmon-") && name.ends_with(".sock") {
                let p = e.path();
                let mtime = e.metadata().and_then(|m| m.modified()).ok();
                if let Some(mt) = mtime {
                    if best.as_ref().map(|(t, _)| mt > *t).unwrap_or(true) {
                        best = Some((mt, p));
                    }
                }
            }
        }
    }
    best.map(|(_, p)| p)
        .ok_or_else(|| "no monitor found: pass --sock or set HBMON_SOCK".to_string())
}

fn run_cleanup(a: CleanupArgs) -> Result<i32, String> {
    let now = std::time::SystemTime::now();
    let mut removed = 0u32;
    if let Ok(dir) = std::fs::read_dir("/tmp") {
        for e in dir.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if !(name.starts_with("hbmon-")
                && (name.ends_with(".sock")
                    || name.ends_with(".pid")
                    || name.ends_with(".jsonl")
                    || name.ends_with(".out")))
            {
                continue;
            }
            let age_ok = e
                .metadata()
                .and_then(|m| m.modified())
                .ok()
                .and_then(|mt| now.duration_since(mt).ok())
                .map(|d| d.as_secs() > a.older_than)
                .unwrap_or(false);
            // sockets: only remove if nobody listens (stale)
            if name.ends_with(".sock")
                && std::os::unix::net::UnixStream::connect(e.path()).is_ok()
            {
                continue;
            }
            if age_ok {
                std::fs::remove_file(e.path()).ok();
                removed += 1;
            }
        }
    }
    println!("{{\"removed\":{}}}", removed);
    Ok(0)
}
