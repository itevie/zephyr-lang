pub mod colors;
pub mod fs;

pub fn format_duration(nanos: u128) -> String {
    if nanos >= 1_000_000_000 {
        format!("{:.3} s", nanos as f64 / 1_000_000_000.0) // Convert to seconds
    } else if nanos >= 1_000_000 {
        format!("{:.3} ms", nanos as f64 / 1_000_000.0) // Convert to milliseconds
    } else if nanos >= 1_000 {
        format!("{:.3} µs", nanos as f64 / 1_000.0) // Convert to microseconds
    } else {
        format!("{} ns", nanos) // Keep as nanoseconds
    }
}
