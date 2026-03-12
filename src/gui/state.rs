use ak820_ctl::protocol::{Direction, LightingMode, SleepTime};

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

pub struct SleepState {
    pub selected: usize,
}

impl Default for SleepState {
    fn default() -> Self {
        Self { selected: 0 }
    }
}

impl SleepState {
    pub const OPTIONS: &[(& str, SleepTime)] = &[
        ("Never", SleepTime::Never),
        ("1 minute", SleepTime::OneMinute),
        ("5 minutes", SleepTime::FiveMinutes),
        ("30 minutes", SleepTime::ThirtyMinutes),
    ];

    pub fn current(&self) -> SleepTime {
        Self::OPTIONS[self.selected].1
    }
}

pub struct ClockState {
    pub last_sync: Option<String>,
}

pub enum ConnectionStatus {
    Connected,
    Error(String),
}
