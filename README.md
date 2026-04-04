# Adele GTK

GTK4-based desktop client for the Adelie Desktop Assistant.

## Requirements

- Rust toolchain (edition 2024, Rust 1.85+)
- GTK4 and WebKitGTK 6.0 system libraries
- A running `desktop-assistant-daemon` instance

### System libraries

| Distro | Packages |
|--------|----------|
| Arch / CachyOS | `gtk4 webkitgtk-6.0` |
| Fedora | `gtk4-devel webkitgtk6.0-devel` |
| Debian / Ubuntu | `libgtk-4-dev libwebkitgtk-6.0-dev` |

## Build

```sh
cargo build
```

To build without WebKitGTK (Label-based fallback instead of webview):

```sh
cargo build --no-default-features
```

## Install

Install binary, desktop entry, and icon:

```sh
just install
```

Or install just the desktop entry and icon (if the binary is already installed):

```sh
just install-desktop
```

To remove the desktop entry and icon:

```sh
just uninstall-desktop
```

## Run

```sh
adele-gtk
```

### CLI options

| Flag | Env var | Default | Description |
|------|---------|---------|-------------|
| `--transport` | `ADELIE_GTK_TRANSPORT` | `ws` | Transport: `ws` or `dbus` |
| `--ws-url` | `ADELIE_GTK_WS_URL` | `ws://127.0.0.1:11339/ws` | WebSocket URL |
| `--ws-jwt` | `ADELIE_GTK_WS_JWT` | | Direct JWT token |
| `--ws-login-username` | `ADELIE_GTK_WS_LOGIN_USERNAME` | | Login username |
| `--ws-login-password` | `ADELIE_GTK_WS_LOGIN_PASSWORD` | | Login password |
| `--ws-subject` | `ADELIE_GTK_WS_SUBJECT` | `desktop-tui` | JWT subject |

## Test

```sh
cargo test
```
