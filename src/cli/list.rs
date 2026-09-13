use clap::Parser;
use serde_json::{json, Value};

use crate::ipc::send_request;
use crate::platform::paths;
use crate::util::generate_uuid;

#[derive(Debug, Parser)]
pub struct ListArgs {
    /// Taranacak dizin (varsayılan: platform convention — unix /tmp,
    /// windows %TEMP%). Hermetik test ve özel durum dizinleri için.
    #[arg(long)]
    pub dir: Option<std::path::PathBuf>,
    /// Yalnızca bu durumdakiler (`state` tam eşleşme; örn. `running`).
    #[arg(long)]
    pub state: Option<String>,
    /// Yalnızca canlı (sock'a bağlanabilen) izleyiciler.
    #[arg(long, default_value = "false")]
    pub live_only: bool,
}

/// Client-side filtre (TASK-029): `state`/`live` zaten her aday için
/// best-effort toplanır, filtre çıktıda uygulanır — protokol değişmez.
fn keep(live: bool, state: &Value, want_state: &Option<String>, live_only: bool) -> bool {
    if live_only && !live {
        return false;
    }
    if let Some(w) = want_state {
        if state.as_str() != Some(w.as_str()) {
            return false;
        }
    }
    true
}

/// Listeleme adayları: (uuid, adres, yaş-sn). Unix'te `<root>` içindeki
/// `.sock` dosyaları taranır; Windows'ta pipe'lar dosya olmadığından
/// `.jsonl` izleri taranıp pipe adı türetilir (canlılık `can_connect`
/// ile yoklanır).
fn entries(root: &std::path::Path) -> Vec<(String, paths::SockAddr, Option<u64>)> {
    let now = std::time::SystemTime::now();
    let age_of = |p: &std::path::Path| {
        std::fs::metadata(p)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|mt| now.duration_since(mt).ok())
            .map(|d| d.as_secs())
    };
    let mut out = Vec::new();
    #[cfg(unix)]
    {
        if let Ok(dir) = std::fs::read_dir(root) {
            for e in dir.flatten() {
                let name = e.file_name().to_string_lossy().to_string();
                if !(name.starts_with("hbmon-") && name.ends_with(".sock")) {
                    continue;
                }
                if let Some(uuid) = paths::uuid_from_base(&name) {
                    let p = e.path();
                    let age = age_of(&p);
                    out.push((uuid, paths::from_explicit(p), age));
                }
            }
        }
    }
    #[cfg(windows)]
    {
        if let Ok(dir) = std::fs::read_dir(root) {
            for e in dir.flatten() {
                let name = e.file_name().to_string_lossy().to_string();
                if !(name.starts_with("hbmon-") && name.ends_with(".jsonl")) {
                    continue;
                }
                if let Some(uuid) = paths::uuid_from_base(&name) {
                    let age = age_of(&e.path());
                    out.push((uuid.clone(), paths::default_sock(&uuid), age));
                }
            }
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// Salt-okunur keşif (TASK-017): bilinen izleyicileri sırala.
/// Hiçbir şeyi öldürmez/silmez (gc için `cleanup` var).
pub fn run(a: ListArgs) -> Result<i32, String> {
    let root = a.dir.unwrap_or_else(paths::scan_dir);
    let mut out = Vec::new();
    for (uuid, addr, age_sec) in entries(&root) {
        let live = crate::ipc::can_connect(&addr);
        // Best-effort state: canlıysa kısa timeout'la sor, yoksa null.
        let state: Value = if live {
            let req = json!({"v":1,"op":"status","id":generate_uuid(),"compact":true});
            send_request(&addr, &req, 2)
                .ok()
                .and_then(|r| r.get("state").cloned())
                .unwrap_or(Value::Null)
        } else {
            Value::Null
        };
        if !keep(live, &state, &a.state, a.live_only) {
            continue;
        }
        out.push(json!({
            "uuid": uuid,
            "sock": paths::sock_display(&addr),
            "live": live,
            "state": state,
            "age_sec": age_sec,
        }));
    }
    println!("{}", Value::Array(out));
    Ok(0)
}

#[cfg(test)]
mod tests {
    use crate::platform::paths::uuid_from_base;

    #[test]
    fn uuid_from_artifact_names() {
        assert_eq!(
            uuid_from_base("hbmon-a3f9c1e2.sock"),
            Some("a3f9c1e2".to_string())
        );
        assert_eq!(
            uuid_from_base("hbmon-a3f9c1e2.jsonl"),
            Some("a3f9c1e2".to_string())
        );
        assert_eq!(
            uuid_from_base(r"\\.\pipe\hbmon-a3f9c1e2"),
            Some("a3f9c1e2".to_string())
        );
        assert_eq!(uuid_from_base("hbmon-.sock"), None);
        assert_eq!(uuid_from_base("other-a3f9c1e2.sock"), None);
    }

    #[test]
    fn keep_filters_state_and_liveness() {
        use super::keep;
        use serde_json::{json, Value};
        let running = json!("running");
        let null = Value::Null;
        assert!(keep(true, &running, &None, false));
        assert!(keep(false, &null, &None, false));
        assert!(keep(true, &running, &Some("running".into()), false));
        assert!(!keep(true, &running, &Some("done".into()), false));
        // Ölü adayda state null'dur — state filtresi onu eler.
        assert!(!keep(false, &null, &Some("running".into()), false));
        assert!(keep(true, &running, &None, true));
        assert!(!keep(false, &null, &None, true));
        assert!(keep(true, &running, &Some("running".into()), true));
    }
}
