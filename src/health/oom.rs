//! Best-effort OOM-killer detection (RFC 7.3, v1: dmesg only).
//! Reads `dmesg` tail and matches "Killed process <pid>".
//! In containers without privileges this returns empty — never fatal.

use std::collections::HashSet;

pub struct OomEvent {
    pub pid: u32,
    pub text: String,
}

pub fn check(watched: &HashSet<u32>) -> Vec<OomEvent> {
    if watched.is_empty() {
        return vec![];
    }
    let out = std::process::Command::new("dmesg")
        .arg("--time-format=iso")
        .arg("-l")
        .arg("err,warn")
        .output();
    let text = match out {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => {
            // fallback: kern.log tail (best effort, may not exist)
            std::fs::read_to_string("/var/log/kern.log").unwrap_or_default()
        }
    };
    if text.is_empty() {
        return vec![];
    }
    let mut hits = vec![];
    for line in text.lines().rev().take(200) {
        let low = line.to_lowercase();
        if low.contains("out of memory") || low.contains("oom-kill") || low.contains("killed process") {
            if let Some(pid) = extract_pid(line) {
                if watched.contains(&pid) {
                    hits.push(OomEvent {
                        pid,
                        text: line.to_string(),
                    });
                }
            } else {
                // OOM line but pid unparseable and we watch few pids:
                // still report if the killed comm looks relevant? v1: skip.
            }
        }
    }
    hits
}

fn extract_pid(line: &str) -> Option<u32> {
    // patterns: "Killed process 1234 (cc)", "pid 1234,"
    for (i, w) in line.split_whitespace().enumerate() {
        if w == "process" {
            if let Some(next) = line.split_whitespace().nth(i + 1) {
                if let Ok(p) = next.trim_matches(|c: char| !c.is_ascii_digit()).parse::<u32>() {
                    if p > 0 {
                        return Some(p);
                    }
                }
            }
        }
    }
    None
}
