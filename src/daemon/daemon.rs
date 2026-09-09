//! Katman 1 (lifecycle) + Katman 2 (UDS/JSONL) + health orchestration.
//!
//! watch flow:
//!  1. parent builds cfg, then (detach) double-forks
//!  2. daemon: pidfile + EventLogger + UDS thread + child spawn
//!     (child stdout/stderr -> .out file redirect, deadlock-free;
//!     dep-scan tails .out)
//!  3. 500ms loop: tree metrics, stall/OOM/dep/timeout, eventlog append
//!  4. child exit -> exit event, linger 60s for wait/status, cleanup.

use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::eventlog::{events, EventLogger};
use crate::health::{dep_missing, oom, stall::StallDetector, timeout::TimeoutWatchdog, State};
use crate::ipc::{error_response, protocol::ok_response};
use crate::metrics::CpuTracker;
use crate::platform::inspector;
use crate::proc::collect_descendants;
use crate::util::time::{now_iso, now_secs};

use super::pidfile;
use super::signals::install_daemon_posture;

#[derive(Debug, Clone)]
pub struct MonitorConfig {
    pub uuid: String,
    pub sock: PathBuf,
    pub log: PathBuf,
    pub pidfile: PathBuf,
    pub out: PathBuf,
    pub cmd: Vec<String>,
    pub timeout_sec: Option<u64>,
    pub label: Option<String>,
    pub workdir: PathBuf,
}

impl MonitorConfig {
    pub fn new(
        uuid: String,
        sock: Option<PathBuf>,
        log: Option<PathBuf>,
        cmd: Vec<String>,
        timeout_sec: Option<u64>,
        label: Option<String>,
    ) -> Self {
        let sock = sock.unwrap_or_else(|| PathBuf::from(format!("/tmp/hbmon-{}.sock", uuid)));
        let log = log.unwrap_or_else(|| PathBuf::from(format!("/tmp/hbmon-{}.jsonl", uuid)));
        let pidfile = PathBuf::from(format!("/tmp/hbmon-{}.pid", uuid));
        let out = PathBuf::from(format!("/tmp/hbmon-{}.out", uuid));
        // Spawn cwd'sini dondur: daemon kendisi /'ye taşınır ama
        // çocuk build, watch'in verildiği dizinde çalışmalıdır.
        let workdir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/tmp"));
        Self {
            uuid,
            sock,
            log,
            pidfile,
            out,
            cmd,
            timeout_sec,
            label,
            workdir,
        }
    }
}

struct Shared {
    uuid: String,
    root_cmd: String,
    started_iso: String,
    started_secs: f64,
    state: Mutex<State>,
    build_code: Mutex<Option<i32>>,
    exit_secs: Mutex<Option<f64>>,
    root_pid: Mutex<Option<u32>>,
    dep_info: Mutex<Option<Value>>,
    oom_info: Mutex<Option<Value>>,
    stall_threshold: Mutex<f32>,
    stall_score: Mutex<f32>,
    last_io_at: Mutex<String>,
    last_cpu_at: Mutex<String>,
    last_spawn_at: Mutex<String>,
    last_event: Mutex<Value>,
    totals: Mutex<HashMap<String, Value>>,
    shutdown: Mutex<bool>,
}

/// Public entry from CLI: optionally daemonize, then run monitor.
pub fn spawn_watch(cfg: MonitorConfig, detach: bool) -> Result<MonitorConfig, String> {
    if cfg.sock.exists() {
        // live monitor? refuse double-spawn (RFC 13.2.1); stale socket? remove
        if std::os::unix::net::UnixStream::connect(&cfg.sock).is_ok() {
            return Err(format!(
                "MONITOR_ALREADY_EXISTS: {} is live; use a fresh --uuid",
                cfg.sock.display()
            ));
        } else {
            std::fs::remove_file(&cfg.sock).ok();
        }
    }
    if !detach {
        run_daemon(cfg.clone())?;
        return Ok(cfg);
    }
    daemonize()?;
    install_daemon_posture();
    let _ = std::env::set_current_dir("/");
    unsafe { libc::umask(0o077) };
    match run_daemon(cfg.clone()) {
        Ok(_) => std::process::exit(0),
        Err(_) => std::process::exit(3),
    }
}

