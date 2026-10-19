//! Local wall-clock time of day. macOS reads the system time zone through libc's `localtime_r`
//! (no time-zone crate — libc is already linked there); other targets aren't shipped and use UTC.

/// Seconds since local midnight at Unix time `t`.
#[cfg(target_os = "macos")]
pub fn secs_of_day(t: i64) -> i64 {
    let secs = t as libc::time_t;
    // SAFETY: `tm` is zero-initialised before the call; `localtime_r` only writes into it and
    // returns null on failure, which falls back to UTC.
    unsafe {
        let mut tm: libc::tm = std::mem::zeroed();
        if libc::localtime_r(&secs, &mut tm).is_null() {
            return t.rem_euclid(86_400);
        }
        i64::from(tm.tm_hour) * 3600 + i64::from(tm.tm_min) * 60 + i64::from(tm.tm_sec)
    }
}

/// Non-macOS is not a shipped target; UTC keeps the CI build honest about everything else.
#[cfg(not(target_os = "macos"))]
pub fn secs_of_day(t: i64) -> i64 {
    t.rem_euclid(86_400)
}
