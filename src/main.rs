#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]

mod alerts;
mod countdown;
mod pomodoro;
mod stopwatch;
mod storage;
mod ui;

use num_traits::{FromPrimitive, ToPrimitive};
use timer_core::TimerState;

use crate::alerts::{AlertConfig, fire_alert};
use crate::countdown::CountdownState;
use crate::pomodoro::PomodoroState;
use crate::stopwatch::StopwatchState;
use crate::storage::TimerStorage;

const SERVER_NAME: &str = "_Timers_";
const APP_NAME: &str = "Timers";

// F-key character codes from Xous keyboard service
const KEY_F1: char = '\u{0011}';
const KEY_F2: char = '\u{0012}';
const KEY_F3: char = '\u{0013}';
const KEY_F4: char = '\u{0014}';

#[derive(Debug, num_derive::FromPrimitive, num_derive::ToPrimitive)]
enum AppOp {
    Redraw = 0,
    Rawkeys,
    FocusChange,
    Pump,
    Quit,
}

#[derive(Clone, Copy, PartialEq)]
pub enum AppMode {
    ModeSelect,
    Pomodoro,
    Stopwatch,
    CountdownList,
    CountdownRun,
    Settings,
}

struct TimersApp {
    gam: gam::Gam,
    #[allow(dead_code)]
    token: [u32; 4],
    content: gam::Gid,
    screensize: gam::menu::Point,
    tt: ticktimer_server::Ticktimer,
    llio: llio::Llio,
    modals: modals::Modals,
    storage: TimerStorage,

    mode: AppMode,
    mode_cursor: usize,
    settings_cursor: usize,
    alert_config: AlertConfig,

    pomodoro: PomodoroState,
    stopwatch: StopwatchState,
    countdown: CountdownState,

    pump_conn: xous::CID,
    pump_running: bool,
    allow_redraw: bool,
    // Menu overlay state
    menu_visible: bool,
    menu_cursor: usize,
    help_visible: bool,
    confirm_exit: bool,
}

impl TimersApp {
    fn new(xns: &xous_names::XousNames, sid: xous::SID, pump_sid: xous::SID) -> Self {
        let gam = gam::Gam::new(xns).expect("can't connect to GAM");

        let token = gam
            .register_ux(gam::UxRegistration {
                app_name: String::from(APP_NAME),
                ux_type: gam::UxType::Chat,
                predictor: None,
                listener: sid.to_array(),
                redraw_id: AppOp::Redraw.to_u32().unwrap(),
                gotinput_id: None,
                audioframe_id: None,
                rawkeys_id: Some(AppOp::Rawkeys.to_u32().unwrap()),
                focuschange_id: Some(AppOp::FocusChange.to_u32().unwrap()),
            })
            .expect("couldn't register UX")
            .unwrap();

        let content = gam.request_content_canvas(token).expect("couldn't get canvas");
        let screensize = gam.get_canvas_bounds(content).expect("couldn't get dimensions");

        let tt = ticktimer_server::Ticktimer::new().unwrap();
        let llio = llio::Llio::new(xns);
        let modals = modals::Modals::new(xns).unwrap();
        let storage = TimerStorage::new();

        let alert_config = storage.load_alert_config();
        let pomodoro = match storage.load_pomodoro_settings() {
            Some((work, short, long, cycles)) => {
                PomodoroState::from_settings(work, short, long, cycles)
            }
            None => PomodoroState::new(),
        };

        let mut countdown = CountdownState::new();
        countdown.entries = storage.load_countdowns();

        let pump_conn = xous::connect(pump_sid).expect("can't connect to pump");

        Self {
            gam,
            token,
            content,
            screensize,
            tt,
            llio,
            modals,
            storage,
            mode: AppMode::ModeSelect,
            mode_cursor: 0,
            settings_cursor: 0,
            alert_config,
            pomodoro,
            stopwatch: StopwatchState::new(),
            countdown,
            pump_conn,
            pump_running: false,
            allow_redraw: true,
            menu_visible: false,
            menu_cursor: 0,
            help_visible: false,
            confirm_exit: false,
        }
    }

