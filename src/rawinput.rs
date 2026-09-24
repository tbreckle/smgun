//! Windows Raw Input mouse enumeration.
//!
//! On Windows, Supermodel3 (with `InputSystem = rawinput`) numbers mice in the order it
//! finds them via the Raw Input API, not by USB bus. To produce matching `MOUSEx` indices
//! this module mirrors Supermodel3's enumeration (`CDirectInputSystem::OpenKeyboardsAndMice`):
//!
//! 1. Query all devices with `GetRawInputDeviceList`.
//! 2. Walk the list **backwards** (Supermodel3 does this because new devices are usually
//!    added at the beginning).
//! 3. Skip devices whose name can't be read or is longer than 255 characters.
//! 4. Skip Remote Desktop devices (`Root#RDP_`).
//! 5. Keep only devices of type `RIM_TYPEMOUSE`.
//!
//! Every mouse counts towards the index, including non-USB ones (e.g. PS/2 or virtual
//! mice), which are reported with VID:PID 0000:0000.

use crate::device::MouseDevice;
use std::ffi::c_void;
use std::mem::{size_of, size_of_val};
use std::ptr;
use windows_sys::Win32::Devices::HumanInterfaceDevice::{
    HidD_GetManufacturerString, HidD_GetProductString,
};
use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_INSUFFICIENT_BUFFER, HANDLE, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows_sys::Win32::UI::Input::{
    GetRawInputDeviceInfoW, GetRawInputDeviceList, RAWINPUTDEVICELIST, RIDI_DEVICENAME,
    RIM_TYPEMOUSE,
};

/// Maximum device name length Supermodel3 accepts (`MAX_NAME_LENGTH`). Devices with
/// longer names fail its `GetRawInputDeviceInfo` call and are skipped.
const MAX_NAME_LENGTH: u32 = 255;

/// Enumerates all Raw Input mice in the same order as Supermodel3.
///
/// # Returns
///
/// - `Ok(Vec<MouseDevice>)` - Vector of detected mice, indexed like Supermodel3's `MOUSEx`.
/// - `Err(String)` - Error message if the Raw Input device list can't be queried.
pub fn enumerate_mice() -> Result<Vec<MouseDevice>, String> {
    let devices = get_raw_input_devices()?;
    let mut found_mice = Vec::new();

    for device in devices.iter().rev() {
        let Some(path) = get_device_path(device) else {
            continue;
        };
        let path_str = String::from_utf16_lossy(&path);

        if path_str.contains("Root#RDP_") || device.dwType != RIM_TYPEMOUSE {
            continue;
        }

        let (vid, pid) = parse_vid_pid(&path_str).unwrap_or((0, 0));
        found_mice.push(MouseDevice {
            index: found_mice.len() + 1,
            name: get_device_name(&path).unwrap_or_else(|| "Unknown Mouse".to_string()),
            vid,
            pid,
            bus_id: 0, // No bus ordering on Windows.
        });
    }

    Ok(found_mice)
}

/// Returns all Raw Input devices in the order reported by `GetRawInputDeviceList`.
fn get_raw_input_devices() -> Result<Vec<RAWINPUTDEVICELIST>, String> {
    let entry_size = size_of::<RAWINPUTDEVICELIST>() as u32;

    loop {
        let mut count: u32 = 0;
        // SAFETY: Passing a null list only queries the number of devices.
        if unsafe { GetRawInputDeviceList(ptr::null_mut(), &mut count, entry_size) } != 0 {
            return Err(format!(
                "Failed to query Raw Input device count: {}",
                std::io::Error::last_os_error()
            ));
        }

        let mut devices: Vec<RAWINPUTDEVICELIST> = Vec::with_capacity(count as usize);
        // SAFETY: The buffer has capacity for `count` entries.
        let written =
            unsafe { GetRawInputDeviceList(devices.as_mut_ptr(), &mut count, entry_size) };
        if written != u32::MAX {
            // SAFETY: The call initialized `written` entries.
            unsafe { devices.set_len(written as usize) };
            return Ok(devices);
        }

        let error = std::io::Error::last_os_error();
        // A device was connected between both calls, so query the new count and retry.
        if error.raw_os_error() != Some(ERROR_INSUFFICIENT_BUFFER as i32) {
            return Err(format!("Failed to get Raw Input device list: {}", error));
        }
    }
}

