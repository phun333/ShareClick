//! Build script: on Windows, embed the ShareClick icon into the `.exe` so it
//! shows in Explorer, the taskbar and the tray. No-op on other platforms.

fn main() {
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("shareclick.ico");
        // Ignore errors so a missing toolchain never blocks the build.
        let _ = res.compile();
    }
}
