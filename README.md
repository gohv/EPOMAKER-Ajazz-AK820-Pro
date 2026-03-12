# ak820-ctl
Full name on Amazon.fr
### EPOMAKER Ajazz AK820 Pro 75% Gasket Mechanical Keyboard, Wireless/BT/USB, RGB Backlit, TFT Screen, Soundproofing Foam for PC/Mac/Win,QWERTY (White Purple, Ajazz Gift Switch) 

<img width="682" height="590" alt="image" src="https://github.com/user-attachments/assets/383badf8-4a99-4e98-81f2-b89cdc9fa692" />


Linux control software for the Ajazz AK820 Pro keyboard. Includes a CLI tool and a GUI app.

The official software only supports Windows. This project talks to the keyboard over USB HID to control lighting, sleep timer, and the built-in clock.

## Features

- 20 lighting modes with RGB color, rainbow, brightness, speed, and direction controls
- Sleep timer (never, 1 min, 5 min, 30 min)
- Clock sync (pushes system time to the keyboard's internal clock)
- GUI control panel (egui)

## Requirements

- Linux (tested on Arch/KDE Plasma with Wayland)
- Rust toolchain
- `pkg-config` and `libusb` dev headers (`libusb-1.0-0-dev` on Debian/Ubuntu)

## Building

Build the CLI only:

```
cargo build --release --bin ak820-ctl
```

Build the GUI:

```
cargo build --release --features gui --bin ak820-gui
```

Build everything:

```
cargo build --release --features gui
```

Binaries go to `target/release/`.

## USB permissions

By default the USB device requires root access. To use it as a normal user, install the udev rule:

```
sudo cp 99-ak820.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules
sudo udevadm trigger
```

Then unplug and replug the keyboard. After that you can run both `ak820-ctl` and `ak820-gui` without sudo.

If you skip this step, you can still run everything with `sudo`.

## Usage

### GUI

```
./target/release/ak820-gui
```

Or double-click the desktop shortcut if you set one up. The GUI has sections for lighting, sleep timer, and clock sync. Everything is point and click.

### CLI

**List all lighting modes:**

```
ak820-ctl list-modes
```

**Set a lighting mode:**

```
ak820-ctl light static --color ff0000 --brightness 5 --speed 3
ak820-ctl light breath --color 00ff00 --rainbow
ak820-ctl light spectrum --brightness 4 --speed 2
ak820-ctl light scrolling --direction right --color 0000ff
```

Available modes: off, static, single-on, single-off, glittering, falling, colourful, breath, spectrum, outward, scrolling, rolling, rotating, explode, launch, ripples, flowing, pulsating, tilt, shuttle.

Options:
- `--color` / `-c` : hex RGB color (default: ff0000)
- `--rainbow` / `-r` : use rainbow colors instead of a single color
- `--brightness` / `-b` : 0 to 5 (default: 5)
- `--speed` / `-s` : 0 to 5 (default: 3)
- `--direction` / `-d` : left, right, up, down (only for modes that support it)

**Set sleep timer:**

```
ak820-ctl sleep never
ak820-ctl sleep 1m
ak820-ctl sleep 5m
ak820-ctl sleep 30m
```

**Sync the keyboard clock:**

```
ak820-ctl sync-time
```

This reads your system time and pushes it to the keyboard. Useful if the clock on the keyboard display is wrong.

**Check if the keyboard is detected:**

```
ak820-ctl probe
```

## How it works

The keyboard uses a Sonix SN32F299 MCU with 4 USB HID interfaces. Interface 3 handles control commands (lighting, sleep, clock) via HID feature reports. The protocol was reverse-engineered using Wireshark USB captures and the [TaxMachine C++ reference](https://github.com/TaxMachine/ajazz-keyboard-software-linux). Clock sync protocol is from [KyleBoyer/TFTTimeSync-node](https://github.com/KyleBoyer/TFTTimeSync-node).

## License

MIT
