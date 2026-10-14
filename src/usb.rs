/// Direct USB control transfer implementation for the AK820 Pro.
/// Uses rusb (libusb) to detach the kernel driver and send SET_REPORT directly.
/// The kernel's hid-generic driver interferes with feature reports, so we must
/// bypass it via libusb.

use anyhow::{bail, Context, Result};
use rusb::{DeviceHandle, GlobalContext};
use std::time::Duration;

use crate::protocol::*;

const CONTROL_INTERFACE: u8 = 3;
const DATA_INTERFACE: u8 = 2;
const DATA_EP_OUT: u8 = 0x03; // EP3 OUT on interface 2
const TIMEOUT: Duration = Duration::from_millis(1000);

// USB HID Class request codes
const HID_SET_REPORT: u8 = 0x09;
const HID_GET_REPORT: u8 = 0x01;

// Report type: Feature = 3
const HID_FEATURE_REPORT: u16 = 0x0300;

// Short timeout for GET_REPORT handshake — device may STALL, that's OK
const HANDSHAKE_TIMEOUT: Duration = Duration::from_millis(100);

pub struct UsbDevice {
    handle: DeviceHandle<GlobalContext>,
    iface: u8,
    data_iface_claimed: bool,
    model: &'static Model,
}

impl UsbDevice {
    pub fn open() -> Result<Self> {
        let iface = std::env::var("AK820_IFACE")
            .ok()
            .and_then(|v| v.parse::<u8>().ok())
            .unwrap_or(CONTROL_INTERFACE);

        let (device, model) = rusb::devices()?
            .iter()
            .find_map(|d| {
                let desc = d.device_descriptor().ok()?;
                find_model(desc.vendor_id(), desc.product_id()).map(|m| (d, m))
            })
            .ok_or_else(|| anyhow::anyhow!(
                "AK820 Pro not found (VID {:04x}, PID {}). Is it connected via USB cable?",
                VENDOR_ID, supported_pids()
            ))?;

        let handle = device.open().context("Failed to open USB device")?;

        // Detach kernel driver if attached (required — hid-generic blocks feature reports)
        if handle.kernel_driver_active(iface)? {
            handle.detach_kernel_driver(iface)
                .context("Failed to detach kernel driver")?;
            eprintln!("Detached kernel driver from interface {}", iface);
        }

        handle.claim_interface(iface)
            .context(format!("Failed to claim interface {}", iface))?;

        eprintln!("Connected to {} [{:04x}] (interface {})", model.name, model.pid, iface);
        Ok(Self { handle, iface, data_iface_claimed: false, model })
    }

