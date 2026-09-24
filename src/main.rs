// anstream strips colors when not writing to a terminal and translates them for legacy
// Windows consoles.
use anstream::println;
use clap::Parser;
use log::{error, info};
use sm3lgs::MouseDevice;
use std::path::PathBuf;
use std::process;

/// Supermodel3 lightgun auto-configurator - automatically select connected lightguns by VID:PID
///
/// This tool searches for USB devices matching specified VID:PID combinations and automatically
/// updates the Supermodel3 emulator configuration to use them for player 1 and 2 lightgun inputs.
#[derive(Parser, Debug)]
#[command(name = "sm3lgs")]
#[command(about = "Auto-configure Supermodel3 lightguns by USB VID:PID matching", long_about = None)]
struct Args {
    /// List all connected USB devices and their VID:PID values, then exit.
    #[arg(short, long)]
    list: bool,

    /// USB VID:PID for Player 1's lightgun (e.g., 046D:C05A).
    #[arg(
        short = '1',
        long,
        value_parser = parse_vid_pid,
        required_unless_present = "list"
    )]
    gun1: Option<(u16, u16)>,

    /// USB VID:PID for Player 2's lightgun (e.g., 046D:C05B).
    #[arg(short = '2', long, value_parser = parse_vid_pid)]
    gun2: Option<(u16, u16)>,

    /// Path to the Supermodel3 INI file to update.
    #[arg(
        short,
        long,
        value_name = "FILE",
        default_value = "Config/Supermodel.ini"
    )]
    ini: PathBuf,

    /// Use InputAnalogGunX/Y and InputAnalogTriggerLeft/Right instead of InputGunX/Y, InputTrigger and InputOffscreen.
    #[arg(long)]
    use_analog: bool,
}

fn parse_vid_pid(s: &str) -> Result<(u16, u16), String> {
    let parse_id = |id: &str, what: &str| {
        let id = id.trim();
        if id.is_empty() || id.len() > 4 || !id.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(format!(
                "Invalid {} '{}'. Must be a hex value with up to 4 digits.",
                what, id
            ));
        }
        u16::from_str_radix(id, 16).map_err(|e| e.to_string())
    };

    let (vid, pid) = s.split_once(':').ok_or_else(|| {
        format!(
            "Invalid VID:PID format '{}'. Expected VID:PID (e.g., 046D:C05A).",
            s
        )
    })?;

    Ok((parse_id(vid, "VID")?, parse_id(pid, "PID")?))
}

/// Searches enumerated devices for a matching VID:PID combination.
///
/// Returns the device index (1-based) of the first device with the specified VID:PID,
/// skipping the device with index `exclude` (e.g. the one already assigned to Player 1).
fn find_device_by_vid_pid(
    mice: &[MouseDevice],
    vid: u16,
    pid: u16,
    exclude: Option<usize>,
) -> Option<usize> {
    mice.iter()
        .find(|m| m.vid == vid && m.pid == pid && Some(m.index) != exclude)
        .map(|m| m.index)
}

