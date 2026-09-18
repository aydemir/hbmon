//! UDS istek karşılama (TASK-014): `dispatch` + snapshot'lar + `wait_match`.
//! Yaşam döngüsü (`run_daemon`, `poll_once`) `daemon.rs`'dedir. Davranış
//! değişikliği yok — saf kod taşıma.

use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::eventlog::EventLogger;
use crate::health::State;
use crate::ipc::{
    error_response,
    protocol::{ok_response, validate_until, WAIT_SIGNALS},
};
use crate::platform::inspector;
use crate::platform::signal::{kill_pgroup, Sig};
use crate::util::time::now_secs;

pub(crate) struct Shared {
    pub(crate) uuid: String,
    pub(crate) root_cmd: String,
    pub(crate) started_iso: String,
    pub(crate) started_secs: f64,
    pub(crate) state: Mutex<State>,
    pub(crate) build_code: Mutex<Option<i32>>,
    pub(crate) exit_secs: Mutex<Option<f64>>,
    pub(crate) root_pid: Mutex<Option<u32>>,
    pub(crate) parent_pid: Mutex<Option<u32>>,
    pub(crate) label: Mutex<Option<String>>,
    pub(crate) dep_info: Mutex<Option<Value>>,
    pub(crate) oom_info: Mutex<Option<Value>>,
    pub(crate) stall_threshold: Mutex<f32>,
    pub(crate) stall_score: Mutex<f32>,
    pub(crate) last_io_at: Mutex<String>,
    pub(crate) last_cpu_at: Mutex<String>,
    pub(crate) last_spawn_at: Mutex<String>,
    pub(crate) last_event: Mutex<Value>,
    pub(crate) totals: Mutex<HashMap<String, Value>>,
    pub(crate) shutdown: Mutex<bool>,
}

