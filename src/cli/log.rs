use clap::Parser;
use serde_json::json;
use std::path::PathBuf;

use super::resolve_sock;
use crate::ipc::send_request;
use crate::util::generate_uuid;

#[derive(Debug, Parser)]
pub struct LogArgs {
    #[arg(long)]
    pub sock: Option<PathBuf>,
    /// Son N olay (varsayılan 20)
    #[arg(long, default_value = "20")]
    pub tail: usize,
    /// Yalnızca bu olay adı (`ev` tam eşleşme; örn. `metric`, `exit`).
    /// Server-side filtrelenir — eşleşenlerin son N'i döner (TASK-029).
    #[arg(long)]
    pub event: Option<String>,
}

/// Salt-okunur olay günlüğü (TASK-023): `log_tail` op'una ince CLI.
/// Ajanın tüm `.jsonl`'u cat'lemesine gerek bırakmaz (context ekonomisi).
pub fn run(a: LogArgs) -> Result<i32, String> {
    let sock = resolve_sock(a.sock)?;
    let mut req = json!({"v":1,"op":"log_tail","id":generate_uuid(),"n":a.tail});
    if let Some(ev) = a.event {
        req["event"] = json!(ev);
    }
    let resp = send_request(&sock, &req, 10)?;
    println!("{}", serde_json::to_string(&resp).unwrap());
    let ok = resp.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
    Ok(if ok { 0 } else { 1 })
}