    fn now_ms(&self) -> u64 {
        self.tt.elapsed_ms()
    }

    fn redraw(&self) {
        if !self.allow_redraw {
            return;
        }

        if self.help_visible {
            ui::draw_help(&self.gam, self.content, self.screensize, self.help_text());
            return;
        }
        if self.confirm_exit {
            ui::draw_confirm_exit(&self.gam, self.content, self.screensize);
            return;
        }
        if self.menu_visible {
            ui::draw_menu(&self.gam, self.content, self.screensize, self.menu_items(), self.menu_cursor);
            return;
        }

        let now = self.now_ms();
        match self.mode {
            AppMode::ModeSelect => {
                ui::draw_mode_select(&self.gam, self.content, self.screensize, self.mode_cursor);
            }
            AppMode::Pomodoro => {
                ui::draw_pomodoro(&self.gam, self.content, self.screensize, &self.pomodoro, now);
            }
            AppMode::Stopwatch => {
                ui::draw_stopwatch(&self.gam, self.content, self.screensize, &self.stopwatch, now);
            }
            AppMode::CountdownList => {
                ui::draw_countdown_list(&self.gam, self.content, self.screensize, &self.countdown);
            }
            AppMode::CountdownRun => {
                ui::draw_countdown_running(&self.gam, self.content, self.screensize, &self.countdown, now);
            }
            AppMode::Settings => {
                ui::draw_settings(&self.gam, self.content, self.screensize, &self.alert_config, self.settings_cursor);
            }
        }
    }

    fn start_pump(&mut self, interval_ms: u64) {
        if !self.pump_running {
            self.pump_running = true;
            xous::send_message(
                self.pump_conn,
                xous::Message::new_scalar(0, interval_ms as usize, 0, 0, 0),
            ).ok();
        }
    }

    fn stop_pump(&mut self) {
        if self.pump_running {
            self.pump_running = false;
            xous::send_message(
                self.pump_conn,
                xous::Message::new_scalar(1, 0, 0, 0, 0),
            ).ok();
        }
    }

    fn handle_pump(&mut self) {
        let now = self.now_ms();

        match self.mode {
            AppMode::Pomodoro => {
                if self.pomodoro.timer.is_expired(now) {
                    self.pomodoro.timer.pause(now);
                    let msg = self.pomodoro.advance_phase();
                    fire_alert(&self.alert_config, &self.llio, &self.modals, msg);
                    // Auto-start next phase
                    let now2 = self.now_ms();
                    self.pomodoro.timer.start(now2);
                }
                self.redraw();
            }
            AppMode::Stopwatch => {
                self.redraw();
            }
            AppMode::CountdownRun => {
                let expired = self.countdown.active_timer.as_ref()
                    .map(|t| t.is_expired(now))
                    .unwrap_or(false);
                if expired {
                    let name = self.countdown.active_name()
                        .unwrap_or("Timer").to_string();
                    let msg = format!("{} expired!", name);
                    self.countdown.stop_active();
                    self.stop_pump();
                    fire_alert(&self.alert_config, &self.llio, &self.modals, &msg);
                    self.mode = AppMode::CountdownList;
                }
                self.redraw();
            }
            _ => {
                self.stop_pump();
            }
        }
    }

