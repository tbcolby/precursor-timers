//! Named countdown presets, one of which can be running.

use serde::{Deserialize, Serialize};

use crate::timer::TimerCore;

pub const MAX_COUNTDOWNS: usize = 20;
pub const MAX_NAME_CHARS: usize = 20;
/// Longest countdown: 24 hours.
pub const MAX_DURATION_MS: u64 = 24 * 3600 * 1000;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CountdownEntry {
    pub name: String,
    pub duration_ms: u64,
}

/// Parse `SS`, `MM:SS` or `H:MM:SS`. With a colon, seconds (and minutes when
/// hours are given) must be below 60. The result is 1 s to 24 h.
pub fn parse_duration(s: &str) -> Result<u64, &'static str> {
    let parts: Vec<&str> = s.trim().split(':').collect();
    if parts.len() > 3 || parts.iter().any(|p| p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit()) || p.len() > 6) {
        return Err("Use SS, MM:SS or H:MM:SS");
    }
    let n: Vec<u64> = parts.iter().map(|p| p.parse().unwrap_or(u64::MAX)).collect();
    let secs = match n.as_slice() {
        [s] => *s,
        [m, s] if *s < 60 => m * 60 + s,
        [h, m, s] if *s < 60 && *m < 60 => h * 3600 + m * 60 + s,
        _ => return Err("Seconds and minutes must be under 60"),
    };
    match secs.checked_mul(1000) {
        Some(0) => Err("Duration must be at least 1 second"),
        Some(ms) if ms <= MAX_DURATION_MS => Ok(ms),
        _ => Err("Longest countdown is 24 hours"),
    }
}

/// Trim, drop control characters, keep at most 20 characters.
pub fn clean_name(s: &str) -> Result<String, &'static str> {
    let name: String = s.trim().chars().filter(|c| !c.is_control()).take(MAX_NAME_CHARS).collect();
    if name.is_empty() {
        Err("Name can't be empty")
    } else {
        Ok(name)
    }
}

#[derive(Clone, Debug, Default)]
pub struct Countdowns {
    entries: Vec<CountdownEntry>,
    active: Option<(usize, TimerCore)>,
}

impl Countdowns {
    pub fn new(entries: Vec<CountdownEntry>) -> Self { Self { entries, active: None } }

    pub fn entries(&self) -> &[CountdownEntry] { &self.entries }

    pub fn is_full(&self) -> bool { self.entries.len() >= MAX_COUNTDOWNS }

    pub fn add(&mut self, entry: CountdownEntry) -> Result<usize, &'static str> {
        if self.is_full() {
            return Err("20 countdowns is the limit; delete one first");
        }
        self.entries.push(entry);
        Ok(self.entries.len() - 1)
    }

    /// Delete entry `i`, stopping it if it is the active one.
    pub fn delete(&mut self, i: usize) {
        if i >= self.entries.len() {
            return;
        }
        self.entries.remove(i);
        self.active = match self.active.take() {
            Some((a, _)) if a == i => None,
            Some((a, t)) if a > i => Some((a - 1, t)),
            other => other,
        };
    }

    /// Make entry `i` active and start it.
    pub fn start(&mut self, i: usize, now: u64) {
        if let Some(e) = self.entries.get(i) {
            let mut t = TimerCore::new_countdown(e.duration_ms);
            t.start(now);
            self.active = Some((i, t));
        }
    }

    pub fn active_index(&self) -> Option<usize> { self.active.as_ref().map(|(i, _)| *i) }

    pub fn active(&self) -> Option<&TimerCore> { self.active.as_ref().map(|(_, t)| t) }

    pub fn active_mut(&mut self) -> Option<&mut TimerCore> { self.active.as_mut().map(|(_, t)| t) }

    pub fn active_name(&self) -> Option<&str> { self.active_index().map(|i| self.entries[i].name.as_str()) }

    pub fn is_running(&self) -> bool { self.active().is_some_and(|t| t.is_running()) }

    pub fn stop(&mut self) { self.active = None; }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations() {
        assert_eq!(parse_duration("90"), Ok(90_000));
        assert_eq!(parse_duration("05:00"), Ok(300_000));
        assert_eq!(parse_duration(" 1:02:03 "), Ok(3_723_000));
        assert_eq!(parse_duration("120:00"), Ok(7_200_000));
        // Audit: these were dropped silently or accepted.
        assert!(parse_duration("1:75").is_err());
        assert!(parse_duration("1:2:3:4").is_err());
        assert!(parse_duration("abc").is_err());
        assert!(parse_duration("0").is_err());
        assert!(parse_duration("00:00").is_err());
        assert!(parse_duration("").is_err());
        assert!(parse_duration("-5").is_err());
        assert!(parse_duration("25:00:00").is_err());
        assert!(parse_duration("99999999999999999999").is_err());
        assert_eq!(parse_duration("24:00:00"), Ok(MAX_DURATION_MS));
    }

    #[test]
    fn names() {
        assert_eq!(clean_name("  Tea  "), Ok("Tea".into()));
        assert_eq!(clean_name(&"福".repeat(30)).unwrap().chars().count(), 20);
        assert!(clean_name(" \u{7} ").is_err());
    }

    #[test]
    fn limit_and_delete_keep_active_index() {
        let mut c = Countdowns::default();
        for i in 0..MAX_COUNTDOWNS {
            c.add(CountdownEntry { name: format!("t{i}"), duration_ms: 1_000 }).unwrap();
        }
        assert!(c.add(CountdownEntry { name: "x".into(), duration_ms: 1 }).is_err());
        c.start(5, 0);
        c.delete(2);
        assert_eq!(c.active_index(), Some(4));
        assert_eq!(c.active_name(), Some("t5"));
        c.delete(9);
        assert_eq!(c.active_index(), Some(4));
        c.delete(4);
        assert_eq!(c.active_index(), None);
    }
}
