use timer_core::TimerCore;

const MAX_LAPS: usize = 99;

pub struct StopwatchState {
    pub timer: TimerCore,
    pub laps: Vec<u64>,
    pub lap_scroll_offset: usize,
}

impl StopwatchState {
    pub fn new() -> Self {
        Self {
            timer: TimerCore::new_stopwatch(),
            laps: Vec::new(),
            lap_scroll_offset: 0,
        }
    }

    pub fn record_lap(&mut self, now_ms: u64) {
        if self.laps.len() >= MAX_LAPS {
            return;
        }
        let lap_time = self.timer.lap(now_ms);
        if lap_time > 0 {
            self.laps.push(lap_time);
        }
    }

    pub fn reset(&mut self) {
        self.timer.reset();
        self.laps.clear();
        self.lap_scroll_offset = 0;
    }

}
