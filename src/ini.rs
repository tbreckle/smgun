//! INI file handling for Supermodel3 lightgun configuration.
//!
//! This module reads and writes Supermodel3 INI configuration files, automatically
//! updating the lightgun input mappings based on matched USB devices. It preserves
//! the original file layout, comments, and other settings.
//!
//! Parsing follows Supermodel3's own INI rules (`Src/Util/ConfigBuilders.cpp`):
//! - `;` starts a comment unless it is inside double quotes.
//! - Settings before the first section header belong to `[ Global ]`.
//! - A header may list several sections (`[ a, b ]`); an empty name means `Global`.

use log::{debug, info};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

const UTF8_BOM: &str = "\u{feff}";

/// Builds the INI key/value mappings for Player 1 and optionally Player 2 lightguns.
///
/// Based on the matched device indices from USB VID:PID matching, this generates
/// the settings that will be written to the Supermodel3 INI file.
/// Supports both analog and digital (light) gun inputs.
fn build_key_mappings(
    player1_index: usize,
    player2_index: Option<usize>,
    use_analog: bool,
) -> Vec<(String, String)> {
    let (gun_x, gun_y, trigger, offscreen) = if use_analog {
        (
            "InputAnalogGunX",
            "InputAnalogGunY",
            "InputAnalogTriggerLeft",
            "InputAnalogTriggerRight",
        )
    } else {
        ("InputGunX", "InputGunY", "InputTrigger", "InputOffscreen")
    };

    let mut players = vec![(player1_index, "")];
    if let Some(p2_index) = player2_index {
        players.push((p2_index, "2"));
    }

    players
        .into_iter()
        .flat_map(|(index, suffix)| {
            [
                (gun_x, "XAXIS"),
                (gun_y, "YAXIS"),
                (trigger, "LEFT_BUTTON"),
                (offscreen, "RIGHT_BUTTON"),
            ]
            .map(|(key, input)| (format!("{key}{suffix}"), format!("MOUSE{index}_{input}")))
        })
        .collect()
}

/// Returns the byte offset of the first `;` that is not enclosed in double quotes.
fn find_comment_start(line: &str) -> Option<usize> {
    let mut inside_quotes = false;
    for (pos, c) in line.char_indices() {
        match c {
            '"' => inside_quotes = !inside_quotes,
            ';' if !inside_quotes => return Some(pos),
            _ => {}
        }
    }
    None
}

/// Parses a section header (`[ a, b ]`) and returns whether it includes `Global`.
fn parse_section_header(content: &str) -> Option<bool> {
    let names = content.strip_prefix('[')?.strip_suffix(']')?;
    Some(
        names
            .split(',')
            .map(str::trim)
            .any(|name| name.is_empty() || name == "Global"),
    )
}

/// Replaces the value of a `key = value ; comment` line, keeping the key, spacing and comment.
fn replace_value(line: &str, eq_pos: usize, comment_start: usize, new_value: &str) -> String {
    let old_value = &line[eq_pos + 1..comment_start];
    let trailing_ws = &old_value[old_value.trim_end().len()..];
    let comment = &line[comment_start..];
    let separator = if !comment.is_empty() && trailing_ws.is_empty() {
        " "
    } else {
        trailing_ws
    };
    format!(
        "{} \"{}\"{}{}",
        &line[..=eq_pos],
        new_value,
        separator,
        comment
    )
}

