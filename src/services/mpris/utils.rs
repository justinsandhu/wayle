use std::time::Duration;

/// Convert Duration to MPRIS position in microseconds
pub fn to_mpris_micros(duration: Duration) -> i64 {
    duration.as_micros() as i64
}
