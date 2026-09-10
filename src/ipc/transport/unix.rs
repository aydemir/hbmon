//! Unix transport: UDS server + client (`src/ipc/uds.rs` taşındı).
//!
//! Bağlantı modeli değişmez: accept → tek request → handler →
//! tek response → kapat (RFC: kalıcı conn yok).

use serde_json::Value;
use std::io::BufReader;
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::Arc;
use std::time::Duration;

use super::super::codec::{read_message, write_message};
use super::super::protocol::error_response;
use crate::platform::paths::{sock_display, SockAddr};
use crate::platform::perm::secure_fix;

/// Blocking per-connection server: accept -> read one request ->
/// handler -> write one response -> close (RFC: no persistent conns).
pub fn serve<F>(sock_path: &SockAddr, handler: Arc<F>) -> Result<(), String>
where
    F: Fn(Value) -> Value + Send + Sync + 'static,
{
    if sock_path.exists() {
        std::fs::remove_file(sock_path).ok();
    }
    let listener = UnixListener::bind(sock_path)
        .map_err(|e| format!("bind {}: {}", sock_display(sock_path), e))?;
    secure_fix(sock_path);
    for conn in listener.incoming() {
        match conn {
            Ok(stream) => {
                let h = handler.clone();
                std::thread::spawn(move || handle_one(stream, &*h));
            }
            Err(_) => break,
        }
    }
    Ok(())
}

fn handle_one(stream: UnixStream, handler: &dyn Fn(Value) -> Value) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut writer = stream;
    let req = match read_message(&mut reader) {
        Ok(v) => v,
        Err(e) => {
            let _ = write_message(
                &mut writer,
                &error_response("unknown", "INVALID_REQUEST", &e),
            );
            return;
        }
    };
    let resp = handler(req);
    let _ = write_message(&mut writer, &resp);
}

/// Client: connect, send one request, read one response.
pub fn send_request(sock: &SockAddr, req: &Value, timeout_secs: u64) -> Result<Value, String> {
    let stream =
        UnixStream::connect(sock).map_err(|e| format!("connect {}: {}", sock_display(sock), e))?;
    set_timeouts(&stream, timeout_secs.max(2));
    let mut reader = BufReader::new(stream.try_clone().map_err(|e| e.to_string())?);
    let mut writer = stream;
    write_message(&mut writer, req)?;
    read_message(&mut reader)
}

/// Bu adreste canlı monitor dinliyor mu? (double-spawn guard, cleanup)
pub fn can_connect(sock: &SockAddr) -> bool {
    UnixStream::connect(sock).is_ok()
}

fn set_timeouts(stream: &UnixStream, secs: u64) {
    let d = Duration::from_secs(secs.max(1));
    let _ = stream.set_read_timeout(Some(d));
    let _ = stream.set_write_timeout(Some(d));
}
