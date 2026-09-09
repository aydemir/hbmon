pub fn rss_mb_from_pages(pages: i64) -> u32 {
    let ps = unsafe { libc::sysconf(libc::_SC_PAGESIZE) as i64 }.max(4096);
    ((pages.max(0) * ps) / (1024 * 1024)) as u32
}
