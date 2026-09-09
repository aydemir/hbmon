//! Newline-delimited JSON wire codec (transport-agnostik).
//!
//! `read_message`/`write_message` `BufRead`/`Write` generiğiyle çalışır:
//! unix'te UDS stream'i, Windows'ta named-pipe handle'ı bağlanır.
//! Wire format v1 değişmez (`protocol.rs`).

use serde_json::Value;
use std::io::{BufRead, Write};

pub fn read_message(reader: &mut impl BufRead) -> Result<Value, String> {
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .map_err(|e| format!("ipc read: {}", e))?;
    if line.trim().is_empty() {
        return Err("empty request".to_string());
    }
    serde_json::from_str(line.trim()).map_err(|e| format!("bad json: {}", e))
}

pub fn write_message(stream: &mut impl Write, v: &Value) -> Result<(), String> {
    let mut s = serde_json::to_string(v).map_err(|e| e.to_string())?;
    s.push('\n');
    stream
        .write_all(s.as_bytes())
        .map_err(|e| format!("ipc write: {}", e))?;
    stream.flush().map_err(|e| format!("ipc flush: {}", e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::BufReader;

    #[test]
    fn roundtrip_one_message() {
        let v = serde_json::json!({"v":1,"op":"status","id":"t-1"});
        let mut buf = Vec::new();
        write_message(&mut buf, &v).unwrap();
        let mut reader = BufReader::new(buf.as_slice());
        assert_eq!(read_message(&mut reader).unwrap(), v);
    }

    #[test]
    fn empty_line_is_error() {
        let mut reader = BufReader::new(b"\n".as_slice());
        assert!(read_message(&mut reader).is_err());
    }

    #[test]
    fn bad_json_is_error() {
        let mut reader = BufReader::new(b"{oops\n".as_slice());
        assert!(read_message(&mut reader).is_err());
    }
}
