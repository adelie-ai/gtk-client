# Adelie GTK Client

GTK4-based desktop client for the Adelie Desktop Assistant.

## Building

### Prerequisites

**Rust toolchain** (edition 2024, Rust 1.85+):

```sh
rustup update stable
```

**System libraries:**

| Distro | Packages |
|--------|----------|
| Arch / CachyOS | `gtk4 webkitgtk-6.0` |
| Fedora | `gtk4-devel webkitgtk6.0-devel` |
| Debian / Ubuntu | `libgtk-4-dev libwebkitgtk-6.0-dev` |

On Arch:

```sh
sudo pacman -S gtk4 webkitgtk-6.0
```

### Build

```sh
cd gtk-client
cargo build
```

To build without WebKitGTK (Label-based fallback instead of webview):

```sh
cargo build --no-default-features
```

### Run

The daemon (`desktop-assistant-daemon`) must be running first.

```sh
cargo run
```

#### CLI options

| Flag | Env var | Default | Description |
|------|---------|---------|-------------|
| `--transport` | `ADELIE_GTK_TRANSPORT` | `ws` | Transport: `ws` or `dbus` |
| `--ws-url` | `ADELIE_GTK_WS_URL` | `ws://127.0.0.1:11339/ws` | WebSocket URL |
| `--ws-jwt` | `ADELIE_GTK_WS_JWT` | | Direct JWT token |
| `--ws-login-username` | `ADELIE_GTK_WS_LOGIN_USERNAME` | | Login username |
| `--ws-login-password` | `ADELIE_GTK_WS_LOGIN_PASSWORD` | | Login password |
| `--ws-subject` | `ADELIE_GTK_WS_SUBJECT` | `desktop-tui` | JWT subject |

### Test

```sh
cargo test
```