pub(crate) fn dispatch(req: Value, shared: &Shared, logger_path: &Path) -> Value {
    let op = req.get("op").and_then(|v| v.as_str()).unwrap_or("");
    let id = req
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    match op {
        "status" => {
            let compact = req
                .get("compact")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if compact {
                ok_response(&id, status_compact(shared))
            } else {
                status_snapshot(shared, &id, logger_path)
            }
        }
        "metrics" => {
            let t = shared.totals.lock().unwrap();
            let mut m = serde_json::Map::new();
            m.insert(
                "cpu".to_string(),
                t.get("cpu_pct").cloned().unwrap_or(json!(0.0)),
            );
            m.insert(
                "rss_mb".to_string(),
                t.get("rss_mb").cloned().unwrap_or(json!(0)),
            );
            m.insert(
                "io_r".to_string(),
                t.get("io_r").cloned().unwrap_or(json!(0)),
            );
            m.insert(
                "io_w".to_string(),
                t.get("io_w").cloned().unwrap_or(json!(0)),
            );
            m.insert(
                "fds".to_string(),
                t.get("fds_open").cloned().unwrap_or(json!(0)),
            );
            m.insert(
                "net_tcp".to_string(),
                t.get("net_tcp").cloned().unwrap_or(json!(0)),
            );
            ok_response(&id, m)
        }
        "log_tail" => {
            let n = req.get("n").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
            // TASK-029: opsiyonel server-side olay filtresi (yoksa filtresiz).
            let event = req.get("event").and_then(|v| v.as_str());
            let lines = EventLogger::tail_filter(logger_path, n, event);
            let mut m = serde_json::Map::new();
            m.insert("lines".to_string(), json!(lines));
            ok_response(&id, m)
        }
        "wait" => {
            let timeout = req
                .get("timeout_sec")
                .and_then(|v| v.as_f64())
                .unwrap_or(300.0);
            let poll_ms = req
                .get("poll_ms")
                .and_then(|v| v.as_u64())
                .unwrap_or(500)
                .max(50);
            // until yoksa boş liste → yalnızca terminal state'ler (eski davranış).
            let until: Vec<String> = req
                .get("until")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .filter(|v: &Vec<String>| !v.is_empty())
                .unwrap_or_default();
            if let Some(bad) = validate_until(&until) {
                return error_response(
                    &id,
                    "INVALID_UNTIL",
                    &format!(
                        "unknown until signal: {} (valid: {})",
                        bad,
                        WAIT_SIGNALS.join(",")
                    ),
                );
            }
            let deadline = Instant::now() + Duration::from_secs_f64(timeout.max(1.0));
            loop {
                let st = *shared.state.lock().unwrap();
                let dep = shared.dep_info.lock().unwrap().is_some();
                let oom = shared.oom_info.lock().unwrap().is_some();
                if let Some(woke) = wait_match(st, &until, dep, oom) {
                    let terminal = matches!(
                        st,
                        State::Done
                            | State::Failed
                            | State::DepMissing
                            | State::OomKilled
                            | State::Timeout
                    );
                    if terminal {
                        let code = shared.build_code.lock().unwrap().unwrap_or(1);
                        let dur = shared
                            .exit_secs
                            .lock()
                            .unwrap()
                            .map(|e| e - shared.started_secs)
                            .unwrap_or(0.0);
                        let mut m = serde_json::Map::new();
                        m.insert("state".to_string(), json!(st.as_str()));
                        m.insert("code".to_string(), json!(code));
                        m.insert("duration_sec".to_string(), json!(dur));
                        m.insert(
                            "exit_event".to_string(),
                            shared.last_event.lock().unwrap().clone(),
                        );
                        m.insert("woke_on".to_string(), json!(woke));
                        return ok_response(&id, m);
                    }
                    // Erken dönüş: anlık snapshot + hangi sinyal uyandırdı.
                    let mut m = status_map(shared, logger_path);
                    m.insert("woke_on".to_string(), json!(woke));
                    return ok_response(&id, m);
                }
                if Instant::now() >= deadline {
                    let mut m = status_map(shared, logger_path);
                    m.insert("timeout".to_string(), json!(true));
                    m.insert("woke_on".to_string(), json!("timeout"));
                    return ok_response(&id, m);
                }
                std::thread::sleep(Duration::from_millis(poll_ms));
            }
        }
        "kill" => {
            let sig = Sig::from_num(req.get("signal").and_then(|v| v.as_i64()).unwrap_or(15));
            let pid = shared.root_pid.lock().unwrap().unwrap_or(0);
            if pid == 0 {
                return error_response(&id, "BUILD_NOT_FOUND", "no root pid");
            }
            // Check BEFORE signal: already-dead builds → killed:false (S4a).
            let was_alive = crate::platform::inspector().is_alive(pid);
            if was_alive {
                kill_pgroup(pid, sig);
            }
            let killed = was_alive;
            let mut m = serde_json::Map::new();
            m.insert("killed".to_string(), json!(killed));
            ok_response(&id, m)
        }
        "shutdown" => {
            *shared.shutdown.lock().unwrap() = true;
            let force = req.get("force").and_then(|v| v.as_bool()).unwrap_or(false);
            let pid = shared.root_pid.lock().unwrap().unwrap_or(0);
            if pid != 0 {
                kill_pgroup(pid, Sig::Term);
                if force {
                    std::thread::sleep(Duration::from_millis(200));
                    kill_pgroup(pid, Sig::Kill);
                }
            }
            let ok = *shared.shutdown.lock().unwrap();
            let mut m = serde_json::Map::new();
            m.insert("ok_shutdown".to_string(), json!(ok));
            ok_response(&id, m)
        }
        "" => error_response(&id, "INVALID_REQUEST", "missing op"),
        other => error_response(&id, "UNKNOWN_OP", &format!("unknown op: {}", other)),
    }
}

