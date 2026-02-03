use std::io::{Read, Write, Seek, SeekFrom};

use crate::countdown::CountdownEntry;
use crate::alerts::AlertConfig;

const DICT_NAME: &str = "timers";
const KEY_POMODORO: &str = "pomodoro_settings";
const KEY_ALERTS: &str = "alert_config";
const KEY_COUNTDOWNS: &str = "countdowns";

pub struct TimerStorage {
    pddb: pddb::Pddb,
}

impl TimerStorage {
    pub fn new() -> Self {
        let pddb = pddb::Pddb::new();
        pddb.try_mount();
        Self { pddb }
    }

    pub fn load_pomodoro_settings(&self) -> Option<(u64, u64, u64, u8)> {
        match self.pddb.get(DICT_NAME, KEY_POMODORO, None, false, false, None, None::<fn()>) {
            Ok(mut key) => {
                let mut buf = [0u8; 25]; // 3 * u64 + 1 * u8
                key.seek(SeekFrom::Start(0)).ok();
                if key.read_exact(&mut buf).is_ok() {
                    let work = u64::from_le_bytes(buf[0..8].try_into().unwrap());
                    let short = u64::from_le_bytes(buf[8..16].try_into().unwrap());
                    let long = u64::from_le_bytes(buf[16..24].try_into().unwrap());
                    let cycles = buf[24];
                    Some((work, short, long, cycles))
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }

    pub fn save_pomodoro_settings(&self, work: u64, short: u64, long: u64, cycles: u8) {
        let mut data = [0u8; 25];
        data[0..8].copy_from_slice(&work.to_le_bytes());
        data[8..16].copy_from_slice(&short.to_le_bytes());
        data[16..24].copy_from_slice(&long.to_le_bytes());
        data[24] = cycles;

        match self.pddb.get(DICT_NAME, KEY_POMODORO, None, true, true, Some(25), None::<fn()>) {
            Ok(mut key) => {
                key.seek(SeekFrom::Start(0)).ok();
                key.write_all(&data).ok();
                self.pddb.sync().ok();
            }
            Err(e) => log::error!("Failed to save pomodoro settings: {:?}", e),
        }
    }

    pub fn load_alert_config(&self) -> AlertConfig {
        match self.pddb.get(DICT_NAME, KEY_ALERTS, None, false, false, None, None::<fn()>) {
            Ok(mut key) => {
                let mut buf = [0u8; 3];
                key.seek(SeekFrom::Start(0)).ok();
                if key.read_exact(&mut buf).is_ok() {
                    AlertConfig {
                        vibration: buf[0] != 0,
                        audio: buf[1] != 0,
                        notification: buf[2] != 0,
                    }
                } else {
                    AlertConfig::default()
                }
            }
            Err(_) => AlertConfig::default(),
        }
    }

    pub fn save_alert_config(&self, config: &AlertConfig) {
        let data = [
            config.vibration as u8,
            config.audio as u8,
            config.notification as u8,
        ];

        match self.pddb.get(DICT_NAME, KEY_ALERTS, None, true, true, Some(3), None::<fn()>) {
            Ok(mut key) => {
                key.seek(SeekFrom::Start(0)).ok();
                key.write_all(&data).ok();
                self.pddb.sync().ok();
            }
            Err(e) => log::error!("Failed to save alert config: {:?}", e),
        }
    }

    pub fn load_countdowns(&self) -> Vec<CountdownEntry> {
        match self.pddb.get(DICT_NAME, KEY_COUNTDOWNS, None, false, false, None, None::<fn()>) {
            Ok(mut key) => {
                let mut data = Vec::new();
                key.seek(SeekFrom::Start(0)).ok();
                if key.read_to_end(&mut data).is_ok() && data.len() >= 4 {
                    deserialize_countdowns(&data)
                } else {
                    Vec::new()
                }
            }
            Err(_) => Vec::new(),
        }
    }

    pub fn save_countdowns(&self, entries: &[CountdownEntry]) {
        let data = serialize_countdowns(entries);
        match self.pddb.get(DICT_NAME, KEY_COUNTDOWNS, None, true, true, Some(data.len()), None::<fn()>) {
            Ok(mut key) => {
                key.seek(SeekFrom::Start(0)).ok();
                key.write_all(&data).ok();
                self.pddb.sync().ok();
            }
            Err(e) => log::error!("Failed to save countdowns: {:?}", e),
        }
    }
}

fn serialize_countdowns(entries: &[CountdownEntry]) -> Vec<u8> {
    let mut data = Vec::new();
    let count = entries.len() as u32;
    data.extend_from_slice(&count.to_le_bytes());
    for entry in entries {
        let name_bytes = entry.name.as_bytes();
        let name_len = name_bytes.len() as u16;
        data.extend_from_slice(&name_len.to_le_bytes());
        data.extend_from_slice(name_bytes);
        data.extend_from_slice(&entry.duration_ms.to_le_bytes());
    }
    data
}

fn deserialize_countdowns(data: &[u8]) -> Vec<CountdownEntry> {
    let mut entries = Vec::new();
    if data.len() < 4 {
        return entries;
    }
    let count = u32::from_le_bytes(data[0..4].try_into().unwrap()) as usize;
    let mut offset = 4;

    for _ in 0..count {
        if offset + 2 > data.len() {
            break;
        }
        let name_len = u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap()) as usize;
        offset += 2;

        if offset + name_len > data.len() {
            break;
        }
        let name = String::from_utf8_lossy(&data[offset..offset + name_len]).to_string();
        offset += name_len;

        if offset + 8 > data.len() {
            break;
        }
        let duration_ms = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
        offset += 8;

        entries.push(CountdownEntry { name, duration_ms });
    }
    entries
}
