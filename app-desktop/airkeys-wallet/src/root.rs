//! The application's single stateful entity.
//!
//! Everything that is *not* a decision lives here: which screen is up, which
//! row is selected, whether the search field has focus, what the window chrome
//! is doing. The decisions — is a second unlock press allowed, does a cancelled
//! prompt raise an alert, when is the clipboard wiped — are all in
//! `airkeys-core`, and this file cannot reach them except by dispatching an
//! event.
//!
//! ```text
//!   user gesture ─► Intent ─► core Event ─► CoreHost ─► Pending ops
//!                                              ▲            │
//!                                              └── result ◄──┘ DemoShell / timer
//! ```
//!
//! When a rule feels like it wants to live here, that is the signal it belongs
//! in the core: three other clients need it too.

use std::collections::VecDeque;
use std::rc::Rc;
use std::time::Duration;

use gpui::{
    AsyncApp, Context, Div, Entity, FocusHandle, InteractiveElement as _, IntoElement,
    KeyDownEvent, MouseButton, ParentElement, Render, SharedString, Stateful,
    StatefulInteractiveElement as _, Styled, Tiling, WeakEntity, Window, WindowControlArea, div,
    px,
};

use airkeys_core::app::PromptKind;
use airkeys_core::app::item_list::{Event as ItemListEvent, ItemList};
use airkeys_core::app::shell::{CopyField, ShellOperation, ShellResult};
use airkeys_core::app::unlock::{Event as UnlockEvent, Unlock};
use airkeys_core::types::Item;

use crate::loc::Loc;
use crate::screens::{Note, ScreenCtx, Sink, unlock as unlock_screen, vault as vault_screen};
use crate::shell_host::{CoreHost, DemoShell, Pending, Performed};
use crate::strings::Strings;
use crate::theme::{Theme, ThemeMode};
use crate::ui::StatusTone;
use crate::window_frame::{FRAME_SHADOW, frame_tiling, round_to_frame, window_frame};

/// Height of the draggable strip under client-side decorations: the empty band
/// above the content, so the strip never covers anything interactive.
const DRAG_STRIP_H: f32 = 36.;

/// Which machine is driving the window.
enum Screen {
    Unlock(CoreHost<Unlock>),
    Vault(CoreHost<ItemList>),
}

pub struct Root {
    mode: ThemeMode,
    loc: Loc,
    strings: Strings,
    shell: DemoShell,
    screen: Screen,

    /// A prompt the core asked for, or an error the screen should show.
    note: Option<Note>,
    /// Presentation state the machines have no opinion about.
    nav: vault_screen::Nav,
    selected_id: Option<String>,
    search_focused: bool,
    vault_label: SharedString,

    focus_handle: FocusHandle,
    /// True between a press on the drag strip and the first movement after it.
    /// Moving on the *move*, not the press, is what keeps double-click
    /// detection alive.
    drag_pending: bool,
}

impl Root {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let loc = Loc::from_env();
        eprintln!("[airkeys] locale resolved to `{}`", loc.language());
        let strings = Strings::resolve(&loc);
        let shell = DemoShell::from_env();
        eprintln!("[airkeys] demo scenario: {:?}", shell.scenario());

        // Restyle when the OS appearance flips, unless AIRKEYS_THEME pins it.
        let weak = cx.weak_entity();
        window
            .observe_window_appearance(move |window, cx| {
                if ThemeMode::is_pinned() {
                    return;
                }
                let mode = ThemeMode::detect(window);
                if let Some(root) = weak.upgrade() {
                    root.update(cx, |this, cx| {
                        this.mode = mode;
                        cx.notify();
                    });
                }
            })
            .detach();

        let focus_handle = cx.focus_handle();
        focus_handle.focus(window, cx);

        let mut root = Self {
            mode: ThemeMode::detect(window),
            loc,
            strings,
            shell,
            screen: Screen::Unlock(CoreHost::new()),
            note: None,
            nav: vault_screen::Nav::Items,
            selected_id: None,
            search_focused: true,
            vault_label: SharedString::default(),
            focus_handle,
            drag_pending: false,
        };

