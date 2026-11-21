//! Key-and-clock sequences as the device delivers them.

use precursor_appkit_core::keys::Key;

use crate::app::{ScreenId, Timers};
use crate::saved::Saved;
use crate::timer::TimerState;

const MIN: u64 = 60_000;
const F1: Key = Key::F(1);
const F3: Key = Key::F(3);
const F4: Key = Key::F(4);

fn app() -> Timers { Timers::new(Saved::default()) }

fn open(t: &mut Timers, item: usize) {
    for _ in 0..3 {
        t.on_key(Key::Up, 0);
    }
    for _ in 0..item {
        t.on_key(Key::Down, 0);
    }
    t.on_key(Key::Enter, 0);
}

fn typ(t: &mut Timers, s: &str) {
    for c in s.chars() {
        t.on_key(Key::Char(c), 0);
    }
}

fn add_countdown(t: &mut Timers, name: &str, dur: &str) {
    t.on_key(Key::Char('n'), 0);
    typ(t, name);
    t.on_key(Key::Enter, 0);
    assert_eq!(t.screen(), ScreenId::NewDuration);
    for _ in 0..10 {
        t.on_key(Key::Backspace, 0);
    }
    typ(t, dur);
    t.on_key(Key::Enter, 0);
}

#[test]
fn pomodoro_alert_fires_from_background() {
    // Audit: the pump stopped on background and only the visible mode was
    // checked, so a Pomodoro finished silently while another app was in front.
    let mut t = app();
    open(&mut t, 0);
    t.on_key(Key::Enter, 0);
    assert_eq!(t.pump_interval(true), Some(1000));
    t.on_key(F4, 0);
    assert_eq!(t.on_key(F4, 0), true, "leave the app from home");
    assert_eq!(t.pump_interval(false), Some(1000), "armed: keep ticking in the background");
    assert!(t.tick(24 * MIN).is_empty());
    let alerts = t.tick(25 * MIN);
    assert_eq!(alerts.len(), 1);
    assert!(alerts[0].message.contains("Short break"));
    assert!(alerts[0].vibrate && alerts[0].notify);
    assert!(t.pomodoro.timer.is_running(), "next phase starts");
}

#[test]
fn countdown_alert_fires_from_another_screen() {
    let mut t = app();
    open(&mut t, 2);
    add_countdown(&mut t, "Tea", "3:00");
    t.on_key(Key::Enter, 0);
    assert_eq!(t.screen(), ScreenId::CountdownRun);
    t.on_key(F4, 0); // back to list, still running
    t.on_key(F4, 0);
    open(&mut t, 1); // stopwatch screen
    assert!(t.countdowns.is_running());
    let alerts = t.tick(3 * MIN);
    assert_eq!(alerts[0].message, "Tea: time's up.");
    assert!(t.countdowns.active().is_none());
    assert_eq!(t.status(), "Tea: time's up.");
}

#[test]
fn pump_stops_when_nothing_to_do() {
    let mut t = app();
    assert_eq!(t.pump_interval(true), None);
    open(&mut t, 1);
    t.on_key(Key::Enter, 0);
    assert_eq!(t.pump_interval(true), Some(250), "stopwatch on screen: tenths at 4 Hz");
    assert_eq!(t.pump_interval(false), None, "stopwatch alone has no alert");
    t.on_key(F4, 0);
    assert_eq!(t.pump_interval(true), Some(1000), "home shows the running stopwatch");
    open(&mut t, 1);
    t.on_key(Key::Enter, 1_000);
    assert_eq!(t.pump_interval(true), None);
}

#[test]
fn alerts_respect_settings() {
    let mut t = app();
    t.on_key(Key::Char('s'), 0);
    t.on_key(Key::Enter, 0); // vibrate off
    assert!(t.take_dirty());
    assert!(!t.take_dirty(), "dirty is reported once");
    t.on_key(F4, 0);
    open(&mut t, 0);
    t.on_key(Key::Enter, 0);
    let a = t.tick(25 * MIN);
    assert!(!a[0].vibrate && a[0].notify);
}

#[test]
fn settings_return_to_previous_screen_and_timer_keeps_running() {
    // Audit: Settings from a running Pomodoro froze it and exited to the mode list.
    let mut t = app();
    open(&mut t, 0);
    t.on_key(Key::Enter, 0);
    t.on_key(Key::Char('s'), MIN);
    assert_eq!(t.screen(), ScreenId::Settings);
    assert_eq!(t.pump_interval(true), Some(1000));
    t.on_key(Key::Backspace, MIN);
    assert_eq!(t.screen(), ScreenId::Pomodoro);
    assert!(t.pomodoro.timer.is_running());
    assert_eq!(t.tick(25 * MIN).len(), 1);
}

