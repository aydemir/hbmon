//! hbmon exec: foreground build with handshake on stdout line 1.
//! LLM parses line 1 (JSON ready), rest is raw build output.
//! Exit code follows RFC 5.2.3 mapping (0/1/2).

use clap::Parser;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

use crate::health::dep_missing;
use crate::platform::paths;
use crate::util::generate_uuid;

#[derive(Debug, Parser)]
pub struct ExecArgs {
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
        .stdout(Stdio::inherit())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("spawn {}: {}", a.cmd[0], e))?;

    // tee stderr: forward to our stderr + dep-scan
    let mut dep_hit: Option<(String, String)> = None;
    if let Some(err) = child.stderr.take() {
        let reader = BufReader::new(err);
        let stderr = std::io::stderr();
        for line in reader.lines() {
            let Ok(line) = line else { continue };
            eprintln!("{}", line);
            if dep_hit.is_none() {
                if let Some(m) = dep_missing::match_line(&line) {
                    dep_hit = Some((m.pattern_id, m.category));
                }
            }
        }
        let _ = stderr;
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    let raw = status.code().unwrap_or(1);
    let dur = start.elapsed().as_secs_f64();
    let dep_info = dep_hit;
    let (state, code) = if dep_info.is_some() {
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
