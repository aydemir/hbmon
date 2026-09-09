use clap::Parser;
use serde_json::json;
use std::path::PathBuf;

use super::resolve_sock;
use crate::ipc::send_request;
use crate::util::generate_uuid;

#[derive(Debug, Parser)]
pub struct ShutdownArgs {
    #[arg(long)]
    pub sock: Option<PathBuf>,
    #[arg(long, default_value = "false")]
    pub force: bool,
}

pub fn run(a: ShutdownArgs) -> Result<i32, String> {
    let sock = resolve_sock(a.sock)?;
    let req = json!({"v":1,"op":"shutdown","id":generate_uuid(),"force":a.force});
    let resp = send_request(&sock, &req, 10)?;
    println!("{}", serde_json::to_string(&resp).unwrap());
    Ok(0)
}