        // Mounting the unlock screen reads the vault metadata — and nothing
        // else. No OS sheet appears until the user asks for one.
        let ops = match &mut root.screen {
            Screen::Unlock(host) => host.dispatch(UnlockEvent::Start),
            Screen::Vault(_) => Vec::new(),
        };
        root.pump_unlock(ops.into(), cx);
        root
    }

    // -- the pump -----------------------------------------------------------

    /// Drain the unlock machine's operations until nothing is left to do
    /// synchronously.
    fn pump_unlock(&mut self, mut queue: VecDeque<Pending>, cx: &mut Context<Self>) {
        while let Some(pending) = queue.pop_front() {
            match &pending.operation {
                ShellOperation::Prompt { kind, .. } => {
                    self.note = Some(self.note_for(kind));
                    // The prompt is answered immediately with "not accepted":
                    // none of the unlock prompts is a question yet. When one
                    // becomes confirmable, this is where the dialog goes.
                    if let Screen::Unlock(host) = &mut self.screen {
                        queue.extend(
                            host.resolve(
                                pending.id,
                                ShellResult::PromptAnswered { accepted: false },
                            ),
                        );
                    }
                }
                ShellOperation::CompleteUnlock { vault } => {
                    let vault = vault.clone();
                    if let Screen::Unlock(host) = &mut self.screen {
                        host.resolve(pending.id, ShellResult::UnlockCompleted);
                    }
                    // Anything still queued belonged to the screen we are
                    // leaving; the vault screen starts from its own Start.
                    self.enter_vault(vault, cx);
                    return;
                }
                operation => match self.shell.perform(operation) {
                    Performed::Now(result) => {
                        if let Screen::Unlock(host) = &mut self.screen {
                            queue.extend(host.resolve(pending.id, result));
                        }
                    }
                    Performed::After(delay, result) => {
                        self.schedule(pending.id, delay, result, true, cx);
                    }
                    Performed::Screen => {}
                },
            }
        }
        cx.notify();
    }

    /// The same drain for the item-list machine.
    fn pump_vault(&mut self, mut queue: VecDeque<Pending>, cx: &mut Context<Self>) {
        while let Some(pending) = queue.pop_front() {
            match &pending.operation {
                ShellOperation::Prompt { kind, .. } => {
                    self.note = Some(self.note_for(kind));
                    if let Screen::Vault(host) = &mut self.screen {
                        queue.extend(
                            host.resolve(
                                pending.id,
                                ShellResult::PromptAnswered { accepted: false },
                            ),
                        );
                    }
                }
                operation => match self.shell.perform(operation) {
                    Performed::Now(result) => {
                        if let Screen::Vault(host) = &mut self.screen {
                            queue.extend(host.resolve(pending.id, result));
                        }
                    }
                    Performed::After(delay, result) => {
                        self.schedule(pending.id, delay, result, false, cx);
                    }
                    Performed::Screen => {}
                },
            }
        }
        cx.notify();
    }

    /// Answer an operation later.
    ///
    /// The core is never told that time passed — it asked for a `Wait` and gets
    /// a `Waited`, exactly as the test driver hands it one instantly. That
    /// symmetry is why the elapsed-counter rule can be tested in microseconds
    /// and still be the code that runs here.
    fn schedule(
        &mut self,
        id: u64,
        delay: Duration,
        result: ShellResult,
        for_unlock: bool,
        cx: &mut Context<Self>,
    ) {
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            cx.background_executor().timer(delay).await;
            this.update(cx, |this, cx| {
                // The screen may have changed while the timer ran. Resolving
                // into the wrong machine is exactly the "answer outlived the
                // question" case, and the id simply will not be found.
                match (&mut this.screen, for_unlock) {
                    (Screen::Unlock(host), true) => {
                        let next = host.resolve(id, result);
                        this.pump_unlock(next.into(), cx);
                    }
                    (Screen::Vault(host), false) => {
                        let next = host.resolve(id, result);
                        this.pump_vault(next.into(), cx);
                    }
                    _ => {}
                }
            })
            .ok();
        })
        .detach();
    }

    // -- screen transitions --------------------------------------------------

    fn enter_vault(&mut self, vault: airkeys_core::types::VaultMeta, cx: &mut Context<Self>) {
        self.vault_label = SharedString::from(vault.label.clone());
        self.note = None;
        self.selected_id = None;
        let mut host = CoreHost::<ItemList>::new();
        let ops = host.dispatch(ItemListEvent::Start {
            vault_id: vault.id.clone(),
        });
        self.screen = Screen::Vault(host);
        self.pump_vault(ops.into(), cx);
    }

    fn lock(&mut self, cx: &mut Context<Self>) {
        self.note = None;
        self.selected_id = None;
        let mut host = CoreHost::<Unlock>::new();
        let ops = host.dispatch(UnlockEvent::Start);
        self.screen = Screen::Unlock(host);
        self.pump_unlock(ops.into(), cx);
    }

    // -- intents -------------------------------------------------------------

    fn on_unlock_intent(&mut self, intent: unlock_screen::Intent, cx: &mut Context<Self>) {
        match intent {
            unlock_screen::Intent::Unlock => {
                self.note = None;
                let ops = match &mut self.screen {
                    Screen::Unlock(host) => host.dispatch(UnlockEvent::Unlock),
                    Screen::Vault(_) => Vec::new(),
                };
                self.pump_unlock(ops.into(), cx);
            }
            unlock_screen::Intent::CreateVault => {
                // Vault creation is the next feature; recording the press keeps
                // the button honest rather than silently doing nothing.
                eprintln!("[airkeys] create-vault pressed (not implemented yet)");
            }
            unlock_screen::Intent::DismissNote => {
                self.note = None;
                cx.notify();
            }
        }
    }

    fn on_vault_intent(&mut self, intent: vault_screen::Intent, cx: &mut Context<Self>) {
        let event = match intent {
            vault_screen::Intent::Select(id) => {
                self.selected_id = Some(id);
                cx.notify();
                return;
            }
            vault_screen::Intent::Lock => return self.lock(cx),
            vault_screen::Intent::DismissNote => {
                self.note = None;
                cx.notify();
                return;
            }
            vault_screen::Intent::ToggleFavorite(item_id) => {
                ItemListEvent::FavoriteToggled { item_id }
            }
            vault_screen::Intent::ToggleFavoritesOnly => {
                self.nav = match self.nav {
                    vault_screen::Nav::Favorites => vault_screen::Nav::Items,
                    _ => vault_screen::Nav::Favorites,
                };
                ItemListEvent::FavoritesOnlyToggled
            }
            vault_screen::Intent::CopyAccount(item_id) => ItemListEvent::CopyRequested {
                item_id,
                field: CopyField::Account,
            },
            vault_screen::Intent::CopySecret(item_id) => ItemListEvent::CopyRequested {
                item_id,
                field: CopyField::Secret,
            },
        };

        let ops = match &mut self.screen {
            Screen::Vault(host) => host.dispatch(event),
            Screen::Unlock(_) => Vec::new(),
        };
        self.pump_vault(ops.into(), cx);
    }

    /// Typing in the search field.
    ///
    /// The query lives in the machine's model, not in a widget: a text input
    /// that owned its own buffer would be a second copy of the query outside
    /// the state machine, and the two would disagree the first time the machine
    /// cleared it.
    fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let keystroke = &event.keystroke;

        // Fullscreen keeps a keyboard exit on every platform: F11 everywhere,
        // plus the native ⌃⌘F chord on macOS. This matters most on a second
        // display, where the titlebar reveal is the only pointer path back out.
        let macos_chord = cfg!(target_os = "macos")
            && keystroke.key == "f"
            && keystroke.modifiers.control
            && keystroke.modifiers.platform;
        if keystroke.key == "f11" || macos_chord {
            window.toggle_fullscreen();
            return;
        }

        let Screen::Vault(host) = &mut self.screen else {
            return;
        };
        if !self.search_focused || keystroke.modifiers.platform || keystroke.modifiers.control {
            return;
        }

        let query = host.view().query;
        let next = if keystroke.key == "backspace" {
            let mut query = query;
            query.pop();
            Some(query)
        } else if keystroke.key == "escape" {
            Some(String::new())
        } else {
            // `key_char`, not `key`: on a layout without ASCII keys `key` is the
            // ASCII-equivalent and `key_char` is what the user actually typed.
            keystroke.key_char.as_ref().map(|typed| {
                let mut query = query;
                query.push_str(typed);
                query
            })
        };

        let Some(query) = next else { return };
        let ops = host.dispatch(ItemListEvent::QueryChanged { query });
        self.pump_vault(ops.into(), cx);
    }

    // -- prompt copy ---------------------------------------------------------

    /// The one place a `PromptKind` becomes words. Exhaustive over the core's
    /// enum, so a new prompt is a compile error here rather than a silent blank
    /// banner.
    fn note_for(&self, kind: &PromptKind) -> Note {
        let (tone, title, body) = match kind {
            PromptKind::Unsupported => (
                StatusTone::Warning,
                self.loc.t("unlock.unsupportedTitle"),
                Some(self.loc.t("unlock.unsupportedBody")),
            ),
            PromptKind::LockedOut => (
                StatusTone::Warning,
                self.loc.t("unlock.failedTitle"),
                Some(self.loc.t("unlock.failedBody")),
            ),
            PromptKind::VaultMissing => (
                StatusTone::Info,
                self.loc.t("unlock.missingTitle"),
                Some(self.loc.t("unlock.missingBody")),
            ),
            PromptKind::UnlockFailed { detail } => (
                StatusTone::Error,
                self.loc.t("unlock.failedTitle"),
                Some(self.loc.t_vars("unlock.failedBody", &[("detail", detail)])),
            ),
            PromptKind::StorageFailed { detail } => (
                StatusTone::Error,
                self.loc.t("unlock.failedTitle"),
                Some(self.loc.t_vars("unlock.failedBody", &[("detail", detail)])),
            ),
        };
        Note { tone, title, body }
    }

    // -- window chrome -------------------------------------------------------

    fn drag_strip(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        div()
            .id("drag-strip")
            .absolute()
            .top_0()
            .left_0()
            .w_full()
            .h(px(DRAG_STRIP_H))
            .window_control_area(WindowControlArea::Drag)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, _| this.drag_pending = true),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, _| this.drag_pending = false),
            )
            .on_mouse_down_out(cx.listener(|this, _, _, _| this.drag_pending = false))
            // Press-then-move is a drag: hand the window to the compositor.
            .on_mouse_move(cx.listener(|this, _, window, _| {
                if this.drag_pending {
                    this.drag_pending = false;
                    window.start_window_move();
                }
            }))
            .on_click(|event, window, _| {
                if event.click_count() == 2 {
                    window.zoom_window();
                }
            })
            .on_mouse_down(MouseButton::Right, |event, window, _| {
                window.show_window_menu(event.position);
            })
    }

    /// Native-style controls for custom-decorated Linux and Windows windows.
    /// The `WindowControlArea` bindings preserve Windows 11's maximize hover
    /// affordance; the click handlers provide the same behaviour on GNOME.
    fn window_controls(&self, window: &Window) -> Stateful<Div> {
        let theme = Theme::of(self.mode);
        let control = |id: &'static str, glyph: &'static str, area: WindowControlArea| {
            div()
                .id(id)
                .w(px(46.))
                .h(px(DRAG_STRIP_H))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .text_size(px(16.))
                .text_color(theme.fg_base)
                .window_control_area(area)
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(glyph)
        };

        let minimize = control("window-minimize", "−", WindowControlArea::Min)
            .hover(|el| el.bg(theme.bg_sunken))
            .on_click(|_, window, cx| {
                cx.stop_propagation();
                window.minimize_window();
            });
        let maximize_glyph = if window.is_maximized() { "❐" } else { "□" };
        let maximize = control("window-maximize", maximize_glyph, WindowControlArea::Max)
            .hover(|el| el.bg(theme.bg_sunken))
            .on_click(|_, window, cx| {
                cx.stop_propagation();
                window.zoom_window();
            });
        let close = control("window-close", "×", WindowControlArea::Close)
            .text_size(px(20.))
            .hover(|el| el.bg(theme.error).text_color(theme.fg_inverse))
            .on_click(|_, window, cx| {
                cx.stop_propagation();
                window.remove_window();
            });

        div()
            .id("window-controls")
            .absolute()
            .top_0()
            .right_0()
            .flex()
            .flex_row()
            .children([minimize, maximize, close])
    }

    fn titlebar(&self, window: &Window, cx: &mut Context<Self>) -> Stateful<Div> {
        div()
            .id("titlebar")
            .absolute()
            .top_0()
            .left_0()
            .w_full()
            .h(px(DRAG_STRIP_H))
            .child(self.drag_strip(cx))
            .child(self.window_controls(window))
    }

    fn selected_item(&self, view: &airkeys_core::app::item_list::ItemListView) -> Option<Item> {
        let id = self.selected_id.as_deref()?;
        view.visible.iter().find(|item| item.id == id).cloned()
    }
}

