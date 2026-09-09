use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Newline-delimited JSON request (RFC 5.2.2 + Section 8).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub v: u8,
    pub op: String,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_sec: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub poll_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signal: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub n: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub force: Option<bool>,
}

impl Request {
    pub fn new(op: &str, id: &str) -> Self {
        Self {
            v: 1,
            op: op.to_string(),
            id: id.to_string(),
            timeout_sec: None,
            poll_ms: None,
            signal: None,
            n: None,
            force: None,
        }
    }
}

/// Single-shape response covering all ops; irrelevant fields omitted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub v: u8,
    pub id: String,
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub err: Option<Value>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}

pub fn ok_response(id: &str, extra: serde_json::Map<String, Value>) -> Value {
    let mut m = extra;
    m.insert("v".to_string(), json!(1));
    m.insert("id".to_string(), json!(id));
    m.insert("ok".to_string(), json!(true));
    Value::Object(m)
}

pub fn error_response(id: &str, code: &str, message: &str) -> Value {
    json!({
        "v": 1,
        "id": id,
        "ok": false,
        "err": {"code": code, "message": message}
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ok_response_shape() {
        let mut extra = serde_json::Map::new();
        extra.insert("state".to_string(), serde_json::json!("running"));
        let r = ok_response("req-1", extra);
        assert_eq!(r["v"], 1);
        assert_eq!(r["id"], "req-1");
        assert_eq!(r["ok"], true);
        assert_eq!(r["state"], "running");
    }

    #[test]
    fn error_response_shape() {
        let r = error_response("req-2", "UNKNOWN_OP", "nope");
        assert_eq!(r["ok"], false);
        assert_eq!(r["err"]["code"], "UNKNOWN_OP");
    }

    #[test]
    fn request_roundtrip() {
        let mut q = Request::new("wait", "a");
        q.timeout_sec = Some(30.0);
        let s = serde_json::to_string(&q).unwrap();
        let back: Request = serde_json::from_str(&s).unwrap();
        assert_eq!(back.op, "wait");
        assert_eq!(back.timeout_sec, Some(30.0));
    }
}
