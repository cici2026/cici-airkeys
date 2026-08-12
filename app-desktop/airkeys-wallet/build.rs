//! Embeds the Windows application icon. No-op on every other platform.
//!
//! gpui does not take an icon from `WindowOptions`; on Windows it calls
//! `LoadImageW(module, MAKEINTRESOURCE(1), IMAGE_ICON, ..)` against the running
//! executable's own resources, and the call site is
//! `load_icon().unwrap_or_default()`. A miss is therefore **silent**: the window
//! class registers a null HICON and Windows substitutes its generic application
//! icon in the title bar, the taskbar and Alt-Tab — and the Start-menu and
//! desktop shortcuts inherit the same nothing, because the installer points them
//! at this executable.
//!
//! So the icon must exist AND must be resource id 1. `set_icon_with_id` states
//! that explicitly rather than relying on the default numbering.

use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // CARGO_CFG_TARGET_OS, not #[cfg(windows)]: the latter describes the machine
    // running this build script, which is the wrong answer when cross-compiling
    // to Windows from Linux or macOS.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let icon = "packaging/icons/app.airkeys.Airkeys.ico";
    println!("cargo:rerun-if-changed={icon}");

    // The icon is not in the repo yet. Warn rather than panic while the design
    // is still being drawn — but say exactly what the consequence is, because
    // "the taskbar icon is generic" is otherwise a mystery six months later.
    if !Path::new(icon).exists() {
        println!(
            "cargo:warning=no {icon} — the Windows build will show the generic \
             application icon in the title bar, taskbar and Alt-Tab. Add the \
             .ico before shipping an installer."
        );
        return;
    }

    let mut res = winresource::WindowsResource::new();
    res.set_icon_with_id(icon, "1");
    if let Err(error) = res.compile() {
        // Failing loudly here beats shipping an installer whose every shortcut
        // shows a blank icon.
        panic!("failed to embed the Windows application icon from {icon}: {error}");
    }
}
