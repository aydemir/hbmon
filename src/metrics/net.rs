pub fn count_tcp_for_pid(pid: u32) -> u32 {
    let dir = match std::fs::read_dir(format!("/proc/{}/fd", pid)) {
        Ok(d) => d,
        Err(_) => return 0,
    };
    let mut n = 0u32;
    for e in dir.flatten() {
        if let Ok(link) = std::fs::read_link(e.path()) {
            if link.to_string_lossy().starts_with("socket:") {
                n += 1;
            }
        }
    }
    n
}
