//! Menu-bar / system-tray front-end.
//!
//! * **macOS:** a status item in the top-right menu bar (no dock icon — the app
//!   runs as an "accessory").
//! * **Windows:** a system-tray icon.
//!
//! The menu lets you start the server or client, jump to the settings +
//! monitor-manager file, and quit. The heavy GUI event loop lives here and on
//! macOS must own the main thread, so the actual server/client run in spawned
//! threads.

#![cfg(feature = "tray")]

use std::path::PathBuf;

use tao::event::Event;
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIconBuilder};

use crate::config::Config;

/// Messages pumped into the tao event loop.
enum UserEvent {
    Menu(MenuEvent),
}

/// Launch the tray/menu-bar app. Blocks running the event loop.
pub fn run() -> anyhow::Result<()> {
    let config_path = Config::default_path();

    let event_loop = build_event_loop();

    // Forward menu events into the loop so it wakes without busy-polling.
    let proxy = event_loop.create_proxy();
    MenuEvent::set_event_handler(Some(move |event| {
        let _ = proxy.send_event(UserEvent::Menu(event));
    }));

    // Menu items — ids captured so we can match clicks.
    let item_status = MenuItem::new("ShareClick — idle", false, None);
    let item_serve = MenuItem::new("Start Server (share this Mac/PC)", true, None);
    let item_connect = MenuItem::new("Start Client (control from here)", true, None);
    let item_settings = MenuItem::new("Settings & Monitor Manager…", true, None);
    let item_quit = MenuItem::new("Quit ShareClick", true, None);

    let id_serve = item_serve.id().clone();
    let id_connect = item_connect.id().clone();
    let id_settings = item_settings.id().clone();
    let id_quit = item_quit.id().clone();

    let menu = Menu::new();
    menu.append(&item_status)?;
    menu.append(&PredefinedMenuItem::separator())?;
    menu.append(&item_serve)?;
    menu.append(&item_connect)?;
    menu.append(&PredefinedMenuItem::separator())?;
    menu.append(&item_settings)?;
    menu.append(&PredefinedMenuItem::separator())?;
    menu.append(&item_quit)?;

    // The tray icon must be created after the loop starts on macOS, so we build
    // it lazily on the first `Init` event and keep it alive here (RAII).
    let mut _tray = None;
    let menu_holder = menu;

    event_loop.run(move |event, _target, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::NewEvents(tao::event::StartCause::Init) => {
                let icon = brand_icon();
                match TrayIconBuilder::new()
                    .with_tooltip("ShareClick — low-latency KVM")
                    .with_menu(Box::new(menu_holder.clone()))
                    .with_icon(icon)
                    .build()
                {
                    Ok(t) => _tray = Some(t),
                    Err(e) => {
                        eprintln!("failed to create tray icon: {e}");
                        *control_flow = ControlFlow::Exit;
                    }
                }
            }
            Event::UserEvent(UserEvent::Menu(ev)) => {
                if ev.id == id_quit {
                    *control_flow = ControlFlow::Exit;
                } else if ev.id == id_serve {
                    item_status.set_text("ShareClick — serving");
                    spawn_serve();
                } else if ev.id == id_connect {
                    item_status.set_text("ShareClick — client");
                    spawn_connect();
                } else if ev.id == id_settings {
                    open_settings(&config_path);
                }
            }
            _ => {}
        }
    });
}

#[cfg(target_os = "macos")]
fn build_event_loop() -> tao::event_loop::EventLoop<UserEvent> {
    use tao::platform::macos::{ActivationPolicy, EventLoopExtMacOS};
    let mut event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    // Accessory = menu-bar app with no dock icon.
    event_loop.set_activation_policy(ActivationPolicy::Accessory);
    event_loop
}

#[cfg(not(target_os = "macos"))]
fn build_event_loop() -> tao::event_loop::EventLoop<UserEvent> {
    EventLoopBuilder::<UserEvent>::with_user_event().build()
}

/// Start the server in a background thread, reading the address from config.
fn spawn_serve() {
    std::thread::spawn(|| {
        let bind = format!(
            "0.0.0.0:{}",
            Config::load(&Config::default_path()).map(|c| c.port).unwrap_or(24800)
        );
        if let Err(e) = crate::run::serve(&bind) {
            eprintln!("server error: {e}");
        }
    });
}

/// Start the client, dialing the `server_host` from the config.
fn spawn_connect() {
    std::thread::spawn(|| {
        // `connect(None)` reads `server_host` from the config itself.
        if let Err(e) = crate::run::connect(None) {
            eprintln!("client error: {e} (set `server_host` in Settings)");
        }
    });
}

/// Reveal / open the settings file (creating a starter if missing).
fn open_settings(path: &PathBuf) {
    if !path.exists() {
        if let Err(e) = Config::example().save(path) {
            eprintln!("could not create config: {e}");
            return;
        }
    }
    let _ = open_path(path);
}

#[cfg(target_os = "macos")]
fn open_path(path: &PathBuf) -> std::io::Result<()> {
    std::process::Command::new("open").arg(path).spawn().map(|_| ())
}

#[cfg(target_os = "windows")]
fn open_path(path: &PathBuf) -> std::io::Result<()> {
    std::process::Command::new("explorer").arg(path).spawn().map(|_| ())
}

#[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
fn open_path(path: &PathBuf) -> std::io::Result<()> {
    std::process::Command::new("xdg-open").arg(path).spawn().map(|_| ())
}

/// A simple 32×32 brand icon (rounded blue square) generated in code so we do
/// not need to ship an image asset.
fn brand_icon() -> Icon {
    const S: u32 = 32;
    let mut rgba = Vec::with_capacity((S * S * 4) as usize);
    for y in 0..S {
        for x in 0..S {
            let (cx, cy) = (S as i32 / 2, S as i32 / 2);
            let d = (x as i32 - cx).abs().max((y as i32 - cy).abs());
            if d < 14 {
                rgba.extend_from_slice(&[0x2b, 0x7a, 0xff, 0xff]); // blue fill
            } else {
                rgba.extend_from_slice(&[0, 0, 0, 0]); // transparent
            }
        }
    }
    Icon::from_rgba(rgba, S, S).expect("valid rgba icon")
}
