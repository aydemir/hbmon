use clap::Parser;
use std::path::PathBuf;

use crate::daemon::{spawn_watch, MonitorConfig};
use crate::util::generate_uuid;

#[derive(Debug, Parser)]
pub struct WatchArgs {
    /// Monitor id (generated if absent)
    #[arg(long)]
    pub uuid: Option<String>,
    /// Socket path (default: platform convention, see platform::paths)
    #[arg(long)]
    pub sock: Option<PathBuf>,
    /// Event log path (default: platform convention, see platform::paths)
    #[arg(long)]
    pub log: Option<PathBuf>,
    /// Parent pid hint (validation/logging only; daemon is detached anyway)
    #[arg(long)]
    pub pid: Option<u32>,
    /// Detach into background daemon (setsid + double fork)
    #[arg(long, default_value = "false")]
    pub detach: bool,
    /// Kill build after N seconds (SIGTERM, 5s grace, SIGKILL)
    #[arg(long)]
    pub timeout_sec: Option<u64>,
    /// Human label (future multi-build use)
    #[arg(long)]
    pub label: Option<String>,
    /// Build command after `--`
    #[arg(last = true)]
    pub cmd: Vec<String>,
}

pub fn run(a: WatchArgs) -> Result<i32, String> {
    if a.cmd.is_empty() {
        return Err("usage: hbmon watch [--detach] -- <cmd> [args...]".to_string());
    }
    let uuid = a.uuid.unwrap_or_else(generate_uuid);
    let cfg = MonitorConfig::new(uuid, a.sock, a.log, a.cmd, a.timeout_sec, a.label);
    if a.detach {
        // spawn_watch double-forks; the caller (LLM shell) returns immediately.
        // Print handshake BEFORE forking so the LLM captures uuid/sock/log.
        // serde_json ile serialize edilir: Windows pipe yolu tersbölüleri
        // manuel format ile geçersiz JSON üretirdi (bkz. exec.rs).
        println!(
            "{}",
            serde_json::json!({
                "v": 1, "ev": "ready", "uuid": cfg.uuid,
                "sock": crate::platform::paths::sock_display(&cfg.sock),
                "log": cfg.log.to_string_lossy(),
            })
        );
        use std::io::Write;
        let _ = std::io::stdout().flush();
        // Small delay to let stdout flush through harness pipes before fork
        std::thread::sleep(std::time::Duration::from_millis(50));
        spawn_watch(cfg, true)?;
        Ok(0)
    } else {
        // foreground: run monitor inline (blocks until build exit + linger)
        spawn_watch(cfg, false)?;
        Ok(0)
    }
}
