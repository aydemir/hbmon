use clap::Parser;
use serde_json::json;
use std::path::PathBuf;

use crate::ipc::uds::send_request;
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
}

pub fn run(a: WaitArgs) -> Result<i32, String> {
    let sock = resolve_sock(a.sock)?;
    let req = json!({
        "v": 1, "op": "wait", "id": generate_uuid(),
        "timeout_sec": a.timeout, "poll_ms": a.poll_ms,
    });
    // UDS timeout must exceed wait timeout so the daemon (not the socket)
    // decides when to return.
    let resp = send_request(&sock, &req, (a.timeout as u64) + 15)?;
    println!("{}", serde_json::to_string(&resp).unwrap());
    if resp.get("timeout").and_then(|v| v.as_bool()).unwrap_or(false) {
        return Ok(124);
    }
    Ok(resp.get("code").and_then(|v| v.as_i64()).unwrap_or(1) as i32)
}
