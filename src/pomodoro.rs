use timer_core::TimerCore;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PomPhase {
    Work,
    ShortBreak,
    LongBreak,
}

pub struct PomodoroState {
    pub timer: TimerCore,
    pub phase: PomPhase,
    pub work_duration_ms: u64,
    pub short_break_ms: u64,
    pub long_break_ms: u64,
    pub cycles_before_long: u8,
    pub current_cycle: u8,
    pub total_completed: u32,
}

impl PomodoroState {
    pub fn new() -> Self {
        Self {
            timer: TimerCore::new_countdown(25 * 60 * 1000),
            phase: PomPhase::Work,
            work_duration_ms: 25 * 60 * 1000,
            short_break_ms: 5 * 60 * 1000,
            long_break_ms: 15 * 60 * 1000,
            cycles_before_long: 4,
            current_cycle: 0,
            total_completed: 0,
        }
    }

    pub fn from_settings(work_ms: u64, short_ms: u64, long_ms: u64, cycles: u8) -> Self {
        Self {
            timer: TimerCore::new_countdown(work_ms),
            phase: PomPhase::Work,
            work_duration_ms: work_ms,
            short_break_ms: short_ms,
            long_break_ms: long_ms,
            cycles_before_long: cycles,
            current_cycle: 0,
            total_completed: 0,
        }
    }

    /// Transition to the next phase after timer expires.
    /// Returns the alert message to display.
    pub fn advance_phase(&mut self) -> &'static str {
        match self.phase {
            PomPhase::Work => {
                self.current_cycle += 1;
                self.total_completed += 1;
                if self.current_cycle >= self.cycles_before_long {
                    self.phase = PomPhase::LongBreak;
                    self.timer = TimerCore::new_countdown(self.long_break_ms);
                    "Work done! Long break."
                } else {
                    self.phase = PomPhase::ShortBreak;
                    self.timer = TimerCore::new_countdown(self.short_break_ms);
                    "Work done! Short break."
                }
            }
            PomPhase::ShortBreak | PomPhase::LongBreak => {
                if self.phase == PomPhase::LongBreak {
                    self.current_cycle = 0;
                }
                self.phase = PomPhase::Work;
                self.timer = TimerCore::new_countdown(self.work_duration_ms);
                "Break over! Time to work."
            }
        }
    }

    pub fn reset(&mut self) {
        let duration = match self.phase {
            PomPhase::Work => self.work_duration_ms,
            PomPhase::ShortBreak => self.short_break_ms,
            PomPhase::LongBreak => self.long_break_ms,
        };
        self.timer = TimerCore::new_countdown(duration);
    }

    pub fn phase_label(&self) -> &'static str {
        match self.phase {
            PomPhase::Work => "Work",
            PomPhase::ShortBreak => "Short Break",
            PomPhase::LongBreak => "Long Break",
        }
    }

    pub fn progress_fraction(&self, now_ms: u64) -> f32 {
        let target = match self.phase {
            PomPhase::Work => self.work_duration_ms,
            PomPhase::ShortBreak => self.short_break_ms,
            PomPhase::LongBreak => self.long_break_ms,
        };
        if target == 0 {
            return 1.0;
        }
        let elapsed = self.timer.elapsed_ms(now_ms);
        let frac = elapsed as f32 / target as f32;
        if frac > 1.0 { 1.0 } else { frac }
    }
}
