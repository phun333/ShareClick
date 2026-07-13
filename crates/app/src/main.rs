//! ShareClick — a low-latency, open-source software KVM.
//!
//! Move one keyboard & mouse (and the clipboard, and files) between machines
//! over the LAN with the lowest possible input lag.

mod bench;
mod bulk;
mod config;
mod control;
mod edge;
mod filexfer;
mod transport;

#[cfg(feature = "native")]
mod capture;
#[cfg(feature = "native")]
mod clipboard;
#[cfg(feature = "native")]
mod discovery;
#[cfg(feature = "native")]
mod emit;
#[cfg(feature = "gui")]
mod gui;
#[cfg(feature = "native")]
mod keymap;
#[cfg(feature = "native")]
mod run;
#[cfg(feature = "native")]
mod service;
#[cfg(feature = "tray")]
mod tray;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "shareclick",
    version,
    about = "Low-latency open-source software KVM"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Measure input-channel round-trip latency over loopback.
    Bench {
        /// Number of ping/pong round trips to measure.
        #[arg(short, long, default_value_t = 20_000)]
        count: usize,
        /// Measure with full X25519 + ChaCha20-Poly1305 encryption enabled.
        #[arg(long)]
        encrypted: bool,
    },
    /// Run as the server (the machine whose keyboard & mouse are shared).
    Serve {
        /// Address to bind the input channel to.
        #[arg(long, default_value = "0.0.0.0:24800")]
        bind: String,
    },
    /// Connect to a server as a client (receives input, injects it locally).
    Connect {
        /// Server address, e.g. 192.168.1.20:24800. Omit to use `server_host`
        /// from the config.
        server: Option<String>,
    },
    /// Send a file to a listening peer's bulk channel.
    SendFile {
        /// Target address, e.g. 192.168.1.20:24800
        to: String,
        /// Path to the file to send.
        path: String,
    },
    /// Write a starter config file (settings + monitor manager) you can edit.
    InitConfig {
        /// Where to write it (defaults to the platform config dir).
        #[arg(long)]
        path: Option<String>,
    },
    /// Run in the background using the role saved in the config ("server" or
    /// "client"). This is what the auto-start / background service launches.
    Run,
    /// Zero-config auto-pairing: find the other machine on the LAN and connect
    /// automatically — no IP, no role picking.
    Pair,
    /// Install ShareClick as a login service: auto-starts in the background on
    /// every login, runs the configured role, no terminal, no second app.
    InstallService,
    /// Remove the login service installed by `install-service`.
    UninstallService,
    /// Launch the menu-bar (macOS) / system-tray (Windows) app.
    Tray,
    /// Discover ShareClick servers on the local network via mDNS.
    Discover,
    /// Print the detected screen size (debug).
    ScreenInfo,
    /// Open the visual settings & monitor-arrangement window.
    Settings,
}

