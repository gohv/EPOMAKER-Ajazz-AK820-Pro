/// HID protocol definitions for the Ajazz AK820 Pro keyboard.
/// Based on reverse-engineering from TaxMachine/ajazz-keyboard-software-linux.

pub const VENDOR_ID: u16 = 0x0C45;

/// Layout of the settings block sent after the CMD_SETTINGS preamble.
/// Firmware revisions disagree on where each setting lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsLayout {
    /// Original reverse-engineering: sleep time at byte 8, no key response time.
    Legacy,
    /// Captured from the official Windows app (v1.0.0.5) on PID 800A:
    /// sleep time at byte 6, key response time (debounce) at byte 8.
    V1_1,
}

/// A supported keyboard revision.
#[derive(Debug)]
pub struct Model {
    pub pid: u16,
    pub name: &'static str,
    pub settings: SettingsLayout,
}

impl Model {
    pub fn supports_key_response(&self) -> bool {
        self.settings == SettingsLayout::V1_1
    }
}

/// All known AK820 Pro revisions. Add new PIDs here only.
pub const SUPPORTED_MODELS: &[Model] = &[
    Model { pid: 0x8009, name: "AK820 Pro", settings: SettingsLayout::Legacy },
    Model { pid: 0x800A, name: "AK820 Pro (v1.1)", settings: SettingsLayout::V1_1 },
];

/// Model used for a PID forced via AK820_PID that isn't in SUPPORTED_MODELS.
static OVERRIDE_MODEL: std::sync::OnceLock<Option<Model>> = std::sync::OnceLock::new();

/// PID forced via the AK820_PID environment variable (hex, e.g. "800b" or "0x800b").
fn override_pid() -> Option<u16> {
    let v = std::env::var("AK820_PID").ok()?;
    u16::from_str_radix(v.trim().trim_start_matches("0x").trim_start_matches("0X"), 16).ok()
}

/// Look up the model for a USB VID/PID, honouring the AK820_PID override.
/// Unlisted override PIDs are assumed to use the newest settings layout.
pub fn find_model(vid: u16, pid: u16) -> Option<&'static Model> {
    if vid != VENDOR_ID {
        return None;
    }
    if let Some(model) = SUPPORTED_MODELS.iter().find(|m| m.pid == pid) {
        return Some(model);
    }
    OVERRIDE_MODEL
        .get_or_init(|| override_pid().map(|pid| Model {
            pid,
            name: "AK820 Pro (AK820_PID override)",
            settings: SettingsLayout::V1_1,
        }))
        .as_ref()
        .filter(|m| m.pid == pid)
}

/// Human-readable list of supported PIDs, e.g. "8009|800a".
pub fn supported_pids() -> String {
    let mut pids: Vec<String> = SUPPORTED_MODELS.iter().map(|m| format!("{:04x}", m.pid)).collect();
    if let Some(pid) = override_pid().filter(|p| SUPPORTED_MODELS.iter().all(|m| m.pid != *p)) {
        pids.push(format!("{:04x}", pid));
    }
    pids.join("|")
}

pub const PACKET_LENGTH: usize = 64;
pub const REPORT_ID: u8 = 0x04;

// Image upload constants
pub const IMAGE_CHUNK_SIZE: usize = 4123;
pub const IMAGE_NUM_CHUNKS: usize = 9;
pub const LCD_WIDTH: u32 = 128;
pub const LCD_HEIGHT: u32 = 128;
pub const LCD_PIXELS: usize = (LCD_WIDTH * LCD_HEIGHT) as usize;
pub const LCD_DATA_SIZE: usize = LCD_PIXELS * 2; // RGB565 = 2 bytes per pixel

// Command codes (byte 1 of control packets)
pub const CMD_START: u8 = 0x18;
pub const CMD_FINISH: u8 = 0xF0;
pub const CMD_MODE: u8 = 0x13;
pub const CMD_SETTINGS: u8 = 0x17; // sleep time + key response time block
pub const CMD_IMAGE: u8 = 0x72;
pub const CMD_TIME: u8 = 0x28;
pub const CMD_SAVE: u8 = 0x02;

// Delimiter magic bytes
pub const DELIMITER_HI: u8 = 0xAA;
pub const DELIMITER_LO: u8 = 0x55;

