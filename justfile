default:
    @just --list

install:
    cargo install --path .
    mkdir -p ~/.local/share/applications
    cp adelie-gtk.desktop ~/.local/share/applications/
    mkdir -p ~/.local/share/icons/hicolor/512x512/apps
    cp assets/adele.png ~/.local/share/icons/hicolor/512x512/apps/adelie-gtk.png

install-system:
    cargo build --release
    sudo install -Dm755 target/release/adelie-gtk /usr/local/bin/adelie-gtk
    sudo install -Dm644 adelie-gtk.desktop /usr/local/share/applications/adelie-gtk.desktop
    sudo install -Dm644 assets/adele.png /usr/local/share/icons/hicolor/512x512/apps/adelie-gtk.png
