# Development Guide

This document covers technical details about building, developing, and contributing to the Supermodel Lightgun Auto-Configurator.

## Requirements

- **Rust** - Install rustup from https://rustup.rs/. The toolchain version, components and the
  Windows target are pinned in `rust-toolchain.toml`; rustup installs them on the first `cargo` call.
- **libusb development libraries** (not needed on Windows)
  - Ubuntu/Debian: `sudo apt-get install libusb-1.0-0-dev`
  - macOS: `brew install libusb`

## Building

Build the release binary (optimized):

```bash
cargo build --release
```

The compiled binary will be at:
```bash
./target/release/smgun
```

Build a debug binary (faster compile, slower runtime):

```bash
cargo build
```

Debug binary location:
```bash
./target/debug/smgun
```

## Running

From source:
```bash
cargo run --release -- --list
```

Or run the compiled binary directly:
```bash
./target/release/smgun --list
```

## Usage

### List all connected USB devices:
```bash
smgun --list
```

### Configure with VID:PID values:
```bash
smgun --gun1 046D:C05A --gun2 046D:C05B --ini Config/Supermodel.ini
```

### Configure single gun (Gun 1 only):
```bash
smgun --gun1 046D:C05A --ini Config/Supermodel.ini
```

### Use analog gun controls (InputAnalogGunX/Y, InputAnalogTriggerLeft/Right):
```bash
smgun --gun1 046D:C05A --gun2 046D:C05B --ini Config/Supermodel.ini --use-analog
```

### Enable debug logging:
```bash
RUST_LOG=debug smgun --gun1 046D:C05A --ini Config/Supermodel.ini
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

- Devices are enumerated with libusb and sorted by bus number
- Device names are read from `/sys/bus/usb/devices/<bus>-<port path>/` (no permissions needed)
- Falls back to USB descriptor reading if sysfs is unavailable
- VID:PID matching works across all device names

### macOS

- Uses USB descriptor reading (requires brief permission dialog on first run)
- Devices may appear with manufacturer names instead of product names
- VID:PID values are consistent regardless of device name

### Windows

- Does not use libusb. Mice are enumerated via the Raw Input API (`GetRawInputDeviceList`)
  in the same order as Supermodel's RawInput system, so the `MOUSEx` indices match
  (list walked backwards, `Root#RDP_` devices skipped, only `RIM_TYPEMOUSE` counted)
- Every mouse counts towards the index, including non-USB ones (shown as VID:0000 PID:0000)
- Supermodel must use `InputSystem = rawinput` for per-mouse lightgun input
- Device names come from the HID product/manufacturer strings

## Versioning & CI

Versions are SemVer and computed by `scripts/version.sh` from the GitFlow branch and the
`vX.Y.Z` tags. `Cargo.toml` stays at `0.0.0`; don't bump it. The version is injected at build
time via `SMGUN_VERSION` (read by `build.rs`):

- tag `vX.Y.Z` → `X.Y.Z`
- `release/X.Y.Z` or `hotfix/X.Y.Z` → `X.Y.Z-rc.N`
- anything else (incl. `develop`) → `0.0.0+<sha>`

```bash
SMGUN_VERSION=$(scripts/version.sh) cargo build --release
scripts/test.sh    # tests for version.sh and changelog.sh
```

Add changes to `## [Unreleased]` in [CHANGELOG.md](CHANGELOG.md). Branching and releases are
described in [GITFLOW.md](GITFLOW.md), the workflows in [.github/WORKFLOWS.md](.github/WORKFLOWS.md).

CI checks (fix these before pushing):

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --target x86_64-pc-windows-msvc -- -D warnings   # Windows path, from Linux
cargo test
shellcheck scripts/*.sh
```

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
