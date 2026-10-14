use ak820_ctl::protocol::{Direction, KeyResponse, LightingMode, SleepTime};
use ak820_ctl::settings_store::StoredSettings;

pub struct LightingState {
    pub mode_index: usize,
    pub color: [u8; 3],
    pub rainbow: bool,
    pub brightness: u8,
    pub speed: u8,
    pub direction_index: usize,
}

impl Default for LightingState {
    fn default() -> Self {
        Self {
            mode_index: 1, // Static
            color: [255, 0, 0],
            rainbow: false,
            brightness: 5,
            speed: 3,
            direction_index: 0,
        }
    }
}

impl LightingState {
    pub fn current_mode(&self) -> LightingMode {
        LightingMode::ALL[self.mode_index]
    }

    pub fn current_direction(&self) -> Direction {
        let mode = self.current_mode();
        let dirs = mode.supported_directions();
        if dirs.is_empty() {
            Direction::Left
        } else {
            dirs[self.direction_index.min(dirs.len() - 1)]
        }
    }
}

/// Sleep timer + key response time, written to the keyboard together.
pub struct SettingsState {
    pub sleep_selected: usize,
    pub key_response: u8,
}

impl Default for SettingsState {
    /// Start from the last applied values, since the keyboard can't report them.
    fn default() -> Self {
        let stored = StoredSettings::load();
        Self {
            sleep_selected: stored
                .sleep
                .and_then(|s| Self::SLEEP_OPTIONS.iter().position(|(_, o)| *o == s))
                .unwrap_or(0),
            key_response: stored.key_response.map_or(1, |k| k.level()),
        }
    }
}

impl SettingsState {
    pub const SLEEP_OPTIONS: &[(& str, SleepTime)] = &[
        ("Never", SleepTime::Never),
        ("1 minute", SleepTime::OneMinute),
        ("5 minutes", SleepTime::FiveMinutes),
        ("30 minutes", SleepTime::ThirtyMinutes),
    ];

    pub fn sleep(&self) -> SleepTime {
        Self::SLEEP_OPTIONS[self.sleep_selected].1
    }

    pub fn key_response(&self) -> KeyResponse {
        KeyResponse::new(self.key_response).expect("slider is limited to valid levels")
    }
}

pub struct ClockState {
    pub last_sync: Option<String>,
}

pub enum ConnectionStatus {
    Connected,
    Error(String),
}
