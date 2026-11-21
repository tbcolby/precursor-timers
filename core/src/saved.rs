//! The saved settings record (one JSON value) and the 0.1 binary formats.

use serde::{Deserialize, Serialize};

use crate::countdown::{clean_name, CountdownEntry, MAX_COUNTDOWNS, MAX_DURATION_MS};
use crate::pomodoro::PomodoroSettings;

pub const DICT: &str = "timers";
pub const KEY: &str = "settings";
/// 0.1 keys, read once if `settings` is missing.
pub const LEGACY_POMODORO: &str = "pomodoro_settings";
pub const LEGACY_ALERTS: &str = "alert_config";
pub const LEGACY_COUNTDOWNS: &str = "countdowns";
pub const VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Saved {
    pub version: u32,
    pub vibrate: bool,
    pub notify: bool,
    pub pomodoro: PomodoroSettings,
    pub countdowns: Vec<CountdownEntry>,
}

impl Default for Saved {
    fn default() -> Self {
        Self { version: VERSION, vibrate: true, notify: true, pomodoro: PomodoroSettings::default(), countdowns: Vec::new() }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadError {
    Unreadable,
    Newer,
}

impl LoadError {
    pub fn message(self) -> &'static str {
        match self {
            LoadError::Unreadable => "Saved settings unreadable; changes won't be saved",
            LoadError::Newer => "Settings from a newer version; changes won't be saved",
        }
    }
}

impl Saved {
    /// Clamp values a hand-edited or old record might carry.
    pub fn validated(mut self) -> Self {
        self.version = VERSION;
        self.pomodoro = self.pomodoro.validated();
        self.countdowns = self
            .countdowns
            .into_iter()
            .filter(|c| c.duration_ms > 0 && c.duration_ms <= MAX_DURATION_MS)
            .filter_map(|c| clean_name(&c.name).ok().map(|name| CountdownEntry { name, duration_ms: c.duration_ms }))
            .take(MAX_COUNTDOWNS)
            .collect();
        self
    }

    pub fn decode(bytes: &[u8]) -> Result<Saved, LoadError> {
        let s: Saved = serde_json::from_slice(bytes).map_err(|_| LoadError::Unreadable)?;
        if s.version > VERSION {
            return Err(LoadError::Newer);
        }
        Ok(s.validated())
    }

    pub fn encode(&self) -> Vec<u8> { serde_json::to_vec(self).expect("plain struct always serializes") }

    /// Build from the 0.1 keys (any may be missing).
    pub fn from_legacy(pomodoro: Option<&[u8]>, alerts: Option<&[u8]>, countdowns: Option<&[u8]>) -> Saved {
        let mut s = Saved::default();
        if let Some(p) = pomodoro.and_then(legacy_pomodoro) {
            s.pomodoro = p;
        }
        if let Some(&[vibrate, _audio, notify, ..]) = alerts {
            s.vibrate = vibrate != 0;
            s.notify = notify != 0;
        }
        if let Some(c) = countdowns {
            s.countdowns = legacy_countdowns(c);
        }
        s.validated()
    }
}

/// 0.1: `[u64 work_ms][u64 short_ms][u64 long_ms][u8 cycles]`, little-endian.
fn legacy_pomodoro(b: &[u8]) -> Option<PomodoroSettings> {
    if b.len() < 25 {
        return None;
    }
    let ms = |i: usize| u64::from_le_bytes(b[i..i + 8].try_into().unwrap());
    let min = |v: u64| u32::try_from(v / 60_000).unwrap_or(u32::MAX);
    Some(PomodoroSettings { work_min: min(ms(0)), short_min: min(ms(8)), long_min: min(ms(16)), cycles: u32::from(b[24]) })
}

/// 0.1: `[u32 count]` then `[u16 name_len][name][u64 duration_ms]` each.
fn legacy_countdowns(b: &[u8]) -> Vec<CountdownEntry> {
    let mut out = Vec::new();
    let Some(count) = b.get(0..4) else { return out };
    let count = u32::from_le_bytes(count.try_into().unwrap()) as usize;
    let mut at = 4;
    for _ in 0..count.min(MAX_COUNTDOWNS) {
        let Some(len) = b.get(at..at + 2) else { break };
        let len = u16::from_le_bytes(len.try_into().unwrap()) as usize;
        at += 2;
        let Some(name) = b.get(at..at + len) else { break };
        at += len;
        let Some(d) = b.get(at..at + 8) else { break };
        at += 8;
        out.push(CountdownEntry {
            name: String::from_utf8_lossy(name).into_owned(),
            duration_ms: u64::from_le_bytes(d.try_into().unwrap()),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_versions() {
        let mut s = Saved::default();
        s.countdowns.push(CountdownEntry { name: "Tea".into(), duration_ms: 180_000 });
        assert_eq!(Saved::decode(&s.encode()), Ok(s.clone()));
        assert_eq!(Saved::decode(b"{\"version\":2}"), Err(LoadError::Newer));
        assert_eq!(Saved::decode(b"{\"version\":1,\"vib"), Err(LoadError::Unreadable));
        // missing fields take defaults
        assert_eq!(Saved::decode(b"{\"version\":1}"), Ok(Saved::default()));
    }

    #[test]
    fn decode_clamps_bad_values() {
        let j = br#"{"version":1,"pomodoro":{"work_min":0,"short_min":5,"long_min":15,"cycles":99},
            "countdowns":[{"name":"ok","duration_ms":1000},{"name":"zero","duration_ms":0},{"name":"  ","duration_ms":5}]}"#;
        let s = Saved::decode(j).unwrap();
        assert_eq!(s.pomodoro.work_min, 1);
        assert_eq!(s.pomodoro.cycles, 12);
        assert_eq!(s.countdowns.len(), 1);
    }

    #[test]
    fn legacy_records() {
        let mut p = Vec::new();
        for ms in [50u64 * 60_000, 10 * 60_000, 0] {
            p.extend_from_slice(&ms.to_le_bytes());
        }
        p.push(3);
        let mut c = 2u32.to_le_bytes().to_vec();
        for (name, ms) in [("Tea", 180_000u64), ("Eggs", 420_000)] {
            c.extend_from_slice(&(name.len() as u16).to_le_bytes());
            c.extend_from_slice(name.as_bytes());
            c.extend_from_slice(&ms.to_le_bytes());
        }
        let s = Saved::from_legacy(Some(&p), Some(&[0, 1, 1]), Some(&c));
        assert_eq!(s.pomodoro, PomodoroSettings { work_min: 50, short_min: 10, long_min: 1, cycles: 3 });
        assert!(!s.vibrate && s.notify);
        assert_eq!(s.countdowns[1], CountdownEntry { name: "Eggs".into(), duration_ms: 420_000 });
        // truncated data: keep what parsed
        let s = Saved::from_legacy(Some(&p[..10]), None, Some(&c[..20]));
        assert_eq!(s.pomodoro, PomodoroSettings::default());
        assert_eq!(s.countdowns.len(), 1);
        assert_eq!(Saved::from_legacy(None, None, None), Saved::default());
    }
}
