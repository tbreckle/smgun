//! INI file handling for Supermodel3 lightgun configuration.
//!
//! This module reads and writes Supermodel3 INI configuration files, automatically
//! updating the lightgun input mappings based on matched USB devices. It preserves
//! the original file layout, comments, and other settings.

use log::debug;
use std::fs;
use std::path::PathBuf;

pub fn test(index: u32, key: &str, find_key: &str, template: &str) -> Option<String> {
    if key == find_key {
        let new_value = template.replace("{}", &index.to_string());
        debug!("Updating {} to {} for player 1", key, new_value);
        return Some(new_value);
    }
    None
}

/// Builds INI key mappings for Player 1 and optionally Player 2 lightguns.
///
/// Based on the matched device indices from USB VID:PID matching, this generates
/// the configuration key mappings that will be written to the Supermodel3 INI file.
/// Supports both analog and digital input modes.
fn build_key_mappings(
    player1_index: usize,
    player2_index: Option<usize>,
    use_analog: bool,
) -> Vec<(u32, String, &'static str)> {
    let (gun_x, gun_y, trigger_left, trigger_right) = if use_analog {
        (
            "InputAnalogGunX",
            "InputAnalogGunY",
            "InputAnalogTriggerLeft",
            "InputAnalogTriggerRight",
        )
    } else {
        (
            "InputGunX",
            "InputGunY",
            "InputTriggerLeft",
            "InputTriggerRight",
        )
    };

    let mut mappings: Vec<(u32, String, &'static str)> = vec![
        (player1_index as u32, gun_x.to_string(), "MOUSE{}_XAXIS"),
        (player1_index as u32, gun_y.to_string(), "MOUSE{}_YAXIS"),
        (
            player1_index as u32,
            trigger_left.to_string(),
            "MOUSE{}_LEFT_BUTTON",
        ),
        (
            player1_index as u32,
            trigger_right.to_string(),
            "MOUSE{}_RIGHT_BUTTON",
        ),
    ];

    if let Some(p2_index) = player2_index {
        let gun_x2 = format!("{}2", gun_x);
        let gun_y2 = format!("{}2", gun_y);
        let trigger_left2 = format!("{}2", trigger_left);
        let trigger_right2 = format!("{}2", trigger_right);

        mappings.push((p2_index as u32, gun_x2, "MOUSE{}_XAXIS"));
        mappings.push((p2_index as u32, gun_y2, "MOUSE{}_YAXIS"));
        mappings.push((p2_index as u32, trigger_left2, "MOUSE{}_LEFT_BUTTON"));
        mappings.push((p2_index as u32, trigger_right2, "MOUSE{}_RIGHT_BUTTON"));
    }

    mappings
}

/// Splits a line into processed content and inline comment.
fn split_inline_comment(trimmed: &str) -> (&str, Option<&str>) {
    if trimmed.contains(';') {
        let parts: Vec<&str> = trimmed.splitn(2, ';').collect();
        (parts[0].trim(), Some(parts[1]))
    } else {
        (trimmed, None)
    }
}

/// Extracts section name from a section header line.
fn extract_section_name(line: &str) -> Option<String> {
    if line.starts_with('[') && line.ends_with(']') {
        Some(
            line.trim_start_matches('[')
                .trim_end_matches(']')
                .trim()
                .to_string(),
        )
    } else {
        None
    }
}

/// Processes a key=value line and returns the updated line if a mapping exists.
fn process_key_value_line(
    key: &str,
    _value: &str,
    inline_comment: Option<&str>,
    key_mappings: &[(u32, String, &'static str)],
) -> Option<String> {
    for (index, find_key, template) in key_mappings {
        if let Some(new_value) = test(*index, key, find_key, template) {
            return Some(if let Some(comment) = inline_comment {
                format!("{} = {} ; {}", key, new_value, comment)
            } else {
                format!("{} = {}", key, new_value)
            });
        }
    }
    None
}

/// Processes a single line and returns the updated line.
fn process_line(
    line: &str,
    has_section: &mut bool,
    current_section_name: &mut String,
    key_mappings: &[(u32, String, &'static str)],
) -> String {
    let trimmed = line.trim();

    // Handle comment lines.
    if trimmed.starts_with(';') {
        debug!("Found comment line: {}", line);
        return line.to_string();
    }

    // Split inline comments.
    let (processed_line, inline_comment) = split_inline_comment(trimmed);

    // Handle section headers.
    if let Some(section_name) = extract_section_name(processed_line) {
        *has_section = true;
        *current_section_name = section_name;
        debug!("Found section: {}", current_section_name);
        return line.to_string();
    }

    // Process key=value pairs in current section.
    if *has_section {
        if let Some(eq_pos) = processed_line.find('=') {
            let key = processed_line[..eq_pos].trim();
            let value = processed_line[eq_pos + 1..].trim();

            debug!("Found key: '{}', value: '{}'", key, value);

            if let Some(updated_line) =
                process_key_value_line(key, value, inline_comment, key_mappings)
            {
                debug!("Updated line: {}", updated_line);
                return updated_line;
            }
        }
    }

    // No update needed, return original line.
    line.to_string()
}

/// Writes lightgun device configuration to a Supermodel3 INI file.
///
/// Updates the INI file with the matched lightgun device indices based on VID:PID matching.
/// Preserves the original file layout, comments, and other settings while updating only
/// the necessary lightgun input mapping keys in the [Global] section.
///
/// # Arguments
///
/// - `path` - Path to the Supermodel3 INI file.
/// - `player1_index` - Index of the matched Player 1 lightgun device.
/// - `player2_index` - Optional index of the matched Player 2 lightgun device.
/// - `use_analog` - If true, uses InputAnalogGunX/Y and InputAnalogTriggerLeft/Right;
///   if false, uses InputGunX/Y and InputTriggerLeft/Right.
///
/// # Returns
///
/// - `Ok(())` - If the file was successfully updated.
/// - `Err(String)` - If there was an error reading or writing the file.
pub fn write_mouse_config(
    path: &PathBuf,
    player1_index: usize,
    player2_index: Option<usize>,
    use_analog: bool,
) -> Result<(), String> {
    if !path.exists() {
        return Err(format!("INI file does not exist: {}", path.display()));
    }

    let key_mappings = build_key_mappings(player1_index, player2_index, use_analog);

    let content =
        fs::read_to_string(path).map_err(|e| format!("Failed to read INI file: {}", e))?;

    let lines: Vec<String> = content.lines().map(|line| line.to_string()).collect();

    let mut new_lines: Vec<String> = Vec::new();
    let mut has_section = false;
    let mut current_section_name = String::new();

    for line in lines {
        let processed_line = process_line(
            &line,
            &mut has_section,
            &mut current_section_name,
            &key_mappings,
        );
        new_lines.push(processed_line);
    }

    debug!("Final INI content:");
    for line in &new_lines {
        debug!("{}", line);
    }

    let output = new_lines.join("\n");
    fs::write(path, output).map_err(|e| format!("Failed to write INI file: {}", e))?;
    Ok(())
}
