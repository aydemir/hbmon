use serde_json::{json, Value};
use std::collections::HashMap;

use crate::util::time::now_iso;

/// Build a v1 log event: {"ts","v":1,"ev",...extra}
pub fn new_event(ev: &str, uuid: &str, extra: HashMap<String, Value>) -> Value {
    let mut m = serde_json::Map::new();
    m.insert("ts".to_string(), Value::String(now_iso()));
    m.insert("v".to_string(), json!(1));
    m.insert("ev".to_string(), Value::String(ev.to_string()));
    m.insert("uuid".to_string(), Value::String(uuid.to_string()));
    for (k, v) in extra {
        m.insert(k, v);
    }
    Value::Object(m)
}

pub fn kv(pairs: &[(&str, Value)]) -> HashMap<String, Value> {
    pairs.iter().map(|(k, v)| (k.to_string(), v.clone())).collect()
}
