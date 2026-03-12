use ak820_ctl::protocol::{Direction, LightingMode, SleepTime};
use ak820_ctl::usb::UsbDevice;

pub fn apply_lighting(
    mode: LightingMode,
    r: u8, g: u8, b: u8,
    rainbow: bool,
    brightness: u8,
    speed: u8,
    direction: Direction,
) -> Result<String, String> {
    let dev = UsbDevice::open().map_err(|e| format_usb_error(e))?;
    dev.set_lighting(mode, r, g, b, rainbow, brightness, speed, direction)
        .map_err(|e| format!("{:#}", e))?;
    Ok(format!(
        "Lighting: {} | #{:02x}{:02x}{:02x} | bright={} speed={}",
        mode.name(), r, g, b, brightness, speed
    ))
}

pub fn apply_sleep(sleep_time: SleepTime) -> Result<String, String> {
    let dev = UsbDevice::open().map_err(|e| format_usb_error(e))?;
    dev.set_sleep_time(sleep_time).map_err(|e| format!("{:#}", e))?;
    Ok(format!("Sleep timer: {:?}", sleep_time))
}

pub fn sync_time() -> Result<String, String> {
    let now = chrono::Local::now();
    let dev = UsbDevice::open().map_err(|e| format_usb_error(e))?;
    dev.set_time(
        now.format("%Y").to_string().parse::<u16>().unwrap(),
        now.format("%m").to_string().parse::<u8>().unwrap(),
        now.format("%d").to_string().parse::<u8>().unwrap(),
        now.format("%H").to_string().parse::<u8>().unwrap(),
        now.format("%M").to_string().parse::<u8>().unwrap(),
        now.format("%S").to_string().parse::<u8>().unwrap(),
    ).map_err(|e| format!("{:#}", e))?;
    Ok(format!("Clock synced to {}", now.format("%Y-%m-%d %H:%M:%S")))
}

pub fn probe_device() -> Result<String, String> {
    let _dev = UsbDevice::open().map_err(|e| format_usb_error(e))?;
    Ok("AK820 Pro connected".to_string())
}

fn format_usb_error(e: anyhow::Error) -> String {
    let msg = format!("{:#}", e);
    if msg.contains("Access denied") || msg.contains("Permission denied") || msg.contains("LIBUSB_ERROR_ACCESS") {
        format!(
            "Permission denied. Install udev rule:\n\
             sudo cp 99-ak820.rules /etc/udev/rules.d/\n\
             sudo udevadm control --reload-rules && sudo udevadm trigger\n\
             Then replug the keyboard."
        )
    } else {
        msg
    }
}
