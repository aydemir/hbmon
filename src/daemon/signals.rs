/// Daemon signal posture (RFC 13.1.4):
/// SIGHUP ignored (harness may send it on exit; we are detached),
/// SIGPIPE ignored; SIGTERM/SIGINT left to default handling in the
/// monitor loop which installs its own graceful shutdown.
pub fn install_daemon_posture() {
    unsafe {
        libc::signal(libc::SIGHUP, libc::SIG_IGN);
        libc::signal(libc::SIGPIPE, libc::SIG_IGN);
    }
}

pub fn parse_signal(s: &str) -> Result<i32, String> {
    let u = s.to_uppercase();
    let v = u.strip_prefix("SIG").unwrap_or(&u);
    match v {
        "TERM" | "15" => Ok(libc::SIGTERM),
        "KILL" | "9" => Ok(libc::SIGKILL),
        "INT" | "2" => Ok(libc::SIGINT),
        "HUP" | "1" => Ok(libc::SIGHUP),
        _ => Err(format!("unknown signal: {}", s)),
    }
}
