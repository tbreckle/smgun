# Supermodel Lightgun Auto-Configurator

[![CI](https://github.com/tbreckle/smgun/actions/workflows/ci.yml/badge.svg?branch=develop)](https://github.com/tbreckle/smgun/actions/workflows/ci.yml)

Automatically configure Supermodel emulator to select connected lightguns by searching for USB VID:PID combinations and automatically updating the Supermodel configuration file. No manual assignment needed—just specify the VID:PID of your lightguns and the tool handles the rest.

## What It Does

- **Searches** all connected USB devices for specified VID:PID combinations
- **Matches** lightguns by their USB vendor ID and product ID
- **Auto-Configures** Player 1 and Player 2 settings based on device discovery
- **Updates** the Supermodel INI file automatically
- **Preserves** your existing INI file layout and other settings

## Quick Start

1. Connect your lightgun devices to your computer
2. Identify your lightguns' USB VID:PID values
3. Run the tool with your VID:PID combinations:
   ```bash
   smgun --gun1 046D:C05A --gun2 046D:C05B --ini Config/Supermodel.ini
   ```
4. Or list available devices first:
   ```bash
   smgun --list
   ```

The tool automatically updates your Supermodel INI file with the matched lightgun devices.

## How It Works

The tool searches for devices matching your specified VID:PID values and updates your INI file with the following mappings (the mouse numbers depend on the order in which Supermodel sees your mice):

### For Player 1:
```ini
InputGunX = "MOUSE1_XAXIS"               ; Horizontal aiming
InputGunY = "MOUSE1_YAXIS"               ; Vertical aiming
InputTrigger = "MOUSE1_LEFT_BUTTON"      ; Trigger
InputOffscreen = "MOUSE1_RIGHT_BUTTON"   ; Point off-screen (reload)
```

### For Player 2:
```ini
InputGunX2 = "MOUSE2_XAXIS"
InputGunY2 = "MOUSE2_YAXIS"
InputTrigger2 = "MOUSE2_LEFT_BUTTON"
InputOffscreen2 = "MOUSE2_RIGHT_BUTTON"
```

With `--use-analog`, the analog gun settings (Ocean Hunter, LA Machineguns) are written instead: `InputAnalogGunX`, `InputAnalogGunY`, `InputAnalogTriggerLeft` and `InputAnalogTriggerRight` (with a `2` suffix for Player 2).

Settings are updated wherever they appear (including per-game sections, so they can't override the new values). Settings missing from `[ Global ]` are added to it.

These values tell Supermodel which mouse device to use for each player's gun control.

### Windows

On Windows, the tool enumerates mice through the Raw Input API in exactly the same order as Supermodel, so the `MOUSEx` numbers match. Supermodel has to use the Raw Input system for multiple mice to work (or run it with `-input-system=rawinput`):

```ini
InputSystem = "rawinput"
```

## Features

- **VID:PID Matching** - Automatically finds lightguns by USB vendor and product IDs
- **Batch Configuration** - Configure both players in a single command
- **Conflict Prevention** - The tool never assigns the same device to both players (two guns with the same VID:PID are assigned in device order)
- **Flexible Input** - Supports both single-gun and dual-gun setups
- **INI Preservation** - Your existing INI structure, comments, and settings are maintained
- **No Root Required** - Works without administrator/sudo privileges on Linux

## Supported Devices

The tool detects any USB HID (Human Interface Device) with input endpoints, which includes:
- Standard USB mice
- Arcade lightguns
- Spinner/trackball controllers
- Any similar USB input devices

## Installation

### Pre-built Binaries

Download the latest release for Linux and Windows from [GitHub Releases](https://github.com/tbreckle/smgun/releases):
- `smgun-<version>-linux-x86_64.tar.gz` - Linux binary
- `smgun-<version>-windows-x86_64.zip` - Windows binary
- `SHA256SUMS` - checksums of the archives

Unofficial builds of every branch (`0.0.0+<commit>`) are attached to the [CI runs](https://github.com/tbreckle/smgun/actions/workflows/ci.yml) as artifacts.

### Build from Source

See [Development Guide](DEV.md) for instructions on building from source.

## Need Help Building or Developing?

See [Development Guide](DEV.md) for technical details about building from source and contributing.

## License

MIT


