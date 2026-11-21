//! Pomodoro cycle: work, short break, ..., long break after N work sessions.

use serde::{Deserialize, Serialize};

use crate::timer::TimerCore;

/// Allowed minutes for each phase.
pub const MINUTES: core::ops::RangeInclusive<u32> = 1..=180;
/// Allowed work sessions before a long break.
pub const CYCLES: core::ops::RangeInclusive<u32> = 1..=12;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PomodoroSettings {
    pub work_min: u32,
    pub short_min: u32,
    pub long_min: u32,
    pub cycles: u32,
}

impl Default for PomodoroSettings {
    fn default() -> Self { Self { work_min: 25, short_min: 5, long_min: 15, cycles: 4 } }
}

impl PomodoroSettings {
    /// Clamp every field into its allowed range (0.1 accepted 0 minutes,
    /// which expired on every tick and raised alerts forever).
    pub fn validated(self) -> Self {
        let m = |v: u32| v.clamp(*MINUTES.start(), *MINUTES.end());
        Self {
            work_min: m(self.work_min),
            short_min: m(self.short_min),
            long_min: m(self.long_min),
            cycles: self.cycles.clamp(*CYCLES.start(), *CYCLES.end()),
        }
    }
}

/// Minutes to ms. Inputs are validated (≤ 180), so this can't overflow.
pub fn minutes_ms(m: u32) -> u64 { u64::from(m) * 60_000 }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    Work,
    ShortBreak,
    LongBreak,
}

impl Phase {
    pub fn label(self) -> &'static str {
        match self {
            Phase::Work => "Work",
            Phase::ShortBreak => "Short break",
            Phase::LongBreak => "Long break",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Pomodoro {
    pub timer: TimerCore,
    phase: Phase,
    settings: PomodoroSettings,
    /// Work sessions finished in the current set (resets after a long break).
    cycle: u32,
    /// Work sessions finished since the app started.
    completed: u32,
}

impl Pomodoro {
    pub fn new(settings: PomodoroSettings) -> Self {
        let settings = settings.validated();
        Self { timer: TimerCore::new_countdown(minutes_ms(settings.work_min)), phase: Phase::Work, settings, cycle: 0, completed: 0 }
    }

    pub fn phase(&self) -> Phase { self.phase }

    pub fn settings(&self) -> PomodoroSettings { self.settings }

    pub fn cycle(&self) -> u32 { self.cycle }

    pub fn completed(&self) -> u32 { self.completed }

    fn phase_ms(&self, phase: Phase) -> u64 {
        minutes_ms(match phase {
            Phase::Work => self.settings.work_min,
            Phase::ShortBreak => self.settings.short_min,
            Phase::LongBreak => self.settings.long_min,
        })
    }

    /// The phase ended: move to the next one and start it at `now`.
    /// Returns the alert text.
    pub fn advance(&mut self, now: u64) -> String {
        let (next, msg) = match self.phase {
            Phase::Work => {
                self.cycle += 1;
                self.completed += 1;
                if self.cycle >= self.settings.cycles {
                    (Phase::LongBreak, "Work done. Long break")
                } else {
                    (Phase::ShortBreak, "Work done. Short break")
                }
            }
            Phase::ShortBreak => (Phase::Work, "Break over. Back to work"),
            Phase::LongBreak => {
                self.cycle = 0;
                (Phase::Work, "Long break over. Back to work")
            }
        };
        self.phase = next;
        self.timer = TimerCore::new_countdown(self.phase_ms(next));
        self.timer.start(now);
        format!("{msg} ({} min).", self.phase_ms(next) / 60_000)
    }

    /// Back to the start of the current phase, stopped.
    pub fn reset(&mut self) { self.timer = TimerCore::new_countdown(self.phase_ms(self.phase)); }

    /// New settings apply to the next phase; a phase that hasn't started
    /// yet takes the new length now.
    pub fn apply_settings(&mut self, s: PomodoroSettings) {
        self.settings = s.validated();
        if self.timer.state() == crate::timer::TimerState::Stopped {
            self.reset();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: u64 = 60_000;

    #[test]
    fn full_cycle() {
        let mut p = Pomodoro::new(PomodoroSettings { work_min: 1, short_min: 2, long_min: 3, cycles: 2 });
        p.timer.start(0);
        assert!(p.timer.is_expired(MIN));
        assert!(p.advance(MIN).contains("Short break (2 min)"));
        assert_eq!(p.phase(), Phase::ShortBreak);
        assert!(p.timer.is_running());
        assert_eq!(p.timer.remaining_ms(MIN), Some(2 * MIN));
        p.advance(3 * MIN);
        assert_eq!(p.phase(), Phase::Work);
        assert!(p.advance(4 * MIN).contains("Long break"));
        assert_eq!(p.cycle(), 2);
        p.advance(7 * MIN);
        assert_eq!((p.phase(), p.cycle(), p.completed()), (Phase::Work, 0, 2));
    }

    #[test]
    fn zero_and_huge_settings_are_clamped() {
        // Audit: 0 minutes expired on every tick; minutes * 60 * 1000 could overflow.
        let s = PomodoroSettings { work_min: 0, short_min: u32::MAX, long_min: 0, cycles: 0 }.validated();
        assert_eq!(s, PomodoroSettings { work_min: 1, short_min: 180, long_min: 1, cycles: 1 });
        let p = Pomodoro::new(PomodoroSettings { work_min: 0, ..Default::default() });
        assert!(!p.timer.is_expired(0));
    }

    #[test]
    fn pause_resume_in_phase() {
        let mut p = Pomodoro::new(PomodoroSettings::default());
        p.timer.start(0);
        p.timer.pause(10 * MIN);
        p.timer.start(100 * MIN);
        assert_eq!(p.timer.remaining_ms(105 * MIN), Some(10 * MIN));
        assert!(!p.timer.is_expired(114 * MIN));
        assert!(p.timer.is_expired(115 * MIN));
    }

    #[test]
    fn settings_apply_to_unstarted_phase_only() {
        let mut p = Pomodoro::new(PomodoroSettings::default());
        p.apply_settings(PomodoroSettings { work_min: 50, ..Default::default() });
        assert_eq!(p.timer.target_ms(), Some(50 * MIN));
        p.timer.start(0);
        p.apply_settings(PomodoroSettings { work_min: 10, ..Default::default() });
        assert_eq!(p.timer.target_ms(), Some(50 * MIN), "running phase keeps its length");
        assert!(p.timer.is_running());
        p.reset();
        assert_eq!(p.timer.target_ms(), Some(10 * MIN));
    }
}
