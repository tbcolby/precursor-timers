//! Time and text formatting.

/// `MM:SS`, or `H:MM:SS` from one hour. Rounds up so a countdown shows
/// 00:01 until it really ends.
pub fn format_remaining(ms: u64) -> String { format_secs(ms.div_ceil(1000)) }

/// `MM:SS` or `H:MM:SS`, truncating.
pub fn format_duration(ms: u64) -> String { format_secs(ms / 1000) }

fn format_secs(total: u64) -> String {
    let (h, m, s) = (total / 3600, (total % 3600) / 60, total % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m:02}:{s:02}")
    }
}

/// Stopwatch reading with tenths: `MM:SS.t` or `H:MM:SS.t`.
pub fn format_tenths(ms: u64) -> String { format!("{}.{}", format_duration(ms), (ms % 1000) / 100) }

/// First `max` characters of `s` (never splits a character).
pub fn truncate_chars(s: &str, max: usize) -> String { s.chars().take(max).collect() }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations() {
        assert_eq!(format_duration(0), "00:00");
        assert_eq!(format_duration(61_999), "01:01");
        assert_eq!(format_duration(3_600_000), "1:00:00");
        assert_eq!(format_duration(36_000_000 + 61_000), "10:01:01");
    }

    #[test]
    fn remaining_rounds_up() {
        assert_eq!(format_remaining(0), "00:00");
        assert_eq!(format_remaining(1), "00:01");
        assert_eq!(format_remaining(1_500_000), "25:00");
        assert_eq!(format_remaining(1_499_001), "25:00");
    }

    #[test]
    fn tenths() {
        assert_eq!(format_tenths(12_340), "00:12.3");
        assert_eq!(format_tenths(3_723_950), "1:02:03.9");
    }

    #[test]
    fn truncation_is_char_safe() {
        // Audit: String::truncate(20) panicked mid-character.
        let name = "福".repeat(25);
        assert_eq!(truncate_chars(&name, 20).chars().count(), 20);
        assert_eq!(truncate_chars("ab", 20), "ab");
    }
}
