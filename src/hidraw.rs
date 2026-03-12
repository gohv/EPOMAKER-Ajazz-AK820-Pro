/// Direct hidraw interface for the AK820 Pro.
/// Uses Linux ioctl for feature reports. The kernel's HID driver handles
/// USB error recovery (STALL clearing) transparently, unlike raw libusb.

use anyhow::{bail, Context, Result};
use std::fs::{File, OpenOptions};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;

use crate::protocol::*;

// Linux HID ioctl constants
// HIDIOCSFEATURE = _IOC(_IOC_WRITE|_IOC_READ, 'H', 0x06, len)
// HIDIOCGFEATURE = _IOC(_IOC_WRITE|_IOC_READ, 'H', 0x07, len)
fn hidiocsfeature(len: usize) -> libc::c_ulong {
    let dir: libc::c_ulong = 3;
    let typ: libc::c_ulong = b'H' as libc::c_ulong;
    let nr: libc::c_ulong = 0x06;
    let size: libc::c_ulong = len as libc::c_ulong;
    (dir << 30) | (size << 16) | (typ << 8) | nr
}

fn hidiocgfeature(len: usize) -> libc::c_ulong {
    let dir: libc::c_ulong = 3;
    let typ: libc::c_ulong = b'H' as libc::c_ulong;
    let nr: libc::c_ulong = 0x07;
    let size: libc::c_ulong = len as libc::c_ulong;
    (dir << 30) | (size << 16) | (typ << 8) | nr
}

pub struct HidrawDevice {
    file: File,
    path: PathBuf,
}

impl HidrawDevice {
    /// Find and open the correct hidraw device for the AK820 Pro.
    pub fn open() -> Result<Self> {
        let iface = std::env::var("AK820_IFACE")
            .ok()
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(3);

        for entry in std::fs::read_dir("/sys/class/hidraw/")? {
            let entry = entry?;
            let name = entry.file_name();
            let uevent_path = entry.path().join("device/uevent");

            if let Ok(uevent) = std::fs::read_to_string(&uevent_path) {
                let has_vid_pid = uevent.contains(&format!(
                    "HID_ID=0003:{:08X}:{:08X}",
                    VENDOR_ID as u32, PRODUCT_ID as u32
                ));

                if !has_vid_pid {
                    continue;
                }

                let phys_iface = uevent
                    .lines()
                    .find(|l| l.starts_with("HID_PHYS="))
                    .and_then(|l| l.rsplit("/input").next())
                    .and_then(|n| n.parse::<i32>().ok());

                if phys_iface != Some(iface) {
                    continue;
                }

                let dev_path = PathBuf::from(format!("/dev/{}", name.to_string_lossy()));
                eprintln!("Opening {} (interface {})", dev_path.display(), iface);

                let file = OpenOptions::new()
                    .read(true)
                    .write(true)
                    .open(&dev_path)
                    .context(format!(
                        "Failed to open {}. Check udev rules / permissions.",
                        dev_path.display()
                    ))?;

                return Ok(Self { file, path: dev_path });
            }
        }

        bail!(
            "AK820 Pro interface {} not found. Is the keyboard connected via USB?",
            iface
        );
    }

    /// Send a SET_REPORT (Feature) via ioctl.
    fn set_feature(&self, data: &[u8; PACKET_LENGTH]) -> Result<()> {
        if std::env::var("AK820_DEBUG").is_ok() {
            eprint!("  TX [{}]: ", PACKET_LENGTH);
            for (i, b) in data.iter().enumerate() {
                if i > 0 && i % 16 == 0 { eprint!("\n          "); }
                eprint!("{:02x} ", b);
            }
            eprintln!();
        }

        let ret = unsafe {
            libc::ioctl(
                self.file.as_raw_fd(),
                hidiocsfeature(PACKET_LENGTH),
                data.as_ptr(),
            )
        };

        if ret < 0 {
            let err = std::io::Error::last_os_error();
            bail!("HIDIOCSFEATURE failed on {}: {}", self.path.display(), err);
        }
        Ok(())
    }

    /// GET_REPORT handshake via ioctl with length=0, matching the C++ reference.
    /// The kernel's HID driver handles USB STALL recovery transparently.
    /// buf[0] must contain the report ID; length in the ioctl is 0.
    fn get_feature_handshake(&self, report_id: u8) {
        let mut buf = [report_id];
        unsafe {
            libc::ioctl(
                self.file.as_raw_fd(),
                hidiocgfeature(0), // length = 0, matching C++ RESPONSE_PACKET_LENGTH
                buf.as_mut_ptr(),
            );
        }
        // Result intentionally ignored — this is just a sync handshake
    }

    /// Send feature report followed by GET_REPORT handshake.
    fn send(&self, data: &[u8; PACKET_LENGTH]) -> Result<()> {
        self.set_feature(data)?;
        self.get_feature_handshake(data[0]);
        Ok(())
    }

    /// Execute a full lighting transaction: START -> preamble -> data -> FINISH
    fn transaction(
        &self,
        preamble: &[u8; PACKET_LENGTH],
        data: &[u8; PACKET_LENGTH],
    ) -> Result<()> {
        self.send(&start_packet())?;
        self.send(preamble)?;
        self.send(data)?;
        self.send(&finish_packet())?;
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

    pub fn set_sleep_time(&self, sleep_time: SleepTime) -> Result<()> {
        let preamble = sleep_preamble_packet();
        let data = sleep_data_packet(sleep_time);
        self.send(&start_packet())?;
        self.send(&preamble)?;
        self.send(&data)?;
        Ok(())
    }
}