fn daemonize() -> Result<(), String> {
    unsafe {
        let p1 = libc::fork();
        if p1 < 0 {
            return Err("fork(1) failed".to_string());
        }
        if p1 > 0 {
            std::process::exit(0);
        }
        if libc::setsid() == -1 {
            return Err("setsid failed".to_string());
        }
        let p2 = libc::fork();
        if p2 < 0 {
            return Err("fork(2) failed".to_string());
        }
        if p2 > 0 {
            std::process::exit(0);
        }
        libc::close(0);
        libc::close(1);
        libc::close(2);
        let fd = libc::open(b"/dev/null\0".as_ptr() as *const libc::c_char, libc::O_RDWR);
        if fd >= 0 {
            libc::dup2(fd, 0);
            libc::dup2(fd, 1);
            libc::dup2(fd, 2);
            if fd > 2 {
                libc::close(fd);
            }
        }
    }
    Ok(())
}

pub fn run_daemon(cfg: MonitorConfig) -> Result<(), String> {
    let logger = EventLogger::create(&cfg.log)?;
    let insp = inspector();
    let started_iso = now_iso();
    let started_secs = now_secs();

    if cfg.cmd.is_empty() {
        return Err("no command to watch".to_string());
    }

    // Child stdio -> .out file (append, 0600). No pipes => no 64KB deadlock.
    let out_file = OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(&cfg.out)
        .map_err(|e| format!("open out {}: {}", cfg.out.display(), e))?;
    std::fs::set_permissions(&cfg.out, std::fs::Permissions::from_mode(0o600)).ok();
    let err_file = out_file.try_clone().map_err(|e| e.to_string())?;

    let mut cmd = Command::new(&cfg.cmd[0]);
    cmd.args(&cfg.cmd[1..]);
    cmd.current_dir(&cfg.workdir);
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::from(out_file));
    cmd.stderr(Stdio::from(err_file));
    unsafe {
        use std::os::unix::process::CommandExt;
        cmd.pre_exec(|| {
            libc::setpgid(0, 0);
            Ok(())
        });
    }
    let mut child = cmd.spawn().map_err(|e| format!("spawn {}: {}", cfg.cmd[0], e))?;
    let child_pid = child.id();
    let root_cmd = cfg.cmd.join(" ");
    // cgroup v2 handle (once; path is stable for process lifetime).
    // None on v1/Android/macOS — /proc accounting covers those.
    let cg = crate::metrics::cgroup::locate(child_pid);
    let mut oom_base: u64 = cg
        .as_ref()
        .and_then(crate::metrics::cgroup::read_stats)
        .map(|s| s.oom_kills)
        .unwrap_or(0);

    pidfile::write_pidfile(&cfg.pidfile, std::process::id())?;

    let shared = Arc::new(Shared {
        uuid: cfg.uuid.clone(),
        root_cmd: root_cmd.clone(),
        started_iso: started_iso.clone(),
        started_secs,
        state: Mutex::new(State::Running),
        build_code: Mutex::new(None),
        exit_secs: Mutex::new(None),
        root_pid: Mutex::new(Some(child_pid)),
        dep_info: Mutex::new(None),
        oom_info: Mutex::new(None),
        stall_threshold: Mutex::new(30.0),
        stall_score: Mutex::new(0.0),
        last_io_at: Mutex::new(started_iso.clone()),
        last_cpu_at: Mutex::new(started_iso.clone()),
        last_spawn_at: Mutex::new(started_iso.clone()),
        last_event: Mutex::new(json!({"ev":"ready"})),
        totals: Mutex::new(HashMap::new()),
        shutdown: Mutex::new(false),
    });

    let ready = events::new_event(
        "ready",
        &cfg.uuid,
        events::kv(&[
            ("sock", json!(cfg.sock.to_string_lossy())),
            ("log", json!(cfg.log.to_string_lossy())),
            ("root_pid", json!(child_pid)),
            ("cmd", json!(root_cmd)),
            ("workdir", json!(cfg.workdir.to_string_lossy())),
        ]),
    );
    logger.append(&ready);
    *shared.last_event.lock().unwrap() = ready.clone();

    {
        let shared = shared.clone();
        let logger_path = cfg.log.clone();
        let sock = cfg.sock.clone();
        std::thread::spawn(move || {
            let handler = Arc::new(move |req: Value| dispatch(req, &shared, &logger_path));
            let _ = crate::ipc::serve(&sock, handler);
        });
    }

    let mut stall = StallDetector::new();
    let watchdog = TimeoutWatchdog::new(cfg.timeout_sec);
    let mut tracker = CpuTracker::new();
    let mut prev_io: u64 = 0;
    let mut prev_count: usize = 0;
    let mut last_poll = Instant::now();
    let mut last_metric_ev = Instant::now() - Duration::from_secs(10);
    let mut last_oom_poll = Instant::now() - Duration::from_secs(10);
    let mut dep_found: Option<DepMatch> = None;

    std::thread::sleep(Duration::from_millis(200));

    loop {
        if *shared.shutdown.lock().unwrap() {
            logger.append(&events::new_event(
                "shutdown",
                &cfg.uuid,
                events::kv(&[("reason", json!("uds shutdown"))]),
            ));
            break;
        }
        let dt = last_poll.elapsed().as_secs_f32().max(0.05);
        last_poll = Instant::now();

        let alive = insp.is_alive(child_pid);
        let exit_code = child
            .try_wait()
            .ok()
            .flatten()
            .map(|s| s.code().unwrap_or(1));

        let pids = collect_descendants(&*insp, child_pid, 1000);
        let bulk = insp.bulk_metrics(&pids).unwrap_or_default();
        // CPU deltas: CpuTracker converts jiffies -> % per pid (Linux).
        // v0.1 review fix: tracker output used to be discarded
        // (`let _ = pct`) while bulk cpu_pct is always 0.0, so tot_cpu
        // stayed 0 and the stall detector lost its CPU leg.
        let mut cpu_by_pid: HashMap<u32, f32> = HashMap::new();
        #[cfg(target_os = "linux")]
        {
            for &p in &pids {
                if let Some(j) = crate::proc::linux::cpu_jiffies(p) {
                    cpu_by_pid.insert(p, tracker.update(p, j));
                }
            }
            tracker.evict_gone(&pids);
        }
        let mut tot_cpu = 0f32;
        let mut tot_rss = 0u32;
        let mut tot_r: u64 = 0;
        let mut tot_w: u64 = 0;
        let mut tot_fds = 0u32;
        let mut tot_tcp = 0u32;
        for (pid, m) in bulk.iter() {
            tot_cpu += cpu_by_pid.get(pid).copied().unwrap_or(m.cpu_pct);
            tot_rss = tot_rss.saturating_add(m.rss_mb);
            tot_r = tot_r.saturating_add(m.io_read_bytes);
            tot_w = tot_w.saturating_add(m.io_write_bytes);
            tot_fds = tot_fds.saturating_add(m.fds_open);
            tot_tcp = tot_tcp.saturating_add(m.net_tcp);
        }
        {
            let mut map = shared.totals.lock().unwrap();
            map.insert("cpu_pct".into(), json!(tot_cpu));
            map.insert("rss_mb".into(), json!(tot_rss));
            map.insert("io_read_mb".into(), json!((tot_r / (1024 * 1024)) as u32));
            map.insert("io_write_mb".into(), json!((tot_w / (1024 * 1024)) as u32));
            map.insert("fds_open".into(), json!(tot_fds));
            map.insert("net_tcp".into(), json!(tot_tcp));
            map.insert("net_udp".into(), json!(0));
            map.insert("io_r".into(), json!(tot_r));
            map.insert("io_w".into(), json!(tot_w));
        }

        let io_total = tot_r.saturating_add(tot_w);
        let io_delta = io_total.saturating_sub(prev_io);
        prev_io = io_total;
        let new_child = pids.len() > prev_count;
        prev_count = pids.len();
        let active = tot_cpu > 0.5 || io_delta > 0 || new_child;
        if active {
            let now = now_iso();
            if tot_cpu > 0.5 {
                *shared.last_cpu_at.lock().unwrap() = now.clone();
            }
            if io_delta > 0 {
                *shared.last_io_at.lock().unwrap() = now;
            }
            if new_child {
                *shared.last_spawn_at.lock().unwrap() = now_iso();
            }
        }

        if last_metric_ev.elapsed() >= Duration::from_secs(5) {
            last_metric_ev = Instant::now();
            let ev = events::new_event(
                "metric",
                &cfg.uuid,
                events::kv(&[
                    ("cpu", json!(tot_cpu)),
                    ("rss_mb", json!(tot_rss)),
                    ("io_r", json!(tot_r)),
                    ("io_w", json!(tot_w)),
                    ("fds", json!(tot_fds)),
                ]),
            );
            logger.append(&ev);
            *shared.last_event.lock().unwrap() = ev;
        }

        let (entered, resolved) = stall.tick(active, dt);
        *shared.stall_score.lock().unwrap() = stall.score();
        *shared.stall_threshold.lock().unwrap() = stall.threshold();
        if entered {
            let ev = events::new_event(
                "stall_suspect",
                &cfg.uuid,
                events::kv(&[
                    ("reason", json!("no_io_no_cpu")),
                    ("idle_sec", json!(stall.idle())),
                    ("threshold_sec", json!(stall.threshold())),
                    ("p95_idle", json!(stall.p95_idle())),
                ]),
            );
            logger.append(&ev);
            *shared.last_event.lock().unwrap() = ev;
            *shared.state.lock().unwrap() = State::Stalled;
        }
        if resolved {
            let ev = events::new_event(
                "stall_resolved",
                &cfg.uuid,
                events::kv(&[("lasted_sec", json!(stall.idle()))]),
            );
            logger.append(&ev);
            *shared.last_event.lock().unwrap() = ev;
            *shared.state.lock().unwrap() = State::Running;
        }

        if dep_found.is_none() {
            if let Some(dm) = scan_out_for_dep(&cfg.out) {
                let ev = events::new_event(
                    "dep_missing",
                    &cfg.uuid,
                    events::kv(&[
                        ("pattern_id", json!(dm.pattern_id)),
                        ("category", json!(dm.category)),
                        ("match_text", json!(dm.match_text)),
                    ]),
                );
                logger.append(&ev);
                *shared.last_event.lock().unwrap() = ev.clone();
                *shared.dep_info.lock().unwrap() = Some(ev);
                dep_found = Some(dm);
            }
        }

        if last_oom_poll.elapsed() >= Duration::from_secs(5) {
            last_oom_poll = Instant::now();
            // cgroup v2 OOM: memory.events oom_kill delta (stronger than dmesg).
            if let Some(ref c) = cg {
                if let Some(st) = crate::metrics::cgroup::read_stats(c) {
                    shared.totals.lock().unwrap().insert(
                        "cgroup".to_string(),
                        json!({
                            "cpu_usec": st.cpu_usec,
                            "mem_bytes": st.mem_bytes,
                            "mem_peak": st.mem_peak,
                            "pids": st.pids,
                            "oom_kills": st.oom_kills,
                        }),
                    );
                    if st.oom_kills > oom_base {
                        oom_base = st.oom_kills;
                        let ev = events::new_event(
                            "oom_suspect",
                            &cfg.uuid,
                            events::kv(&[
                                ("killed_by", json!("cgroup oom-killer")),
                                ("oom_kills", json!(st.oom_kills)),
                            ]),
                        );
                        logger.append(&ev);
                        *shared.last_event.lock().unwrap() = ev.clone();
                        *shared.oom_info.lock().unwrap() = Some(ev);
                        if !alive {
                            *shared.state.lock().unwrap() = State::OomKilled;
                        }
                    }
                }
            }
            let set: HashSet<u32> = pids.iter().copied().collect();
            let hits = oom::check(&set);
            if let Some(h) = hits.first() {
                let ev = events::new_event(
                    "oom_suspect",
                    &cfg.uuid,
                    events::kv(&[
                        ("pid", json!(h.pid)),
                        ("killed_by", json!("oom-killer")),
                        ("text", json!(h.text.chars().take(300).collect::<String>())),
                    ]),
                );
                logger.append(&ev);
                *shared.last_event.lock().unwrap() = ev.clone();
                *shared.oom_info.lock().unwrap() = Some(ev);
                *shared.state.lock().unwrap() = State::OomKilled;
            }
        }

        if watchdog.expired() {
            kill_pgroup(child_pid, libc::SIGTERM);
            std::thread::sleep(Duration::from_secs(5));
            if insp.is_alive(child_pid) {
                kill_pgroup(child_pid, libc::SIGKILL);
            }
            let _ = child.wait();
            *shared.state.lock().unwrap() = State::Timeout;
            *shared.build_code.lock().unwrap() = Some(124);
            *shared.exit_secs.lock().unwrap() = Some(now_secs());
            let ev = events::new_event(
                "timeout",
                &cfg.uuid,
                events::kv(&[
                    ("elapsed_sec", json!(now_secs() - started_secs)),
                    ("limit_sec", json!(watchdog.limit())),
                ]),
            );
            logger.append(&ev);
            let exit_ev = events::new_event(
                "exit",
                &cfg.uuid,
                events::kv(&[
                    ("pid", json!(child_pid)),
                    ("code", json!(124)),
                    ("duration_sec", json!(now_secs() - started_secs)),
                    ("state", json!("timeout")),
                ]),
            );
            logger.append(&exit_ev);
            *shared.last_event.lock().unwrap() = exit_ev;
            break;
        }

        if let Some(code) = exit_code {
            let _ = child.wait();
            let duration = now_secs() - started_secs;
            let oom_hit = shared.oom_info.lock().unwrap().is_some();
            let (state, mapped) = if dep_found.is_some() {
                (State::DepMissing, 2)
            } else if oom_hit {
                (State::OomKilled, 137)
            } else if code == 0 {
                (State::Done, 0)
            } else {
                (State::Failed, 1)
            };
            *shared.state.lock().unwrap() = state;
            *shared.build_code.lock().unwrap() = Some(mapped);
            *shared.exit_secs.lock().unwrap() = Some(now_secs());
            let ev = events::new_event(
                "exit",
                &cfg.uuid,
                events::kv(&[
                    ("pid", json!(child_pid)),
                    ("code", json!(mapped)),
                    ("raw_code", json!(code)),
                    ("duration_sec", json!(duration)),
                    ("state", json!(state.as_str())),
                ]),
            );
            logger.append(&ev);
            *shared.last_event.lock().unwrap() = ev;
            break;
        }

        if !alive {
            *shared.state.lock().unwrap() = State::Failed;
            *shared.build_code.lock().unwrap() = Some(3);
            *shared.exit_secs.lock().unwrap() = Some(now_secs());
            let ev = events::new_event(
                "exit",
                &cfg.uuid,
                events::kv(&[
                    ("pid", json!(child_pid)),
                    ("code", json!(3)),
                    ("duration_sec", json!(now_secs() - started_secs)),
                    ("state", json!("failed")),
                ]),
            );
            logger.append(&ev);
            *shared.last_event.lock().unwrap() = ev;
            break;
        }

        std::thread::sleep(Duration::from_millis(500));
    }

    let linger_until = Instant::now() + Duration::from_secs(60);
    while Instant::now() < linger_until {
        if *shared.shutdown.lock().unwrap() {
            break;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    pidfile::remove(&cfg.pidfile);
    std::fs::remove_file(&cfg.sock).ok();
    Ok(())
}

struct DepMatch {
    pattern_id: String,
    category: String,
    match_text: String,
}

fn scan_out_for_dep(out: &Path) -> Option<DepMatch> {
    let f = std::fs::File::open(out).ok()?;
    let r = BufReader::new(f);
    let lines: Vec<String> = r.lines().flatten().collect();
    for line in lines.iter().rev().take(50) {
        if let Some(m) = dep_missing::match_line(line) {
            return Some(DepMatch {
                pattern_id: m.pattern_id,
                category: m.category,
                match_text: m.match_text,
            });
        }
    }
    None
}

#[allow(dead_code)]
fn kill_pgroup(pid: u32, sig: i32) {
    unsafe {
        libc::kill(-(pid as i32), sig);
    }
}

fn dispatch(req: Value, shared: &Shared, logger_path: &Path) -> Value {
    let op = req.get("op").and_then(|v| v.as_str()).unwrap_or("");
    let id = req
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    match op {
        "status" => status_snapshot(shared, &id, logger_path),
        "metrics" => {
            let t = shared.totals.lock().unwrap();
            let mut m = serde_json::Map::new();
            m.insert(
                "cpu".to_string(),
                t.get("cpu_pct").cloned().unwrap_or(json!(0.0)),
            );
            m.insert("rss_mb".to_string(), t.get("rss_mb").cloned().unwrap_or(json!(0)));
            m.insert("io_r".to_string(), t.get("io_r").cloned().unwrap_or(json!(0)));
            m.insert("io_w".to_string(), t.get("io_w").cloned().unwrap_or(json!(0)));
            m.insert("fds".to_string(), t.get("fds_open").cloned().unwrap_or(json!(0)));
            m.insert(
                "net_tcp".to_string(),
                t.get("net_tcp").cloned().unwrap_or(json!(0)),
            );
            ok_response(&id, m)
        }
        "log_tail" => {
            let n = req.get("n").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
            let lines = EventLogger::tail(logger_path, n);
            let mut m = serde_json::Map::new();
            m.insert("lines".to_string(), json!(lines));
            ok_response(&id, m)
        }
        "wait" => {
            let timeout = req
                .get("timeout_sec")
                .and_then(|v| v.as_f64())
                .unwrap_or(300.0);
            let poll_ms = req.get("poll_ms").and_then(|v| v.as_u64()).unwrap_or(500);
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
            let sig = req.get("signal").and_then(|v| v.as_i64()).unwrap_or(15) as i32;
            let pid = shared.root_pid.lock().unwrap().unwrap_or(0);
            if pid == 0 {
                return error_response(&id, "BUILD_NOT_FOUND", "no root pid");
            }
            unsafe {
                libc::kill(-(pid as i32), sig);
            }
            let mut m = serde_json::Map::new();
            m.insert("killed".to_string(), json!(true));
            ok_response(&id, m)
        }
        "shutdown" => {
            *shared.shutdown.lock().unwrap() = true;
            let force = req.get("force").and_then(|v| v.as_bool()).unwrap_or(false);
            let pid = shared.root_pid.lock().unwrap().unwrap_or(0);
            if pid != 0 {
                unsafe {
                    libc::kill(-(pid as i32), libc::SIGTERM);
                }
                if force {
                    std::thread::sleep(Duration::from_millis(200));
                    unsafe {
                        libc::kill(-(pid as i32), libc::SIGKILL);
                    }
                }
            }
            let mut m = serde_json::Map::new();
            m.insert("ok_shutdown".to_string(), json!(true));
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

/// `until` boşsa yalnızca terminal state'ler eşleşir (eski davranış).
/// Doluysa listedeki ilk sinyal kazanır; ara sinyaller (stall/dep/oom)
/// build bitmeden de eşleşebilir — erken dönüşün çekirdeği.
/// Bilinmeyen adlar yok sayılır.
fn wait_match(
    state: State,
    until: &[String],
    dep_hit: bool,
    oom_hit: bool,
) -> Option<String> {
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
    None
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
            wait_match(
                State::Stalled,
                &["stall_suspect".to_string()],
                false,
                false
            ),
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
}
