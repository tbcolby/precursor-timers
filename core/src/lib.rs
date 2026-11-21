//! Pure logic for the Timers app. Nothing here depends on Xous.
//!
//! * [`timer`]: pausable timer from timestamps, stopwatch with laps.
//! * [`clock`]: suspend-safe monotonic time from ticktimer + wall clock.
//! * [`pomodoro`], [`countdown`]: the two alerting timers.
//! * [`saved`]: settings record and the 0.1 binary formats.
//! * [`app`]: the screen state machine.

pub mod app;
pub mod clock;
pub mod countdown;
pub mod format;
pub mod pomodoro;
pub mod saved;
pub mod timer;

pub use app::{Alert, ScreenId, Timers};
pub use timer::{TimerCore, TimerState};

#[cfg(test)]
mod tests;