/// Build a 64-byte control packet: [report_id, command, b2, 0..0, b8]
fn control_packet(command: u8, byte2: u8, byte8: u8) -> [u8; PACKET_LENGTH] {
    let mut pkt = [0u8; PACKET_LENGTH];
    pkt[0] = REPORT_ID;
    pkt[1] = command;
    pkt[2] = byte2;
    pkt[8] = byte8;
    pkt
}

pub fn start_packet() -> [u8; PACKET_LENGTH] {
    control_packet(CMD_START, 0x00, 0x01)
}

pub fn finish_packet() -> [u8; PACKET_LENGTH] {
    control_packet(CMD_FINISH, 0x00, 0x01)
}

pub fn mode_preamble_packet() -> [u8; PACKET_LENGTH] {
    control_packet(CMD_MODE, 0x00, 0x01)
}

pub fn settings_preamble_packet() -> [u8; PACKET_LENGTH] {
    control_packet(CMD_SETTINGS, 0x01, 0x01)
}

pub fn image_preamble_packet() -> [u8; PACKET_LENGTH] {
    control_packet(CMD_IMAGE, 0x02, 0x09)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LightingMode {
    Off = 0x00,
    Static = 0x01,
    SingleOn = 0x02,
    SingleOff = 0x03,
    Glittering = 0x04,
    Falling = 0x05,
    Colourful = 0x06,
    Breath = 0x07,
    Spectrum = 0x08,
    Outward = 0x09,
    Scrolling = 0x0A,
    Rolling = 0x0B,
    Rotating = 0x0C,
    Explode = 0x0D,
    Launch = 0x0E,
    Ripples = 0x0F,
    Flowing = 0x10,
    Pulsating = 0x11,
    Tilt = 0x12,
    Shuttle = 0x13,
}

impl LightingMode {
    pub const ALL: &[LightingMode] = &[
        Self::Off, Self::Static, Self::SingleOn, Self::SingleOff,
        Self::Glittering, Self::Falling, Self::Colourful, Self::Breath,
        Self::Spectrum, Self::Outward, Self::Scrolling, Self::Rolling,
        Self::Rotating, Self::Explode, Self::Launch, Self::Ripples,
        Self::Flowing, Self::Pulsating, Self::Tilt, Self::Shuttle,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Static => "static",
            Self::SingleOn => "single-on",
            Self::SingleOff => "single-off",
            Self::Glittering => "glittering",
            Self::Falling => "falling",
            Self::Colourful => "colourful",
            Self::Breath => "breath",
            Self::Spectrum => "spectrum",
            Self::Outward => "outward",
            Self::Scrolling => "scrolling",
            Self::Rolling => "rolling",
            Self::Rotating => "rotating",
            Self::Explode => "explode",
            Self::Launch => "launch",
            Self::Ripples => "ripples",
            Self::Flowing => "flowing",
            Self::Pulsating => "pulsating",
            Self::Tilt => "tilt",
            Self::Shuttle => "shuttle",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.iter().find(|m| m.name().eq_ignore_ascii_case(name)).copied()
    }

    pub fn from_index(idx: u8) -> Option<Self> {
        if idx <= 0x13 {
            // Safety: all values 0x00..=0x13 are valid enum variants
            Some(unsafe { std::mem::transmute(idx) })
        } else {
            None
        }
    }

    /// Which directions this mode supports, if any.
    pub fn supported_directions(&self) -> &[Direction] {
        match self {
            Self::Scrolling => &[Direction::Up, Direction::Down],
            Self::Rolling | Self::Flowing | Self::Tilt => &[Direction::Left, Direction::Right],
            _ => &[],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Direction {
    Left = 0,
    Down = 1,
    Up = 2,
    Right = 3,
}

impl Direction {
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "left" | "l" => Some(Self::Left),
            "down" | "d" => Some(Self::Down),
            "up" | "u" => Some(Self::Up),
            "right" | "r" => Some(Self::Right),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SleepTime {
    Never = 0,
    OneMinute = 1,
    FiveMinutes = 2,
    ThirtyMinutes = 3,
}

impl SleepTime {
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "never" | "off" | "0" => Some(Self::Never),
            "1" | "1m" | "1min" => Some(Self::OneMinute),
            "5" | "5m" | "5min" => Some(Self::FiveMinutes),
            "30" | "30m" | "30min" => Some(Self::ThirtyMinutes),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Never => "never",
            Self::OneMinute => "1m",
            Self::FiveMinutes => "5m",
            Self::ThirtyMinutes => "30m",
        }
    }
}

/// Key response time (hardware debounce) level, 1-5.
/// Approximate wired delays from the official app: 1 = 2-3 ms, 2 = 5-6 ms,
/// 3 = 8-9 ms, 4 = 13-14 ms, 5 = 17-18 ms. Too low may cause key chatter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyResponse(u8);

impl KeyResponse {
    pub const MIN: u8 = 1;
    pub const MAX: u8 = 5;

    pub fn new(level: u8) -> Option<Self> {
        (Self::MIN..=Self::MAX).contains(&level).then_some(Self(level))
    }

    pub fn level(&self) -> u8 {
        self.0
    }

    /// Approximate wired debounce delay, as documented by the official app.
    pub fn wired_delay(&self) -> &'static str {
        ["2-3 ms", "5-6 ms", "8-9 ms", "13-14 ms", "17-18 ms"][(self.0 - 1) as usize]
    }
}

pub const MAX_BRIGHTNESS: u8 = 5;
pub const MAX_SPEED: u8 = 5;

/// Build the 64-byte mode data packet.
/// Note: byte 0 is the mode value itself, which hidapi sends as the report ID.
pub fn mode_data_packet(
    mode: LightingMode,
    r: u8, g: u8, b: u8,
    rainbow: bool,
    brightness: u8,
    speed: u8,
    direction: Direction,
) -> [u8; PACKET_LENGTH] {
    let mut pkt = [0u8; PACKET_LENGTH];
    pkt[0] = mode as u8;    // report ID = mode value
    pkt[1] = r;
    pkt[2] = g;
    pkt[3] = b;
    // bytes 4-7: padding (zero)
    pkt[8] = rainbow as u8;
    pkt[9] = brightness.min(MAX_BRIGHTNESS);
    pkt[10] = speed.min(MAX_SPEED);
    pkt[11] = direction as u8;
    // bytes 12-13: padding
    pkt[14] = DELIMITER_LO; // 0x55 (little-endian 0xAA55)
    pkt[15] = DELIMITER_HI; // 0xAA
    pkt
}

/// Encode an RGB888 pixel to RGB565 (little-endian bytes).
/// RGB565: 5 bits red, 6 bits green, 5 bits blue.
pub fn rgb565_encode(r: u8, g: u8, b: u8) -> [u8; 2] {
    let r5 = (r >> 3) as u16;
    let g6 = (g >> 2) as u16;
    let b5 = (b >> 3) as u16;
    let pixel = (r5 << 11) | (g6 << 5) | b5;
    pixel.to_le_bytes()
}

/// Split image data into IMAGE_NUM_CHUNKS chunks of IMAGE_CHUNK_SIZE bytes,
/// padded with 0xFF (matching the C++ reference).
pub fn split_image_data(data: &[u8]) -> Vec<Vec<u8>> {
    let mut chunks = Vec::with_capacity(IMAGE_NUM_CHUNKS);
    for i in 0..IMAGE_NUM_CHUNKS {
        let start = i * IMAGE_CHUNK_SIZE;
        let mut chunk = vec![0xFFu8; IMAGE_CHUNK_SIZE];
        if start < data.len() {
            let end = (start + IMAGE_CHUNK_SIZE).min(data.len());
            let copy_len = end - start;
            chunk[..copy_len].copy_from_slice(&data[start..end]);
        }
        chunks.push(chunk);
    }
    chunks
}

pub fn time_preamble_packet() -> [u8; PACKET_LENGTH] {
    control_packet(CMD_TIME, 0x00, 0x01)
}

pub fn save_packet() -> [u8; PACKET_LENGTH] {
    control_packet(CMD_SAVE, 0x00, 0x00)
}

/// Build the 64-byte time data packet.
/// Report ID is 0x00 (not 0x04), with magic byte 0x5A.
pub fn time_data_packet(
    year: u16, month: u8, day: u8,
    hour: u8, minute: u8, second: u8,
) -> [u8; PACKET_LENGTH] {
    let mut pkt = [0u8; PACKET_LENGTH];
    pkt[0] = 0x00;                      // report ID
    pkt[1] = 0x01;                      // fixed
    pkt[2] = 0x5A;                      // magic marker
    pkt[3] = (year.saturating_sub(2000)) as u8;
    pkt[4] = month;
    pkt[5] = day;
    pkt[6] = hour;
    pkt[7] = minute;
    pkt[8] = second;
    pkt[9] = 0x00;
    pkt[10] = 0x04;                     // fixed
    pkt[PACKET_LENGTH - 2] = DELIMITER_HI; // 0xAA at byte 62
    pkt[PACKET_LENGTH - 1] = DELIMITER_LO; // 0x55 at byte 63
    pkt
}

/// Build the 64-byte settings data packet sent after the CMD_SETTINGS preamble.
/// The block holds every setting at once — the keyboard can't report its
/// current values, so callers must always send the complete set.
/// `key_response` is required on V1_1 and ignored on Legacy.
pub fn settings_data_packet(
    layout: SettingsLayout,
    sleep_time: SleepTime,
    key_response: Option<KeyResponse>,
) -> [u8; PACKET_LENGTH] {
    let mut pkt = [0u8; PACKET_LENGTH];
    match layout {
        SettingsLayout::Legacy => {
            pkt[8] = sleep_time as u8;
        }
        SettingsLayout::V1_1 => {
            // Bytes 1 and 5 are always 0x01 in captures from the official app;
            // their meaning is unknown.
            pkt[1] = 0x01;
            pkt[5] = 0x01;
            pkt[6] = sleep_time as u8;
            debug_assert!(key_response.is_some(), "V1_1 settings need a key response time");
            pkt[8] = key_response.map_or(0, |k| k.level());
        }
    }
    pkt[PACKET_LENGTH - 2] = DELIMITER_HI; // 0xAA at byte 62
    pkt[PACKET_LENGTH - 1] = DELIMITER_LO; // 0x55 at byte 63
    pkt
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(pkt: &[u8]) -> String {
        pkt.iter().map(|b| format!("{:02x}", b)).collect()
    }

    fn captured(prefix: &str) -> String {
        // Captured packets are zero-filled up to the AA55 delimiter at 62-63.
        format!("{:0<124}aa55", prefix)
    }

    #[test]
    fn v1_1_settings_match_official_app_capture() {
        let lvl = |n| KeyResponse::new(n);
        // Key response levels 1-5, sleep 5m (from the debounce capture)
        for n in 1..=5 {
            let pkt = settings_data_packet(SettingsLayout::V1_1, SleepTime::FiveMinutes, lvl(n));
            assert_eq!(hex(&pkt), captured(&format!("00010000000102000{}", n)));
        }
        // Sleep never/1m/5m/30m, key response level 5 (from the sleep capture)
        let sleeps = [SleepTime::Never, SleepTime::OneMinute, SleepTime::FiveMinutes, SleepTime::ThirtyMinutes];
        for (i, s) in sleeps.into_iter().enumerate() {
            let pkt = settings_data_packet(SettingsLayout::V1_1, s, lvl(5));
            assert_eq!(hex(&pkt), captured(&format!("0001000000010{}0005", i)));
        }
    }

    #[test]
    fn settings_preamble_matches_capture() {
        assert_eq!(hex(&settings_preamble_packet()), format!("{:0<128}", "041701000000000001"));
    }

    #[test]
    fn key_response_range() {
        assert!(KeyResponse::new(0).is_none());
        assert!(KeyResponse::new(6).is_none());
        assert_eq!(KeyResponse::new(3).unwrap().wired_delay(), "8-9 ms");
    }

    #[test]
    fn model_lookup() {
        assert_eq!(find_model(VENDOR_ID, 0x800A).unwrap().settings, SettingsLayout::V1_1);
        assert!(find_model(VENDOR_ID, 0x8009).unwrap().settings == SettingsLayout::Legacy);
        assert!(find_model(0x1234, 0x800A).is_none());
    }
}
