use ak820_ctl::protocol::{Direction, KeyResponse, LightingMode, SleepTime};
use ak820_ctl::settings_store::StoredSettings;
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

/// Write sleep timer + key response time and remember them.
/// Key response time is skipped on models that don't support it.
pub fn apply_settings(sleep_time: SleepTime, key_response: KeyResponse) -> Result<String, String> {
    let dev = UsbDevice::open().map_err(|e| format_usb_error(e))?;
    let key_response = dev.model().supports_key_response().then_some(key_response);
    dev.set_settings(sleep_time, key_response).map_err(|e| format!("{:#}", e))?;

    let mut stored = StoredSettings::load();
    stored.sleep = Some(sleep_time);
    stored.key_response = key_response.or(stored.key_response);
    let saved = stored.save();

    let mut msg = format!("Sleep timer: {}", sleep_time.name());
    match key_response {
        Some(k) => msg.push_str(&format!(" | key response: level {} (~{})", k.level(), k.wired_delay())),
        None => msg.push_str(" | key response time not supported on this model"),
    }
    if let Err(e) = saved {
        msg.push_str(&format!(" (couldn't remember settings: {:#})", e));
    }
    Ok(msg)
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
    let dev = UsbDevice::open().map_err(|e| format_usb_error(e))?;
    Ok(format!("{} [{:04x}] connected", dev.model().name, dev.model().pid))
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
