//! Airkeys desktop entry — window management only.
//!
//! Everything below this file is layered: `theme` owns colour and geometry,
//! `loc`/`strings` own words, `ui` owns visuals, `screens` own composition,
//! `root` owns state and interaction, `shell_host` owns side effects, and
//! `airkeys-core` owns the decisions. This file owns the window.

#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

mod gallery;
mod loc;
mod root;
mod screens;
mod shell_host;
mod strings;
mod theme;
mod ui;
mod window_frame;

use gpui::{
    App, AppContext as _, Bounds, KeyBinding, Menu, MenuItem, QuitMode, TitlebarOptions,
    WindowBounds, WindowOptions, actions, point, px, size,
};

use gallery::Gallery;
use root::Root;
use theme::{WINDOW_H, WINDOW_W};

// TODO(i18n): menu labels are English-only until the corpus grows menu keys.
actions!(
    airkeys,
    [Quit, HideApp, HideOthers, ShowAll, ToggleFullScreen]
);

fn open_main_window(cx: &mut App) {
    if gallery::gallery_enabled() {
        open_window_with(cx, |window, cx| cx.new(|cx| Gallery::new(window, cx)));
        return;
    }
    open_window_with(cx, |window, cx| cx.new(|cx| Root::new(window, cx)));
}

fn open_window_with<V: gpui::Render + 'static>(
    cx: &mut App,
    build: impl FnOnce(&mut gpui::Window, &mut App) -> gpui::Entity<V>,
) {
    let bounds = Bounds::centered(None, size(px(WINDOW_W), px(WINDOW_H)), cx);

    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            // The three-column layout does not reflow below the design size, so
            // the design size is the minimum.
            window_min_size: Some(size(px(WINDOW_W), px(WINDOW_H))),
            // Sets the Wayland `xdg_toplevel.app_id` and the X11 `WM_CLASS`.
            // Both are how a desktop shell matches a running window to its
            // installed `.desktop` file, so this string, the packaging file name
            // and the `StartupWMClass` inside it must stay identical —
            // otherwise GNOME shows the app with a generic icon and the binary
            // name.
            app_id: Some("app.airkeys.Airkeys".into()),
            titlebar: Some(TitlebarOptions {
                title: Some("Airkeys".into()),
                // Content owns the full canvas; only the traffic lights remain,
                // inset to clear the sidebar header.
                appears_transparent: true,
                traffic_light_position: Some(point(px(16.), px(16.))),
            }),
            ..Default::default()
        },
        build,
    )
    .expect("failed to open the main window");
}

fn main() {
    // LastWindowClosed rather than gpui's macOS default (keep running).
    //
    // A keep-running single-window app must be able to reopen its window from
    // the Dock icon, and that path is unreliable on recent macOS: AppKit's
    // TextInputUI panel stays `isVisible` after the last real window closes, so
    // the system reports `hasVisibleWindows = YES` on a Dock click and gpui's
    // reopen gate swallows the event before any `on_reopen` callback runs. The
    // app would then be running with no window and no way back to one.
    //
    // Closing the window therefore quits — the same behaviour gpui already
    // defaults to on Windows and Linux.
    let app = gpui_platform::application().with_quit_mode(QuitMode::LastWindowClosed);

    // Insurance, not the fix: unreachable while LastWindowClosed holds, but if a
    // future gpui or quit-mode change ever leaves the app running with no
    // window, a Dock click should build one rather than nothing.
    app.on_reopen(|cx| {
        if cx.windows().is_empty() {
            open_main_window(cx);
        }
    });

    app.run(|cx: &mut App| {
        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.on_action(|_: &HideApp, cx| cx.hide());
        cx.on_action(|_: &HideOthers, cx| cx.hide_other_apps());
        cx.on_action(|_: &ShowAll, cx| cx.unhide_other_apps());
        cx.on_action(|_: &ToggleFullScreen, cx| {
            if let Some(window) = cx.active_window() {
                window
                    .update(cx, |_, window, _| window.toggle_fullscreen())
                    .ok();
            }
        });

        // The main menu is not just convention on macOS: with a nil
        // `NSApp.mainMenu` the menu-bar/titlebar reveal never engages for a
        // fullscreen Space on a secondary display, so a fullscreened window
        // there shows no titlebar and offers no pointer path back out. gpui
        // installs no menu of its own.
        if cfg!(target_os = "macos") {
            cx.bind_keys([
                KeyBinding::new("cmd-q", Quit, None),
                KeyBinding::new("cmd-h", HideApp, None),
                KeyBinding::new("alt-cmd-h", HideOthers, None),
                KeyBinding::new("ctrl-cmd-f", ToggleFullScreen, None),
            ]);
            cx.set_menus(vec![
                // macOS titles the first menu with the bundle name, not this
                // string.
                Menu::new("Airkeys").items(vec![
                    MenuItem::action("Hide Airkeys", HideApp),
                    MenuItem::action("Hide Others", HideOthers),
                    MenuItem::action("Show All", ShowAll),
                    MenuItem::separator(),
                    MenuItem::action("Quit Airkeys", Quit),
                ]),
                Menu::new("View").items(vec![MenuItem::action(
                    "Toggle Full Screen",
                    ToggleFullScreen,
                )]),
            ]);
        }

        open_main_window(cx);
        cx.activate(true);
    });
}