impl Render for Root {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::of(self.mode);
        let tiling = frame_tiling(window);
        let entity: Entity<Self> = cx.entity();

        let ctx = ScreenCtx {
            theme: &theme,
            strings: &self.strings,
            loc: &self.loc,
        };

        let content = match &self.screen {
            Screen::Unlock(host) => {
                let entity = entity.clone();
                let sink: Sink<unlock_screen::Intent> = Rc::new(move |intent, _window, cx| {
                    entity.update(cx, |this, cx| this.on_unlock_intent(intent, cx));
                });
                unlock_screen::render(&host.view(), self.note.as_ref(), &ctx, sink)
            }
            Screen::Vault(host) => {
                let view = host.view();
                let selected = self.selected_item(&view);
                // The strength verdict is computed from a secret the shell
                // holds; only the verdict crosses back.
                let strength = selected
                    .as_ref()
                    .and_then(|item| self.shell.strength_of(&item.id));
                let entity = entity.clone();
                let sink: Sink<vault_screen::Intent> = Rc::new(move |intent, _window, cx| {
                    entity.update(cx, |this, cx| this.on_vault_intent(intent, cx));
                });
                let chrome = vault_screen::VaultChrome {
                    nav: self.nav,
                    selected: selected.as_ref(),
                    strength: strength.as_ref(),
                    search_focused: self.search_focused,
                    note: self.note.as_ref(),
                    vault_label: self.vault_label.clone(),
                };
                vault_screen::render(&view, &chrome, &ctx, sink)
            }
        };

