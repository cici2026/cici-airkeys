# Airkeys — Desktop

The desktop client, built on [gpui](https://github.com/zed-industries/zed) (Zed's
UI framework). This crate is **standalone** — deliberately not a member of the
`rust/` workspace, so `cargo test -p airkeys-core` stays fast enough to run on
every save instead of dragging a Vulkan/Wayland/fontconfig tree behind it. All
commands below run from this directory.

---

## Run it

```bash
cargo run
```

First build pulls gpui from git and takes a while; after that it is quick.

### Environment pins

The app follows the system appearance and the system locale by default. Both can
be pinned without touching system settings, and two more switches exist for
reaching states that are otherwise hard to produce on purpose.

| Variable | Effect |
|---|---|
| `AIRKEYS_THEME` | `dark` / `light`. Pinning it also **stops** the live restyle on OS appearance change |
| `AIRKEYS_LANG` | First entry in the chain `AIRKEYS_LANG` → `LC_ALL` → `LC_MESSAGES` → `LANG`. Unrecognised tags fall back to `en` |
| `AIRKEYS_DEMO` | `no-vault` / `cancel` / `unsupported` / `locked-out` / `storage-error`. Which fixture state the shell starts in |
| `AIRKEYS_GALLERY=1` | Opens the state gallery instead of the app |

```bash
AIRKEYS_THEME=dark AIRKEYS_LANG=zh cargo run      # dark, Chinese
AIRKEYS_DEMO=locked-out cargo run                 # the authenticator refuses
AIRKEYS_GALLERY=1 cargo run                       # every screen state, listed
```

### The state gallery

`AIRKEYS_GALLERY=1` lists every screen state — including the ones that are
nearly impossible to reach by hand: the authenticator locked out, a read that
failed (which must **not** look like an empty vault), a nine-second wait on the
biometric sheet.

It works because screens are pure functions of a view model, so a state is
something you *write down* rather than something you have to *produce*. Arrow
keys or `j`/`k` step through the list.

---

## Layers

Each layer may only reach downwards. The rule earns its keep at review time:
"which layer is this change in?" answers most questions about whether it belongs.

| Module | Owns | Must not |
|---|---|---|
| [main.rs](src/main.rs) | the window, menus, quit behaviour | know anything about the app |
| [root.rs](src/root.rs) | state, interaction, screen switching | contain a business rule |
| [screens/](src/screens/) | composition — view model in, element out | own state, read a clock, call a machine |
| [ui/](src/ui/) | visuals | resolve a translation, know a page |
| [strings.rs](src/strings.rs) | which keys exist, resolved once per locale | — |
| [loc.rs](src/loc.rs) | **the only module that touches `airkeys_core::i18n`** | — |
| [theme.rs](src/theme.rs) | **the only module allowed to name a colour or a magic number** | — |
| [shell_host.rs](src/shell_host.rs) | storage, timers, clipboard, biometrics, plaintext | decide anything |
| `airkeys-core` | every decision | perform anything |

Two of those are absolutes, and both are load-bearing:

- **`theme.rs` is the only module that may write a colour or a layout number.**
  Once a component can write `rgb(0x4f46c9)`, the design system exists in the
  documentation and nowhere else: dark mode becomes a grep that misses three
  places, and the accessibility floor becomes unverifiable because nothing knows
  what the pairs are. `contrast_floor_holds_in_both_themes` can only exist
  because the pairs are all in one file.
- **`loc.rs` is the only module that may call `t()`.** A component that can
  resolve a key is a component that can invent one, and an invented key is a
  string that exists in exactly one language.

### Where a change goes

A rule about *what should happen* — when the clipboard is wiped, whether a
cancel is an error, whether a second press starts a second ceremony — belongs in
`airkeys-core`, even when it feels like UI. Three other clients need it, and the
test for it runs in microseconds there and not at all here.

A rule about *how it looks* belongs in `theme.rs` and `ui/`.

If a change seems to need both, it is usually two changes.

---

## The core, in-process

Desktop is the only client that links the state machines **natively** — no FFI,
no wasm, no bridge. [shell_host.rs](src/shell_host.rs) holds a `Core<A>` and
performs what it asks for:

```text
  GPUI entity                CoreHost              DemoShell
      │ dispatch(event) ────────►│
      │◄──────── Vec<Pending> ───│
      │ perform(op) ─────────────────────────────────►│
      │◄──────────────────── Performed ───────────────│
      │ resolve(id, result) ────►│
      │◄──────── Vec<Pending> ───│   …until the queue drains
```

