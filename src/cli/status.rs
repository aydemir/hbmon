use clap::Parser;
use serde_json::json;
use std::path::PathBuf;

use crate::ipc::send_request;
use crate::util::generate_uuid;

use super::resolve_sock;

#[derive(Debug, Parser)]
pub struct StatusArgs {
    #[arg(long)]
    pub sock: Option<PathBuf>,
    #[arg(long, default_value = "json")]
    pub format: String,
    /// Compact snapshot: state/uuid/elapsed/health(last_event, code varsa).
    /// tree/metrics-detay/log_tail yok — poll maliyeti düşer (TASK-016).
    #[arg(long, default_value = "false")]
    pub compact: bool,
}

pub fn run(a: StatusArgs) -> Result<i32, String> {
    let sock = resolve_sock(a.sock)?;
    let mut req = json!({"v":1,"op":"status","id":generate_uuid()});
    if a.compact {
        req["compact"] = json!(true);
    }
    let resp = send_request(&sock, &req, 10)?;
    if a.format == "pretty" {
        println!("{}", serde_json::to_string_pretty(&resp).unwrap());
    } else {
        println!("{}", serde_json::to_string(&resp).unwrap());
    }
    let ok = resp.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
    Ok(if ok { 0 } else { 1 })
}
