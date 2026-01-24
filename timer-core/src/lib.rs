//! Pure timing logic library with no platform dependencies.
//! Testable on host, usable on Xous target.

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TimerState {
    Stopped,
    Running,
    Paused,
    Expired,
}

pub struct TimerCore {
    pub state: TimerState,
    accumulated_ms: u64,
    segment_start_ms: u64,
    target_ms: Option<u64>,
}

impl TimerCore {
    pub fn new_stopwatch() -> Self {
        Self {
            state: TimerState::Stopped,
            accumulated_ms: 0,
            segment_start_ms: 0,
            target_ms: None,
        }
    }

    pub fn new_countdown(duration_ms: u64) -> Self {
        Self {
            state: TimerState::Stopped,
            accumulated_ms: 0,
            segment_start_ms: 0,
            target_ms: Some(duration_ms),
        }
    }

    pub fn start(&mut self, now_ms: u64) {
        if self.state == TimerState::Running {
            return;
        }
        self.segment_start_ms = now_ms;
        self.state = TimerState::Running;
    }

    pub fn pause(&mut self, now_ms: u64) {
        if self.state != TimerState::Running {
            return;
        }
        self.accumulated_ms += now_ms.saturating_sub(self.segment_start_ms);
        self.state = TimerState::Paused;
    }

    pub fn reset(&mut self) {
        self.accumulated_ms = 0;
        self.segment_start_ms = 0;
        self.state = TimerState::Stopped;
    }

    pub fn elapsed_ms(&self, now_ms: u64) -> u64 {
        match self.state {
            TimerState::Running => {
                self.accumulated_ms + now_ms.saturating_sub(self.segment_start_ms)
            }
            _ => self.accumulated_ms,
        }
    }

    pub fn remaining_ms(&self, now_ms: u64) -> Option<u64> {
        self.target_ms.map(|target| {
            target.saturating_sub(self.elapsed_ms(now_ms))
        })
    }

    pub fn is_expired(&self, now_ms: u64) -> bool {
        match self.target_ms {
            Some(target) => self.elapsed_ms(now_ms) >= target,
            None => false,
        }
    }

    pub fn lap(&mut self, now_ms: u64) -> u64 {
        if self.state != TimerState::Running {
            return 0;
        }
        let elapsed = self.elapsed_ms(now_ms);
        // Reset accumulator but keep running from now
        self.accumulated_ms = 0;
        self.segment_start_ms = now_ms;
        elapsed
    }

    pub fn target_ms(&self) -> Option<u64> {
        self.target_ms
    }
}

/// Format milliseconds as "HH:MM:SS"
pub fn format_hms(ms: u64) -> String {
    let total_secs = ms / 1000;
    let h = total_secs / 3600;
    let m = (total_secs % 3600) / 60;
    let s = total_secs % 60;
    format!("{:02}:{:02}:{:02}", h, m, s)
}

/// Format milliseconds as "HH:MM:SS.cs" (centiseconds)
pub fn format_hms_cs(ms: u64) -> String {
    let total_secs = ms / 1000;
    let cs = (ms % 1000) / 10;
    let h = total_secs / 3600;
    let m = (total_secs % 3600) / 60;
    let s = total_secs % 60;
    format!("{:02}:{:02}:{:02}.{:02}", h, m, s, cs)
}

/// Format milliseconds as "MM:SS" (for pomodoro/countdown)
pub fn format_ms(ms: u64) -> String {
    let total_secs = ms / 1000;
    let m = total_secs / 60;
    let s = total_secs % 60;
    format!("{:02}:{:02}", m, s)
}

/// Serialize a u64 to 8 bytes (little-endian)
pub fn serialize_u64(val: u64) -> [u8; 8] {
    val.to_le_bytes()
}

/// Deserialize a u64 from bytes (little-endian)
pub fn deserialize_u64(bytes: &[u8]) -> u64 {
    if bytes.len() < 8 {
        return 0;
    }
    let mut buf = [0u8; 8];
    buf.copy_from_slice(&bytes[..8]);
    u64::from_le_bytes(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stopwatch_basic() {
        let mut sw = TimerCore::new_stopwatch();
        assert_eq!(sw.state, TimerState::Stopped);
        assert_eq!(sw.elapsed_ms(0), 0);

        sw.start(1000);
        assert_eq!(sw.state, TimerState::Running);
        assert_eq!(sw.elapsed_ms(1500), 500);
        assert_eq!(sw.elapsed_ms(2000), 1000);

        sw.pause(2000);
        assert_eq!(sw.state, TimerState::Paused);
        assert_eq!(sw.elapsed_ms(5000), 1000); // Stays at 1000 when paused

        sw.start(5000);
        assert_eq!(sw.elapsed_ms(5500), 1500);

        sw.reset();
        assert_eq!(sw.state, TimerState::Stopped);
        assert_eq!(sw.elapsed_ms(10000), 0);
    }

    #[test]
    fn test_countdown_basic() {
        let mut cd = TimerCore::new_countdown(10_000); // 10 seconds
        assert_eq!(cd.remaining_ms(0), Some(10_000));

        cd.start(1000);
        assert_eq!(cd.remaining_ms(1000), Some(10_000));
        assert_eq!(cd.remaining_ms(6000), Some(5_000));
        assert!(!cd.is_expired(6000));

        assert_eq!(cd.remaining_ms(11_000), Some(0));
        assert!(cd.is_expired(11_000));
    }

    #[test]
    fn test_lap() {
        let mut sw = TimerCore::new_stopwatch();
        sw.start(0);

        let lap1 = sw.lap(5000);
        assert_eq!(lap1, 5000);
        assert_eq!(sw.elapsed_ms(5000), 0); // Reset after lap

        let lap2 = sw.lap(8000);
        assert_eq!(lap2, 3000);
    }

    #[test]
    fn test_format_hms() {
        assert_eq!(format_hms(0), "00:00:00");
        assert_eq!(format_hms(61_000), "00:01:01");
        assert_eq!(format_hms(3661_000), "01:01:01");
    }

    #[test]
    fn test_format_hms_cs() {
        assert_eq!(format_hms_cs(0), "00:00:00.00");
        assert_eq!(format_hms_cs(12_340), "00:00:12.34");
    }

    #[test]
    fn test_format_ms() {
        assert_eq!(format_ms(0), "00:00");
        assert_eq!(format_ms(1_500_000), "25:00");
        assert_eq!(format_ms(300_000), "05:00");
    }

    #[test]
    fn test_serialize_deserialize() {
        let val = 123456789u64;
        let bytes = serialize_u64(val);
        assert_eq!(deserialize_u64(&bytes), val);
    }
}