        let root = div()
            .size_full()
            .relative()
            .bg(theme.bg_base)
            .child(content);
        let root = match tiling {
            Some(tiling) => round_to_frame(root, tiling),
            None => root,
        };

        let root = root.track_focus(&self.focus_handle).on_key_down(
            cx.listener(|this, event: &KeyDownEvent, window, cx| this.on_key(event, window, cx)),
        );

        // Client-side Linux decorations need their own titlebar. Windows also
        // hides the system-drawn one (`appears_transparent` in main.rs), so it
        // needs the same otherwise-empty strip, registered as the native caption
        // hit-test area. macOS keeps its AppKit titlebar.
        let owns_titlebar = tiling.is_some() || cfg!(target_os = "windows");
        let root = if owns_titlebar {
            root.child(self.titlebar(window, cx))
        } else {
            root
        };

        window_frame(root, &theme, window)
    }
}

/// The viewport width minus the client-decoration shadow band, for anything
/// that must centre on the *content* rather than the surface.
#[allow(dead_code)]
pub fn content_width(window: &Window, tiling: Option<Tiling>) -> f32 {
    let mut width = f32::from(window.viewport_size().width);
    if let Some(tiling) = tiling {
        if !tiling.left {
            width -= f32::from(FRAME_SHADOW);
        }
        if !tiling.right {
            width -= f32::from(FRAME_SHADOW);
        }
    }
    width
}
