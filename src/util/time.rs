use std::time::{SystemTime, UNIX_EPOCH};

pub fn now_secs() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

/// ISO 8601 UTC timestamp, e.g. "2026-09-09T00:00:00Z".
/// Pure-Rust civil calendar (Howard Hinnant's days algorithm):
/// no `chrono` (binary-size budget, RFC 9.2), no `libc::gmtime_r`
/// (Windows'ta yok) — her platformda aynı sonuç.
pub fn now_iso() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    format_epoch_iso(secs)
}

pub fn format_epoch_iso(secs: i64) -> String {
    let (y, mo, d, hh, mm, ss) = civil_from_secs(secs);
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", y, mo, d, hh, mm, ss)
}

/// Unix epoch saniyesi → (yıl, ay, gün, saat, dak, sn).
/// `div_euclid`/`rem_euclid` ile 1970-öncesi de doğru.
fn civil_from_secs(secs: i64) -> (i64, u32, u32, u32, u32, u32) {
    let days = secs.div_euclid(86_400);
    let tod = secs.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    (
        y,
        m,
        d,
        (tod / 3_600) as u32,
        ((tod % 3_600) / 60) as u32,
        (tod % 60) as u32,
    )
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64; // [0, 146096]
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    (if m <= 2 { y + 1 } else { y }, m, d)
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

    #[test]
    fn leap_day_formats() {
        assert_eq!(format_epoch_iso(1582934400), "2020-02-29T00:00:00Z");
    }

    #[test]
    fn pre_epoch_formats() {
        assert_eq!(format_epoch_iso(-1), "1969-12-31T23:59:59Z");
        assert_eq!(format_epoch_iso(-86400), "1969-12-31T00:00:00Z");
    }

    #[test]
    fn end_of_day_formats() {
        assert_eq!(format_epoch_iso(86399), "1970-01-01T23:59:59Z");
    }
}
