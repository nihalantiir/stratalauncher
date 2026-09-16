//! Tiny shared helper for converting filesystem timestamps to RFC3339,
//! used anywhere we report file mtimes to the frontend (worlds, logs).

use std::time::{SystemTime, UNIX_EPOCH};

pub fn system_time_to_rfc3339(t: SystemTime) -> Option<String> {
    let dur = t.duration_since(UNIX_EPOCH).ok()?;
    chrono::DateTime::from_timestamp(dur.as_secs() as i64, dur.subsec_nanos()).map(|d| d.to_rfc3339())
}
