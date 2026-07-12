//! Install ShareClick as a background service that auto-starts on login and
//! keeps running — no terminal, no second app to open.
//!
//! * **macOS:** a per-user LaunchAgent (`~/Library/LaunchAgents`) running
//!   `shareclick run`, relaunched by launchd if it dies (`KeepAlive`).
//! * **Windows:** an `HKCU\…\Run` entry launching `shareclick run`, which hides
//!   its own console window immediately.

#![cfg(feature = "native")]

use std::path::PathBuf;
use std::process::Command;

const LABEL: &str = "com.shareclick.agent";

/// Install + start the background service for the current user.
pub fn install() -> anyhow::Result<()> {
    let exe = std::env::current_exe()?;
    #[cfg(target_os = "macos")]
    {
        install_macos(&exe)
    }
    #[cfg(target_os = "windows")]
    {
        install_windows(&exe)
    }
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        let _ = exe;
        anyhow::bail!("service install is not supported on this platform yet")
    }
}

/// Stop + remove the background service.
pub fn uninstall() -> anyhow::Result<()> {
    #[cfg(target_os = "macos")]
    {
        uninstall_macos()
    }
    #[cfg(target_os = "windows")]
    {
        uninstall_windows()
    }
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        anyhow::bail!("service uninstall is not supported on this platform yet")
    }
}

#[cfg(target_os = "macos")]
fn plist_path() -> anyhow::Result<PathBuf> {
    let home = std::env::var("HOME").map_err(|_| anyhow::anyhow!("no HOME"))?;
    Ok(PathBuf::from(home)
        .join("Library/LaunchAgents")
        .join(format!("{LABEL}.plist")))
}

#[cfg(target_os = "macos")]
fn install_macos(exe: &std::path::Path) -> anyhow::Result<()> {
    let path = plist_path()?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>{LABEL}</string>
  <key>ProgramArguments</key>
  <array>
    <string>{exe}</string>
    <string>run</string>
  </array>
  <key>RunAtLoad</key><true/>
  <key>KeepAlive</key><true/>
  <key>ProcessType</key><string>Interactive</string>
</dict>
</plist>
"#,
        exe = exe.display()
    );
    std::fs::write(&path, plist)?;
    // Reload if already loaded, then load.
    let _ = Command::new("launchctl")
        .args(["unload", &path.to_string_lossy()])
        .status();
    let status = Command::new("launchctl")
        .args(["load", "-w", &path.to_string_lossy()])
        .status()?;
    if !status.success() {
        anyhow::bail!("launchctl load failed");
    }
    println!("ShareClick installed as a login service (LaunchAgent).");
    println!("It will start automatically on every login and run in the background.");
    println!("Stop/remove it with:  shareclick uninstall-service");
    Ok(())
}

#[cfg(target_os = "macos")]
fn uninstall_macos() -> anyhow::Result<()> {
    let path = plist_path()?;
    let _ = Command::new("launchctl")
        .args(["unload", "-w", &path.to_string_lossy()])
        .status();
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    println!("ShareClick login service removed.");
    Ok(())
}

#[cfg(target_os = "windows")]
fn install_windows(exe: &std::path::Path) -> anyhow::Result<()> {
    // Launch `run` at login; the run command hides its own console window.
    let value = format!("\"{}\" run", exe.display());
    let status = Command::new("reg")
        .args([
            "add",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
            "/v",
            "ShareClick",
            "/t",
            "REG_SZ",
            "/d",
            &value,
            "/f",
        ])
        .status()?;
    if !status.success() {
        anyhow::bail!("failed to write the startup registry entry");
    }
    println!("ShareClick installed to start on login (runs in the background, no window).");
    println!("Start it now without logging out:  shareclick run");
    println!("Remove it with:  shareclick uninstall-service");
    Ok(())
}

#[cfg(target_os = "windows")]
fn uninstall_windows() -> anyhow::Result<()> {
    let _ = Command::new("reg")
        .args([
            "delete",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
            "/v",
            "ShareClick",
            "/f",
        ])
        .status();
    println!("ShareClick login entry removed.");
    Ok(())
}