/// Applies the key mappings to the INI content and returns the updated content.
///
/// Matching keys are updated in every section so that per-game overrides can't shadow
/// the new settings. Keys missing from `[ Global ]` are added to it.
fn update_content(content: &str, key_mappings: &[(String, String)]) -> String {
    let (bom, content) = match content.strip_prefix(UTF8_BOM) {
        Some(rest) => (UTF8_BOM, rest),
        None => ("", content),
    };
    let newline = if content.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };

    let mut lines: Vec<String> = content.lines().map(str::to_string).collect();
    let mut in_global = true;
    let mut found_in_global = HashSet::new();
    // Index after the last setting of the first explicit `[ Global ]` section.
    let mut global_insert_pos = None;
    let mut in_explicit_global = false;

    for (line_idx, line) in lines.iter_mut().enumerate() {
        let comment_start = find_comment_start(line).unwrap_or(line.len());
        let stripped = line[..comment_start].trim();
        if stripped.is_empty() {
            continue;
        }

        if let Some(includes_global) = parse_section_header(stripped) {
            debug!("Found section: {}", stripped);
            in_global = includes_global;
            in_explicit_global = includes_global && global_insert_pos.is_none();
            if in_explicit_global {
                global_insert_pos = Some(line_idx + 1);
            }
            continue;
        }

        if in_explicit_global {
            global_insert_pos = Some(line_idx + 1);
        }

        let Some(eq_pos) = line[..comment_start].find('=') else {
            continue;
        };
        let key = line[..eq_pos].trim();
        let Some((_, new_value)) = key_mappings.iter().find(|(k, _)| k == key) else {
            continue;
        };

        if in_global {
            found_in_global.insert(key.to_string());
        }
        let updated = replace_value(line, eq_pos, comment_start, new_value);
        debug!("Updating line '{}' to '{}'", line, updated);
        *line = updated;
    }

    let missing: Vec<String> = key_mappings
        .iter()
        .filter(|(key, _)| !found_in_global.contains(key))
        .map(|(key, value)| {
            info!("Adding missing setting {} to [ Global ]", key);
            format!("{} = \"{}\"", key, value)
        })
        .collect();

    let added_keys = !missing.is_empty();
    if added_keys {
        match global_insert_pos {
            Some(pos) => {
                lines.splice(pos..pos, missing);
            }
            None => {
                if lines.last().is_some_and(|l| !l.trim().is_empty()) {
                    lines.push(String::new());
                }
                lines.push("[ Global ]".to_string());
                lines.extend(missing);
            }
        }
    }

    let mut output = String::from(bom);
    output.push_str(&lines.join(newline));
    if content.ends_with('\n') || added_keys {
        output.push_str(newline);
    }
    output
}

