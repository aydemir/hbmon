use clap::Parser;
use std::path::PathBuf;

#[cfg(windows)]
use crate::daemon::run_daemon;
use crate::daemon::{spawn_watch, MonitorConfig};
use crate::util::{generate_uuid, validate_uuid};

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
    /// Parent pid (sets Shared.parent_pid; ready event includes it).
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
    /// Cap build output log at N megabytes (opt-in; default unbounded).
    /// Exceeded → keep last half, dep-scan offset resets (TASK-024).
    #[arg(long)]
    pub max_log_mb: Option<u64>,
    /// Build command after `--`
    #[arg(last = true)]
    pub cmd: Vec<String>,
}

pub fn run(a: WatchArgs) -> Result<i32, String> {
    if a.cmd.is_empty() {
        return Err("usage: hbmon watch [--detach] -- <cmd> [args...]".to_string());
    }
    if let Some(ref u) = a.uuid {
        validate_uuid(u)?;
    }
    let uuid = a.uuid.unwrap_or_else(generate_uuid);
    let mut cfg = MonitorConfig::new(uuid, a.sock, a.log, a.cmd, a.timeout_sec, a.label, a.pid);
    cfg.max_log_bytes = a.max_log_mb.map(|m| m.saturating_mul(1024 * 1024));
    // TASK-054: Windows detach re-exec'i `--detach`'siz gelir; çocuk
    // `detach=false` kolundan `linger=false` ile koşar, pipe build bitince
    // kapanırdı (7 test: "daemon never came up" / broken pipe 109).
    // Linger kararı bu iç env ile taşınır (değer = uuid; CLI yüzeyi yok).
    // Parent handshake'i zaten bastı → çocuk basmaz, re-detach yapmaz,
    // doğrudan 60 s linger'lı daemon'u koşar. Build env'e sızmasın diye
    // var hemen silinir (`run_daemon` build'i inherit ile spawn'lar).
    #[cfg(windows)]
    if std::env::var("HBMON_DETACHED_CHILD")
        .map(|v| v == cfg.uuid)
        .unwrap_or(false)
    {
        std::env::remove_var("HBMON_DETACHED_CHILD");
        run_daemon(cfg, true)?;
        return Ok(0);
    }
    // Handshake'ten ÖNCE ön-uçuş (TASK-047/S1): var olan yol socket
    // değilse "ready" basıp sonra patlamak yerine hemen exit 3.
    if let Some(msg) = crate::platform::paths::sock_non_socket(&cfg.sock) {
        return Err(msg);
    }
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
