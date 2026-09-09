use serde_json::Value;
use std::io::BufReader;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::sync::Arc;

use super::codec::{read_message, write_message};
use super::protocol::error_response;

/// Blocking per-connection server: accept -> read one request ->
/// handler -> write one response -> close (RFC: no persistent conns).
pub fn serve<F>(sock_path: &Path, handler: Arc<F>) -> Result<(), String>
where
    F: Fn(Value) -> Value + Send + Sync + 'static,
{
    if sock_path.exists() {
        std::fs::remove_file(sock_path).ok();
    }
    let listener =
        UnixListener::bind(sock_path).map_err(|e| format!("bind {}: {}", sock_path.display(), e))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(sock_path, std::fs::Permissions::from_mode(0o600)).ok();
    }
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
            let _ = write_message(&mut writer, &error_response("unknown", "INVALID_REQUEST", &e));
            return;
        }
    };
    let resp = handler(req);
    let _ = write_message(&mut writer, &resp);
}

/// Client: connect, send one request, read one response.
pub fn send_request(sock: &Path, req: &Value, timeout_secs: u64) -> Result<Value, String> {
    let stream =
        UnixStream::connect(sock).map_err(|e| format!("connect {}: {}", sock.display(), e))?;
    super::codec::set_timeouts(&stream, timeout_secs.max(2));
    let mut reader = BufReader::new(stream.try_clone().map_err(|e| e.to_string())?);
    let mut writer = stream;
    super::codec::write_message(&mut writer, req)?;
    super::codec::read_message(&mut reader)
}