#[test]
fn settings_validate_typed_numbers() {
    // Audit: 0 was accepted and a parse failure silently became the default.
    let mut t = app();
    t.on_key(Key::Char('s'), 0);
    t.on_key(Key::Down, 0);
    t.on_key(Key::Down, 0); // Work
    typ(&mut t, "0");
    t.on_key(Key::Enter, 0);
    assert_eq!(t.status(), "Must be 1 to 180");
    assert_eq!(t.pomodoro.settings().work_min, 25);
    typ(&mut t, "999");
    t.on_key(Key::Enter, 0);
    assert_eq!(t.pomodoro.settings().work_min, 25);
    typ(&mut t, "abc50");
    assert_eq!(t.input(), "50", "letters ignored");
    t.on_key(Key::Enter, 0);
    assert_eq!(t.pomodoro.settings().work_min, 50);
    assert_eq!(t.pomodoro.timer.target_ms(), Some(50 * MIN));
    t.on_key(Key::Right, 0);
    assert_eq!(t.pomodoro.settings().work_min, 51);
    for _ in 0..3 {
        t.on_key(Key::Down, 0);
    }
    for _ in 0..20 {
        t.on_key(Key::Right, 0);
    }
    assert_eq!(t.pomodoro.settings().cycles, 12, "cycles capped");
    assert_eq!(t.status(), "Must be 1 to 12");
    assert_eq!(t.saved().pomodoro.work_min, 51);
}

#[test]
fn countdown_reset_then_enter_starts() {
    // Audit: after 'r' the countdown was Stopped and Enter did nothing.
    let mut t = app();
    open(&mut t, 2);
    add_countdown(&mut t, "Eggs", "7:00");
    t.on_key(Key::Enter, 0);
    t.on_key(Key::Char('r'), MIN);
    assert_eq!(t.countdowns.active().unwrap().state(), TimerState::Stopped);
    assert_eq!(t.countdowns.active().unwrap().remaining_ms(MIN), Some(7 * MIN));
    t.on_key(Key::Enter, 2 * MIN);
    assert!(t.countdowns.is_running());
    assert_eq!(t.countdowns.active().unwrap().remaining_ms(3 * MIN), Some(6 * MIN));
}

#[test]
fn countdown_pause_resume_across_ticks() {
    let mut t = app();
    open(&mut t, 2);
    add_countdown(&mut t, "Run", "10:00");
    t.on_key(Key::Enter, 0);
    t.on_key(Key::Enter, 4 * MIN); // pause with 6 left
    assert_eq!(t.pump_interval(false), None, "paused countdown arms nothing");
    assert!(t.tick(60 * MIN).is_empty());
    t.on_key(Key::F(2), 60 * MIN); // resume
    assert!(t.tick(65 * MIN).is_empty());
    assert_eq!(t.tick(66 * MIN).len(), 1);
    assert_eq!(t.screen(), ScreenId::Countdowns, "run screen closes on expiry");
}

#[test]
fn invalid_countdown_input_is_explained() {
    // Audit: bad durations and the 21st timer were dropped silently.
    let mut t = app();
    open(&mut t, 2);
    t.on_key(Key::Char('n'), 0);
    t.on_key(Key::Enter, 0);
    assert_eq!(t.status(), "Name can't be empty");
    typ(&mut t, "Tea");
    t.on_key(Key::Left, 0); // arrows aren't typed
    t.on_key(Key::Char('\u{F702}'), 0);
    t.on_key(Key::Enter, 0);
    assert_eq!(t.pending_name(), "Tea");
    for _ in 0..10 {
        t.on_key(Key::Backspace, 0);
    }
    typ(&mut t, "1:75");
    t.on_key(Key::Enter, 0);
    assert_eq!(t.screen(), ScreenId::NewDuration);
    assert!(t.status().contains("under 60"));
    t.on_key(Key::Char('x'), 0);
    assert_eq!(t.status(), "Digits and : only");
    t.on_key(F4, 0);
    assert_eq!(t.screen(), ScreenId::Countdowns);
    assert!(t.countdowns.entries().is_empty());
    for i in 0..20 {
        add_countdown(&mut t, &format!("t{i}"), "1");
    }
    t.on_key(Key::Char('n'), 0);
    assert_eq!(t.screen(), ScreenId::Countdowns);
    assert!(t.status().contains("limit"));
}

