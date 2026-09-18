//! hbmon exec: foreground build with handshake on stdout line 1.
//! LLM parses line 1 (JSON ready), rest is raw build output.
//! Exit code follows RFC 5.2.3 mapping (0/1/2) + 124 on `--timeout-sec`.
//! Runtime capabilities: no daemon, no UDS, no JSONL — stdout and stderr
//! are tee'd; both streams are scanned for dep-missing patterns
//! (TASK-048/S3b: parity with `watch`; see PROTOCOL.md).

use clap::Parser;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

use crate::health::dep_missing;
use crate::platform::paths;
use crate::util::generate_uuid;

/// Tarama için satır üst sınırı (TASK-048/S3b): daha uzun satır taranmaz,
/// yine de bayt-aynı yönlendirilir (bellek sınırlı kalır).
const MAX_SCAN_LINE: usize = 64 * 1024;
/// Tee thread'lerinin sonucunu toplama sınırı (ms). Aşarsa exec asılı
/// kalmaz (grandchild pipe'ı açık tutuyorsa).
const TEE_GATHER_MS: u64 = 500;

#[derive(Debug, Parser)]
pub struct ExecArgs {
    /// Kill the build after N seconds: TERM → 5 s grace → KILL, exit 124.
    /// 0 = watchdog off (default). Kills the direct child only; for
    /// process-group kill use `watch`.
    #[arg(long)]
    pub timeout_sec: Option<u64>,
    /// Summary format on stderr (last line): human or json
    #[arg(long, default_value = "human")]
    pub format: String,
    #[arg(last = true)]
    pub cmd: Vec<String>,
}

