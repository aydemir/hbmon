//! End-to-end tests: real daemon, UDS, JSONL event log.
//! Each test uses a unique --uuid so parallel cargo-test threads
//! never share a socket.

use assert_cmd::Command;
use serde_json::Value;
use std::time::Duration;

fn uuid(tag: &str) -> String {
    format!("itest-{}-{}", std::process::id(), tag)
}

/// Daemon'un bu uuid için dinlediği adres (metin): unix'te
/// `/tmp/hbmon-<id>.sock`, Windows'ta `\\.\pipe\hbmon-<id>`.
/// Test `--uuid <id>` verir; daemon varsayılan adresi türetir.
fn sock_for(id: &str) -> String {
    hbmon::platform::paths::sock_display(&hbmon::platform::paths::default_sock(id))
}

/// Platform uyku komutu: `["sleep","N"]` / powershell `Start-Sleep`.
fn sleep_cmd(secs: u64) -> Vec<String> {
    #[cfg(unix)]
    {
        vec!["sleep".to_string(), secs.to_string()]
    }
    #[cfg(windows)]
    {
        vec![
            "powershell".to_string(),
            "-NoProfile".to_string(),
            "-Command".to_string(),
            format!("Start-Sleep -Seconds {}", secs),
        ]
    }
}

/// Platform shell: `sh -c <script>` / `powershell -Command <script>`.
fn shell_cmd(script: &str) -> Vec<String> {
    #[cfg(unix)]
    {
        vec!["sh".to_string(), "-c".to_string(), script.to_string()]
    }
    #[cfg(windows)]
    {
        vec![
            "powershell".to_string(),
            "-NoProfile".to_string(),
            "-Command".to_string(),
            script.to_string(),
        ]
    }
}

/// stderr'a dep-missing satırı basıp `code` ile çıkan script.
fn dep_script(code: i32) -> String {
    #[cfg(unix)]
    {
        format!("echo \"Cannot find module 'foo'\" >&2; exit {}", code)
    }
    #[cfg(windows)]
    {
        format!(
            "[Console]::Error.WriteLine(\"Cannot find module 'foo'\"); exit {}",
            code
        )
    }
}

/// Dep satırı basıp uykuya yatan script (erken-dönüş testi).
fn dep_then_sleep(secs: u64) -> String {
    #[cfg(unix)]
    {
        format!("echo \"Cannot find module 'x'\" >&2; sleep {}", secs)
    }
    #[cfg(windows)]
    {
        format!(
            "[Console]::Error.WriteLine(\"Cannot find module 'x'\"); Start-Sleep -Seconds {}",
            secs
        )
    }
}

fn watch_args(id: &str, program: Vec<String>) -> Vec<String> {
    let mut a = vec![
        "watch".to_string(),
        "--detach".to_string(),
        "--uuid".to_string(),
        id.to_string(),
        "--".to_string(),
    ];
    a.extend(program);
    a
}

fn exec_args(program: Vec<String>) -> Vec<String> {
    let mut a = vec!["exec".to_string(), "--".to_string()];
    a.extend(program);
    a
}

fn hbmon() -> Command {
    Command::cargo_bin("hbmon").unwrap()
}

/// Daemon handshake'i fork'tan ÖNCE basar; socket'in bind olması
/// yüklü makinede gecikebilir. Tek atış yerine hazır olana kadar yokla.
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
fn exec_handshake_and_exit_zero() {
    let out = hbmon()
        .args(exec_args(vec!["echo".to_string(), "hi".to_string()]))
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
        .args(exec_args(shell_cmd(&dep_script(1))))
        .timeout(Duration::from_secs(30))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn watch_status_wait_full_cycle() {
    let id = uuid("cycle");
    let s = sock_for(&id);

    let out = hbmon()
        .args(watch_args(&id, sleep_cmd(3)))
        .timeout(Duration::from_secs(30))
        .output()
        .unwrap();
    assert!(out.status.success());
    let hs: Value =
        serde_json::from_slice(&out.stdout).expect("watch prints handshake");
    assert_eq!(hs["uuid"], id.as_str());

    wait_for_ready(&s);
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
    let s = sock_for(&id);

    let out = hbmon()
        .args(watch_args(&id, sleep_cmd(60)))
        .timeout(Duration::from_secs(30))
        .output()
        .unwrap();
    assert!(out.status.success());
    wait_for_ready(&s);

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

#[test]
fn exec_json_summary_on_stderr() {
    let out = hbmon()
        .args({
            let mut a = vec!["exec".to_string(), "--format".to_string(), "json".to_string(), "--".to_string()];
            a.extend(shell_cmd("exit 3"));
            a
        })
        .timeout(Duration::from_secs(30))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let err = String::from_utf8(out.stderr).unwrap();
    let last = err.lines().last().expect("summary line");
    let v: Value = serde_json::from_str(last).expect("summary is JSON");
    assert_eq!(v["ev"], "exit");
    assert_eq!(v["state"], "failed");
    assert_eq!(v["code"], 1);
    assert_eq!(v["raw_code"], 3);
}

#[test]
fn wait_until_dep_missing_returns_early() {
    let id = uuid("until-dep");
    let s = sock_for(&id);
    let out = hbmon()
        .args(watch_args(&id, shell_cmd(&dep_then_sleep(30))))
        .timeout(Duration::from_secs(30))
        .output()
        .unwrap();
    assert!(out.status.success());
    wait_for_ready(&s);
    let w = hbmon()
        .args([
            "wait",
            "--sock",
            &s,
            "--timeout",
            "60",
            "--until",
            "dep_missing,stall_suspect,done",
        ])
        .timeout(Duration::from_secs(90))
        .output()
        .unwrap();
    // Erken dönüşte code alanı yok → exit 0; ajan woke_on'a bakar.
    assert!(w.status.success());
    let wv: Value = serde_json::from_slice(&w.stdout).unwrap();
    assert_eq!(wv["woke_on"], "dep_missing");
    // Temizlik: uykudaki build + daemon.
    hbmon()
        .args(["kill", "--sock", &s, "--signal", "TERM"])
        .timeout(Duration::from_secs(10))
        .output()
        .unwrap();
    hbmon()
        .args(["shutdown", "--sock", &s])
        .timeout(Duration::from_secs(10))
        .output()
        .unwrap();
}

#[test]
fn wait_until_stall_suspect_returns_early() {
    let id = uuid("until-stall");
    let s = sock_for(&id);
    let out = hbmon()
        .args(watch_args(&id, sleep_cmd(45)))
        .timeout(Duration::from_secs(30))
        .output()
        .unwrap();
    assert!(out.status.success());
    wait_for_ready(&s);
    let start = std::time::Instant::now();
    let w = hbmon()
        .args([
            "wait",
            "--sock",
            &s,
            "--timeout",
            "120",
            "--until",
            "stall_suspect",
        ])
        .timeout(Duration::from_secs(150))
        .output()
        .unwrap();
    let el = start.elapsed().as_secs();
    assert!(w.status.success());
    let wv: Value = serde_json::from_slice(&w.stdout).unwrap();
    assert_eq!(wv["woke_on"], "stall_suspect");
    assert_eq!(wv["state"], "stalled");
    // Bitiş 45s'deydi; erken dönüldüğünün kanıtı.
    assert!(el < 45, "erken donmeliydi, {}s surdu", el);
    hbmon()
        .args(["shutdown", "--sock", &s])
        .timeout(Duration::from_secs(10))
        .output()
        .unwrap();
}
