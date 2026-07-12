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
    let item_start = MenuItem::new("Start (find & connect the other PC)", true, None);
    let item_settings = MenuItem::new("Settings & Monitor Manager…", true, None);
    let item_quit = MenuItem::new("Quit ShareClick", true, None);

    let id_start = item_start.id().clone();
    let id_settings = item_settings.id().clone();
    let id_quit = item_quit.id().clone();

    let menu = Menu::new();
    menu.append(&item_status)?;
    menu.append(&PredefinedMenuItem::separator())?;
    menu.append(&item_start)?;
    menu.append(&PredefinedMenuItem::separator())?;
    menu.append(&item_settings)?;
    menu.append(&PredefinedMenuItem::separator())?;
    menu.append(&item_quit)?;

    // Behave like a background service: if the config already says which side
    // this machine is, start it automatically so the user doesn't have to click
    // Start every login. Without a role, wait for a menu choice.
    // One button, one behaviour: auto-pair (role in the config only decides who
    // listens; control is symmetric either way). Starts immediately on launch.
    item_status.set_text("ShareClick — pairing…");
    spawn_pair();

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
                } else if ev.id == id_start {
                    item_status.set_text("ShareClick — pairing…");
                    spawn_pair();
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

/// Zero-config auto-pairing in the background (find the peer + connect).
fn spawn_pair() {
    std::thread::spawn(|| {
        if let Err(e) = crate::run::pair() {
            eprintln!("pairing error: {e}");
        }
    });
}

/// Open the visual settings window (a separate `shareclick settings` process,
/// so it has its own event loop). Falls back to opening the config file.
fn open_settings(path: &PathBuf) {
    if let Ok(exe) = std::env::current_exe() {
        if std::process::Command::new(&exe).arg("settings").spawn().is_ok() {
            return;
        }
    }
    // Fallback: create + open the raw config file.
    if !path.exists() {
        let _ = Config::example().save(path);
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

/// The ShareClick brand icon (a blue cursor-click glyph) — pre-rendered to raw
/// 64×64 RGBA and embedded so we ship no image files or SVG renderer.
fn brand_icon() -> Icon {
    const S: u32 = 64;
    let rgba = include_bytes!("tray_icon_64.rgba").to_vec();
    Icon::from_rgba(rgba, S, S).expect("valid rgba icon")
}
