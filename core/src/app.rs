//! The Timers state machine: keys and clock ticks in, screens and alerts out.
//!
//! Pomodoro, stopwatch and one countdown run independently of the screen
//! shown and of whether the app is in front; [`Timers::tick`] checks all of
//! them. [`Timers::pump_interval`] tells the shell how often to tick.

use precursor_appkit_core::keys::Key;

use crate::countdown::{clean_name, parse_duration, CountdownEntry, Countdowns, MAX_NAME_CHARS};
use crate::format::{format_duration, format_remaining, format_tenths};
use crate::pomodoro::{Pomodoro, PomodoroSettings, CYCLES, MINUTES};
use crate::saved::Saved;
use crate::timer::Stopwatch;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScreenId {
    Home,
    Pomodoro,
    Stopwatch,
    Countdowns,
    CountdownRun,
    NewName,
    NewDuration,
    Settings,
    Help,
}

/// A timer finished. The shell vibrates and/or shows a notification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Alert {
    pub message: String,
    pub vibrate: bool,
    pub notify: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    Help,
    Settings,
    Toggle,
    Reset,
    Lap,
    Open,
    New,
    Delete,
    Stop,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Menu {
    items: Vec<(&'static str, Action)>,
    pub selected: usize,
}

impl Menu {
    pub fn labels(&self) -> Vec<&'static str> { self.items.iter().map(|(l, _)| *l).collect() }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ConfirmKind {
    Delete(usize),
    Replace(usize),
}

pub const HOME_ITEMS: [&str; 3] = ["Pomodoro", "Stopwatch", "Countdowns"];
const SETTINGS_ROWS: usize = 6;

pub struct Timers {
    screen: ScreenId,
    help_return: ScreenId,
    settings_return: ScreenId,
    home_sel: usize,
    cd_sel: usize,
    settings_sel: usize,
    lap_scroll: usize,
    pub pomodoro: Pomodoro,
    pub stopwatch: Stopwatch,
    pub countdowns: Countdowns,
    vibrate: bool,
    notify: bool,
    input: String,
    pending_name: String,
    menu: Option<Menu>,
    confirm: Option<(ConfirmKind, String)>,
    status: String,
    dirty: bool,
}

impl Timers {
    pub fn new(saved: Saved) -> Self {
        Self {
            screen: ScreenId::Home,
            help_return: ScreenId::Home,
            settings_return: ScreenId::Home,
            home_sel: 0,
            cd_sel: 0,
            settings_sel: 0,
            lap_scroll: 0,
            pomodoro: Pomodoro::new(saved.pomodoro),
            stopwatch: Stopwatch::default(),
            countdowns: Countdowns::new(saved.countdowns),
            vibrate: saved.vibrate,
            notify: saved.notify,
            input: String::new(),
            pending_name: String::new(),
            menu: None,
            confirm: None,
            status: String::new(),
            dirty: false,
        }
    }

    /// Replace saved settings (after a basis change), keeping running timers
    /// where the countdown list is unchanged.
    pub fn reload(&mut self, saved: Saved) {
        self.vibrate = saved.vibrate;
        self.notify = saved.notify;
        self.pomodoro.apply_settings(saved.pomodoro);
        if saved.countdowns.as_slice() != self.countdowns.entries() {
            if self.screen == ScreenId::CountdownRun {
                self.screen = ScreenId::Countdowns;
            }
            self.countdowns = Countdowns::new(saved.countdowns);
            self.cd_sel = 0;
        }
    }

    pub fn saved(&self) -> Saved {
        Saved {
            version: crate::saved::VERSION,
            vibrate: self.vibrate,
            notify: self.notify,
            pomodoro: self.pomodoro.settings(),
            countdowns: self.countdowns.entries().to_vec(),
        }
    }

    /// True once after something that must be saved changed.
    pub fn take_dirty(&mut self) -> bool { core::mem::take(&mut self.dirty) }

    // ---------------------------------------------------------------- view

    pub fn screen(&self) -> ScreenId { self.screen }

    pub fn menu(&self) -> Option<&Menu> { self.menu.as_ref() }

    pub fn confirm_text(&self) -> Option<&str> { self.confirm.as_ref().map(|(_, t)| t.as_str()) }

    pub fn status(&self) -> &str { &self.status }

    pub fn set_status(&mut self, s: &str) { self.status = s.to_string(); }

    pub fn home_selected(&self) -> usize { self.home_sel }

    pub fn countdown_selected(&self) -> usize { self.cd_sel }

    pub fn settings_selected(&self) -> usize { self.settings_sel }

    pub fn lap_scroll(&self) -> usize { self.lap_scroll }

    pub fn input(&self) -> &str { &self.input }

    pub fn pending_name(&self) -> &str { &self.pending_name }

    /// Home rows with what each timer is doing.
    pub fn home_rows(&self, now: u64) -> Vec<String> {
        let p = &self.pomodoro.timer;
        let pom = match p.state() {
            crate::timer::TimerState::Running => format!("  {} {}", self.pomodoro.phase().label(), format_remaining(p.remaining_ms(now).unwrap_or(0))),
            crate::timer::TimerState::Paused => "  paused".into(),
            _ => String::new(),
        };
        let sw = &self.stopwatch.timer;
        let stop = match sw.state() {
            crate::timer::TimerState::Running => format!("  {}", format_duration(sw.elapsed_ms(now))),
            crate::timer::TimerState::Paused => "  paused".into(),
            _ => String::new(),
        };
        let cd = match (self.countdowns.active_name(), self.countdowns.active()) {
            (Some(n), Some(t)) if t.is_running() => format!("  {n} {}", format_remaining(t.remaining_ms(now).unwrap_or(0))),
            (Some(n), Some(_)) => format!("  {n} paused"),
            _ => format!("  ({})", self.countdowns.entries().len()),
        };
        vec![format!("Pomodoro{pom}"), format!("Stopwatch{stop}"), format!("Countdowns{cd}")]
    }

    pub fn countdown_rows(&self, now: u64) -> Vec<String> {
        let active = self.countdowns.active_index();
        self.countdowns
            .entries()
            .iter()
            .enumerate()
            .map(|(i, e)| match (active == Some(i), self.countdowns.active()) {
                (true, Some(t)) => {
                    let mark = if t.is_running() { ">" } else { "||" };
                    format!("{mark} {}  {} left", e.name, format_remaining(t.remaining_ms(now).unwrap_or(0)))
                }
                _ => format!("{}  {}", e.name, format_duration(e.duration_ms)),
            })
            .collect()
    }

    pub fn settings_rows(&self) -> Vec<String> {
        let s = self.pomodoro.settings();
        let on = |b: bool| if b { "on" } else { "off" };
        let typed = |row: usize, v: u32| {
            if self.settings_sel == row && !self.input.is_empty() {
                format!("{}_", self.input)
            } else {
                v.to_string()
            }
        };
        vec![
            format!("Vibrate on alert: {}", on(self.vibrate)),
            format!("Notification on alert: {}", on(self.notify)),
            format!("Work: {} min", typed(2, s.work_min)),
            format!("Short break: {} min", typed(3, s.short_min)),
            format!("Long break: {} min", typed(4, s.long_min)),
            format!("Long break after: {} sessions", typed(5, s.cycles)),
        ]
    }

    pub fn stopwatch_text(&self, now: u64) -> (String, String) {
        (format_tenths(self.stopwatch.total_ms(now)), format_tenths(self.stopwatch.split_ms(now)))
    }

    pub fn help_text(&self) -> &'static str { help(self.help_return) }

    // ------------------------------------------------------------- clock

    /// How often the shell should tick, in ms; `None` to stop the pump.
    /// In the background it keeps ticking while an alert is armed (0.1
    /// stopped the pump, so alerts fired only after you came back).
    pub fn pump_interval(&self, foreground: bool) -> Option<u64> {
        let armed = self.pomodoro.timer.is_running() || self.countdowns.is_running();
        let stopwatch = self.stopwatch.timer.is_running();
        if foreground && stopwatch && self.screen == ScreenId::Stopwatch && self.menu.is_none() {
            Some(250)
        } else if armed || (foreground && stopwatch) {
            Some(1000)
        } else {
            None
        }
    }

    /// Check every timer. Returns alerts for timers that finished.
    pub fn tick(&mut self, now: u64) -> Vec<Alert> {
        let mut alerts = Vec::new();
        if self.pomodoro.timer.is_running() && self.pomodoro.timer.is_expired(now) {
            let msg = self.pomodoro.advance(now);
            alerts.push(self.alert(msg));
        }
        if self.countdowns.is_running() && self.countdowns.active().is_some_and(|t| t.is_expired(now)) {
            let name = self.countdowns.active_name().unwrap_or("Countdown").to_string();
            self.countdowns.stop();
            if self.screen == ScreenId::CountdownRun {
                self.screen = ScreenId::Countdowns;
            }
            alerts.push(self.alert(format!("{name}: time's up.")));
        }
        if let Some(a) = alerts.last() {
            self.status = a.message.clone();
        }
        alerts
    }

    fn alert(&self, message: String) -> Alert { Alert { message, vibrate: self.vibrate, notify: self.notify } }

    // ------------------------------------------------------------- input

    /// Returns true when the app should hand focus back.
    pub fn on_key(&mut self, key: Key, now: u64) -> bool {
        self.status.clear();
        if let Some((kind, _)) = self.confirm {
            self.confirm_key(kind, key, now);
            return false;
        }
        if self.menu.is_some() {
            self.menu_key(key, now);
            return false;
        }
        match self.screen {
            ScreenId::Home => return self.home_key(key),
            ScreenId::Pomodoro => self.pomodoro_key(key, now),
            ScreenId::Stopwatch => self.stopwatch_key(key, now),
            ScreenId::Countdowns => self.countdowns_key(key, now),
            ScreenId::CountdownRun => self.run_key(key, now),
            ScreenId::NewName => self.name_key(key),
            ScreenId::NewDuration => self.duration_key(key),
            ScreenId::Settings => self.settings_key(key),
            ScreenId::Help => {
                if matches!(key, Key::Enter | Key::Backspace | Key::F(1) | Key::F(4)) {
                    self.screen = self.help_return;
                }
            }
        }
        false
    }

    fn open_menu(&mut self, mut items: Vec<(&'static str, Action)>) {
        items.push(("Help", Action::Help));
        self.menu = Some(Menu { items, selected: 0 });
    }

    fn open_settings(&mut self) {
        self.settings_return = self.screen;
        self.settings_sel = 0;
        self.input.clear();
        self.screen = ScreenId::Settings;
    }

    fn home_key(&mut self, key: Key) -> bool {
        match key {
            Key::Up => self.home_sel = self.home_sel.saturating_sub(1),
            Key::Down => self.home_sel = (self.home_sel + 1).min(HOME_ITEMS.len() - 1),
            Key::Enter => {
                self.screen = [ScreenId::Pomodoro, ScreenId::Stopwatch, ScreenId::Countdowns][self.home_sel];
                self.lap_scroll = 0;
            }
            Key::Char('s') => self.open_settings(),
            Key::F(1) => self.open_menu(vec![("Settings", Action::Settings)]),
            k if k.is_back() => return true,
            _ => {}
        }
        false
    }

    fn pomodoro_key(&mut self, key: Key, now: u64) {
        match key {
            Key::Enter | Key::F(2) => self.pomodoro.timer.toggle(now),
            Key::Char('r') | Key::F(3) => self.pomodoro.reset(),
            Key::Char('s') => self.open_settings(),
            Key::F(1) => self.open_menu(vec![
                ("Start / pause", Action::Toggle),
                ("Restart phase", Action::Reset),
                ("Settings", Action::Settings),
            ]),
            k if k.is_back() => self.screen = ScreenId::Home,
            _ => {}
        }
    }

    fn stopwatch_key(&mut self, key: Key, now: u64) {
        match key {
            Key::Enter | Key::F(2) => self.stopwatch.timer.toggle(now),
            Key::Char('l') => {
                if self.stopwatch.lap(now) {
                    self.lap_scroll = 0;
                } else if self.stopwatch.timer.is_running() {
                    self.status = "Lap list full".into();
                } else {
                    self.status = "Start the stopwatch to record laps".into();
                }
            }
            Key::Char('r') | Key::F(3) => {
                if self.stopwatch.reset() {
                    self.lap_scroll = 0;
                } else {
                    self.status = "Pause before resetting".into();
                }
            }
            Key::Up => self.lap_scroll = (self.lap_scroll + 1).min(self.stopwatch.laps().len().saturating_sub(1)),
            Key::Down => self.lap_scroll = self.lap_scroll.saturating_sub(1),
            Key::F(1) => self.open_menu(vec![("Start / pause", Action::Toggle), ("Lap", Action::Lap), ("Reset", Action::Reset)]),
            k if k.is_back() => self.screen = ScreenId::Home,
            _ => {}
        }
    }

    fn countdowns_key(&mut self, key: Key, now: u64) {
        let n = self.countdowns.entries().len();
        match key {
            Key::Up => self.cd_sel = self.cd_sel.saturating_sub(1),
            Key::Down => self.cd_sel = (self.cd_sel + 1).min(n.saturating_sub(1)),
            Key::Enter if n > 0 => self.open_countdown(self.cd_sel, now),
            Key::Enter => self.status = "No countdowns yet. Press n to add one".into(),
            Key::Char('n') => self.new_countdown(),
            Key::Char('d') if n > 0 => {
                let name = self.countdowns.entries()[self.cd_sel].name.clone();
                self.confirm = Some((ConfirmKind::Delete(self.cd_sel), format!("Delete \"{name}\"?")));
            }
            Key::Char('s') => self.open_settings(),
            Key::F(1) => self.open_menu(vec![
                ("Start / open", Action::Open),
                ("New countdown", Action::New),
                ("Delete", Action::Delete),
                ("Settings", Action::Settings),
            ]),
            k if k.is_back() => self.screen = ScreenId::Home,
            _ => {}
        }
    }

    fn open_countdown(&mut self, i: usize, now: u64) {
        match self.countdowns.active_index() {
            Some(a) if a == i => self.screen = ScreenId::CountdownRun,
            Some(_) => {
                let name = self.countdowns.active_name().unwrap_or("").to_string();
                self.confirm = Some((ConfirmKind::Replace(i), format!("Stop \"{name}\" and start this one?")));
            }
            None => {
                self.countdowns.start(i, now);
                self.screen = ScreenId::CountdownRun;
            }
        }
    }

    fn new_countdown(&mut self) {
        if self.countdowns.is_full() {
            // Check before asking for a name (0.1 dropped the 21st silently).
            self.status = "20 countdowns is the limit; delete one first".into();
            return;
        }
        self.input.clear();
        self.screen = ScreenId::NewName;
    }

    fn run_key(&mut self, key: Key, now: u64) {
        match key {
            Key::Enter | Key::F(2) => {
                if let Some(t) = self.countdowns.active_mut() {
                    t.toggle(now); // a reset (Stopped) countdown starts too
                }
            }
            Key::Char('r') | Key::F(3) => {
                if let Some(t) = self.countdowns.active_mut() {
                    t.reset();
                }
            }
            Key::Char('x') => {
                self.countdowns.stop();
                self.screen = ScreenId::Countdowns;
            }
            Key::F(1) => self.open_menu(vec![("Start / pause", Action::Toggle), ("Reset", Action::Reset), ("Stop", Action::Stop)]),
            // Back keeps the countdown running; its alert still fires.
            k if k.is_back() => self.screen = ScreenId::Countdowns,
            _ => {}
        }
    }

    fn name_key(&mut self, key: Key) {
        match key {
            Key::Enter => match clean_name(&self.input) {
                Ok(name) => {
                    self.pending_name = name;
                    self.input = "05:00".into();
                    self.screen = ScreenId::NewDuration;
                }
                Err(e) => self.status = e.into(),
            },
            Key::Backspace => {
                self.input.pop();
            }
            Key::F(4) => self.screen = ScreenId::Countdowns,
            Key::Char(c) if typable(c) && self.input.chars().count() < MAX_NAME_CHARS => self.input.push(c),
            _ => {}
        }
    }

    fn duration_key(&mut self, key: Key) {
        match key {
            Key::Enter => match parse_duration(&self.input) {
                Ok(ms) => {
                    let entry = CountdownEntry { name: core::mem::take(&mut self.pending_name), duration_ms: ms };
                    match self.countdowns.add(entry) {
                        Ok(i) => {
                            self.cd_sel = i;
                            self.dirty = true;
                        }
                        Err(e) => self.status = e.into(),
                    }
                    self.screen = ScreenId::Countdowns;
                }
                Err(e) => self.status = e.into(),
            },
            Key::Backspace => {
                self.input.pop();
            }
            Key::F(4) => self.screen = ScreenId::Countdowns,
            Key::Char(c) if (c.is_ascii_digit() || c == ':') && self.input.len() < 10 => self.input.push(c),
            Key::Char(_) => self.status = "Digits and : only".into(),
            _ => {}
        }
    }

    fn settings_key(&mut self, key: Key) {
        let numeric = self.settings_sel >= 2;
        match key {
            Key::Up | Key::Down => {
                self.input.clear();
                self.settings_sel = if key == Key::Up {
                    self.settings_sel.saturating_sub(1)
                } else {
                    (self.settings_sel + 1).min(SETTINGS_ROWS - 1)
                };
            }
            Key::Enter if !numeric => {
                if self.settings_sel == 0 {
                    self.vibrate = !self.vibrate;
                } else {
                    self.notify = !self.notify;
                }
                self.dirty = true;
            }
            Key::Enter if !self.input.is_empty() => {
                let v: u32 = self.input.parse().unwrap_or(u32::MAX);
                self.input.clear();
                self.set_number(v);
            }
            Key::Left | Key::Right if numeric => {
                self.input.clear();
                let cur = self.number();
                let v = if key == Key::Left { cur.saturating_sub(1) } else { cur + 1 };
                self.set_number(v);
            }
            Key::Char(c) if numeric && c.is_ascii_digit() && self.input.len() < 3 => self.input.push(c),
            Key::Backspace if !self.input.is_empty() => {
                self.input.pop();
            }
            k if k.is_back() => {
                self.input.clear();
                self.screen = self.settings_return; // 0.1 always went to the mode list
            }
            _ => {}
        }
    }

    fn number(&self) -> u32 {
        let s = self.pomodoro.settings();
        match self.settings_sel {
            2 => s.work_min,
            3 => s.short_min,
            4 => s.long_min,
            _ => s.cycles,
        }
    }

    /// Set the selected numeric setting, refusing out-of-range values with a message.
    fn set_number(&mut self, v: u32) {
        let range = if self.settings_sel == 5 { CYCLES } else { MINUTES };
        if !range.contains(&v) {
            self.status = format!("Must be {} to {}", range.start(), range.end());
            return;
        }
        let mut s: PomodoroSettings = self.pomodoro.settings();
        match self.settings_sel {
            2 => s.work_min = v,
            3 => s.short_min = v,
            4 => s.long_min = v,
            _ => s.cycles = v,
        }
        self.pomodoro.apply_settings(s);
        self.dirty = true;
    }

    fn menu_key(&mut self, key: Key, now: u64) {
        let menu = self.menu.as_mut().expect("menu open");
        match key {
            Key::Up => menu.selected = menu.selected.saturating_sub(1),
            Key::Down => menu.selected = (menu.selected + 1).min(menu.items.len() - 1),
            Key::Enter => {
                let action = menu.items[menu.selected].1;
                self.menu = None;
                self.menu_action(action, now);
            }
            Key::F(1) | Key::F(4) | Key::Backspace => self.menu = None,
            _ => {}
        }
    }

    fn menu_action(&mut self, action: Action, now: u64) {
        let key = match action {
            Action::Help => {
                self.help_return = self.screen;
                self.screen = ScreenId::Help;
                return;
            }
            Action::Settings => {
                self.open_settings();
                return;
            }
            Action::Toggle | Action::Open => Key::Enter,
            Action::Reset => Key::F(3),
            Action::Lap => Key::Char('l'),
            Action::New => Key::Char('n'),
            Action::Delete => Key::Char('d'),
            Action::Stop => Key::Char('x'),
        };
        self.on_key(key, now);
    }

    fn confirm_key(&mut self, kind: ConfirmKind, key: Key, now: u64) {
        let yes = matches!(key, Key::Char('y') | Key::Char('Y') | Key::Enter);
        let no = matches!(key, Key::Char('n') | Key::Char('N') | Key::F(4) | Key::Backspace);
        if !(yes || no) {
            return;
        }
        self.confirm = None;
        if !yes {
            return;
        }
        match kind {
            ConfirmKind::Delete(i) => {
                self.countdowns.delete(i);
                self.cd_sel = self.cd_sel.min(self.countdowns.entries().len().saturating_sub(1));
                self.dirty = true;
            }
            ConfirmKind::Replace(i) => {
                self.countdowns.start(i, now);
                self.screen = ScreenId::CountdownRun;
            }
        }
    }
}

/// Printable, not an arrow glyph or Apple PUA code.
fn typable(c: char) -> bool { !c.is_control() && !('\u{2190}'..='\u{2193}').contains(&c) && !('\u{E000}'..='\u{F8FF}').contains(&c) }

fn help(screen: ScreenId) -> &'static str {
    match screen {
        ScreenId::Pomodoro => {
            "POMODORO\n\nEnter / F2  Start or pause\nr / F3  Restart this phase\ns  Settings\nF1  Menu\nF4 / Backspace  Back\n\nThe next phase starts by itself.\nIt keeps running when you leave."
        }
        ScreenId::Stopwatch => {
            "STOPWATCH\n\nEnter / F2  Start or pause\nl  Lap\nr / F3  Reset (when paused)\nUp/Down  Scroll laps\nF4 / Backspace  Back"
        }
        ScreenId::Countdowns | ScreenId::CountdownRun => {
            "COUNTDOWNS\n\nEnter  Start / open\nn  New   d  Delete\nOn a running countdown:\nEnter / F2  Pause or resume\nr / F3  Reset   x  Stop\nF4  Back (keeps running)"
        }
        _ => {
            "TIMERS\n\nPomodoro, stopwatch and a countdown\nrun at the same time and keep\nrunning when you leave the app.\nAlerts fire in the background.\n\nF1  Menu   s  Settings\nF4 / Backspace  Back / leave"
        }
    }
}
