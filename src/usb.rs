//! USB device enumeration and lightgun device detection.
//!
//! This module handles USB device enumeration, filtering for HID devices that can be used
//! as lightguns, and retrieving device information. Devices are matched by their USB vendor
//! ID (VID) and product ID (PID) for Supermodel3 configuration.

use crate::device::MouseDevice;
use rusb::{Context, UsbContext};
// Required to suppress unused imports warnings in some configurations (Windows).
#[allow(unused_imports)]
use std::fs;
#[allow(unused_imports)]
use std::path::Path;
use std::time::Duration;

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
/// # use sm3lgs::enumerate_mice;
/// let devices = enumerate_mice()?;
/// for device in devices {
///     println!("{}", device);  // Shows index, name, VID:PID
/// }
/// # Ok::<(), String>(())
/// ```
pub fn enumerate_mice() -> Result<Vec<MouseDevice>, String> {
    let mut found_mice = Vec::new();

    match Context::new() {
        Ok(context) => {
            match context.devices() {
                Ok(devices) => {
                    for device in devices.iter() {
                        match device.device_descriptor() {
                            Ok(desc) => {
                                if is_mouse_device(&device, &desc) {
                                    let name = get_device_name(&device, &desc);
                                    found_mice.push(MouseDevice {
                                        index: 0, // Will be set after sorting.
                                        name,
                                        vid: desc.vendor_id(),
                                        pid: desc.product_id(),
                                        bus_id: device.bus_number(),
                                    });
                                }
                            }
                            Err(_) => continue,
                        }
                    }

                    // Sort by bus ID (ascending).
                    found_mice.sort_by_key(|m| m.bus_id);

                    // Assign indices starting from 1.
                    for (idx, mouse) in found_mice.iter_mut().enumerate() {
                        mouse.index = idx + 1;
                    }

                    Ok(found_mice)
                }
                Err(e) => Err(format!("Failed to get device list: {}", e)),
            }
        }
        Err(e) => Err(format!("Failed to create USB context: {}", e)),
    }
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
/// Platform-specific implementation:
/// - **Linux**: Reads from sysfs (`/sys/bus/usb/devices/`) first, then falls back to USB descriptors.
/// - **Windows**: Reads product/manufacturer strings from USB descriptors.
/// - **Other platforms**: Returns VID:PID as fallback.
///
/// Attempts to get the device name in the following order:
/// 1. **sysfs** (Linux only) - No permissions needed.
/// 2. **USB descriptors** - Product string, then manufacturer string.
/// 3. **Fallback** - Returns "USB Device VID:PID" or "Unknown Device".
///
/// # Arguments
///
/// - `device` - USB device to query.
/// - `desc` - USB device descriptor.
///
/// # Returns
///
/// Device name string, never empty (always has a fallback).
#[cfg(target_os = "linux")]
fn get_device_name<T: UsbContext>(
    device: &rusb::Device<T>,
    desc: &rusb::DeviceDescriptor,
) -> String {
    // Try to read from sysfs first (no permissions needed).
    let bus = device.bus_number();
    let addr = device.address();

    // Try multiple common sysfs path patterns.
    let paths_to_try = vec![
        // Direct device path.
        format!("/sys/bus/usb/devices/{}-{}", bus, addr),
        // Sometimes devices are under numbered ports.
        format!("/sys/bus/usb/devices/usb{}-{}", bus, addr),
    ];

    for device_sysfs in paths_to_try {
        if Path::new(&device_sysfs).exists() {
            // Try product first.
            if let Ok(product) = fs::read_to_string(format!("{}/product", device_sysfs)) {
                let name = product.trim().to_string();
                if !name.is_empty() {
                    return name;
                }
            }

            // Try manufacturer.
            if let Ok(manufacturer) = fs::read_to_string(format!("{}/manufacturer", device_sysfs)) {
                let name = manufacturer.trim().to_string();
                if !name.is_empty() {
                    return name;
                }
            }

            // Try model in case it's stored there.
            if let Ok(model) = fs::read_to_string(format!("{}/model", device_sysfs)) {
                let name = model.trim().to_string();
                if !name.is_empty() {
                    return name;
                }
            }

            // Try searching in interface descriptors subdirectories.
            if let Ok(entries) = fs::read_dir(&device_sysfs) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        // Check interface directories like 1-1:1.0.
                        if let Ok(product) = fs::read_to_string(path.join("product")) {
                            let name = product.trim().to_string();
                            if !name.is_empty() {
                                return name;
                            }
                        }
                        if let Ok(manufacturer) = fs::read_to_string(path.join("manufacturer")) {
                            let name = manufacturer.trim().to_string();
                            if !name.is_empty() {
                                return name;
                            }
                        }
                    }
                }
            }
        }
    }

    // Fallback: try to read USB descriptor strings (requires device access).
    if let Ok(handle) = device.open() {
        // Try product string.
        if let Some(product_idx) = desc.product_string_index() {
            if product_idx > 0 {
                if let Ok(langs) = handle.read_languages(Duration::from_millis(500)) {
                    if !langs.is_empty() {
                        if let Ok(name) =
                            handle.read_product_string(langs[0], desc, Duration::from_millis(500))
                        {
                            if !name.is_empty() && name != "USB Device" {
                                return name;
                            }
                        }
                    }
                }
            }
        }

        // Try manufacturer string as fallback.
        if let Some(mfg_idx) = desc.manufacturer_string_index() {
            if mfg_idx > 0 {
                if let Ok(langs) = handle.read_languages(Duration::from_millis(500)) {
                    if !langs.is_empty() {
                        if let Ok(name) = handle.read_manufacturer_string(
                            langs[0],
                            desc,
                            Duration::from_millis(500),
                        ) {
                            if !name.is_empty() {
                                return name;
                            }
                        }
                    }
                }
            }
        }
    }

    // Final fallback to generic name.
    "Unknown Device".to_string()
}

#[cfg(target_os = "windows")]
fn get_device_name<T: UsbContext>(
    device: &rusb::Device<T>,
    _desc: &rusb::DeviceDescriptor,
) -> String {
    // On Windows, try USB descriptor strings with better error handling.
    if let Ok(handle) = device.open() {
        if let Ok(langs) = handle.read_languages(Duration::from_millis(500)) {
            if !langs.is_empty() {
                // Try product string
                if let Ok(name) =
                    handle.read_product_string(langs[0], _desc, Duration::from_millis(500))
                {
                    if !name.is_empty() && name != "USB Device" {
                        return name;
                    }
                }

                // Try manufacturer string.
                if let Ok(name) =
                    handle.read_manufacturer_string(langs[0], _desc, Duration::from_millis(500))
                {
                    if !name.is_empty() {
                        return name;
                    }
                }
            }
        }
    }

    format!(
        "USB Device {:04X}:{:04X}",
        _desc.vendor_id(),
        _desc.product_id()
    )
}
