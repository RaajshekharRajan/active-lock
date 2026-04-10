# Active Lock

A cross-platform screen locker that overlays all connected monitors with a fake lock screen. Background processes continue running — the OS never enters sleep or a real lock state.

## Features

- **Multi-monitor** — detects all displays and covers each with a fullscreen black overlay
- **OS-level lockdown** — blocks Alt+Tab, Win key, Task Manager (Windows) and Cmd+Tab, Force Quit, Dock/Menu Bar (macOS)
- **Minimal UI** — subtle password field on the primary display with animated error feedback
- **Crash-safe** — three layers of cleanup (Drop, panic hook, signal handler) guarantee system settings are restored
- **Tiny footprint** — ~1 MB binary, near-zero CPU at idle

## Installation

### Download a release binary

Grab the latest binary for your platform from the [Releases](../../releases) page:

| Platform | File |
|---|---|
| macOS (Apple Silicon) | `active-lock-macos-arm64` |
| macOS (Intel) | `active-lock-macos-x86_64` |
| Windows (64-bit) | `active-lock-windows-x86_64.exe` |

On macOS, make it executable after downloading:

```bash
chmod +x active-lock-macos-*
```

### Build from source

Requires [Rust](https://rustup.rs/) 1.70+.

```bash
git clone <repo-url> && cd active-lock
cargo build --release
# Binary is at target/release/active-lock
```

Or install directly:

```bash
cargo install --git <repo-url>
```

## Usage

### Set your password (first time)

```bash
./active-lock --set-password
```

You'll be prompted to enter and confirm a password. The bcrypt hash is stored in `~/.active-lock/password.hash`. If no custom password is set, the default password is **`unlock`**.

### Lock the screen

```bash
./active-lock
```

All monitors go black. Type your password and press **Enter** to unlock. Press **Escape** to clear your input and start over.

### Emergency reset

If the app is killed unexpectedly and system settings are stuck (e.g., Task Manager stays disabled on Windows):

```bash
./active-lock --reset
```

## How it works

### Windows

- A low-level keyboard hook (`WH_KEYBOARD_LL`) intercepts and blocks Alt+Tab, Alt+F4, the Windows key, and Ctrl+Esc
- Task Manager is temporarily disabled via the `HKCU\...\Policies\System\DisableTaskMgr` registry key
- Overlay windows are set to `HWND_TOPMOST` with `WS_EX_TOOLWINDOW` (hidden from taskbar)

### macOS

- `NSApplication.setPresentationOptions` hides the Dock and Menu Bar, and disables process switching (Cmd+Tab), Force Quit (Cmd+Opt+Esc), and the Apple menu
- Overlay windows are set to `NSScreenSaverWindowLevel + 1` with `CanJoinAllSpaces` collection behavior

### Both platforms

- `winit` handles window creation, keyboard input, and monitor enumeration
- `tiny-skia` renders the lock icon and password field to a pixel buffer
- `softbuffer` blits the buffer to each window surface
- Focus is aggressively re-stolen every 500ms and on every focus-loss event

## Configuration

The password hash lives at `~/.active-lock/password.hash`. Delete this file to revert to the default password (`unlock`).

To change the default built-in password, edit the `DEFAULT_HASH` constant in `src/config.rs` and rebuild. Generate a hash with:

```bash
# After building, use the --set-password flow, then copy the hash from the file:
cat ~/.active-lock/password.hash
```

## License

MIT