/// Hide the console window that Windows opens when the exe is double-clicked,
/// so the tray/menu-bar app doesn't show a stray terminal.
#[cfg(all(windows, feature = "tray"))]
fn hide_console_window() {
    use windows_sys::Win32::System::Console::GetConsoleWindow;
    use windows_sys::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};
    unsafe {
        let hwnd = GetConsoleWindow();
        if !hwnd.is_null() {
            ShowWindow(hwnd, SW_HIDE);
        }
    }
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let cli = Cli::parse();

    // No subcommand (e.g. double-clicked app icon) → launch the GUI tray.
    let command = match cli.command {
        Some(c) => c,
        None => {
            #[cfg(feature = "tray")]
            {
                // Double-clicked with no args: this is the GUI. On Windows the
                // exe is a console app, so hide the console window that pops up
                // (CLI subcommands still keep their console).
                #[cfg(windows)]
                hide_console_window();
                return tray::run();
            }
            #[cfg(not(feature = "tray"))]
            {
                use clap::CommandFactory;
                Cli::command().print_help().ok();
                return Ok(());
            }
        }
    };

    match command {
        Command::Bench { count, encrypted } => bench::run(count, encrypted),
        #[cfg(feature = "native")]
        Command::Serve { bind } => run::serve(&bind),
        #[cfg(feature = "native")]
        Command::Connect { server } => run::connect(server.as_deref()),
        #[cfg(not(feature = "native"))]
        Command::Serve { .. } | Command::Connect { .. } => {
            anyhow::bail!(
                "serve/connect require the `native` feature (build without --no-default-features)"
            )
        }
        #[cfg(feature = "native")]
        Command::Pair => {
            #[cfg(all(windows, feature = "tray"))]
            hide_console_window();
            run::pair()
        }
        #[cfg(not(feature = "native"))]
        Command::Pair => anyhow::bail!("pair requires the `native` feature"),
        #[cfg(feature = "native")]
        Command::Run => {
            #[cfg(all(windows, feature = "tray"))]
            hide_console_window();
            let cfg = config::Config::load(&config::Config::default_path())?;
            match cfg.role.as_deref() {
                Some("server") => {
                    tracing::info!("role = server; serving");
                    run::serve(&format!("0.0.0.0:{}", cfg.port))
                }
                Some("client") => {
                    tracing::info!("role = client; connecting");
                    run::connect(None)
                }
                // No role set → zero-config auto-pair.
                _ => run::pair(),
            }
        }
        #[cfg(not(feature = "native"))]
        Command::Run => anyhow::bail!("run requires the `native` feature"),
        #[cfg(feature = "native")]
        Command::InstallService => service::install(),
        #[cfg(feature = "native")]
        Command::UninstallService => service::uninstall(),
        #[cfg(not(feature = "native"))]
        Command::InstallService | Command::UninstallService => {
            anyhow::bail!("service commands require the `native` feature")
        }
        #[cfg(feature = "native")]
        Command::Discover => {
            let found = discovery::list(std::time::Duration::from_secs(3))?;
            if found.is_empty() {
                println!("no ShareClick servers found on the local network");
            } else {
                for (name, addr, _id) in found {
                    println!("{name}  ->  {addr}");
                }
            }
            Ok(())
        }
        #[cfg(not(feature = "native"))]
        Command::Discover => anyhow::bail!("discover requires the `native` feature"),
        #[cfg(feature = "native")]
        Command::ScreenInfo => {
            let (w, h) = emit::main_display_size()?;
            println!("detected main display: {w} x {h}");
            Ok(())
        }
        #[cfg(not(feature = "native"))]
        Command::ScreenInfo => anyhow::bail!("screen-info requires the `native` feature"),
        #[cfg(feature = "gui")]
        Command::Settings => gui::run(),
        #[cfg(not(feature = "gui"))]
        Command::Settings => {
            anyhow::bail!("settings window not built in; rebuild with `--features gui`")
        }
        #[cfg(feature = "tray")]
        Command::Tray => tray::run(),
        #[cfg(not(feature = "tray"))]
        Command::Tray => {
            anyhow::bail!(
                "tray UI not built in; rebuild with `cargo build --release --features tray`"
            )
        }
        Command::InitConfig { path } => {
            let path = path
                .map(std::path::PathBuf::from)
                .unwrap_or_else(config::Config::default_path);
            if path.exists() {
                anyhow::bail!(
                    "config already exists at {} (refusing to overwrite)",
                    path.display()
                );
            }
            config::Config::example().save(&path)?;
            println!("wrote starter config to {}", path.display());
            println!("edit the [psk] and [[machines]] layout, then run `serve` / `connect`.");
            Ok(())
        }
        Command::SendFile { to, path } => {
            use std::net::ToSocketAddrs;
            let addr = to
                .to_socket_addrs()?
                .next()
                .ok_or_else(|| anyhow::anyhow!("could not resolve {to}"))?;
            filexfer::send_file(addr, std::path::Path::new(&path))
        }
    }
}
