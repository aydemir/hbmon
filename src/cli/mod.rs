pub mod exec;
pub mod kill;
pub mod list;
pub mod log;
pub mod shutdown;
pub mod status;
pub mod wait;
pub mod watch;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::platform::paths::{self, SockAddr};

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
    /// List known monitors (read-only discovery)
    List(list::ListArgs),
    /// Show recent event log lines (read-only)
    Log(log::LogArgs),
    /// Remove stale pid/sock files
    Cleanup(CleanupArgs),
}

#[derive(Debug, Parser)]
pub struct CleanupArgs {
    /// Remove files older than N seconds (default 86400)
    #[arg(long, default_value = "86400")]
    pub older_than: u64,
    /// Taranacak dizin (varsayılan: platform convention — unix /tmp).
    /// `list --dir` ile aynı kök.
    #[arg(long)]
    pub dir: Option<PathBuf>,
}

pub fn dispatch(cli: Cli) -> Result<i32, String> {
    match cli.cmd {
        Commands::Watch(a) => watch::run(a),
        Commands::Status(a) => status::run(a),
        Commands::Wait(a) => wait::run(a),
        Commands::Exec(a) => exec::run(a),
        Commands::Kill(a) => kill::run(a),
        Commands::Shutdown(a) => shutdown::run(a),
        Commands::List(a) => list::run(a),
        Commands::Log(a) => log::run(a),
        Commands::Cleanup(a) => run_cleanup(a),
    }
}

/// Resolve --sock > $HBMON_SOCK > newest convention socket (RFC Katman 3).
pub fn resolve_sock(explicit: Option<PathBuf>) -> Result<SockAddr, String> {
    if let Some(p) = explicit {
        return Ok(paths::from_explicit(p));
    }
    if let Ok(e) = std::env::var("HBMON_SOCK") {
        if !e.is_empty() {
            return Ok(paths::from_env(&e));
        }
    }
    // convention scan: newest sock (unix) / pipe (windows).
    paths::scan_newest_sock()
        .ok_or_else(|| "no monitor found: pass --sock or set HBMON_SOCK".to_string())
}

fn run_cleanup(a: CleanupArgs) -> Result<i32, String> {
    let root = a.dir.unwrap_or_else(paths::scan_dir);
    let now = std::time::SystemTime::now();
    let entries: Vec<(String, PathBuf)> = std::fs::read_dir(&root)
        .map(|d| {
            d.flatten()
                .map(|e| (e.file_name().to_string_lossy().to_string(), e.path()))
                .filter(|(n, _)| {
                    n.starts_with("hbmon-")
                        && (n.ends_with(".sock")
                            || n.ends_with(".pid")
                            || n.ends_with(".jsonl")
                            || n.ends_with(".out"))
                })
                .collect()
        })
        .unwrap_or_default();
    // Canlı guard (TASK-022 + TASK-040): sock'u/pipe'ı dinlenen daemon'un
    // kardeş dosyalarına (.jsonl/.out/.pid) dokunma — yaşa bakılmaksızın.
    let live: std::collections::HashSet<String> = live_uuids(&entries);
    let mut removed = 0u32;
    for (name, path) in &entries {
        let age_ok = std::fs::metadata(path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|mt| now.duration_since(mt).ok())
            .map(|d| d.as_secs() > a.older_than)
            .unwrap_or(false);
        if age_ok && sweep_decision(name, &live) {
            std::fs::remove_file(path).ok();
            removed += 1;
        }
    }
    println!("{{\"removed\":{}}}", removed);
    Ok(0)
}

/// Canlı daemon uuid'leri (TASK-022 guard, TASK-040 Windows kolu).
/// Unix: `.sock` girdisine bağlanılabilenler. Windows: pipe dosya
/// değildir, `%TEMP%`'te `.sock` oluşmaz → her aday uuid'nin pipe'ı
/// (`default_sock`) yoklanır; yoksa guard ölü kalırdı.
fn live_uuids(entries: &[(String, PathBuf)]) -> std::collections::HashSet<String> {
    entries
        .iter()
        .filter_map(|(name, path)| {
            let uuid = paths::uuid_from_base(name)?;
            let addr = live_probe_addr(&uuid, path)?;
            if crate::ipc::can_connect(&addr) {
                Some(uuid)
            } else {
                None
            }
        })
        .collect()
}

/// Canlılık yoklaması için adres: unix'te SADECE `.sock` dosyasının
/// kendisi (`.jsonl`/`.pid` yoklanırsa yanlış-aile eşleşmesi olur);
/// Windows'ta uuid'den türetilen pipe adı.
fn live_probe_addr(uuid: &str, path: &std::path::Path) -> Option<paths::SockAddr> {
    #[cfg(unix)]
    {
        let _ = uuid;
        let is_sock = path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.ends_with(".sock"));
        if !is_sock {
            return None;
        }
        Some(paths::from_explicit(path.to_path_buf()))
    }
    #[cfg(windows)]
    {
        let _ = path;
        Some(paths::default_sock(uuid))
    }
}
/// Silme kararı (TASK-022, saf mantık — birim testli): canlı ailenin
/// dosyaları yaşa bakılmaksızın korunur; diğerleri yaş kuralına kalır.
fn sweep_decision(name: &str, live: &std::collections::HashSet<String>) -> bool {
    if let Some(u) = paths::uuid_from_base(name) {
        if live.contains(&u) {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn live_set(names: &[&str]) -> std::collections::HashSet<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn live_family_protected_despite_age() {
        let live = live_set(&["abc123"]);
        assert!(!sweep_decision("hbmon-abc123.sock", &live));
        assert!(!sweep_decision("hbmon-abc123.jsonl", &live));
        assert!(!sweep_decision("hbmon-abc123.out", &live));
        assert!(!sweep_decision("hbmon-abc123.pid", &live));
    }

    #[test]
    fn dead_files_not_protected() {
        // Yaş kuralı caller'da (`age_ok`); burası yalnız aile-koruma kararı:
        // ölü uuid'ler korunmaz.
        let live = live_set(&["abc123"]);
        assert!(sweep_decision("hbmon-dead-x.sock", &live));
        assert!(sweep_decision("hbmon-dead-x.jsonl", &live));
    }

    #[test]
    fn empty_live_set_sweeps_all() {
        let live = live_set(&[]);
        assert!(sweep_decision("hbmon-abc123.sock", &live));
    }

    #[test]
    fn live_probe_addr_platform_rules() {
        use std::path::PathBuf;
        // .sock her platformda yoklanır; .jsonl/.pid unix'te yoklanmaz
        // (yanlış-aile eşleşmesi), Windows'ta uuid→pipe türetilir.
        let sock = PathBuf::from("hbmon-abc123.sock");
        assert!(super::live_probe_addr("abc123", &sock).is_some());
        let log = PathBuf::from("hbmon-abc123.jsonl");
        #[cfg(unix)]
        assert!(super::live_probe_addr("abc123", &log).is_none());
        #[cfg(windows)]
        {
            let addr = super::live_probe_addr("abc123", &log).expect("pipe türetilir");
            assert_eq!(
                crate::platform::paths::sock_display(&addr),
                r"\\.\pipe\hbmon-abc123"
            );
        }
    }
}
