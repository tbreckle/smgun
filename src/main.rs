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

    /// Use InputAnalogGunX/Y instead of InputGunX/Y (and InputAnalogTriggerLeft/Right instead of InputTriggerLeft/Right).
    #[arg(long)]
    use_analog: bool,
}

fn parse_vid_pid(s: &str) -> Result<(u16, u16), String> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        return Err(format!(
            "Invalid VID:PID format '{}'. Expected VID:PID (e.g., 046D:C05A).",
            s
        ));
    }

    let vid = u16::from_str_radix(parts[0], 16)
        .map_err(|_| format!("Invalid VID '{}'. Must be a 4-digit hex value.", parts[0]))?;

    let pid = u16::from_str_radix(parts[1], 16)
        .map_err(|_| format!("Invalid PID '{}'. Must be a 4-digit hex value", parts[1]))?;

    Ok((vid, pid))
}

/// Searches enumerated devices for a matching VID:PID combination.
///
/// Returns the device index (1-based) if a device with the specified VID:PID is found.
fn find_device_by_vid_pid(mice: &[MouseDevice], vid: u16, pid: u16) -> Option<usize> {
    mice.iter()
        .find(|m| m.vid == vid && m.pid == pid)
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
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    print_banner();

    let args = Args::parse();

    // Enumerate all connected USB devices.
    let mice = match sm3lgs::enumerate_mice() {
        Ok(mice) => mice,
        Err(e) => {
            error!("Failed to enumerate USB devices: {}", e);
            process::exit(1);
        }
    };

    if mice.is_empty() {
        error!("No USB HID devices found that can be used as lightguns.");
        process::exit(1);
    }

    info!("Found {} USB device(s):", mice.len());
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
    let gun1_index = match find_device_by_vid_pid(&mice, gun1.0, gun1.1) {
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
        match find_device_by_vid_pid(&mice, vid, pid) {
            Some(idx) => {
                info!("Player 2: Device {} (VID:{:04X} PID:{:04X})", idx, vid, pid);
                Some(idx)
            }
            None => {
                error!(
                    "Player 2 lightgun with VID:{:04X} PID:{:04X} not found.",
                    vid, pid
                );
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