fn status_map(shared: &Shared, logger_path: &Path) -> serde_json::Map<String, Value> {
    let st = *shared.state.lock().unwrap();
    let t = shared.totals.lock().unwrap();
    let mut m = serde_json::Map::new();
    m.insert("state".to_string(), json!(st.as_str()));
    m.insert("uuid".to_string(), json!(shared.uuid));
    m.insert(
        "root_pid".to_string(),
        json!(shared.root_pid.lock().unwrap().unwrap_or(0)),
    );
    m.insert(
        "parent_pid".to_string(),
        json!(*shared.parent_pid.lock().unwrap()),
    );
    m.insert(
        "label".to_string(),
        json!(shared.label.lock().unwrap().clone()),
    );
    m.insert("root_cmd".to_string(), json!(shared.root_cmd));
    m.insert("started_at".to_string(), json!(shared.started_iso));
    m.insert(
        "elapsed_sec".to_string(),
        json!(now_secs() - shared.started_secs),
    );
    m.insert(
        "metrics".to_string(),
        json!({
            "cpu_pct": t.get("cpu_pct").cloned().unwrap_or(json!(0.0)),
            "rss_mb": t.get("rss_mb").cloned().unwrap_or(json!(0)),
            "io_read_mb": t.get("io_read_mb").cloned().unwrap_or(json!(0)),
            "io_write_mb": t.get("io_write_mb").cloned().unwrap_or(json!(0)),
            "fds_open": t.get("fds_open").cloned().unwrap_or(json!(0)),
            "net_tcp": t.get("net_tcp").cloned().unwrap_or(json!(0)),
            "net_udp": json!(0),
        }),
    );
    if let Some(cgv) = t.get("cgroup") {
        m.insert("cgroup".to_string(), cgv.clone());
    }
    let tree = {
        let pid = shared.root_pid.lock().unwrap().unwrap_or(0);
        if pid != 0 {
            let insp = inspector();
            insp.tree(pid).map(|n| json!([n])).unwrap_or(json!([]))
        } else {
            json!([])
        }
    };
    m.insert("tree".to_string(), tree);
    m.insert(
        "health".to_string(),
        json!({
            "stall_score": *shared.stall_score.lock().unwrap(),
            "threshold_sec": *shared.stall_threshold.lock().unwrap(),
            "last_io_at": shared.last_io_at.lock().unwrap().clone(),
            "last_cpu_nonzero_at": shared.last_cpu_at.lock().unwrap().clone(),
            "last_child_spawn_at": shared.last_spawn_at.lock().unwrap().clone(),
        }),
    );
    m.insert(
        "log_tail".to_string(),
        json!(EventLogger::tail(logger_path, 5)),
    );
    m.insert(
        "last_event".to_string(),
        shared.last_event.lock().unwrap().clone(),
    );
    if let Some(c) = *shared.build_code.lock().unwrap() {
        m.insert("code".to_string(), json!(c));
    }
    m
}

fn status_snapshot(shared: &Shared, id: &str, logger_path: &Path) -> Value {
    ok_response(id, status_map(shared, logger_path))
}

/// Compact snapshot (TASK-016): poll eden ajan için en küçük alan seti.
/// tree/metrics-detay/log_tail/root_cmd/cgroup yok — hem bayt hem sunucu
/// maliyeti düşer (inspector/log okuma yok). Varsayılan `status` full kalır.
fn status_compact(shared: &Shared) -> serde_json::Map<String, Value> {
    let st = *shared.state.lock().unwrap();
    let mut m = serde_json::Map::new();
    m.insert("state".to_string(), json!(st.as_str()));
    m.insert("uuid".to_string(), json!(shared.uuid));
    m.insert(
        "elapsed_sec".to_string(),
        json!(now_secs() - shared.started_secs),
    );
    m.insert(
        "health".to_string(),
        json!({
            "stall_score": *shared.stall_score.lock().unwrap(),
            "threshold_sec": *shared.stall_threshold.lock().unwrap(),
        }),
    );
    m.insert(
        "last_event".to_string(),
        shared.last_event.lock().unwrap().clone(),
    );
    if let Some(c) = *shared.build_code.lock().unwrap() {
        m.insert("code".to_string(), json!(c));
    }
    m
}

/// `until` boşsa yalnızca terminal state'ler eşleşir (eski davranış).
/// Doluysa listedeki ilk sinyal kazanır; ara sinyaller (stall/dep/oom)
/// build bitmeden de eşleşebilir — erken dönüşün çekirdeği.
/// Bilinmeyen adlar `dispatch`'te `validate_until` ile reddedilir
/// (INVALID_UNTIL); burada savunma amaçlı yok sayılır.
///
/// TASK-047/S2: terminal state listede YOKSA bile döner (kanonik adla).
/// Aksi hâlde bitmiş build'de `--until stall_suspect` deadline'a kadar
/// bekliyordu: timeout → 124, deadline linger'ı (60 s) aşarsa daemon
/// çekilip istemci "connection closed" → exit 3 alıyordu. Bir daha
/// sinyal üretemeyecek terminal durum, beklemeye değmez.
fn wait_match(state: State, until: &[String], dep_hit: bool, oom_hit: bool) -> Option<String> {
    if until.is_empty() {
        if matches!(
            state,
            State::Done | State::Failed | State::DepMissing | State::OomKilled | State::Timeout
        ) {
            return Some(state.as_str().to_string());
        }
        return None;
    }
    for want in until {
        let hit = match want.as_str() {
            "done" => state == State::Done,
            "failed" => state == State::Failed,
            "timeout" => state == State::Timeout,
            "dep_missing" => dep_hit || state == State::DepMissing,
            "stall_suspect" | "stalled" => state == State::Stalled,
            "oom_suspect" | "oom_killed" => oom_hit || state == State::OomKilled,
            _ => false,
        };
        if hit {
            return Some(want.clone());
        }
    }
    terminal_signal(state)
}