/// Reads the device interface path (e.g. `\\?\HID#VID_046D&PID_C05A#...`) of a Raw Input
/// device, without the trailing NUL.
///
/// Returns `None` in the same cases where Supermodel3 skips the device.
fn get_device_path(device: &RAWINPUTDEVICELIST) -> Option<Vec<u16>> {
    let mut len: u32 = 0;
    // SAFETY: Passing a null buffer only queries the name length (in characters).
    if unsafe { GetRawInputDeviceInfoW(device.hDevice, RIDI_DEVICENAME, ptr::null_mut(), &mut len) }
        != 0
        || len > MAX_NAME_LENGTH
    {
        return None;
    }

    let mut buf = vec![0u16; len as usize];
    // SAFETY: The buffer holds `len` UTF-16 characters.
    let copied = unsafe {
        GetRawInputDeviceInfoW(
            device.hDevice,
            RIDI_DEVICENAME,
            buf.as_mut_ptr() as *mut c_void,
            &mut len,
        )
    };
    if copied == u32::MAX {
        return None;
    }

    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    buf.truncate(end);
    Some(buf)
}

/// Extracts VID and PID from a device path.
///
/// Handles USB (`VID_046D&PID_C05A`) and Bluetooth (`VID&0002046D_PID&B01A`, where the
/// first four digits are the vendor ID source) formats.
fn parse_vid_pid(path: &str) -> Option<(u16, u16)> {
    let upper = path.to_ascii_uppercase();

    let read_id = |key: &str| -> Option<u16> {
        let start = upper.find(key)? + key.len();
        let digits: String = upper[start..]
            .chars()
            .take_while(|c| c.is_ascii_hexdigit())
            .collect();
        let id = match digits.len() {
            4 => &digits[..],
            8 => &digits[4..],
            _ => return None,
        };
        u16::from_str_radix(id, 16).ok()
    };

    let vid = read_id("VID_").or_else(|| read_id("VID&"))?;
    let pid = read_id("PID_").or_else(|| read_id("PID&"))?;
    Some((vid, pid))
}

/// Retrieves the HID product (or manufacturer) string for a device path.
fn get_device_name(path: &[u16]) -> Option<String> {
    let mut path_z = path.to_vec();
    path_z.push(0);

    // SAFETY: `path_z` is NUL-terminated. Zero access rights are enough to query HID strings
    // and don't require exclusive access to the mouse.
    let handle = unsafe {
        CreateFileW(
            path_z.as_ptr(),
            0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            ptr::null(),
            OPEN_EXISTING,
            0,
            ptr::null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return None;
    }

    let name = read_hid_string(handle, HidD_GetProductString)
        .or_else(|| read_hid_string(handle, HidD_GetManufacturerString));

    // SAFETY: `handle` was returned by `CreateFileW` and is closed exactly once.
    unsafe { CloseHandle(handle) };

    name
}

/// Reads a HID string descriptor using one of the `HidD_Get*String` functions.
fn read_hid_string(
    handle: HANDLE,
    get_string: unsafe extern "system" fn(HANDLE, *mut c_void, u32) -> bool,
) -> Option<String> {
    // HID strings are limited to 126 wide characters plus NUL.
    let mut buf = [0u16; 127];
    // SAFETY: `handle` is an open HID device and the length is the buffer size in bytes.
    if !unsafe {
        get_string(
            handle,
            buf.as_mut_ptr() as *mut c_void,
            size_of_val(&buf) as u32,
        )
    } {
        return None;
    }

    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    let name = String::from_utf16_lossy(&buf[..end]).trim().to_string();
    (!name.is_empty()).then_some(name)
}

#[cfg(test)]
mod tests {
    use super::parse_vid_pid;

    #[test]
    fn parses_usb_path() {
        assert_eq!(
            parse_vid_pid(
                r"\\?\HID#VID_046D&PID_C05A#7&1a2b3c4d&0&0000#{378de44c-56ef-11d1-bc8c-00a0c91405dd}"
            ),
            Some((0x046D, 0xC05A))
        );
    }

    #[test]
    fn parses_composite_usb_path() {
        assert_eq!(
            parse_vid_pid(
                r"\\?\HID#VID_16c0&PID_0f39&MI_02#8&2d3e&0&0000#{378de44c-56ef-11d1-bc8c-00a0c91405dd}"
            ),
            Some((0x16C0, 0x0F39))
        );
    }

    #[test]
    fn parses_bluetooth_path() {
        assert_eq!(
            parse_vid_pid(
                r"\\?\HID#{00001124-0000-1000-8000-00805f9b34fb}_VID&0002046d_PID&b01a&Col01#9&1&0&0000#{378de44c-56ef-11d1-bc8c-00a0c91405dd}"
            ),
            Some((0x046D, 0xB01A))
        );
    }

    #[test]
    fn rejects_non_usb_path() {
        assert_eq!(
            parse_vid_pid(r"\\?\ACPI#PNP0F13#4&2f94427b&0#{378de44c-56ef-11d1-bc8c-00a0c91405dd}"),
            None
        );
    }
}
