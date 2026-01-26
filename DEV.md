# Development Guide

This document covers technical details about building, developing, and contributing to the Supermodel3 Lightgun Auto-Configurator.

## Requirements

- **Rust 2021 Edition** - Install from https://rustup.rs/
- **libusb development libraries**
  - Ubuntu/Debian: `sudo apt-get install libusb-1.0-0-dev`
  - macOS: `brew install libusb`
  - Windows: Download from https://libusb.info/

## Building

Build the release binary (optimized):

```bash
cargo build --release
```

The compiled binary will be at:
```bash
```bash
./target/release/sm3lgs
```

Build a debug binary (faster compile, slower runtime):

```bash
cargo build
```

Debug binary location:
```bash
./target/debug/sm3lgs
```

## Running

From source:
```bash
cargo run --release -- --list
```

Or run the compiled binary directly:
```bash
./target/release/sm3lgs --list
```

## Usage

### List all connected USB devices:
```bash
sm3lgs --list
```

### Configure with VID:PID values:
```bash
sm3lgs --gun1 046D:C05A --gun2 046D:C05B --ini Config/Supermodel.ini
```

### Configure single gun (Gun 1 only):
```bash
sm3lgs --gun1 046D:C05A --ini Config/Supermodel.ini
```

### Use non-analog controls:
```bash
sm3lgs --gun1 046D:C05A --gun2 046D:C05B --ini Config/Supermodel.ini --use_analog false
```

## Testing & Debugging

Check for compile errors without building:
```bash
cargo check
```

Run clippy linter for code quality:
```bash
cargo clippy
```

Check code formatting:
```bash
cargo fmt --check
```

Auto-format code:
```bash
cargo fmt
```

Show detailed dependency tree:
```bash
cargo tree
```

## Platform-Specific Notes

### Linux

- Devices are detected via `/sys/bus/usb/devices/` first (no permissions needed)
- Falls back to USB descriptor reading if sysfs unavailable
- Works without root/sudo when using sysfs path
- VID:PID matching works across all device names

### macOS

- Uses USB descriptor reading (requires brief permission dialog on first run)
- Devices may appear with manufacturer names instead of product names
- VID:PID values are consistent regardless of device name

### Windows

- Uses USB descriptor reading
- May require driver installation for some devices
- VID:PID values are the primary matching mechanism

## Contributing

When making changes:

1. Maintain the module separation - keep USB enumeration in `usb.rs`, INI logic in `ini.rs`, CLI in `main.rs`
2. Add documentation comments to public functions explaining VID:PID matching behavior
3. Test builds for Windows/macOS/Linux when possible
4. Preserve INI file layout - avoid reformatting user files
5. Keep error messages user-friendly and include VID:PID values when relevant
6. Ensure VID:PID matching works reliably across different device types

## License

MIT