#[test]
fn long_unicode_name_is_char_truncated() {
    let mut t = app();
    open(&mut t, 2);
    add_countdown(&mut t, &"福".repeat(30), "5");
    assert_eq!(t.countdowns.entries()[0].name.chars().count(), 20);
}

#[test]
fn delete_asks_and_replace_asks() {
    let mut t = app();
    open(&mut t, 2);
    add_countdown(&mut t, "A", "60");
    add_countdown(&mut t, "B", "60");
    assert!(t.take_dirty());
    t.on_key(Key::Up, 0);
    t.on_key(Key::Enter, 0); // start A
    t.on_key(Key::Backspace, 0);
    t.on_key(Key::Down, 0);
    t.on_key(Key::Enter, 0); // B while A runs
    assert!(t.confirm_text().unwrap().contains("Stop \"A\""));
    t.on_key(Key::Char('n'), 0);
    assert_eq!(t.countdowns.active_name(), Some("A"));
    t.on_key(Key::Char('d'), 0);
    assert!(t.confirm_text().unwrap().contains("Delete \"B\""));
    t.on_key(F4, 0);
    assert_eq!(t.countdowns.entries().len(), 2);
    t.on_key(Key::Char('d'), 0);
    t.on_key(Key::Char('y'), 0);
    assert_eq!(t.countdowns.entries().len(), 1);
    assert!(t.take_dirty());
    assert_eq!(t.countdowns.active_name(), Some("A"));
}

#[test]
fn stopwatch_laps_and_reset_feedback() {
    let mut t = app();
    open(&mut t, 1);
    t.on_key(Key::Char('l'), 0);
    assert!(t.status().contains("Start"));
    t.on_key(Key::Enter, 0);
    t.on_key(Key::Char('l'), 3_000);
    t.on_key(Key::Char('l'), 5_500);
    assert_eq!(t.stopwatch.laps(), [3_000, 2_500]);
    assert_eq!(t.stopwatch_text(6_000), ("00:06.0".into(), "00:00.5".into()));
    t.on_key(F3, 7_000);
    assert_eq!(t.status(), "Pause before resetting");
    t.on_key(Key::Enter, 7_000);
    t.on_key(Key::Char('r'), 8_000);
    assert!(t.stopwatch.laps().is_empty());
}

#[test]
fn every_screen_has_a_back_key() {
    let mut t = app();
    for item in 0..3 {
        open(&mut t, item);
        assert_ne!(t.screen(), ScreenId::Home);
        t.on_key(Key::Backspace, 0);
        assert_eq!(t.screen(), ScreenId::Home);
    }
    t.on_key(F1, 0);
    assert!(t.menu().is_some());
    t.on_key(F4, 0);
    assert!(t.menu().is_none());
    t.on_key(F1, 0);
    t.on_key(Key::Down, 0);
    t.on_key(Key::Enter, 0); // Help
    assert_eq!(t.screen(), ScreenId::Help);
    assert!(!t.help_text().contains("Quit"));
    t.on_key(Key::Backspace, 0);
    assert_eq!(t.screen(), ScreenId::Home);
    // F4 at home leaves (relinquish focus), it doesn't stop timers
    open(&mut t, 0);
    t.on_key(Key::Enter, 0);
    t.on_key(F4, 0);
    assert!(t.on_key(F4, 0));
    assert!(t.pomodoro.timer.is_running());
}

#[test]
fn home_rows_show_running_timers() {
    let mut t = app();
    open(&mut t, 0);
    t.on_key(Key::Enter, 0);
    t.on_key(F4, 0);
    let rows = t.home_rows(MIN);
    assert_eq!(rows[0], "Pomodoro  Work 24:00");
    assert_eq!(rows[2], "Countdowns  (0)");
}

#[test]
fn reload_keeps_running_countdown_if_list_unchanged() {
    let mut t = app();
    open(&mut t, 2);
    add_countdown(&mut t, "A", "60");
    t.on_key(Key::Enter, 0);
    let saved = t.saved();
    t.reload(saved);
    assert!(t.countdowns.is_running());
    t.reload(Saved::default());
    assert!(t.countdowns.active().is_none());
    assert_eq!(t.screen(), ScreenId::Countdowns);
}

#[test]
fn menu_actions_route_to_keys() {
    let mut t = app();
    open(&mut t, 1);
    t.on_key(F1, 0);
    t.on_key(Key::Enter, 0); // Start / pause
    assert!(t.stopwatch.timer.is_running());
    t.on_key(F1, 1_000);
    t.on_key(Key::Down, 1_000);
    t.on_key(Key::Enter, 1_000); // Lap
    assert_eq!(t.stopwatch.laps(), [1_000]);
}