fn print_banner() {
    let l1 = "\x1b[96m";
    let l2 = "\x1b[94m";
    let l3 = "\x1b[36m";
    let l4 = "\x1b[34m";
    let l5 = "\x1b[34m";
    let l6 = "\x1b[34m";

    let r = "\x1b[0m";
    println!("{}               ________ .__                 {}", l1, r);
    println!("{}  ______ _____ \\_____  \\|  |    ____  ______{}", l2, r);
    println!("{} /  ___//     \\  _(__  <|  |   / ___\\/  ___/{}", l3, r);
    println!(
        "{} \\___ \\|  Y Y  \\/       \\  |__/ /_/  >___ \\ {}",
        l4, r
    );
    println!("{}/____  >__|_|  /______  /____/\\___  /____  >{}", l5, r);
    println!(
        "{}     \\/      \\/       \\/     /_____/     \\/ {}",
        l6, r
    );

    println!();
    println!(
        "Supermodel3 Lightgun Auto-Configurator v{} (hash: {}, date: {})",
        env!("CARGO_PKG_VERSION"),
        env!("GIT_HASH"),
        env!("BUILD_DATE")
    );
    println!();
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    print_banner();

    let args = Args::parse();

    // Enumerate all connected USB devices.
    let mice = match sm3lgs::enumerate_mice() {
        Ok(mice) => mice,
        Err(e) => {
            error!("Failed to enumerate devices: {}", e);
            process::exit(1);
        }
    };

    if mice.is_empty() {
        error!("No devices found that can be used as lightguns.");
        process::exit(if args.list { 0 } else { 1 });
    }

    info!("Found {} device(s):", mice.len());
    for mouse in &mice {
        info!("  {}", mouse);
    }
    info!("");

    // If --list flag is set, exit after listing devices.
    if args.list {
        info!("Device listing complete.");
        process::exit(0);
    }

    // Find gun1 device by VID:PID.
    let gun1 = match args.gun1 {
        Some(g) => g,
        None => {
            error!("--gun1 is required unless --list is specified.");
            process::exit(2);
        }
    };
    let gun1_index = match find_device_by_vid_pid(&mice, gun1.0, gun1.1, None) {
        Some(idx) => idx,
        None => {
            error!(
                "Player 1 lightgun with VID:{:04X} PID:{:04X} not found.",
                gun1.0, gun1.1
            );
            process::exit(1);
        }
    };

    info!(
        "Player 1: Device {} (VID:{:04X} PID:{:04X})",
        gun1_index, gun1.0, gun1.1
    );

    // Find gun2 device if specified.
    let gun2_index = if let Some((vid, pid)) = args.gun2 {
        match find_device_by_vid_pid(&mice, vid, pid, Some(gun1_index)) {
            Some(idx) => {
                info!("Player 2: Device {} (VID:{:04X} PID:{:04X})", idx, vid, pid);
                Some(idx)
            }
            None => {
                if (vid, pid) == gun1 {
                    error!(
                        "Player 2 lightgun with VID:{:04X} PID:{:04X} not found (only one device with this VID:PID, already assigned to Player 1).",
                        vid, pid
                    );
                } else {
                    error!(
                        "Player 2 lightgun with VID:{:04X} PID:{:04X} not found.",
                        vid, pid
                    );
                }
                process::exit(1);
            }
        }
    } else {
        info!("Player 2: Not configured.");
        None
    };

    info!("");

    // Write configuration to INI file based on matched devices.
    match sm3lgs::ini::write_mouse_config(&args.ini, gun1_index, gun2_index, args.use_analog) {
        Ok(_) => {
            info!(
                "Successfully updated Supermodel3 configuration: {}",
                args.ini.display()
            );
        }
        Err(e) => {
            error!("Failed to update INI file: {}", e);
            process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mouse(index: usize, vid: u16, pid: u16) -> MouseDevice {
        MouseDevice {
            index,
            name: String::new(),
            vid,
            pid,
            bus_id: 0,
        }
    }

    #[test]
    fn parses_vid_pid() {
        assert_eq!(parse_vid_pid("046D:C05A"), Ok((0x046D, 0xC05A)));
        assert_eq!(parse_vid_pid("46d:c05a"), Ok((0x046D, 0xC05A)));
        assert!(parse_vid_pid("046D").is_err());
        assert!(parse_vid_pid("046D:C05A:1").is_err());
        assert!(parse_vid_pid("+46D:C05A").is_err());
        assert!(parse_vid_pid("0046D:C05A").is_err());
        assert!(parse_vid_pid(":C05A").is_err());
    }

    #[test]
    fn player2_skips_player1_device() {
        let mice = [mouse(1, 1, 2), mouse(2, 3, 4), mouse(3, 1, 2)];
        assert_eq!(find_device_by_vid_pid(&mice, 1, 2, None), Some(1));
        assert_eq!(find_device_by_vid_pid(&mice, 1, 2, Some(1)), Some(3));
        assert_eq!(find_device_by_vid_pid(&mice, 3, 4, Some(1)), Some(2));
        assert_eq!(find_device_by_vid_pid(&mice[..2], 1, 2, Some(1)), None);
    }
}