    fn handle_key(&mut self, key: char) {
        // F-keys always processed first
        match key {
            KEY_F1 => { self.toggle_menu(); return; }
            KEY_F4 => { self.handle_f4(); return; }
            KEY_F2 => { self.handle_f2(); return; }
            KEY_F3 => { self.handle_f3(); return; }
            _ => {}
        }

        // If help screen is showing, any key dismisses it
        if self.help_visible {
            self.help_visible = false;
            self.redraw();
            return;
        }

        // If confirm exit dialog is showing
        if self.confirm_exit {
            match key {
                'y' => {
                    // Stop timers and exit
                    self.stop_all_timers();
                    self.confirm_exit = false;
                    self.mode = AppMode::ModeSelect;
                    self.redraw();
                }
                'n' => {
                    self.confirm_exit = false;
                    self.redraw();
                }
                _ => {}
            }
            return;
        }

        // If menu is open, handle menu navigation only
        if self.menu_visible {
            match key {
                '↑' | 'k' => {
                    if self.menu_cursor > 0 {
                        self.menu_cursor -= 1;
                        self.redraw();
                    }
                }
                '↓' | 'j' => {
                    let items = self.menu_items();
                    if self.menu_cursor + 1 < items.len() {
                        self.menu_cursor += 1;
                        self.redraw();
                    }
                }
                '\r' | '\n' => {
                    self.menu_select_item();
                }
                _ => {}
            }
            return;
        }

        // Normal mode-specific key handling
        match self.mode.clone() {
            AppMode::ModeSelect => self.handle_key_mode_select(key),
            AppMode::Pomodoro => self.handle_key_pomodoro(key),
            AppMode::Stopwatch => self.handle_key_stopwatch(key),
            AppMode::CountdownList => self.handle_key_countdown_list(key),
            AppMode::CountdownRun => self.handle_key_countdown_run(key),
            AppMode::Settings => self.handle_key_settings(key),
        }
    }

    fn any_timer_running(&self) -> bool {
        self.pomodoro.timer.state == TimerState::Running
            || self.stopwatch.timer.state == TimerState::Running
            || self.countdown.active_timer.as_ref()
                .map(|t| t.state == TimerState::Running)
                .unwrap_or(false)
    }

    fn stop_all_timers(&mut self) {
        let now = self.now_ms();
        if self.pomodoro.timer.state == TimerState::Running {
            self.pomodoro.timer.pause(now);
        }
        if self.stopwatch.timer.state == TimerState::Running {
            self.stopwatch.timer.pause(now);
        }
        if let Some(timer) = &mut self.countdown.active_timer {
            if timer.state == TimerState::Running {
                timer.pause(now);
            }
        }
        self.stop_pump();
    }