pub fn run(a: ExecArgs) -> Result<i32, String> {
    if a.cmd.is_empty() {
        return Err("usage: hbmon exec -- <cmd> [args...]".to_string());
    }
    let uuid = generate_uuid();
    let sock = paths::default_sock(&uuid);
    let log = paths::default_log(&uuid);
    // handshake FIRST so LLM can grab it even if build floods output.
    // serde_json ile serialize edilir: Windows pipe yolu (`\\.\pipe\…`)
    // tersbölüleri manuel format ile geçersiz JSON üretirdi.
    // Ephemeral contract (TASK-007): no daemon is spawned, so no socket or
    // JSONL is created; status/wait are unavailable. sock/log are reserved
    // names only, kept for forward-compatibility.
    println!(
        "{}",
        serde_json::json!({
            "v": 1, "ev": "ready", "uuid": uuid,
            "sock": paths::sock_display(&sock),
            "log": log.to_string_lossy(),
            "ephemeral": true,
            "note": "no daemon; status/wait unavailable",
        })
    );
    std::io::stdout().flush().ok();
    let start = std::time::Instant::now();

    let mut child = Command::new(&a.cmd[0])
        .args(&a.cmd[1..])
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("spawn {}: {}", a.cmd[0], e))?;

    // stdout tee + dep-scan ayrı thread'te (TASK-048/S3b): bayt-aynı
    // stdout'a yönlendirilir, taranmaz; dep taraması için satır
    // üst sınırıyla sınırlı dep taraması yapılır. Ana döngü
    // watchdog'u zamanında uygulayabilmek için bloklanmamalı.
    let (dep_tx, dep_rx) = std::sync::mpsc::channel::<Option<(String, String)>>();
    if let Some(out) = child.stdout.take() {
        let stdout = std::io::stdout();
        std::thread::spawn(move || {
            let reader = BufReader::new(out);
            let mut stdout = stdout.lock();
            let mut hit: Option<(String, String)> = None;
            for line in reader.lines() {
                let Ok(line) = line else { continue };
                let _ = writeln!(stdout, "{}", line);
                if hit.is_none() {
                    let line = if line.len() > MAX_SCAN_LINE {
                        let mut truncated =
                            line[..line.floor_char_boundary(MAX_SCAN_LINE)].to_string();
                        truncated.push('…');
                        truncated
                    } else {
                        line
                    };
                    if let Some(m) = dep_missing::match_line(&line) {
                        hit = Some((m.pattern_id, m.category));
                    }
                }
            }
            let _ = stdout.flush();
            let _ = dep_tx.send(hit);
        });
    }

    // stderr tee + dep-scan ayrı thread'te (TASK-047/S3a): ana döngü
    // watchdog'u zamanında uygulayabilmek için bloklanmamalı. Sonuç
    // `recv_timeout` ile sınırlı toplanır — grandchild stderr'i açık
    // tutsa bile exec asılı kalmaz.
    let (stderr_dep_tx, stderr_dep_rx) = std::sync::mpsc::channel::<Option<(String, String)>>();
    if let Some(err) = child.stderr.take() {
        std::thread::spawn(move || {
            let reader = BufReader::new(err);
            let mut hit: Option<(String, String)> = None;
            for line in reader.lines() {
                let Ok(line) = line else { continue };
                eprintln!("{}", line);
                let line = if line.len() > MAX_SCAN_LINE {
                    let mut truncated = line[..line.floor_char_boundary(MAX_SCAN_LINE)].to_string();
                    truncated.push('…');
                    truncated
                } else {
                    line
                };
                if hit.is_none() {
                    if let Some(m) = dep_missing::match_line(&line) {
                        hit = Some((m.pattern_id, m.category));
                    }
                }
            }
            let _ = stderr_dep_tx.send(hit);
        });
    }

    // Watchdog (TASK-047/S3a): `--timeout-sec 0`/yok = kapalı.
    // Kapalıyken doğrudan `wait()` — poll gecikmesi eklenmez; açıkken
    // 100 ms aralıkla yoklanır ve deadline'da TERM → grace → KILL.
    let deadline = a
        .timeout_sec
        .filter(|s| *s > 0)
        .map(|s| std::time::Instant::now() + std::time::Duration::from_secs(s));
    let (timed_out, raw) = match deadline {
        None => {
            let st = child.wait().map_err(|e| e.to_string())?;
            (false, st.code().unwrap_or(1))
        }
        Some(dl) => loop {
            if let Some(st) = child.try_wait().map_err(|e| e.to_string())? {
                break (false, st.code().unwrap_or(1));
            }
            if std::time::Instant::now() >= dl {
                stop_child(&mut child);
                let raw = child
                    .try_wait()
                    .ok()
                    .flatten()
                    .and_then(|s| s.code())
                    .unwrap_or(1);
                break (true, raw);
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        },
    };
    let dur = start.elapsed().as_secs_f64();
    // Hem stdout hem stderr thread'lerinden dep taraması
    // (TASK-048/S3b): ilk bulunandan öne geçilir.
    let dep_info = dep_rx
        .recv_timeout(std::time::Duration::from_millis(TEE_GATHER_MS))
        .ok()
        .flatten()
        .or_else(|| {
            stderr_dep_rx
                .recv_timeout(std::time::Duration::from_millis(TEE_GATHER_MS))
                .ok()
                .flatten()
        });
    // Timeout öncelikli: dep satırı görülüp build asılsa da exit 124.
    let (state, code) = if timed_out {
        ("timeout", 124)
    } else if dep_info.is_some() {
        ("dep_missing", 2)
    } else if raw == 0 {
        ("done", 0)
    } else {
        ("failed", 1)
    };
    if a.format == "json" {
        eprintln!(
            "{}",
            serde_json::json!({
                "v": 1, "ev": "exit", "uuid": uuid,
                "code": code, "raw_code": raw,
                "duration_sec": dur, "state": state,
            })
        );
    } else if let Some((pat, cat)) = dep_info {
        eprintln!(
            "hbmon: dep_missing hint pattern={} category={} (exit 2)",
            pat, cat
        );
    } else {
        eprintln!("hbmon: {} in {:.1}s (exit {})", state, dur, code);
    }
    Ok(code)
}

/// Watchdog durdurma (TASK-047/S3a): doğrudan çocuğa TERM → 5 s grace →
/// KILL. Grup kill bilinçli olarak YOK: `exec` stdin'i devralır
/// (etkileşimli Ctrl-C semantiği korunur), bu yüzden `setpgid`
/// kullanılmaz; torunlar kendiliğinden biter (grup için `watch`).
fn stop_child(child: &mut std::process::Child) {
    #[cfg(unix)]
    {
        unsafe {
            libc::kill(child.id() as i32, libc::SIGTERM);
        }
        let grace = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while std::time::Instant::now() < grace {
            if matches!(child.try_wait(), Ok(Some(_))) {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
    let _ = child.kill();
    let _ = child.wait();
}
