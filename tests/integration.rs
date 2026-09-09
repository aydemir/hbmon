//! End-to-end tests: real daemon, UDS, JSONL event log.
//! Each test uses a unique --uuid so parallel cargo-test threads
//! never share a socket.

use assert_cmd::Command;
use serde_json::Value;
use std::path::PathBuf;
use std::time::Duration;

fn uuid(tag: &str) -> String {
    format!("itest-{}-{}", std::process::id(), tag)
}

fn sock_for(id: &str) -> PathBuf {
    PathBuf::from(format!("/tmp/hbmon-{}.sock", id))
}

fn hbmon() -> Command {
    Command::cargo_bin("hbmon").unwrap()
}

#[test]
fn exec_handshake_and_exit_zero() {
    let out = hbmon()
        .args(["exec", "--", "echo", "hi"])
        .timeout(Duration::from_secs(30))
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let first = stdout.lines().next().expect("handshake line");
    let hs: Value = serde_json::from_str(first).expect("handshake is JSON");
    assert_eq!(hs["ev"], "ready");
    assert!(hs["sock"].as_str().unwrap().contains("hbmon-"));
    assert!(stdout.contains("hi"));
}

#[test]
fn exec_dep_missing_exit_two() {
    let out = hbmon()
        .args([
            "exec",
            "--",
            "sh",
            "-c",
            "echo \"Cannot find module 'foo'\" >&2; exit 1",
        ])
        .timeout(Duration::from_secs(30))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn watch_status_wait_full_cycle() {
    let id = uuid("cycle");
    let sock = sock_for(&id);
    let s = sock.to_str().unwrap().to_string();

    let out = hbmon()
        .args(["watch", "--detach", "--uuid", &id, "--", "sleep", "3"])
        .timeout(Duration::from_secs(30))
        .output()
        .unwrap();
    assert!(out.status.success());
    let hs: Value =
        serde_json::from_slice(&out.stdout).expect("watch prints handshake");
    assert_eq!(hs["uuid"], id.as_str());

    let st = hbmon()
        .args(["status", "--sock", &s])
        .timeout(Duration::from_secs(30))
        .output()
        .unwrap();
    assert!(st.status.success());
    let v: Value = serde_json::from_slice(&st.stdout).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["uuid"], id.as_str());

    let w = hbmon()
        .args(["wait", "--sock", &s, "--timeout", "60"])
        .timeout(Duration::from_secs(90))
        .output()
        .unwrap();
    assert!(w.status.success());
    let wv: Value = serde_json::from_slice(&w.stdout).unwrap();
    assert_eq!(wv["state"], "done");
    assert_eq!(wv["code"], 0);
}

#[test]
fn kill_terminates_build() {
    let id = uuid("kill");
    let sock = sock_for(&id);
    let s = sock.to_str().unwrap().to_string();

    let out = hbmon()
        .args(["watch", "--detach", "--uuid", &id, "--", "sleep", "60"])
        .timeout(Duration::from_secs(30))
        .output()
        .unwrap();
    assert!(out.status.success());
    std::thread::sleep(Duration::from_secs(2));

    let k = hbmon()
        .args(["kill", "--sock", &s, "--signal", "TERM"])
        .timeout(Duration::from_secs(30))
        .output()
        .unwrap();
    assert!(k.status.success());
    let kv: Value = serde_json::from_slice(&k.stdout).unwrap();
    assert_eq!(kv["killed"], true);

    let w = hbmon()
        .args(["wait", "--sock", &s, "--timeout", "30"])
        .timeout(Duration::from_secs(60))
        .output()
        .unwrap();
    // killed build maps to failed(1)
    assert_eq!(w.status.code(), Some(1));
}
