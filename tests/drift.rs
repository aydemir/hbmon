//! Drift kilitleri (TASK-019): IPC op sözlüğü + bağımlılık allow-list.
//! Davranış testi değil — sessiz sözleşme kaymasına karşı tripwire.

use hbmon::ipc::protocol::{Request, IPC_OPS};

/// RFC op tablosu ↔ kod: biri değişirse test kızarır, ikisi el ele güncellenir.
#[test]
fn ipc_ops_locked() {
    assert_eq!(
        IPC_OPS,
        &["status", "metrics", "log_tail", "wait", "kill", "shutdown"]
    );
}

#[test]
fn every_op_roundtrips_typed() {
    for op in IPC_OPS {
        let q = Request::new(op, "drift-1");
        let s = serde_json::to_string(&q).unwrap();
        let back: Request = serde_json::from_str(&s).unwrap();
        assert_eq!(&back.op, op);
    }
}

/// Bağımlılık allow-list: yeni runtime bağımlılık hedef kilidini deler.
/// Kasıtlı ekleme = bu listeyi + `decisions.md`'yi güncelle.
#[test]
fn dependencies_allow_listed() {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let text = std::fs::read_to_string(&manifest).unwrap();
    let allow = [
        "clap",
        "serde",
        "serde_json",
        "regex",
        "once_cell",
        "libc",
        "rand",
    ];
    let dev_allow = ["tempfile", "assert_cmd"];
    let mut section = "";
    for raw in text.lines() {
        let t = raw.trim();
        if t.starts_with('[') {
            section = t;
            continue;
        }
        if section != "[dependencies]" && section != "[dev-dependencies]" {
            continue;
        }
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        let name = t.split(['=', ' ']).next().unwrap().trim();
        let ok = if section == "[dependencies]" {
            allow.contains(&name)
        } else {
            dev_allow.contains(&name)
        };
        assert!(ok, "allow-list dışı bağımlılık: {} ({})", name, section);
    }
}
