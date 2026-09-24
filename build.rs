use std::process::Command;

/// Runs a command and returns its trimmed stdout, or "unknown" if it fails.
fn command_output(program: &str, args: &[&str]) -> String {
    Command::new(program)
        .args(args)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

fn main() {
    // Rebuild when the checked out commit changes, not only when package files change.
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs");
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=Cargo.toml");

    // Generate git hash
    let git_hash = command_output("git", &["rev-parse", "--short", "HEAD"]);
    println!("cargo:rustc-env=GIT_HASH={}", git_hash);

    // Generate build date (cross-platform compatible)
    let build_date = if cfg!(target_os = "windows") {
        // Windows PowerShell command
        command_output("powershell", &["-Command", "Get-Date -Format 'yyyy-MM-dd'"])
    } else {
        // Unix/Linux date command
        command_output("date", &["+%Y-%m-%d"])
    };
    println!("cargo:rustc-env=BUILD_DATE={}", build_date);
}