`DemoShell` is a fixture-backed stand-in for the platform: storage, biometrics,
clipboard. It exists so the whole app runs, and every state is reachable, before
a real keystore does. **Replacing it is a change to that one file** — the
machines, the screens and the tests do not move.

Note what it means that a `Wait` is answered by a real timer here and answered
instantly by the test driver in `rust/crates/airkeys-core/tests/`: the core
cannot tell the difference, which is exactly why the elapsed-counter rule is
testable at all.

---

## Tests

```bash
cargo test
```

Seventeen of them, and three are worth knowing about:

- **`contrast_floor_holds_in_both_themes`** — every text-on-surface pair the app
  paints, in both themes, against its WCAG floor. Change a hex and this is what
  tells you a pair dropped below 4.5:1, instead of a user with low vision.
- **`no_key_echoes_in_any_shipped_language`** — every key in `strings::KEYS`, in
  every shipped language: no echo, no empty. A missing translation fails the
  build rather than rendering `vault.navItems` on someone's screen.
- **`copying_resolves_plaintext_from_an_id_and_nothing_else_holds_it`** — the
  secret is resolved inside `DemoShell` and never reaches the `Item` that the
  core, the view model and any log line would carry.

---

## System dependencies

macOS needs the Xcode command-line tools; Windows needs MSVC build tools. Linux
has the largest list, all of it driven by gpui:

**Fedora / RHEL**
```bash
sudo dnf install -y gcc-c++ clang cmake make pkgconf-pkg-config \
  wayland-devel libxkbcommon-x11-devel libxcb-devel \
  fontconfig-devel freetype-devel alsa-lib-devel \
  openssl-devel libzstd-devel vulkan-loader mesa-vulkan-drivers
```

**Debian / Ubuntu**
```bash
sudo apt install -y build-essential clang cmake pkg-config \
  libwayland-dev libxkbcommon-x11-dev libx11-xcb-dev \
  libfontconfig-dev libasound2-dev \
  libssl-dev libzstd-dev libvulkan1 mesa-vulkan-drivers
```

**Arch**
```bash
sudo pacman -S --needed base-devel clang cmake pkgconf \
  wayland libxkbcommon-x11 libxcb fontconfig alsa-lib \
  openssl zstd vulkan-icd-loader
```

Why each group: `wayland`/`xkbcommon`/`xcb` are gpui's windowing and keyboard
handling on **both** display servers; `fontconfig`/`freetype` are font
enumeration and rasterization; `vulkan-loader` plus a driver is how gpui renders,
and it is a **runtime** dependency, not just build-time.

If a gpui bump ever fails on a missing library,
[Zed's own `script/linux`](https://github.com/zed-industries/zed/blob/main/script/linux)
is the place to look first — add `glib2-devel` before anything else.

---

## Things that will bite you

**gpui is pinned to a revision** ([Cargo.toml](Cargo.toml)), not tracking master.
It is pre-1.0 and unversioned on crates.io, so an unpinned git dependency means
`cargo update` can change the UI framework's API underneath a green build. Bump
it deliberately and re-read this section afterwards.

**`QuitMode::LastWindowClosed` is not a preference.** A keep-running single-window
app must be able to reopen its window from the Dock icon, and that path is
unreliable on recent macOS: AppKit's TextInputUI panel stays `isVisible` after
the last real window closes, so the system reports `hasVisibleWindows = YES` and
gpui's reopen gate swallows the event. The app would be running with no window
and no way back to one.

**A nil `NSApp.mainMenu` breaks fullscreen on a secondary display.** The
menu-bar/titlebar reveal never engages for a fullscreen Space when the app has no
main menu, so a fullscreened window on a second display has no titlebar, no
traffic lights and no pointer path back out. [main.rs](src/main.rs) installs a
menu; F11 and ⌃⌘F remain as keyboard exits either way.

**Flex children must opt into shrinking.** A column without `min_w(px(0.))` has
its content width as its automatic minimum, so a narrow window pushes the
fixed-width panel clean out of the window instead of compressing the column. The
same lesson applies to `min_h(px(0.))` on a scroll container.

**gpui does not clip children to a parent's rounding.** An element that owns a
window edge has to round itself to match the frame, or it paints square corners
straight over the rounded ones — see `round_to_frame` in
[window_frame.rs](src/window_frame.rs).

**Client-side decorations are a runtime condition, not a platform.** GNOME's
Mutter never implemented xdg-decoration, so under Wayland the app draws its own
shadow, border, corners and resize handles; under X11 and on macOS the server
does it. `frame_tiling(window)` is the check, and it is why `window_frame` is a
transparent passthrough on two platforms out of three.