/// Writes lightgun device configuration to a Supermodel3 INI file.
///
/// Updates the INI file with the matched lightgun device indices based on VID:PID matching.
/// Preserves the original file layout, comments, and other settings while updating only
/// the lightgun input mapping keys (in all sections, so per-game overrides are updated too).
/// Keys missing from `[ Global ]` are added to it.
///
/// # Arguments
///
/// - `path` - Path to the Supermodel3 INI file.
/// - `player1_index` - Index of the matched Player 1 lightgun device.
/// - `player2_index` - Optional index of the matched Player 2 lightgun device.
/// - `use_analog` - If true, uses InputAnalogGunX/Y and InputAnalogTriggerLeft/Right;
///   if false, uses InputGunX/Y, InputTrigger and InputOffscreen.
///
/// # Returns
///
/// - `Ok(())` - If the file was successfully updated.
/// - `Err(String)` - If there was an error reading or writing the file.
pub fn write_mouse_config(
    path: &Path,
    player1_index: usize,
    player2_index: Option<usize>,
    use_analog: bool,
) -> Result<(), String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read INI file {}: {}", path.display(), e))?;

    let key_mappings = build_key_mappings(player1_index, player2_index, use_analog);
    let output = update_content(&content, &key_mappings);

    debug!("Final INI content:\n{}", output);

    fs::write(path, output)
        .map_err(|e| format!("Failed to write INI file {}: {}", path.display(), e))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mappings() -> Vec<(String, String)> {
        build_key_mappings(3, Some(1), false)
    }

    #[test]
    fn builds_digital_and_analog_keys() {
        let digital = build_key_mappings(2, None, false);
        assert_eq!(
            digital,
            vec![
                ("InputGunX".into(), "MOUSE2_XAXIS".into()),
                ("InputGunY".into(), "MOUSE2_YAXIS".into()),
                ("InputTrigger".into(), "MOUSE2_LEFT_BUTTON".into()),
                ("InputOffscreen".into(), "MOUSE2_RIGHT_BUTTON".into()),
            ]
        );

        let analog = build_key_mappings(1, Some(2), true);
        assert_eq!(analog.len(), 8);
        assert_eq!(
            analog[7],
            (
                "InputAnalogTriggerRight2".into(),
                "MOUSE2_RIGHT_BUTTON".into()
            )
        );
    }

    #[test]
    fn updates_values_and_keeps_comments() {
        let input = "[ Global ]\n\
                     InputGunX = \"MOUSE_XAXIS,JOY1_XAXIS\"    ; analog, full X axis\n\
                     InputGunY=\"x\";c\n\
                     InputTrigger = \"KEY_A;B\"\n\
                     InputOffscreen = \"KEY_S\"\n\
                     InputGunX2 = \"JOY2_XAXIS\"\n\
                     InputGunY2 = \"JOY2_YAXIS\"\n\
                     InputTrigger2 = \"JOY2_BUTTON1\"\n\
                     InputOffscreen2 = \"JOY2_BUTTON2\"\n\
                     ; InputGunX = \"untouched\"\n";
        let expected = "[ Global ]\n\
                        InputGunX = \"MOUSE3_XAXIS\"    ; analog, full X axis\n\
                        InputGunY= \"MOUSE3_YAXIS\" ;c\n\
                        InputTrigger = \"MOUSE3_LEFT_BUTTON\"\n\
                        InputOffscreen = \"MOUSE3_RIGHT_BUTTON\"\n\
                        InputGunX2 = \"MOUSE1_XAXIS\"\n\
                        InputGunY2 = \"MOUSE1_YAXIS\"\n\
                        InputTrigger2 = \"MOUSE1_LEFT_BUTTON\"\n\
                        InputOffscreen2 = \"MOUSE1_RIGHT_BUTTON\"\n\
                        ; InputGunX = \"untouched\"\n";
        assert_eq!(update_content(input, &mappings()), expected);
    }

    #[test]
    fn treats_settings_before_first_section_as_global() {
        let input = "InputGunX = \"a\"\n[ lostwsga ]\nInputGunX = \"b\"\n";
        let output = update_content(input, &build_key_mappings(1, None, false));
        assert!(output.starts_with("InputGunX = \"MOUSE1_XAXIS\"\n[ lostwsga ]\n"));
        assert!(output.contains("[ lostwsga ]\nInputGunX = \"MOUSE1_XAXIS\"\n"));
        // The remaining keys are missing from Global and appended in a new Global section.
        assert!(output.ends_with(
            "\n[ Global ]\n\
             InputGunY = \"MOUSE1_YAXIS\"\n\
             InputTrigger = \"MOUSE1_LEFT_BUTTON\"\n\
             InputOffscreen = \"MOUSE1_RIGHT_BUTTON\"\n"
        ));
    }

    #[test]
    fn adds_missing_keys_to_end_of_global_section() {
        let input = "[ Global ]\nInputGunX = \"a\"\n\n[ game ]\nFoo = 1\n";
        let output = update_content(input, &build_key_mappings(1, None, false));
        assert_eq!(
            output,
            "[ Global ]\n\
             InputGunX = \"MOUSE1_XAXIS\"\n\
             InputGunY = \"MOUSE1_YAXIS\"\n\
             InputTrigger = \"MOUSE1_LEFT_BUTTON\"\n\
             InputOffscreen = \"MOUSE1_RIGHT_BUTTON\"\n\
             \n\
             [ game ]\n\
             Foo = 1\n"
        );
    }

    #[test]
    fn handles_multi_section_headers() {
        assert_eq!(parse_section_header("[ a, Global ]"), Some(true));
        assert_eq!(parse_section_header("[]"), Some(true));
        assert_eq!(parse_section_header("[ a,b ]"), Some(false));
        assert_eq!(parse_section_header("Key = [x]"), None);
    }

    #[test]
    fn preserves_crlf_bom_and_missing_trailing_newline() {
        let keys = build_key_mappings(1, None, false);
        let input = "\u{feff}[ Global ]\r\n\
                     InputGunX = \"a\"\r\n\
                     InputGunY = \"a\"\r\n\
                     InputTrigger = \"a\"\r\n\
                     InputOffscreen = \"a\"";
        let output = update_content(input, &keys);
        assert_eq!(
            output,
            "\u{feff}[ Global ]\r\n\
             InputGunX = \"MOUSE1_XAXIS\"\r\n\
             InputGunY = \"MOUSE1_YAXIS\"\r\n\
             InputTrigger = \"MOUSE1_LEFT_BUTTON\"\r\n\
             InputOffscreen = \"MOUSE1_RIGHT_BUTTON\""
        );
    }
}