    fn menu_items(&self) -> &'static [&'static str] {
        match self.mode {
            AppMode::ModeSelect => &["Help", "Settings"],
            AppMode::Pomodoro => &["Help", "Start/Pause", "Reset", "Settings"],
            AppMode::Stopwatch => &["Help", "Start/Pause", "Lap", "Reset"],
            AppMode::CountdownList => &["Help", "New Timer", "Delete", "Settings"],
            AppMode::CountdownRun => &["Help", "Pause/Resume", "Reset", "Back"],
            AppMode::Settings => &["Help", "Back"],
        }
    }

    fn toggle_menu(&mut self) {
        if self.help_visible {
            self.help_visible = false;
            self.redraw();
            return;
        }
        if self.confirm_exit {
            return;
        }
        self.menu_visible = !self.menu_visible;
        self.menu_cursor = 0;
        self.redraw();
    }

    fn menu_select_item(&mut self) {
        self.menu_visible = false;

        match self.mode {
            AppMode::ModeSelect => {
                match self.menu_cursor {
                    0 => { self.help_visible = true; }
                    1 => {
                        self.mode = AppMode::Settings;
                        self.settings_cursor = 0;
                    }
                    _ => {}
                }
            }
            AppMode::Pomodoro => {
                match self.menu_cursor {
                    0 => { self.help_visible = true; }
                    1 => {
                        // Start/Pause - same as Enter
                        let now = self.now_ms();
                        match self.pomodoro.timer.state {
                            TimerState::Stopped | TimerState::Paused => {
                                self.pomodoro.timer.start(now);
                                self.start_pump(1000);
                            }
                            TimerState::Running => {
                                self.pomodoro.timer.pause(now);
                                self.stop_pump();
                            }
                            _ => {}
                        }
                    }
                    2 => {
                        self.pomodoro.reset();
                        self.stop_pump();
                    }
                    3 => {
                        self.mode = AppMode::Settings;
                        self.settings_cursor = 0;
                    }
                    _ => {}
                }
            }
            AppMode::Stopwatch => {
                match self.menu_cursor {
                    0 => { self.help_visible = true; }
                    1 => {
                        let now = self.now_ms();
                        match self.stopwatch.timer.state {
                            TimerState::Stopped | TimerState::Paused => {
                                self.stopwatch.timer.start(now);
                                self.start_pump(100);
                            }
                            TimerState::Running => {
                                self.stopwatch.timer.pause(now);
                                self.stop_pump();
                            }
                            _ => {}
                        }
                    }
                    2 => {
                        let now = self.now_ms();
                        if self.stopwatch.timer.state == TimerState::Running {
                            self.stopwatch.record_lap(now);
                        }
                    }
                    3 => {
                        if self.stopwatch.timer.state != TimerState::Running {
                            self.stopwatch.reset();
                        }
                    }
                    _ => {}
                }
            }
            AppMode::CountdownList => {
                match self.menu_cursor {
                    0 => { self.help_visible = true; }
                    1 => {
                        self.menu_visible = false;
                        self.redraw();
                        self.create_new_countdown();
                        return;
                    }
                    2 => {
                        if !self.countdown.entries.is_empty() {
                            self.countdown.delete_selected();
                            self.storage.save_countdowns(&self.countdown.entries);
                        }
                    }
                    3 => {
                        self.mode = AppMode::Settings;
                        self.settings_cursor = 0;
                    }
                    _ => {}
                }
            }
            AppMode::CountdownRun => {
                match self.menu_cursor {
                    0 => { self.help_visible = true; }
                    1 => {
                        let now = self.now_ms();
                        let action = if let Some(timer) = &mut self.countdown.active_timer {
                            match timer.state {
                                TimerState::Running => { timer.pause(now); Some(false) }
                                TimerState::Paused => { timer.start(now); Some(true) }
                                _ => None,
                            }
                        } else { None };
                        match action {
                            Some(true) => self.start_pump(1000),
                            Some(false) => self.stop_pump(),
                            None => {}
                        }
                    }
                    2 => {
                        self.countdown.start_selected();
                        self.stop_pump();
                    }
                    3 => {
                        self.countdown.stop_active();
                        self.stop_pump();
                        self.mode = AppMode::CountdownList;
                    }
                    _ => {}
                }
            }
            AppMode::Settings => {
                match self.menu_cursor {
                    0 => { self.help_visible = true; }
                    1 => { self.mode = AppMode::ModeSelect; }
                    _ => {}
                }
            }
        }
        self.redraw();
    }

    fn handle_f2(&mut self) {
        if self.help_visible { self.help_visible = false; self.redraw(); return; }
        if self.confirm_exit { return; }
        if self.menu_visible { self.menu_visible = false; }
        // F2 = Start/Stop (same as Enter in timer modes)
        let now = self.now_ms();
        match self.mode {
            AppMode::Pomodoro => {
                match self.pomodoro.timer.state {
                    TimerState::Stopped | TimerState::Paused => {
                        self.pomodoro.timer.start(now);
                        self.start_pump(1000);
                    }
                    TimerState::Running => {
                        self.pomodoro.timer.pause(now);
                        self.stop_pump();
                    }
                    _ => {}
                }
            }
            AppMode::Stopwatch => {
                match self.stopwatch.timer.state {
                    TimerState::Stopped | TimerState::Paused => {
                        self.stopwatch.timer.start(now);
                        self.start_pump(100);
                    }
                    TimerState::Running => {
                        self.stopwatch.timer.pause(now);
                        self.stop_pump();
                    }
                    _ => {}
                }
            }
            AppMode::CountdownRun => {
                let action = if let Some(timer) = &mut self.countdown.active_timer {
                    match timer.state {
                        TimerState::Running => { timer.pause(now); Some(false) }
                        TimerState::Paused => { timer.start(now); Some(true) }
                        _ => None,
                    }
                } else { None };
                match action {
                    Some(true) => self.start_pump(1000),
                    Some(false) => self.stop_pump(),
                    None => {}
                }
            }
            _ => {}
        }
        self.redraw();
    }

    fn handle_f3(&mut self) {
        if self.help_visible { self.help_visible = false; self.redraw(); return; }
        if self.confirm_exit { return; }
        if self.menu_visible { self.menu_visible = false; }
        // F3 = Reset (same as 'r')
        match self.mode {
            AppMode::Pomodoro => {
                self.pomodoro.reset();
                self.stop_pump();
            }
            AppMode::Stopwatch => {
                if self.stopwatch.timer.state != TimerState::Running {
                    self.stopwatch.reset();
                }
            }
            AppMode::CountdownRun => {
                self.countdown.start_selected();
                self.stop_pump();
            }
            _ => {}
        }
        self.redraw();
    }

    fn handle_f4(&mut self) {
        // F4 closes help/menu/confirm first
        if self.help_visible {
            self.help_visible = false;
            self.redraw();
            return;
        }
        if self.menu_visible {
            self.menu_visible = false;
            self.redraw();
            return;
        }
        if self.confirm_exit {
            self.confirm_exit = false;
            self.redraw();
            return;
        }
        // F4 = Back/Exit
        match self.mode {
            AppMode::Pomodoro | AppMode::Stopwatch | AppMode::CountdownList => {
                if self.any_timer_running() {
                    self.confirm_exit = true;
                    self.redraw();
                } else {
                    self.mode = AppMode::ModeSelect;
                    self.redraw();
                }
            }
            AppMode::CountdownRun => {
                self.countdown.stop_active();
                self.stop_pump();
                self.mode = AppMode::CountdownList;
                self.redraw();
            }
            AppMode::Settings => {
                self.mode = AppMode::ModeSelect;
                self.redraw();
            }
            AppMode::ModeSelect => {
                // Top level - quit
            }
        }
    }

    fn help_text(&self) -> &'static str {
        match self.mode {
            AppMode::ModeSelect => {
                "TIMERS HELP\n\n\
                 F1     Menu\n\
                 F4     Quit\n\n\
                 Up/Dn  Move cursor\n\
                 Enter  Open mode\n\
                 s      Settings\n\
                 q      Quit"
            }
            AppMode::Pomodoro => {
                "POMODORO HELP\n\n\
                 F1     Menu\n\
                 F2     Start/Pause\n\
                 F3     Reset\n\
                 F4     Back\n\n\
                 Enter  Start/Pause\n\
                 r      Reset\n\
                 s      Settings\n\
                 q      Back"
            }
            AppMode::Stopwatch => {
                "STOPWATCH HELP\n\n\
                 F1     Menu\n\
                 F2     Start/Pause\n\
                 F3     Reset\n\
                 F4     Back\n\n\
                 Enter  Start/Pause\n\
                 l      Record lap\n\
                 r      Reset (stopped)\n\
                 q      Back"
            }
            AppMode::CountdownList => {
                "COUNTDOWN HELP\n\n\
                 F1     Menu\n\
                 F2     Start/Pause\n\
                 F3     Reset\n\
                 F4     Back\n\n\
                 Enter  Start timer\n\
                 n      New timer\n\
                 d      Delete timer\n\
                 q      Back"
            }
            AppMode::CountdownRun => {
                "COUNTDOWN HELP\n\n\
                 F1     Menu\n\
                 F2     Pause/Resume\n\
                 F3     Reset\n\
                 F4     Back to list\n\n\
                 Enter  Pause/Resume\n\
                 r      Reset\n\
                 q      Back to list"
            }
            AppMode::Settings => {
                "SETTINGS HELP\n\n\
                 F1     Menu\n\
                 F4     Back\n\n\
                 Up/Dn  Move cursor\n\
                 Enter  Toggle setting\n\
                 q      Back"
            }
        }
    }

    fn handle_key_mode_select(&mut self, key: char) {
        match key {
            '↑' | 'k' => {
                if self.mode_cursor > 0 {
                    self.mode_cursor -= 1;
                    self.redraw();
                }
            }
            '↓' | 'j' => {
                if self.mode_cursor < 2 {
                    self.mode_cursor += 1;
                    self.redraw();
                }
            }
            '\r' | '\n' => {
                match self.mode_cursor {
                    0 => self.mode = AppMode::Pomodoro,
                    1 => self.mode = AppMode::Stopwatch,
                    2 => self.mode = AppMode::CountdownList,
                    _ => {}
                }
                self.redraw();
            }
            's' => {
                self.mode = AppMode::Settings;
                self.settings_cursor = 0;
                self.redraw();
            }
            _ => {}
        }
    }

    fn handle_key_pomodoro(&mut self, key: char) {
        let now = self.now_ms();
        match key {
            '\r' | '\n' => {
                match self.pomodoro.timer.state {
                    TimerState::Stopped | TimerState::Paused => {
                        self.pomodoro.timer.start(now);
                        self.start_pump(1000);
                    }
                    TimerState::Running => {
                        self.pomodoro.timer.pause(now);
                        self.stop_pump();
                    }
                    _ => {}
                }
                self.redraw();
            }
            'r' => {
                self.pomodoro.reset();
                self.stop_pump();
                self.redraw();
            }
            's' => {
                self.mode = AppMode::Settings;
                self.settings_cursor = 0;
                self.redraw();
            }
            'q' => {
                if self.pomodoro.timer.state == TimerState::Running {
                    self.pomodoro.timer.pause(now);
                }
                self.stop_pump();
                self.mode = AppMode::ModeSelect;
                self.redraw();
            }
            _ => {}
        }
    }

    fn handle_key_stopwatch(&mut self, key: char) {
        let now = self.now_ms();
        match key {
            '\r' | '\n' => {
                match self.stopwatch.timer.state {
                    TimerState::Stopped | TimerState::Paused => {
                        self.stopwatch.timer.start(now);
                        self.start_pump(100);
                    }
                    TimerState::Running => {
                        self.stopwatch.timer.pause(now);
                        self.stop_pump();
                    }
                    _ => {}
                }
                self.redraw();
            }
            'l' => {
                if self.stopwatch.timer.state == TimerState::Running {
                    self.stopwatch.record_lap(now);
                    self.redraw();
                }
            }
            'r' => {
                if self.stopwatch.timer.state != TimerState::Running {
                    self.stopwatch.reset();
                    self.redraw();
                }
            }
            'q' => {
                if self.stopwatch.timer.state == TimerState::Running {
                    self.stopwatch.timer.pause(now);
                }
                self.stop_pump();
                self.mode = AppMode::ModeSelect;
                self.redraw();
            }
            _ => {}
        }
    }

    fn handle_key_countdown_list(&mut self, key: char) {
        match key {
            '↑' | 'k' => {
                if self.countdown.cursor > 0 {
                    self.countdown.cursor -= 1;
                    self.redraw();
                }
            }
            '↓' | 'j' => {
                if !self.countdown.entries.is_empty()
                    && self.countdown.cursor < self.countdown.entries.len() - 1
                {
                    self.countdown.cursor += 1;
                    self.redraw();
                }
            }
            '\r' | '\n' => {
                if !self.countdown.entries.is_empty() {
                    self.countdown.start_selected();
                    let now = self.now_ms();
                    if let Some(timer) = &mut self.countdown.active_timer {
                        timer.start(now);
                    }
                    self.mode = AppMode::CountdownRun;
                    self.start_pump(1000);
                    self.redraw();
                }
            }
            'n' => {
                self.create_new_countdown();
            }
            'd' => {
                if !self.countdown.entries.is_empty() {
                    self.countdown.delete_selected();
                    self.storage.save_countdowns(&self.countdown.entries);
                    self.redraw();
                }
            }
            'q' => {
                self.mode = AppMode::ModeSelect;
                self.redraw();
            }
            's' => {
                self.mode = AppMode::Settings;
                self.settings_cursor = 0;
                self.redraw();
            }
            _ => {}
        }
    }

    fn handle_key_countdown_run(&mut self, key: char) {
        let now = self.now_ms();
        match key {
            '\r' | '\n' => {
                // Determine action without holding borrow across pump calls
                let action = if let Some(timer) = &mut self.countdown.active_timer {
                    match timer.state {
                        TimerState::Running => {
                            timer.pause(now);
                            Some(false) // need to stop pump
                        }
                        TimerState::Paused => {
                            timer.start(now);
                            Some(true) // need to start pump
                        }
                        _ => None,
                    }
                } else {
                    None
                };
                match action {
                    Some(true) => self.start_pump(1000),
                    Some(false) => self.stop_pump(),
                    None => {}
                }
                self.redraw();
            }
            'r' => {
                // Reset to original duration
                self.countdown.start_selected();
                self.stop_pump();
                self.redraw();
            }
            'q' => {
                self.countdown.stop_active();
                self.stop_pump();
                self.mode = AppMode::CountdownList;
                self.redraw();
            }
            _ => {}
        }
    }

    fn handle_key_settings(&mut self, key: char) {
        match key {
            '↑' | 'k' => {
                if self.settings_cursor > 0 {
                    self.settings_cursor -= 1;
                    self.redraw();
                }
            }
            '↓' | 'j' => {
                if self.settings_cursor < 2 {
                    self.settings_cursor += 1;
                    self.redraw();
                }
            }
            '\r' | '\n' => {
                match self.settings_cursor {
                    0 => self.alert_config.vibration = !self.alert_config.vibration,
                    1 => self.alert_config.notification = !self.alert_config.notification,
                    2 => self.alert_config.audio = !self.alert_config.audio,
                    _ => {}
                }
                self.storage.save_alert_config(&self.alert_config);
                self.redraw();
            }
            'q' => {
                // Return to previous mode
                self.mode = AppMode::ModeSelect;
                self.redraw();
            }
            _ => {}
        }
    }

    fn create_new_countdown(&mut self) {
        // Use modals for name input
        let name = match self.modals.alert_builder("Timer name:")
            .field(Some("Timer".to_string()), None)
            .build()
        {
            Ok(response) => {
                let payload = response.first();
                if payload.content.is_empty() {
                    return;
                }
                let mut name = payload.content.clone();
                name.truncate(20);
                name
            }
            Err(_) => return,
        };

        // Use modals for duration input (in seconds)
        let duration_ms = match self.modals.alert_builder("Duration (MM:SS):")
            .field(Some("05:00".to_string()), None)
            .build()
        {
            Ok(response) => {
                let payload = response.first();
                parse_mmss(&payload.content)
            }
            Err(_) => return,
        };

        if duration_ms > 0 {
            self.countdown.add_entry(name, duration_ms);
            self.storage.save_countdowns(&self.countdown.entries);
        }
        self.redraw();
    }
}

