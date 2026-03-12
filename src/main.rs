use anyhow::{bail, Result};
use clap::{Parser, Subcommand};

use ak820_ctl::lcd;
use ak820_ctl::protocol::*;
use ak820_ctl::stats::SystemStats;
use ak820_ctl::usb::UsbDevice;

#[derive(Parser)]
#[command(name = "ak820-ctl", about = "Control Ajazz AK820 Pro keyboard on Linux")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List available lighting modes
    ListModes,

    /// Set the lighting mode
    Light {
        /// Mode name (use 'list-modes' to see options)
        mode: String,

        /// Color as hex RGB (e.g., ff0000 for red). Ignored if rainbow is set.
        #[arg(short, long, default_value = "ff0000")]
        color: String,

        /// Use rainbow colors instead of a fixed color
        #[arg(short, long, default_value_t = false)]
        rainbow: bool,

        /// Brightness (0-5)
        #[arg(short, long, default_value_t = 5)]
        brightness: u8,

        /// Speed (0-5)
        #[arg(short, long, default_value_t = 3)]
        speed: u8,

        /// Direction: left, right, up, down (only for modes that support it)
        #[arg(short, long)]
        direction: Option<String>,
    },

    /// Set the sleep timer
    Sleep {
        /// Sleep time: never, 1m, 5m, 30m
        time: String,
    },

    /// Show current system stats (CPU, RAM, time) — preview for LCD display
    Stats {
        /// Refresh interval in seconds
        #[arg(short, long, default_value_t = 1)]
        interval: u64,
    },

    /// Display live system stats (CPU/GPU temp, time) on the LCD screen.
    /// Only updates when temps change by >=2C or every max-interval seconds.
    Lcd {
        /// Poll interval in seconds (how often to check temps)
        #[arg(short, long, default_value_t = 5)]
        interval: u64,

        /// Max seconds between forced screen refreshes even if temps unchanged
        #[arg(short, long, default_value_t = 60)]
        max_interval: u64,
    },

    /// Upload a solid test color to the LCD screen (for protocol debugging)
    LcdTest {
        /// Color: red, green, blue, white
        #[arg(default_value = "red")]
        color: String,
    },

    /// Sync the keyboard's clock to the system time
    SyncTime,

    /// Probe the keyboard and show device info
    Probe,

    /// Test: send two lighting changes in one session (green -> red)
    Test,
}

