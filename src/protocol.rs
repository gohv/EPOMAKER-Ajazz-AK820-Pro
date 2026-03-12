/// HID protocol definitions for the Ajazz AK820 Pro keyboard.
/// Based on reverse-engineering from TaxMachine/ajazz-keyboard-software-linux.

pub const VENDOR_ID: u16 = 0x0C45;
pub const PRODUCT_ID: u16 = 0x8009;

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
pub const CMD_SLEEP: u8 = 0x17;
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

pub fn sleep_preamble_packet() -> [u8; PACKET_LENGTH] {
    control_packet(CMD_SLEEP, 0x01, 0x01)
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

#[derive(Debug, Clone, Copy)]
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

/// Build the 64-byte sleep data packet.
pub fn sleep_data_packet(sleep_time: SleepTime) -> [u8; PACKET_LENGTH] {
    let mut pkt = [0u8; PACKET_LENGTH];
    pkt[8] = sleep_time as u8;
    pkt[PACKET_LENGTH - 2] = DELIMITER_HI; // 0xAA at byte 62
    pkt[PACKET_LENGTH - 1] = DELIMITER_LO; // 0x55 at byte 63
    pkt
}
