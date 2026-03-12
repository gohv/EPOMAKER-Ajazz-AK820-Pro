/// HID device communication for the Ajazz AK820 Pro.
/// Uses the libusb backend (matching the reference C++ implementation).
/// Interface 3 is the vendor control interface for lighting/config.

use anyhow::{bail, Context, Result};
use hidapi::HidApi;

use crate::protocol::*;

/// The correct HID interface for vendor control commands (lighting, config).
const CONTROL_INTERFACE: i32 = 3;

pub struct AK820Device {
    device: hidapi::HidDevice,
}

impl AK820Device {
    /// Open the AK820 Pro keyboard on the control interface.
    pub fn open() -> Result<Self> {
        let iface = std::env::var("AK820_IFACE")
            .ok()
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(CONTROL_INTERFACE);

        let api = HidApi::new().context("Failed to initialize hidapi")?;

        let devices: Vec<_> = api
            .device_list()
            .filter(|d| d.vendor_id() == VENDOR_ID && d.product_id() == PRODUCT_ID)
            .collect();

        if devices.is_empty() {
            bail!(
                "AK820 Pro not found (VID {:04x}, PID {:04x}). \
                 Is it connected via USB cable?",
                VENDOR_ID,
                PRODUCT_ID
            );
        }

        eprintln!("Found {} HID interface(s) for AK820", devices.len());

        let info = devices
            .iter()
            .find(|d| d.interface_number() == iface)
            .ok_or_else(|| anyhow::anyhow!(
                "Interface {} not found. Available: {:?}",
                iface,
                devices.iter().map(|d| d.interface_number()).collect::<Vec<_>>()
            ))?;

        eprintln!("Opening interface {}", iface);
        let device = info
            .open_device(&api)
            .context(format!("Failed to open interface {}", iface))?;

        device.set_blocking_mode(true)?;
        eprintln!("Connected to AK820 Pro (interface {})", iface);
        Ok(Self { device })
    }

    /// Send a feature report followed by a GET_REPORT handshake.
    /// The C++ reference does get_feature_report with length=0 after every send.
    /// This handshake appears required for the device to process commands.
    fn send_feature(&self, data: &[u8; PACKET_LENGTH]) -> Result<()> {
        if std::env::var("AK820_DEBUG").is_ok() {
            eprint!("  TX [{}]: ", PACKET_LENGTH);
            for (i, b) in data.iter().enumerate() {
                if i > 0 && i % 16 == 0 { eprint!("\n          "); }
                eprint!("{:02x} ", b);
            }
            eprintln!();
        }

        self.device
            .send_feature_report(data)
            .context("Failed to send feature report")?;

        // Handshake: GET_REPORT with minimal buffer (report ID only).
        // The C++ reference uses length=0; we use 1 byte (the minimum for hidapi).
        // Errors are non-fatal — some report IDs don't support GET.
        let mut rbuf = [data[0]];
        let _ = self.device.get_feature_report(&mut rbuf);

        Ok(())
    }

    /// Execute a full transaction: START -> preamble -> data -> FINISH
    fn transaction(
        &self,
        preamble: &[u8; PACKET_LENGTH],
        data: &[u8; PACKET_LENGTH],
    ) -> Result<()> {
        self.send_feature(&start_packet())?;
        self.send_feature(preamble)?;
        self.send_feature(data)?;
        self.send_feature(&finish_packet())?;
        Ok(())
    }

    /// Set the lighting mode with all parameters.
    pub fn set_lighting(
        &self,
        mode: LightingMode,
        r: u8, g: u8, b: u8,
        rainbow: bool,
        brightness: u8,
        speed: u8,
        direction: Direction,
    ) -> Result<()> {
        let (actual_mode, actual_speed) = match mode {
            LightingMode::Off => (LightingMode::SingleOn, 0),
            LightingMode::Static => (LightingMode::Breath, 0),
            _ => (mode, speed),
        };
        let actual_brightness = if mode == LightingMode::Off { 0 } else { brightness };

        let data = mode_data_packet(actual_mode, r, g, b, rainbow, actual_brightness, actual_speed, direction);
        self.transaction(&mode_preamble_packet(), &data)
            .context("Failed to set lighting mode")
    }

    /// Set the sleep timer.
    pub fn set_sleep_time(&self, sleep_time: SleepTime) -> Result<()> {
        let preamble = sleep_preamble_packet();
        let data = sleep_data_packet(sleep_time);
        self.send_feature(&start_packet())?;
        self.send_feature(&preamble)?;
        self.send_feature(&data)?;
        Ok(())
    }
}
