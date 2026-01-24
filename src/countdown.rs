use timer_core::TimerCore;

const MAX_COUNTDOWNS: usize = 20;

#[derive(Clone)]
pub struct CountdownEntry {
    pub name: String,
    pub duration_ms: u64,
}

pub struct CountdownState {
    pub entries: Vec<CountdownEntry>,
    pub cursor: usize,
    pub active_timer: Option<TimerCore>,
    pub active_index: Option<usize>,
}

impl CountdownState {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            cursor: 0,
            active_timer: None,
            active_index: None,
        }
    }

    pub fn add_entry(&mut self, name: String, duration_ms: u64) -> bool {
        if self.entries.len() >= MAX_COUNTDOWNS {
            return false;
        }
        self.entries.push(CountdownEntry { name, duration_ms });
        true
    }

    pub fn delete_selected(&mut self) {
        if self.cursor < self.entries.len() {
            // If the active timer is the one being deleted, stop it
            if self.active_index == Some(self.cursor) {
                self.active_timer = None;
                self.active_index = None;
            } else if let Some(idx) = self.active_index {
                // Adjust active index if needed
                if self.cursor < idx {
                    self.active_index = Some(idx - 1);
                }
            }
            self.entries.remove(self.cursor);
            if self.cursor >= self.entries.len() && self.cursor > 0 {
                self.cursor = self.entries.len() - 1;
            }
        }
    }

    pub fn start_selected(&mut self) {
        if self.cursor < self.entries.len() {
            let duration = self.entries[self.cursor].duration_ms;
            self.active_timer = Some(TimerCore::new_countdown(duration));
            self.active_index = Some(self.cursor);
        }
    }

    pub fn active_name(&self) -> Option<&str> {
        self.active_index
            .and_then(|idx| self.entries.get(idx))
            .map(|e| e.name.as_str())
    }

    pub fn active_duration_ms(&self) -> Option<u64> {
        self.active_index
            .and_then(|idx| self.entries.get(idx))
            .map(|e| e.duration_ms)
    }

    pub fn progress_fraction(&self, now_ms: u64) -> f32 {
        if let (Some(timer), Some(duration)) = (&self.active_timer, self.active_duration_ms()) {
            if duration == 0 {
                return 1.0;
            }
            let elapsed = timer.elapsed_ms(now_ms);
            let frac = elapsed as f32 / duration as f32;
            if frac > 1.0 { 1.0 } else { frac }
        } else {
            0.0
        }
    }

    pub fn stop_active(&mut self) {
        self.active_timer = None;
        self.active_index = None;
    }
}
