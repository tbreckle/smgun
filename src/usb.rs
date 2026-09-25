//! USB device enumeration and lightgun device detection.
//!
//! This module handles USB device enumeration, filtering for HID devices that can be used
//! as lightguns, and retrieving device information. Devices are matched by their USB vendor
//! ID (VID) and product ID (PID) for Supermodel configuration.

use crate::device::MouseDevice;
use rusb::{Context, UsbContext};
use std::time::Duration;

/// Timeout for reading USB string descriptors.
const DESCRIPTOR_TIMEOUT: Duration = Duration::from_millis(500);

/// Enumerates all connected USB lightgun devices.
///
/// Discovers HID class devices with input endpoints that can be used as lightguns.
/// Devices are automatically sorted by bus ID (ascending) for consistent ordering.
/// Each device includes its USB VID:PID which can be matched against user specifications.
///
/// # Returns
///
/// - `Ok(Vec<MouseDevice>)` - Vector of detected lightgun devices, sorted by bus ID.
/// - `Err(String)` - Error message if USB context creation or device enumeration fails.
///
/// # Example
///
/// ```no_run
/// # use smgun::enumerate_mice;
/// let devices = enumerate_mice()?;
/// for device in devices {
///     println!("{}", device);  // Shows index, name, VID:PID
/// }
/// # Ok::<(), String>(())
/// ```
pub fn enumerate_mice() -> Result<Vec<MouseDevice>, String> {
    let context = Context::new().map_err(|e| format!("Failed to create USB context: {}", e))?;
    let devices = context
        .devices()
        .map_err(|e| format!("Failed to get device list: {}", e))?;

    let mut found_mice: Vec<MouseDevice> = devices
        .iter()
        .filter_map(|device| {
            let desc = device.device_descriptor().ok()?;
            is_mouse_device(&device, &desc).then(|| MouseDevice {
                index: 0, // Will be set after sorting.
                name: get_device_name(&device, &desc),
                vid: desc.vendor_id(),
                pid: desc.product_id(),
                bus_id: device.bus_number(),
            })
        })
        .collect();

    // Sort by bus ID (ascending).
    found_mice.sort_by_key(|m| m.bus_id);

    // Assign indices starting from 1.
    for (idx, mouse) in found_mice.iter_mut().enumerate() {
        mouse.index = idx + 1;
    }

    Ok(found_mice)
}

/// Checks if a USB device is a lightgun.
///
/// A device is considered suitable as a lightgun if it has:
/// - HID class interface (class code 0x03).
/// - At least one input endpoint.
fn is_mouse_device<T: UsbContext>(device: &rusb::Device<T>, desc: &rusb::DeviceDescriptor) -> bool {
    // Try to get configuration and interface descriptors.
    for i in 0..desc.num_configurations() {
        match device.config_descriptor(i) {
            Ok(config) => {
                for interface in config.interfaces() {
                    for interface_desc in interface.descriptors() {
                        // HID class = 0x03, mouse protocols often use subclass 0x01 and protocol 0x02.
                        // However, some mice are just HID class 0x03.
                        if interface_desc.class_code() == 0x03 {
                            // Additional check for mouse: look for input endpoint.
                            if interface_desc.num_endpoints() > 0 {
                                for endpoint in interface_desc.endpoint_descriptors() {
                                    if endpoint.direction() == rusb::Direction::In {
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Err(_) => continue,
        }
    }

    false
}

/// Retrieves the device name from multiple sources.
///
/// Attempts to get the device name in the following order:
/// 1. **sysfs** (Linux only) - `/sys/bus/usb/devices/<bus>-<port path>/`, no permissions needed.
/// 2. **USB descriptors** - Product string, then manufacturer string (may require permissions).
/// 3. **Fallback** - Returns "USB Device VID:PID".
///
/// # Arguments
///
/// - `device` - USB device to query.
/// - `desc` - USB device descriptor.
///
/// # Returns
///
/// Device name string, never empty (always has a fallback).
fn get_device_name<T: UsbContext>(
    device: &rusb::Device<T>,
    desc: &rusb::DeviceDescriptor,
) -> String {
    #[cfg(target_os = "linux")]
    if let Some(name) = read_sysfs_name(device) {
        return name;
    }

    read_descriptor_name(device, desc).unwrap_or_else(|| {
        format!(
            "USB Device {:04X}:{:04X}",
            desc.vendor_id(),
            desc.product_id()
        )
    })
}

/// Reads the product (or manufacturer) name from sysfs.
///
/// sysfs names USB devices by bus and port path (e.g. `1-2.3` for bus 1, port 2, port 3).
#[cfg(target_os = "linux")]
fn read_sysfs_name<T: UsbContext>(device: &rusb::Device<T>) -> Option<String> {
    let ports = device.port_numbers().ok()?;
    if ports.is_empty() {
        // Root hubs have no port path.
        return None;
    }
    let port_path = ports
        .iter()
        .map(u8::to_string)
        .collect::<Vec<_>>()
        .join(".");
    let sysfs_dir = format!("/sys/bus/usb/devices/{}-{}", device.bus_number(), port_path);

    ["product", "manufacturer"].iter().find_map(|attr| {
        let name = std::fs::read_to_string(format!("{}/{}", sysfs_dir, attr)).ok()?;
        let name = name.trim();
        (!name.is_empty()).then(|| name.to_string())
    })
}

/// Reads the product (or manufacturer) string from the USB device descriptors.
fn read_descriptor_name<T: UsbContext>(
    device: &rusb::Device<T>,
    desc: &rusb::DeviceDescriptor,
) -> Option<String> {
    let handle = device.open().ok()?;
    let language = *handle.read_languages(DESCRIPTOR_TIMEOUT).ok()?.first()?;

    let product = handle
        .read_product_string(language, desc, DESCRIPTOR_TIMEOUT)
        .ok()
        .filter(|name| !name.trim().is_empty() && name != "USB Device");
    product.or_else(|| {
        handle
            .read_manufacturer_string(language, desc, DESCRIPTOR_TIMEOUT)
            .ok()
            .filter(|name| !name.trim().is_empty())
    })
}
