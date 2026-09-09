use clap::Parser;
use serde_json::json;
use std::path::PathBuf;

use crate::ipc::send_request;
use crate::util::generate_uuid;

use super::resolve_sock;

#[derive(Debug, Parser)]
pub struct WaitArgs {
    #[arg(long)]
    pub sock: Option<PathBuf>,
    #[arg(long, default_value = "300")]
    pub timeout: f64,
    #[arg(long, default_value = "500")]
    pub poll_ms: u64,
    /// Erken dönüş sinyalleri (virgüllü): done,dep_missing,stall_suspect,oom_suspect.
    /// Yoksa yalnızca bitişte dönülür.
    #[arg(long)]
    pub until: Option<String>,
}

pub fn run(a: WaitArgs) -> Result<i32, String> {
    let sock = resolve_sock(a.sock)?;
    let mut req = json!({
        "v": 1, "op": "wait", "id": generate_uuid(),
        "timeout_sec": a.timeout, "poll_ms": a.poll_ms,
    });
    if let Some(u) = &a.until {
        let list: Vec<&str> = u
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        if !list.is_empty() {
            req["until"] = json!(list);
        }
    }
    // UDS timeout must exceed wait timeout so the daemon (not the socket)
    // decides when to return.
    let resp = send_request(&sock, &req, (a.timeout as u64) + 15)?;
    println!("{}", serde_json::to_string(&resp).unwrap());
    if resp.get("timeout").and_then(|v| v.as_bool()).unwrap_or(false) {
        return Ok(124);
    }
    // Erken dönüşte (ara sinyal) code alanı yoksa exit 0; ajan woke_on'a bakar.
    Ok(resp.get("code").and_then(|v| v.as_i64()).map(|c| c as i32).unwrap_or_else(|| {
        if resp.get("woke_on").is_some() {
            0
        } else {
            1
        }
    }))
}