/// Parse "MM:SS" format into milliseconds
fn parse_mmss(s: &str) -> u64 {
    let parts: Vec<&str> = s.split(':').collect();
    match parts.len() {
        1 => {
            // Just seconds
            if let Ok(secs) = parts[0].trim().parse::<u64>() {
                secs * 1000
            } else {
                0
            }
        }
        2 => {
            let mins = parts[0].trim().parse::<u64>().unwrap_or(0);
            let secs = parts[1].trim().parse::<u64>().unwrap_or(0);
            (mins * 60 + secs) * 1000
        }
        _ => 0,
    }
}

fn pump_thread(pump_sid: xous::SID, main_conn: xous::CID) {
    let tt = ticktimer_server::Ticktimer::new().unwrap();
    let mut interval_ms = 1000u64;
    let mut running = false;

    loop {
        if running {
            tt.sleep_ms(interval_ms as usize).ok();
            xous::send_message(
                main_conn,
                xous::Message::new_scalar(AppOp::Pump.to_u32().unwrap() as usize, 0, 0, 0, 0),
            ).ok();
        }

        // Check for control messages (non-blocking when running, blocking when stopped)
        let envelope = if running {
            match xous::try_receive_message(pump_sid) {
                Ok(Some(env)) => Some(env),
                _ => None,
            }
        } else {
            // Block-wait when stopped
            xous::receive_message(pump_sid).ok()
        };

        if let Some(env) = envelope {
            // Extract opcode and arg from scalar message
            if let xous::Message::Scalar(scalar) = &env.body {
                match scalar.id {
                    0 => {
                        // Start with interval
                        interval_ms = scalar.arg1 as u64;
                        if interval_ms == 0 { interval_ms = 100; }
                        running = true;
                    }
                    1 => {
                        // Stop
                        running = false;
                    }
                    2 => {
                        // Quit
                        break;
                    }
                    _ => {}
                }
            }
        }
    }
}