/// Terminal state → kanonik `wait --until` sinyal adı (TASK-047/S2).
/// Ara durumlar (running/stalled) `None`: erken sinyal sabrı korunur.
fn terminal_signal(state: State) -> Option<String> {
    match state {
        State::Done => Some("done".to_string()),
        State::Failed => Some("failed".to_string()),
        State::DepMissing => Some("dep_missing".to_string()),
        State::Timeout => Some("timeout".to_string()),
        State::OomKilled => Some("oom_suspect".to_string()),
        State::Running | State::Stalled => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_wakes_only_on_terminal() {
        let empty: Vec<String> = vec![];
        assert_eq!(
            wait_match(State::Done, &empty, false, false),
            Some("done".to_string())
        );
        assert_eq!(wait_match(State::Running, &empty, false, false), None);
        // Ara sinyal bile olsa boş until uyandırmaz (eski davranış).
        assert_eq!(wait_match(State::Running, &empty, true, true), None);
    }

    #[test]
    fn dep_signal_wakes_early() {
        let u = vec!["dep_missing".to_string()];
        assert_eq!(
            wait_match(State::Running, &u, true, false),
            Some("dep_missing".to_string())
        );
        assert_eq!(wait_match(State::Running, &u, false, false), None);
    }

    #[test]
    fn stall_and_oom_wake_early() {
        assert_eq!(
            wait_match(State::Stalled, &["stall_suspect".to_string()], false, false),
            Some("stall_suspect".to_string())
        );
        assert_eq!(
            wait_match(State::Running, &["oom_suspect".to_string()], false, true),
            Some("oom_suspect".to_string())
        );
    }

    #[test]
    fn unknown_names_ignored_until_order_wins() {
        let u = vec!["nope".to_string(), "failed".to_string()];
        assert_eq!(
            wait_match(State::Failed, &u, false, false),
            Some("failed".to_string())
        );
        assert_eq!(wait_match(State::Running, &u, false, false), None);
    }

    #[test]
    fn aliases_match_canonical_states() {
        assert_eq!(
            wait_match(State::Stalled, &["stalled".to_string()], false, false),
            Some("stalled".to_string())
        );
        assert_eq!(
            wait_match(State::OomKilled, &["oom_killed".to_string()], false, false),
            Some("oom_killed".to_string())
        );
    }

    /// TASK-047/S2: terminal state listede olmasa da döner — bitmiş
    /// build'de deadline/linger beklemek yerine dürüst sonuç verilir.
    #[test]
    fn terminal_state_wakes_even_when_not_listed() {
        let u = vec!["stall_suspect".to_string()];
        assert_eq!(
            wait_match(State::Done, &u, false, false),
            Some("done".into())
        );
        assert_eq!(
            wait_match(State::Failed, &u, false, false),
            Some("failed".into())
        );
        assert_eq!(
            wait_match(State::DepMissing, &u, false, false),
            Some("dep_missing".into())
        );
        assert_eq!(
            wait_match(State::Timeout, &u, false, false),
            Some("timeout".into())
        );
        assert_eq!(
            wait_match(State::OomKilled, &u, false, false),
            Some("oom_suspect".into())
        );
    }

    /// Ara durumlar terminal değil: erken sinyal sabrı bozulmaz.
    #[test]
    fn non_terminal_states_still_wait() {
        let u = vec!["done".to_string()];
        assert_eq!(wait_match(State::Running, &u, false, false), None);
        assert_eq!(wait_match(State::Stalled, &u, false, false), None);
        // İstenen ara sinyal yine kazanır (kanonik ad).
        assert_eq!(
            wait_match(State::Stalled, &["stall_suspect".to_string()], false, false),
            Some("stall_suspect".to_string())
        );
    }
}
