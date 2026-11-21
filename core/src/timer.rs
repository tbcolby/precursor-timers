//! A pausable timer measured from timestamps. Time is never counted in pump
//! ticks: every reading is `accumulated + (now - segment_start)`, so a late or
//! missing tick changes when the screen updates, not how much time passed.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TimerState {
    /// Never started since the last reset.
    Stopped,
    Running,
    Paused,
}

#[derive(Clone, Debug)]
pub struct TimerCore {
    state: TimerState,
    accumulated_ms: u64,
    segment_start_ms: u64,
    target_ms: Option<u64>,
}

impl TimerCore {
    pub fn new_stopwatch() -> Self { Self { state: TimerState::Stopped, accumulated_ms: 0, segment_start_ms: 0, target_ms: None } }

    pub fn new_countdown(duration_ms: u64) -> Self { Self { target_ms: Some(duration_ms), ..Self::new_stopwatch() } }

    pub fn state(&self) -> TimerState { self.state }

    pub fn is_running(&self) -> bool { self.state == TimerState::Running }

    /// Start or resume. No effect while running.
    pub fn start(&mut self, now_ms: u64) {
        if self.state != TimerState::Running {
            self.segment_start_ms = now_ms;
            self.state = TimerState::Running;
        }
    }

    /// No effect unless running.
    pub fn pause(&mut self, now_ms: u64) {
        if self.state == TimerState::Running {
            self.accumulated_ms = self.accumulated_ms.saturating_add(now_ms.saturating_sub(self.segment_start_ms));
            self.state = TimerState::Paused;
        }
    }

    /// Start when stopped or paused, pause when running.
    pub fn toggle(&mut self, now_ms: u64) {
        if self.is_running() {
            self.pause(now_ms);
        } else {
            self.start(now_ms);
        }
    }

    pub fn reset(&mut self) {
        self.accumulated_ms = 0;
        self.segment_start_ms = 0;
        self.state = TimerState::Stopped;
    }

    pub fn elapsed_ms(&self, now_ms: u64) -> u64 {
        match self.state {
            TimerState::Running => self.accumulated_ms.saturating_add(now_ms.saturating_sub(self.segment_start_ms)),
            _ => self.accumulated_ms,
        }
    }

    pub fn remaining_ms(&self, now_ms: u64) -> Option<u64> { self.target_ms.map(|t| t.saturating_sub(self.elapsed_ms(now_ms))) }

    pub fn is_expired(&self, now_ms: u64) -> bool { self.target_ms.is_some_and(|t| self.elapsed_ms(now_ms) >= t) }

    pub fn target_ms(&self) -> Option<u64> { self.target_ms }

    /// Fraction of the target elapsed, 0.0 to 1.0 (0 for stopwatches).
    pub fn progress(&self, now_ms: u64) -> f32 {
        match self.target_ms {
            Some(0) => 1.0,
            Some(t) => (self.elapsed_ms(now_ms) as f32 / t as f32).min(1.0),
            None => 0.0,
        }
    }
}

/// Maximum laps kept.
pub const MAX_LAPS: usize = 99;

/// Stopwatch with laps. A lap records a split; the total keeps running
/// (0.1 zeroed the total on every lap).
#[derive(Clone, Debug)]
pub struct Stopwatch {
    pub timer: TimerCore,
    /// Split times, oldest first.
    laps: Vec<u64>,
    /// Total at the last lap.
    last_lap_total: u64,
}

impl Default for Stopwatch {
    fn default() -> Self { Self { timer: TimerCore::new_stopwatch(), laps: Vec::new(), last_lap_total: 0 } }
}

impl Stopwatch {
    pub fn total_ms(&self, now_ms: u64) -> u64 { self.timer.elapsed_ms(now_ms) }

    /// Time since the last lap (or start).
    pub fn split_ms(&self, now_ms: u64) -> u64 { self.total_ms(now_ms).saturating_sub(self.last_lap_total) }

    pub fn laps(&self) -> &[u64] { &self.laps }

