//! USB lightgun device representation.

use std::fmt;

/// Represents a USB lightgun device with its metadata.
///
/// This struct stores information about a connected USB device that can be used
/// as a lightgun input for Supermodel3 emulator. Devices are identified by their
/// USB vendor ID (VID) and product ID (PID), which are used for matching against
/// user-specified lightgun configurations.
///
/// # Fields
///
/// - `index` - Sequential index starting from 1, ordered by bus ID.
/// - `name` - Device name (product name or manufacturer name from sysfs/descriptors).
/// - `vid` - USB Vendor ID in hexadecimal (used for matching).
/// - `pid` - USB Product ID in hexadecimal (used for matching).
/// - `bus_id` - USB bus number used for sorting and device identification.
#[derive(Clone, Debug)]
pub struct MouseDevice {
    pub index: usize,
    pub name: String,
    pub vid: u16,
    pub pid: u16,
    pub bus_id: u8,
}

impl fmt::Display for MouseDevice {
    /// Formats the device as "Mouse X: Y (VID:XXXX PID:XXXX)".
    ///
    /// This format displays the device index, name, and USB identifiers
    /// which are used for matching against VID:PID specifications.
    ///
    /// # Example
    ///
    /// ```ignore
    /// Mouse 1: Logitech USB Mouse (VID:046D PID:C05A)
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Mouse {}: {} (VID:{:04X} PID:{:04X})",
            self.index, self.name, self.vid, self.pid
        )
    }
}
