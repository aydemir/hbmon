use clap::Parser;
use std::path::PathBuf;

use super::resolve_sock;
use crate::daemon::signals::parse_signal;
use crate::ipc::send_request;
use crate::util::generate_uuid;
use serde_json::json;

#[derive(Debug, Parser)]
pub struct KillArgs {
    #[arg(long)]
    pub sock: Option<PathBuf>,
    #[arg(long)]
    pub pid: Option<u32>,
    #[arg(long, default_value = "TERM")]
    pub signal: String,
}

pub fn run(a: KillArgs) -> Result<i32, String> {
    let sig = parse_signal(&a.signal)?;
    if let Some(pid) = a.pid {
        // direct kill without UDS: whole process group (RFC 4.5)
        if !crate::platform::signal::kill_pgroup(pid, sig) {
            return Err(format!("kill({}) failed", pid));
        }
        println!("{{\"killed\":true,\"pid\":{}}}", pid);
        return Ok(0);
    }
    let sock = resolve_sock(a.sock)?;
    let req = json!({"v":1,"op":"kill","id":generate_uuid(),"signal":sig.to_num()});
    let resp = send_request(&sock, &req, 10)?;
    println!("{}", serde_json::to_string(&resp).unwrap());
    Ok(0)
}
