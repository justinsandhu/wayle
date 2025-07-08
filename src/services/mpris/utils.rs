use std::time::Duration;

/// Convert MPRIS position in microseconds to Duration
pub fn from_mpris_micros(micros: i64) -> Duration {
    if micros < 0 {
        Duration::ZERO
    } else {
        Duration::from_micros(micros as u64)
    }
}

/// Convert Duration to MPRIS position in microseconds
pub fn to_mpris_micros(duration: Duration) -> i64 {
    duration.as_micros() as i64
}