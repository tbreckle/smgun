use std::process::Command;

fn main() {
    // Generate git hash
    let git_hash = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    println!("cargo:rustc-env=GIT_HASH={}", git_hash);

    // Generate build date (cross-platform compatible)
    let build_date = if cfg!(target_os = "windows") {
        // Windows PowerShell command
        Command::new("powershell")
            .args(["-Command", "Get-Date -Format 'yyyy-MM-dd'"])
            .output()
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
            .unwrap_or_else(|_| "unknown".to_string())
    } else {
        // Unix/Linux date command
        Command::new("date")
            .arg("+%Y-%m-%d")
            .output()
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
            .unwrap_or_else(|_| "unknown".to_string())
    };

    println!("cargo:rustc-env=BUILD_DATE={}", build_date);
}
