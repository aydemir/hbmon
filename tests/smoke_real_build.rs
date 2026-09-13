//! Real-toolchain smoke test (TASK-034): a genuine `cargo build` under watch.
//! Sleep/echo-based integration tests prove the machinery; this one proves
//! a real compiler invocation completes and reports `done`/`code 0`.
//! Needs the cargo toolchain (dev machines and CI have it).

use assert_cmd::Command;
use serde_json::Value;
use std::time::Duration;

fn uuid(tag: &str) -> String {
    // pid + nanos: pid reuse across rapid re-runs must not collide with a
    // previous run's lingering daemon socket (TASK-012 flake rule).
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("smoke-{}-{}-{}", std::process::id(), nanos, tag)
}

fn sock_for(id: &str) -> String {
    hbmon::platform::paths::sock_display(&hbmon::platform::paths::default_sock(id))
}

fn hbmon() -> Command {
    Command::cargo_bin("hbmon").unwrap()
}

fn wait_for_ready(sock: &str) {
    let deadline = std::time::Instant::now() + Duration::from_secs(15);
    loop {
        let st = hbmon()
            .args(["status", "--sock", sock])
            .timeout(Duration::from_secs(10))
            .output()
            .unwrap();
        if st.status.success() {
            return;
        }
        if std::time::Instant::now() >= deadline {
            panic!("daemon never came up for {}", sock);
        }
        std::thread::sleep(Duration::from_millis(300));
    }
}

#[test]
fn smoke_cargo_build_completes() {
    // Minimal dependency-free crate: no network, no index access.
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"hbmon-smoke\"\nversion = \"0.0.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    let src = dir.path().join("src");
    std::fs::create_dir(&src).unwrap();
    std::fs::write(
        src.join("main.rs"),
        "fn main() {\n    println!(\"smoke\");\n}\n",
    )
    .unwrap();
    let manifest = dir.path().join("Cargo.toml");
    let target = dir.path().join("target");

    let id = uuid("cargo");
    let s = sock_for(&id);
    let out = hbmon()
        .args([
            "watch",
            "--detach",
            "--uuid",
            &id,
            "--",
            "cargo",
            "build",
            "--manifest-path",
            &manifest.to_string_lossy(),
            "--target-dir",
            &target.to_string_lossy(),
        ])
        .timeout(Duration::from_secs(60))
        .output()
        .unwrap();
    assert!(out.status.success());
    let hs: Value = serde_json::from_slice(&out.stdout).expect("watch prints handshake");
    assert_eq!(hs["uuid"], id.as_str());

    wait_for_ready(&s);
    let w = hbmon()
        .args(["wait", "--sock", &s, "--timeout", "300"])
        .timeout(Duration::from_secs(330))
        .output()
        .unwrap();
    assert!(w.status.success());
    let wv: Value = serde_json::from_slice(&w.stdout).unwrap();
    assert_eq!(wv["state"], "done");
    assert_eq!(wv["code"], 0);

    let _ = hbmon()
        .args(["shutdown", "--sock", &s])
        .timeout(Duration::from_secs(30))
        .output();
}
