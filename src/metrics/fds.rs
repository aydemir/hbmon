pub fn count_open_fds(pid: u32) -> u32 {
    std::fs::read_dir(format!("/proc/{}/fd", pid))
        .map(|d| d.count() as u32)
        .unwrap_or(0)
}
