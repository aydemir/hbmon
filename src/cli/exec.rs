//! hbmon exec: foreground build with handshake on stdout line 1.
//! LLM parses line 1 (JSON ready), rest is raw build output.
//! Exit code follows RFC 5.2.3 mapping (0/1/2).

use clap::Parser;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

use crate::health::dep_missing;
use crate::util::generate_uuid;

#[derive(Debug, Parser)]
pub struct ExecArgs {
    #[arg(long)]
    pub timeout_sec: Option<u64>,
    #[arg(last = true)]
    pub cmd: Vec<String>,
}

pub fn run(a: ExecArgs) -> Result<i32, String> {
    if a.cmd.is_empty() {
        return Err("usage: hbmon exec -- <cmd> [args...]".to_string());
    }
    let uuid = generate_uuid();
    let sock = format!("/tmp/hbmon-{}.sock", uuid);
    let log = format!("/tmp/hbmon-{}.jsonl", uuid);
    // handshake FIRST so LLM can grab it even if build floods output
    println!(
        "{{\"v\":1,\"ev\":\"ready\",\"uuid\":\"{}\",\"sock\":\"{}\",\"log\":\"{}\"}}",
        uuid, sock, log
    );
    std::io::stdout().flush().ok();

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
        for line in reader.lines().flatten() {
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
    if dep_hit.is_some() {
        if let Some((pid, cat)) = dep_hit {
            eprintln!(
                "hbmon: dep_missing hint pattern={} category={} (exit 2)",
                pid, cat
            );
        }
        return Ok(2);
    }
    Ok(if raw == 0 { 0 } else { 1 })
}