    pub fn model(&self) -> &'static Model {
        self.model
    }

    /// Claim interface 2 for sustained image transfers (LCD loop).
    /// Call once before repeated upload_image calls.
    pub fn claim_data_interface(&mut self) -> Result<()> {
        if self.data_iface_claimed {
            return Ok(());
        }
        let data_iface = DATA_INTERFACE;
        if self.handle.kernel_driver_active(data_iface).unwrap_or(false) {
            self.handle.detach_kernel_driver(data_iface)
                .context("Failed to detach kernel driver from data interface")?;
        }
        self.handle.claim_interface(data_iface)
            .context(format!("Failed to claim data interface {}", data_iface))?;
        self.data_iface_claimed = true;
        Ok(())
    }

    /// Send a HID SET_REPORT(Feature) via USB control transfer.
    /// No GET_REPORT — it corrupts EP0 state on this device.
    fn send_feature(&self, data: &[u8; PACKET_LENGTH]) -> Result<()> {
        let report_id = data[0];

        if std::env::var("AK820_DEBUG").is_ok() {
            eprint!("  TX [{}]: ", PACKET_LENGTH);
            for (i, b) in data.iter().enumerate() {
                if i > 0 && i % 16 == 0 { eprint!("\n          "); }
                eprint!("{:02x} ", b);
            }
            eprintln!();
        }

        let request_type = rusb::request_type(
            rusb::Direction::Out,
            rusb::RequestType::Class,
            rusb::Recipient::Interface,
        );
        let w_value = HID_FEATURE_REPORT | (report_id as u16);

        self.handle.write_control(
            request_type,
            HID_SET_REPORT,
            w_value,
            self.iface as u16,
            data,
            TIMEOUT,
        ).context("SET_REPORT failed")?;

        // GET_REPORT handshake — only for control report ID 0x04.
        // The device firmware needs this to advance its state machine.
        // Mode data packets use mode value as report ID — device doesn't
        // support GET_REPORT for those and will crash if we try too many.
        if report_id == crate::protocol::REPORT_ID {
            let read_request_type = rusb::request_type(
                rusb::Direction::In,
                rusb::RequestType::Class,
                rusb::Recipient::Interface,
            );
            let mut rbuf = [0u8; PACKET_LENGTH];
            rbuf[0] = report_id;
            let _ = self.handle.read_control(
                read_request_type,
                HID_GET_REPORT,
                w_value,
                self.iface as u16,
                &mut rbuf,
                HANDSHAKE_TIMEOUT,
            );
        }

        // Small delay for device to process
        std::thread::sleep(Duration::from_millis(10));

        Ok(())
    }

    fn transaction(
        &self,
        preamble: &[u8; PACKET_LENGTH],
        data: &[u8; PACKET_LENGTH],
    ) -> Result<()> {
        self.send_feature(&start_packet())?;
        self.send_feature(preamble)?;
        self.send_feature(data)?;
        self.send_feature(&finish_packet())?;
        // Extra delay after full transaction for device processing
        std::thread::sleep(Duration::from_millis(100));
        Ok(())
    }

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

    /// Upload a 128x128 RGB565 image to the LCD screen.
    /// `rgb565_data` must be exactly LCD_DATA_SIZE bytes (32768).
    /// For repeated uploads (LCD loop), call `claim_data_interface()` first.
    pub fn upload_image(&mut self, rgb565_data: &[u8]) -> Result<()> {
        assert_eq!(rgb565_data.len(), LCD_DATA_SIZE, "Image must be {} bytes", LCD_DATA_SIZE);

        // Claim interface 2 if not already held
        let need_release = !self.data_iface_claimed;
        if need_release {
            self.claim_data_interface()?;
        }

        // Transaction: START → IMAGE_PREAMBLE → 9 chunks via interrupt OUT → FINISH
        self.send_feature(&start_packet())?;
        self.send_feature(&image_preamble_packet())?;

        let debug = std::env::var("AK820_DEBUG").is_ok();

        // Split into 9 chunks of 4123 bytes (padded with 0xFF).
        // Each chunk is sent as 64-byte interrupt packets; the last packet of
        // each chunk is 27 bytes (short packet = chunk boundary delimiter).
        let chunks = split_image_data(rgb565_data);

        for (ci, chunk) in chunks.iter().enumerate() {
            for pkt in chunk.chunks(PACKET_LENGTH) {
                self.handle.write_interrupt(
                    DATA_EP_OUT,
                    pkt,
                    TIMEOUT,
                ).with_context(|| format!(
                    "Interrupt write failed at chunk {}/9 ({} bytes)",
                    ci + 1, pkt.len()
                ))?;
            }
            if debug {
                eprintln!("  Chunk {}/9 sent ({} bytes)", ci + 1, chunk.len());
            }
            // Inter-chunk delay — device needs time to process each chunk
            std::thread::sleep(Duration::from_millis(50));
        }

        self.send_feature(&finish_packet())?;

        // Only release if we claimed it ourselves (one-off upload)
        if need_release {
            let _ = self.handle.release_interface(DATA_INTERFACE);
            let _ = self.handle.attach_kernel_driver(DATA_INTERFACE);
            self.data_iface_claimed = false;
        }

        std::thread::sleep(Duration::from_millis(100));
        Ok(())
    }

    /// Sync the keyboard's internal clock to the given time.
    /// Protocol: START → TIME_CONFIGURE → TIME_DATA → SAVE
    pub fn set_time(
        &self,
        year: u16, month: u8, day: u8,
        hour: u8, minute: u8, second: u8,
    ) -> Result<()> {
        self.send_feature(&start_packet())?;
        self.send_feature(&time_preamble_packet())?;
        self.send_feature(&time_data_packet(year, month, day, hour, minute, second))?;
        self.send_feature(&save_packet())?;
        std::thread::sleep(Duration::from_millis(100));
        Ok(())
    }

    /// Write the settings block (sleep timer + key response time).
    /// Both values are always sent together; `key_response` is required on
    /// models that support it and must be None on models that don't.
    /// Protocol (from the official app): START → SETTINGS → DATA → SAVE
    pub fn set_settings(&self, sleep_time: SleepTime, key_response: Option<KeyResponse>) -> Result<()> {
        match (self.model.supports_key_response(), key_response) {
            (true, None) => bail!("{} needs a key response time with every settings write", self.model.name),
            (false, Some(_)) => bail!("Key response time isn't supported on {} [{:04x}]", self.model.name, self.model.pid),
            _ => {}
        }
        let data = settings_data_packet(self.model.settings, sleep_time, key_response);
        self.send_feature(&start_packet())?;
        self.send_feature(&settings_preamble_packet())?;
        self.send_feature(&data)?;
        self.send_feature(&save_packet())?;
        std::thread::sleep(Duration::from_millis(100));
        Ok(())
    }
}

impl Drop for UsbDevice {
    fn drop(&mut self) {
        if self.data_iface_claimed {
            let _ = self.handle.release_interface(DATA_INTERFACE);
            let _ = self.handle.attach_kernel_driver(DATA_INTERFACE);
        }
        let _ = self.handle.release_interface(self.iface);
        let _ = self.handle.attach_kernel_driver(self.iface);
    }
}