fn parse_hex_color(s: &str) -> Result<(u8, u8, u8)> {
    let s = s.trim_start_matches('#');
    if s.len() != 6 {
        bail!("Color must be 6 hex digits (e.g., ff0000)");
    }
    let r = u8::from_str_radix(&s[0..2], 16)?;
    let g = u8::from_str_radix(&s[2..4], 16)?;
    let b = u8::from_str_radix(&s[4..6], 16)?;
    Ok((r, g, b))
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::ListModes => {
            println!("Available lighting modes:");
            for mode in LightingMode::ALL {
                let dirs = mode.supported_directions();
                if dirs.is_empty() {
                    println!("  {:2} - {}", *mode as u8, mode.name());
                } else {
                    let dir_names: Vec<_> = dirs.iter().map(|d| format!("{:?}", d)).collect();
                    println!(
                        "  {:2} - {} (directions: {})",
                        *mode as u8,
                        mode.name(),
                        dir_names.join(", ")
                    );
                }
            }
            println!("\nBrightness: 0-5, Speed: 0-5");
        }

        Commands::Light {
            mode,
            color,
            rainbow,
            brightness,
            speed,
            direction,
        } => {
            let mode = LightingMode::from_name(&mode)
                .or_else(|| mode.parse::<u8>().ok().and_then(LightingMode::from_index))
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "Unknown mode '{}'. Use 'list-modes' to see options.",
                        mode
                    )
                })?;

            let (r, g, b) = parse_hex_color(&color)?;

            let dir = match direction {
                Some(d) => Direction::from_name(&d)
                    .ok_or_else(|| anyhow::anyhow!("Unknown direction '{}'. Use: left, right, up, down", d))?,
                None => Direction::Left,
            };

            if brightness > MAX_BRIGHTNESS {
                bail!("Brightness must be 0-{}", MAX_BRIGHTNESS);
            }
            if speed > MAX_SPEED {
                bail!("Speed must be 0-{}", MAX_SPEED);
            }

            let dev = UsbDevice::open()?;
            dev.set_lighting(mode, r, g, b, rainbow, brightness, speed, dir)?;
            println!(
                "Set mode: {} | color: #{:02x}{:02x}{:02x} | rainbow: {} | brightness: {} | speed: {} | direction: {:?}",
                mode.name(), r, g, b, rainbow, brightness, speed, dir
            );
        }

        Commands::Sleep { time } => {
            let sleep_time = SleepTime::from_name(&time)
                .ok_or_else(|| anyhow::anyhow!("Unknown sleep time '{}'. Use: never, 1m, 5m, 30m", time))?;

            let dev = UsbDevice::open()?;
            dev.set_sleep_time(sleep_time)?;
            println!("Sleep timer set to: {:?}", sleep_time);
        }

        Commands::Stats { interval } => {
            let mut sys = SystemStats::new();
            println!("System stats (Ctrl+C to stop):");
            loop {
                let stats = sys.refresh();
                let now = chrono::Local::now().format("%H:%M:%S");
                print!("\r{} | {}  ", now, stats);
                use std::io::Write;
                std::io::stdout().flush()?;
                std::thread::sleep(std::time::Duration::from_secs(interval));
            }
        }

        Commands::Lcd { interval, max_interval } => {
            let mut dev = UsbDevice::open()?;
            dev.claim_data_interface()?;
            let mut sys = SystemStats::new();
            let mut fb = lcd::LcdFramebuffer::new();
            let mut consecutive_errors = 0u32;
            let mut last_cpu_temp: Option<f32> = None;
            let mut last_gpu_temp: Option<f32> = None;
            let mut last_upload = std::time::Instant::now()
                - std::time::Duration::from_secs(max_interval + 1); // force first upload

            println!("LCD stats display (Ctrl+C to stop):");
            println!("  Poll every {}s, refresh screen on >=2C change or every {}s", interval, max_interval);
            loop {
                let stats = sys.refresh();

                // Check if temps changed enough to warrant a screen update
                let cpu_changed = match (last_cpu_temp, stats.cpu_temp_c) {
                    (Some(old), Some(new)) => (new - old).abs() >= 2.0,
                    (None, Some(_)) => true,
                    _ => false,
                };
                let gpu_changed = match (last_gpu_temp, stats.gpu_temp_c) {
                    (Some(old), Some(new)) => (new - old).abs() >= 2.0,
                    (None, Some(_)) => true,
                    _ => false,
                };
                let forced = last_upload.elapsed().as_secs() >= max_interval;
                let need_update = cpu_changed || gpu_changed || forced;

                if need_update {
                    fb.render_stats(&stats);
                    let data = fb.as_rgb565_bytes();
                    match dev.upload_image(&data) {
                        Ok(()) => {
                            consecutive_errors = 0;
                            last_cpu_temp = stats.cpu_temp_c;
                            last_gpu_temp = stats.gpu_temp_c;
                            last_upload = std::time::Instant::now();
                        }
                        Err(e) => {
                            consecutive_errors += 1;
                            eprintln!("\nLCD upload error: {:#}", e);
                            if consecutive_errors >= 3 {
                                bail!("Too many consecutive LCD upload failures");
                            }
                            std::thread::sleep(std::time::Duration::from_secs(2));
                        }
                    }
                }

                let now = chrono::Local::now().format("%H:%M:%S");
                print!("\r{} | {}  ", now, stats);
                use std::io::Write;
                std::io::stdout().flush()?;
                std::thread::sleep(std::time::Duration::from_secs(interval));
            }
        }

        Commands::LcdTest { color } => {
            let mut data = Vec::with_capacity(LCD_DATA_SIZE);

            if color.to_lowercase() == "diag" {
                let red = rgb565_encode(255, 0, 0);
                let green = rgb565_encode(0, 255, 0);
                let blue = rgb565_encode(0, 0, 255);
                let white = rgb565_encode(255, 255, 255);
                for row in 0..LCD_HEIGHT {
                    for col in 0..LCD_WIDTH {
                        let pixel = match (row < 64, col < 64) {
                            (true, true) => red,
                            (true, false) => green,
                            (false, true) => blue,
                            (false, false) => white,
                        };
                        data.extend_from_slice(&pixel);
                    }
                }
                println!("Uploading diagnostic pattern (TL=red, TR=green, BL=blue, BR=white)...");
            } else {
                let (r, g, b) = match color.to_lowercase().as_str() {
                    "red" => (255u8, 0u8, 0u8),
                    "green" => (0, 255, 0),
                    "blue" => (0, 0, 255),
                    "white" => (255, 255, 255),
                    "black" => (0, 0, 0),
                    other => parse_hex_color(other)?,
                };
                let pixel = rgb565_encode(r, g, b);
                for _ in 0..LCD_PIXELS {
                    data.extend_from_slice(&pixel);
                }
                println!("Uploading solid color ({}, {}, {}) to LCD...", r, g, b);
            }

            let mut dev = UsbDevice::open()?;
            dev.upload_image(&data)?;
            println!("Done.");
        }

        Commands::SyncTime => {
            let now = chrono::Local::now();
            let dev = UsbDevice::open()?;
            dev.set_time(
                now.format("%Y").to_string().parse::<u16>().unwrap(),
                now.format("%m").to_string().parse::<u8>().unwrap(),
                now.format("%d").to_string().parse::<u8>().unwrap(),
                now.format("%H").to_string().parse::<u8>().unwrap(),
                now.format("%M").to_string().parse::<u8>().unwrap(),
                now.format("%S").to_string().parse::<u8>().unwrap(),
            )?;
            println!("Keyboard clock synced to {}", now.format("%Y-%m-%d %H:%M:%S"));
        }

        Commands::Probe => {
            println!("Probing for AK820 Pro (VID {:04X}, PID {:04X})...", VENDOR_ID, PRODUCT_ID);
            let dev = UsbDevice::open()?;
            println!("Successfully connected to AK820 Pro!");
            println!("Device is ready for commands.");
            drop(dev);
        }

        Commands::Test => {
            println!("=== Test: two lighting changes in one session ===");
            let dev = UsbDevice::open()?;

            println!("\n1) Setting RED breath...");
            dev.set_lighting(LightingMode::Breath, 255, 0, 0, false, 5, 3, Direction::Left)?;
            println!("   Done. Waiting 5 seconds — look at keyboard...");
            std::thread::sleep(std::time::Duration::from_secs(5));

            println!("2) Setting GREEN breath...");
            dev.set_lighting(LightingMode::Breath, 0, 255, 0, false, 5, 3, Direction::Left)?;
            println!("   Done. Waiting 5 seconds — look at keyboard...");
            std::thread::sleep(std::time::Duration::from_secs(5));

            println!("3) Setting BLUE breath...");
            dev.set_lighting(LightingMode::Breath, 0, 0, 255, false, 5, 3, Direction::Left)?;
            println!("   Done.");

            println!("\n=== Which colors did you see? (red, green, blue) ===");
        }
    }

    Ok(())
}
