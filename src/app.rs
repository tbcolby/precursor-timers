//! Glue: clock, pump, storage and alerts around `timer-core`.

use std::sync::mpsc::{channel, Sender};

use precursor_appkit::keys::Key;
use precursor_appkit::pump::Pump;
use precursor_appkit::screen::Screen;
use precursor_appkit::store::Store;
use timer_core::clock::SuspendSafeClock;
use timer_core::saved::{Saved, KEY, LEGACY_ALERTS, LEGACY_COUNTDOWNS, LEGACY_POMODORO};
use timer_core::{Alert, Timers};

pub struct App {
    screen: Screen,
    timers: Timers,
    store: Store,
    /// Saved data couldn't be read; don't overwrite it.
    read_only: bool,
    pump: Pump,
    pump_ms: Option<u64>,
    foreground: bool,
    tt: ticktimer_server::Ticktimer,
    wall: llio::LocalTime,
    clock: SuspendSafeClock,
    alerts: Sender<Alert>,
}

/// Alerts run on their own thread: the notification modal blocks until it
/// is dismissed, and the main loop must keep ticking meanwhile.
fn spawn_alert_thread() -> Sender<Alert> {
    let (tx, rx) = channel::<Alert>();
    std::thread::spawn(move || {
        let xns = xous_names::XousNames::new().unwrap();
        let llio = llio::Llio::new(&xns);
        let modals = modals::Modals::new(&xns).expect("timers: no modals server");
        while let Ok(alert) = rx.recv() {
            if alert.vibrate {
                llio.vibe(llio::VibePattern::Double).ok();
            }
            if alert.notify {
                modals.show_notification(&alert.message, None).ok();
            }
        }
    });
    tx
}

fn load(store: &Store) -> (Saved, Option<&'static str>) {
    match store.read_bytes(KEY) {
        Ok(Some(bytes)) => match Saved::decode(&bytes) {
            Ok(s) => (s, None),
            Err(e) => (Saved::default(), Some(e.message())),
        },
        Ok(None) => {
            let get = |k: &str| store.read_bytes(k).ok().flatten();
            let (p, a, c) = (get(LEGACY_POMODORO), get(LEGACY_ALERTS), get(LEGACY_COUNTDOWNS));
            let migrate = p.is_some() || a.is_some() || c.is_some();
            let saved = Saved::from_legacy(p.as_deref(), a.as_deref(), c.as_deref());
            if migrate && store.write_bytes(KEY, &saved.encode()).is_ok() {
                for k in [LEGACY_POMODORO, LEGACY_ALERTS, LEGACY_COUNTDOWNS] {
                    store.delete(k).ok();
                }
            }
            (saved, None)
        }
        Err(e) => {
            log::warn!("timers: settings read failed: {:?}", e.kind());
            (Saved::default(), Some("Saved settings unreadable; changes won't be saved"))
        }
    }
}

impl App {
    pub fn new(screen: Screen, store: Store, pump: Pump) -> Self {
        let (saved, problem) = load(&store);
        let mut timers = Timers::new(saved);
        if let Some(p) = problem {
            timers.set_status(p);
        }
        let mut app = Self {
            screen,
            timers,
            store,
            read_only: problem.is_some(),
            pump,
            pump_ms: None,
            foreground: true,
            tt: ticktimer_server::Ticktimer::new().expect("timers: no ticktimer"),
            wall: llio::LocalTime::new(),
            clock: SuspendSafeClock::default(),
            alerts: spawn_alert_thread(),
        };
        app.draw();
        app
    }

    pub fn screen(&self) -> &Screen { &self.screen }

    /// Monotonic ms that keeps counting across suspend.
    fn now(&mut self) -> u64 {
        let tick = self.tt.elapsed_ms();
        let wall = self.wall.get_local_time_ms();
        self.clock.now(tick, wall)
    }

    /// Run, retime or stop the pump to match what the timers need.
    fn update_pump(&mut self) {
        let want = self.timers.pump_interval(self.foreground);
        if want == self.pump_ms {
            return;
        }
        match want {
            Some(ms) => {
                self.pump.set_interval(ms as usize);
                self.pump.run();
            }
            None => self.pump.stop(),
        }
        self.pump_ms = want;
    }

    fn save_if_dirty(&mut self) {
        if !self.timers.take_dirty() {
            return;
        }
        if self.read_only {
            self.timers.set_status("Not saved: stored settings are unreadable");
            return;
        }
        if let Err(e) = self.store.write_bytes(KEY, &self.timers.saved().encode()) {
            log::warn!("timers: save failed: {:?}", e.kind());
            self.timers.set_status("Not saved: storage error");
        }
    }

    /// Returns true when the app should hand focus back.
    pub fn on_key(&mut self, key: Key) -> bool {
        let now = self.now();
        let leave = self.timers.on_key(key, now);
        self.save_if_dirty();
        self.update_pump();
        leave
    }

    pub fn tick(&mut self) {
        let now = self.now();
        for alert in self.timers.tick(now) {
            log::info!("timers: alert");
            self.alerts.send(alert).ok();
        }
        self.update_pump();
        self.draw();
    }

    pub fn set_foreground(&mut self, fg: bool) {
        self.foreground = fg;
        if fg {
            self.screen.refresh_size();
            self.draw();
        }
        // Background keeps the pump only while an alert is armed.
        self.update_pump();
    }

    pub fn reload(&mut self) {
        let (saved, problem) = load(&self.store);
        self.read_only = problem.is_some();
        self.timers.reload(saved);
        if let Some(p) = problem {
            self.timers.set_status(p);
        }
        self.update_pump();
        self.draw();
    }

    /// Drawing is skipped in the background.
    pub fn draw(&mut self) {
        if self.foreground {
            let now = self.now();
            crate::ui::draw(&self.screen, &self.timers, now);
        }
    }

    pub fn quit(self) { self.pump.quit(); }
}
