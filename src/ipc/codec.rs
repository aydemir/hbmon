use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::time::Duration;

pub fn read_message(reader: &mut BufReader<UnixStream>) -> Result<Value, String> {
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .map_err(|e| format!("uds read: {}", e))?;
    if line.trim().is_empty() {
        return Err("empty request".to_string());
    }
    serde_json::from_str(line.trim()).map_err(|e| format!("bad json: {}", e))
}

pub fn write_message(stream: &mut UnixStream, v: &Value) -> Result<(), String> {
    let mut s = serde_json::to_string(v).map_err(|e| e.to_string())?;
    s.push('\n');
    stream
        .write_all(s.as_bytes())
        .map_err(|e| format!("uds write: {}", e))?;
    stream.flush().map_err(|e| format!("uds flush: {}", e))?;
    Ok(())
}

pub fn set_timeouts(stream: &UnixStream, secs: u64) {
    let d = Duration::from_secs(secs.max(1));
    let _ = stream.set_read_timeout(Some(d));
    let _ = stream.set_write_timeout(Some(d));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_one_message() {
        let (a, b) = UnixStream::pair().unwrap();
        let mut reader = BufReader::new(a);
        let mut writer = b;
        let v = serde_json::json!({"v":1,"op":"status","id":"t-1"});
        write_message(&mut writer, &v).unwrap();
        assert_eq!(read_message(&mut reader).unwrap(), v);
    }

    #[test]
    fn empty_line_is_error() {
        let (a, b) = UnixStream::pair().unwrap();
        let mut reader = BufReader::new(a);
        let mut writer = b;
        writer.write_all(b"\n").unwrap();
        assert!(read_message(&mut reader).is_err());
    }

    #[test]
    fn bad_json_is_error() {
        let (a, b) = UnixStream::pair().unwrap();
        let mut reader = BufReader::new(a);
        let mut writer = b;
        writer.write_all(b"{oops\n").unwrap();
        assert!(read_message(&mut reader).is_err());
    }
}
