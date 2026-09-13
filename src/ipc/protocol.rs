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
    /// wait: erken dönüş için izlenecek sinyaller
    /// (done, failed, dep_missing, timeout, stall_suspect, oom_suspect).
    /// Yoksa yalnızca terminal state'lerde dönülür (eski davranış).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub until: Option<Vec<String>>,
    /// status: compact snapshot (TASK-016, opt-in; yoksa full).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compact: Option<bool>,
}

/// UDS op sözlüğü (TASK-019 drift kilidi): RFC'deki op tablosunun kod
/// karşılığı. Yeni op eklenirse burası + `dispatch` + RFC tablosu +
/// `tests/drift.rs` birlikte güncellenir; test sessiz kaymayı yakalar.
pub const IPC_OPS: &[&str] = &["status", "metrics", "log_tail", "wait", "kill", "shutdown"];

/// `wait --until` sinyal sözlüğü (TASK-015): kanonik adlar.
/// `woke_on` istekte yazılan adı aynen yansıtır (kanonikleştirme yok).
pub const WAIT_SIGNALS: &[&str] = &[
    "done",
    "failed",
    "dep_missing",
    "timeout",
    "stall_suspect",
    "oom_suspect",
];

/// Eski/alyas adlar: (alyas, kanonik). `wait_match` iki yazımı da kabul eder.
pub const WAIT_ALIASES: &[(&str, &str)] =
    &[("stalled", "stall_suspect"), ("oom_killed", "oom_suspect")];

/// `until` listesindeki ilk bilinmeyen sinyal adını döner (yoksa `None`).
/// Boş liste her zaman geçerlidir (yalnızca terminal state'ler).
pub fn validate_until(until: &[String]) -> Option<String> {
    until
        .iter()
        .find(|w| {
            let s = w.as_str();
            !WAIT_SIGNALS.contains(&s) && !WAIT_ALIASES.iter().any(|(a, _)| *a == s)
        })
        .cloned()
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
            until: None,
            compact: None,
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

    #[test]
    fn validate_until_accepts_canonical_and_aliases() {
        let ok = [
            "done",
            "failed",
            "dep_missing",
            "timeout",
            "stall_suspect",
            "oom_suspect",
            "stalled",
            "oom_killed",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
        assert_eq!(validate_until(&ok), None);
        assert_eq!(validate_until(&[]), None);
    }

    #[test]
    fn validate_until_rejects_unknown_first() {
        let u = ["done".to_string(), "bogus".to_string()];
        assert_eq!(validate_until(&u), Some("bogus".to_string()));
    }
}
