use std::time::{SystemTime, UNIX_EPOCH};

pub fn now_secs() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

/// ISO 8601 UTC timestamp, e.g. "2026-09-09T00:00:00Z".
/// Implemented via libc gmtime to avoid a chrono dependency
/// (binary-size budget: < 5MB, see RFC 9.2).
pub fn now_iso() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    format_epoch_iso(secs)
}

pub fn format_epoch_iso(secs: i64) -> String {
    unsafe {
        let t = secs as libc::time_t;
        let mut tm: libc::tm = std::mem::zeroed();
        if libc::gmtime_r(&t as *const _ as *mut _, &mut tm).is_null() {
            return "1970-01-01T00:00:00Z".to_string();
        }
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
            tm.tm_year + 1900,
            tm.tm_mon + 1,
            tm.tm_mday,
            tm.tm_hour,
            tm.tm_min,
            tm.tm_sec
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_zero_formats() {
        assert_eq!(format_epoch_iso(0), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn known_date_formats() {
        assert_eq!(format_epoch_iso(1788912000), "2026-09-09T00:00:00Z");
    }
}
