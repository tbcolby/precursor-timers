use llio::{Llio, VibePattern};

#[derive(Clone)]
pub struct AlertConfig {
    pub vibration: bool,
    pub audio: bool,
    pub notification: bool,
}

impl AlertConfig {
    pub fn default() -> Self {
        Self {
            vibration: true,
            audio: false,
            notification: true,
        }
    }
}

pub fn fire_alert(config: &AlertConfig, llio: &Llio, modals: &modals::Modals, message: &str) {
    if config.vibration {
        llio.vibe(VibePattern::Double).ok();
    }
    if config.notification {
        modals.show_notification(message, None).ok();
    }
    // Audio tone generation could be added here with codec support
}
