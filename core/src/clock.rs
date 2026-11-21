//! Suspend-safe monotonic time.
//!
//! The ticktimer's `elapsed_ms` is the time base (it never jumps, never goes
//! backwards), but it stops while the device is suspended. The local wall
//! clock (`llio::LocalTime`) is re-synced from the RTC on resume, so when the
//! wall clock advances by more than the ticktimer between two readings, the
//! difference is time spent suspended and is added to the offset.

/// Gaps shorter than this are measurement noise, not a suspend.
pub const SUSPEND_THRESHOLD_MS: u64 = 2_000;

#[derive(Clone, Debug, Default)]
pub struct SuspendSafeClock {
    last_tick: Option<u64>,
    last_wall: Option<u64>,
    offset: u64,
}

impl SuspendSafeClock {
    /// Feed one reading: `tick` from ticktimer `elapsed_ms`, `wall` from the
    /// local clock (or `None` if it isn't set). Returns monotonic ms.
    pub fn now(&mut self, tick: u64, wall: Option<u64>) -> u64 {
        if let (Some(lt), Some(lw), Some(w)) = (self.last_tick, self.last_wall, wall) {
            let tick_delta = tick.saturating_sub(lt);
            let wall_delta = w.saturating_sub(lw);
            if wall_delta > tick_delta + SUSPEND_THRESHOLD_MS {
                self.offset += wall_delta - tick_delta;
            }
        }
        self.last_tick = Some(tick);
        if wall.is_some() {
            self.last_wall = wall;
        }
        tick + self.offset
    }

    /// Total suspended time detected so far.
    pub fn offset(&self) -> u64 { self.offset }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn follows_ticks_while_awake() {
        let mut c = SuspendSafeClock::default();
        assert_eq!(c.now(1_000, Some(50_000)), 1_000);
        assert_eq!(c.now(2_000, Some(51_000)), 2_000);
        assert_eq!(c.now(2_500, Some(51_400)), 2_500, "small drift ignored");
    }

    #[test]
    fn adds_suspended_time() {
        // Audit: a countdown spanning a suspend ran long by the suspend time.
        let mut c = SuspendSafeClock::default();
        c.now(1_000, Some(100_000));
        // ten minutes asleep: ticktimer advanced 1 s, wall clock 601 s
        assert_eq!(c.now(2_000, Some(701_000)), 602_000);
        assert_eq!(c.offset(), 600_000);
        assert_eq!(c.now(3_000, Some(702_000)), 603_000, "offset sticks");
    }

    #[test]
    fn wall_clock_set_back_is_ignored() {
        let mut c = SuspendSafeClock::default();
        c.now(1_000, Some(100_000));
        assert_eq!(c.now(2_000, Some(10_000)), 2_000);
        assert_eq!(c.now(3_000, Some(11_000)), 3_000);
    }

    #[test]
    fn works_without_wall_clock() {
        let mut c = SuspendSafeClock::default();
        assert_eq!(c.now(5, None), 5);
        assert_eq!(c.now(10, None), 10);
        // clock becomes available later: no spurious jump
        assert_eq!(c.now(20, Some(1_000_000)), 20);
        assert_eq!(c.now(30, Some(1_000_010)), 30);
    }
}