fn main() -> ! {
    log_server::init_wait().unwrap();
    log::set_max_level(log::LevelFilter::Info);
    log::info!("Timers PID is {}", xous::process::id());

    let xns = xous_names::XousNames::new().unwrap();
    let sid = xns.register_name(SERVER_NAME, None).expect("can't register server");
    let main_conn = xous::connect(sid).expect("can't connect to self");

    // Create pump thread
    let pump_sid = xous::create_server().expect("can't create pump server");
    std::thread::spawn(move || {
        pump_thread(pump_sid, main_conn);
    });

    let mut app = TimersApp::new(&xns, sid, pump_sid);
    app.allow_redraw = true;

    loop {
        let msg = xous::receive_message(sid).unwrap();
        match FromPrimitive::from_usize(msg.body.id()) {
            Some(AppOp::Redraw) => {
                app.redraw();
            }
            Some(AppOp::Rawkeys) => xous::msg_scalar_unpack!(msg, k1, k2, k3, k4, {
                let keys = [
                    core::char::from_u32(k1 as u32).unwrap_or('\u{0000}'),
                    core::char::from_u32(k2 as u32).unwrap_or('\u{0000}'),
                    core::char::from_u32(k3 as u32).unwrap_or('\u{0000}'),
                    core::char::from_u32(k4 as u32).unwrap_or('\u{0000}'),
                ];
                for &key in keys.iter() {
                    if key != '\u{0000}' {
                        app.handle_key(key);
                    }
                }
            }),
            Some(AppOp::FocusChange) => xous::msg_scalar_unpack!(msg, new_state_code, _, _, _, {
                let new_state = gam::FocusState::convert_focus_change(new_state_code);
                match new_state {
                    gam::FocusState::Background => {
                        app.allow_redraw = false;
                        app.stop_pump();
                    }
                    gam::FocusState::Foreground => {
                        app.allow_redraw = true;
                        // Restart pump if a timer is running
                        match app.mode {
                            AppMode::Stopwatch if app.stopwatch.timer.state == TimerState::Running => {
                                app.start_pump(100);
                            }
                            AppMode::Pomodoro if app.pomodoro.timer.state == TimerState::Running => {
                                app.start_pump(1000);
                            }
                            AppMode::CountdownRun => {
                                let should_pump = app.countdown.active_timer.as_ref()
                                    .map(|t| t.state == TimerState::Running)
                                    .unwrap_or(false);
                                if should_pump {
                                    app.start_pump(1000);
                                }
                            }
                            _ => {}
                        }
                        app.redraw();
                    }
                }
            }),
            Some(AppOp::Pump) => {
                app.handle_pump();
            }
            Some(AppOp::Quit) => break,
            _ => log::error!("unknown opcode: {:?}", msg),
        }
    }

    // Clean up
    app.stop_pump();
    xous::send_message(app.pump_conn, xous::Message::new_scalar(2, 0, 0, 0, 0)).ok();
    xns.unregister_server(sid).unwrap();
    xous::destroy_server(sid).unwrap();
    xous::terminate_process(0)
}