    /// Record a lap while running. Returns false if not running or full.
    pub fn lap(&mut self, now_ms: u64) -> bool {
        if !self.timer.is_running() || self.laps.len() >= MAX_LAPS {
            return false;
        }
        let total = self.total_ms(now_ms);
        self.laps.push(total - self.last_lap_total);
        self.last_lap_total = total;
        true
    }

    /// Reset when not running.
    pub fn reset(&mut self) -> bool {
        if self.timer.is_running() {
            return false;
        }
        *self = Self::default();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pause_resume_accumulates_over_segments() {
        let mut t = TimerCore::new_stopwatch();
        t.start(1_000);
        t.pause(3_000); // 2 s
        assert_eq!(t.elapsed_ms(10_000), 2_000, "paused time doesn't count");
        t.start(10_000);
        t.pause(10_500); // +0.5 s
        t.start(20_000);
        assert_eq!(t.elapsed_ms(21_000), 3_500);
        t.pause(21_000);
        t.pause(99_000); // second pause is ignored
        assert_eq!(t.elapsed_ms(99_000), 3_500);
        t.start(50_000);
        t.start(60_000); // second start doesn't move the segment
        assert_eq!(t.elapsed_ms(60_000), 13_500);
    }

    #[test]
    fn countdown_paused_never_expires() {
        let mut c = TimerCore::new_countdown(10_000);
        c.start(0);
        c.pause(4_000);
        assert!(!c.is_expired(1_000_000));
        assert_eq!(c.remaining_ms(1_000_000), Some(6_000));
        c.start(2_000_000);
        assert!(!c.is_expired(2_005_999));
        assert!(c.is_expired(2_006_000));
        assert_eq!(c.remaining_ms(3_000_000), Some(0));
    }

    #[test]
    fn stopped_countdown_starts_on_toggle() {
        // Audit: after reset the countdown was Stopped and Enter did nothing.
        let mut c = TimerCore::new_countdown(5_000);
        c.start(0);
        c.reset();
        assert_eq!(c.state(), TimerState::Stopped);
        c.toggle(100);
        assert!(c.is_running());
        c.toggle(1_100);
        assert_eq!(c.state(), TimerState::Paused);
        assert_eq!(c.remaining_ms(9_999), Some(4_000));
    }

    #[test]
    fn zero_length_countdown_is_expired_and_full_progress() {
        let c = TimerCore::new_countdown(0);
        assert!(c.is_expired(0));
        assert_eq!(c.progress(0), 1.0);
    }

    #[test]
    fn clock_going_backwards_does_not_underflow() {
        let mut t = TimerCore::new_stopwatch();
        t.start(5_000);
        assert_eq!(t.elapsed_ms(4_000), 0);
        t.pause(4_000);
        assert_eq!(t.elapsed_ms(4_000), 0);
    }

    #[test]
    fn laps_keep_total_running() {
        // Audit: lap() zeroed the accumulator, so the total was lost.
        let mut s = Stopwatch::default();
        assert!(!s.lap(0), "not running");
        s.timer.start(0);
        assert!(s.lap(3_000));
        assert_eq!(s.total_ms(4_000), 4_000);
        assert_eq!(s.split_ms(4_000), 1_000);
        s.timer.pause(5_000);
        s.timer.start(9_000);
        assert!(s.lap(10_000)); // 2 s of running since the last lap
        assert_eq!(s.laps(), [3_000, 3_000]);
        assert_eq!(s.total_ms(10_000), 6_000);
        assert!(!s.reset(), "can't reset while running");
        s.timer.pause(10_000);
        assert!(s.reset());
        assert!(s.laps().is_empty());
        assert_eq!(s.total_ms(20_000), 0);
    }

    #[test]
    fn laps_capped() {
        let mut s = Stopwatch::default();
        s.timer.start(0);
        for i in 1..=MAX_LAPS as u64 {
            assert!(s.lap(i * 10));
        }
        assert!(!s.lap(10_000));
        assert_eq!(s.laps().len(), MAX_LAPS);
    }
}
