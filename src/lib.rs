//! Supermodel3 Lightgun Auto-Configurator
//!
//! A library for automatically configuring Supermodel3 emulator to select connected lightguns
//! by searching for USB VID:PID combinations. Enumerates HID class devices and matches them
//! against user-specified vendor and product IDs, then updates the Supermodel3 INI configuration.
//!
//! # Features
//!
//! - **VID:PID Matching** - Searches for lightguns by USB vendor and product IDs
//! - **Non-invasive enumeration** - Works without requiring elevated permissions when possible
//! - **Hybrid approach** - Uses sysfs on Linux first (no sudo), falls back to USB descriptors
//! - **Device filtering** - Automatically identifies HID class devices with input endpoints
//! - **INI Preservation** - Maintains original file layout when updating configuration
//!
//! # Usage
//!
//! ```no_run
//! use sm3lgs::enumerate_mice;
//!
//! fn main() -> Result<(), String> {
//!     let mice = enumerate_mice()?;
//!     
//!     // Find a device with specific VID:PID
//!     let target_device = mice.iter()
//!         .find(|m| m.vid == 0x046D && m.pid == 0xC05A);
//!     
//!     if let Some(device) = target_device {
//!         println!("Found lightgun: {}", device);
//!     }
//!     
//!     Ok(())
//! }
//! ```
//!
//! # Device Discovery Strategy
//!
//! The library uses a multi-stage approach to identify and match devices:
//!
//! 1. **Enumeration** - Lists all USB devices using libusb.
//! 2. **Filtering** - Identifies devices with HID class (0x03) and input endpoints.
//! 3. **VID:PID Matching** - Matches against user-specified vendor and product IDs.
//! 4. **Name Resolution** (in order of preference):
//!    - Reads from sysfs `/sys/bus/usb/devices/` (Linux, no permissions needed).
//!    - Falls back to USB device descriptor strings (may require sudo).
//!    - Uses "Unknown Device" if nothing is found.
//! 5. **Sorting** - Orders devices by bus ID (ascending).
//! 6. **Indexing** - Assigns sequential indices starting from 1.
//!
//! On Windows, devices are instead enumerated through the Raw Input API in the same order
//! Supermodel3 uses, so indices match its `MOUSEx` numbering (see the `rawinput` module).

pub mod device;
pub mod ini;
#[cfg(windows)]
pub mod rawinput;
#[cfg(not(windows))]
pub mod usb;

pub use device::MouseDevice;
#[cfg(windows)]
pub use rawinput::enumerate_mice;
#[cfg(not(windows))]
pub use usb::enumerate_mice;
