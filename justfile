# VoskDictation - Offline Voice-to-Text Daemon
# Run `just` to see all available recipes

set dotenv-load := false

# Project metadata
name := "vosk-dictation"
version := `grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/'`
extension_uuid := "vosk-dictation@vosk-dictation"
extension_src := "gnome-extension/" + extension_uuid
extension_dest := env("HOME") + "/.local/share/gnome-shell/extensions/" + extension_uuid
vosk_version := "0.3.45"
vosk_model_url := "https://alphacephei.com/vosk/models/vosk-model-small-en-us-0.15.zip"
vosk_lib_url := "https://github.com/alphacep/vosk-api/releases/download/v" + vosk_version + "/vosk-linux-x86_64-" + vosk_version + ".zip"
model_dir := env("XDG_DATA_HOME", env("HOME") + "/.local/share") + "/vosk/model"
vendor_lib := "vendor/vosk-linux-x86_64-" + vosk_version

# Default recipe: list all commands
default:
    @just --list

# Install all system dependencies (requires sudo)
deps:
    sudo apt-get update
    sudo apt-get install -y \
        build-essential \
        pkg-config \
        libasound2-dev \
        libgtk-4-dev \
        libx11-dev \
        wget \
        unzip

# Download the Vosk native library into vendor/
fetch-vosk:
    mkdir -p vendor
    @if [ ! -f "{{vendor_lib}}/libvosk.so" ]; then \
        echo "Downloading Vosk native library..."; \
        wget -q "{{vosk_lib_url}}" -O vendor/vosk.zip; \
        unzip -qo vendor/vosk.zip -d vendor/; \
        rm vendor/vosk.zip; \
        echo "Vosk library downloaded to {{vendor_lib}}/"; \
    else \
        echo "Vosk library already present at {{vendor_lib}}/"; \
    fi

# Download the default English speech model
fetch-model:
    mkdir -p "$(dirname "{{model_dir}}")"
    @if [ ! -d "{{model_dir}}" ]; then \
        echo "Downloading Vosk English model..."; \
        wget -q "{{vosk_model_url}}" -O /tmp/vosk-model.zip; \
        unzip -qo /tmp/vosk-model.zip -d /tmp/; \
        mv /tmp/vosk-model-small-en-us-0.15 "{{model_dir}}"; \
        rm /tmp/vosk-model.zip; \
        echo "Model installed to {{model_dir}}"; \
    else \
        echo "Model already present at {{model_dir}}"; \
    fi

# Download all external dependencies (Vosk lib + model)
fetch: fetch-vosk fetch-model

# Build the project (release mode)
build:
    cargo build --release

# Build with HUD support (requires libgtk-4-dev)
build-hud:
    cargo build --release --features hud

# Build in debug mode
build-debug:
    cargo build

# Run the daemon
run *ARGS:
    LD_LIBRARY_PATH="{{justfile_directory()}}/{{vendor_lib}}:${LD_LIBRARY_PATH:-}" \
        cargo run --release -- {{ARGS}}

# Run in debug mode with verbose logging
run-debug *ARGS:
    RUST_LOG=debug LD_LIBRARY_PATH="{{justfile_directory()}}/{{vendor_lib}}:${LD_LIBRARY_PATH:-}" \
        cargo run -- {{ARGS}}

# Run clippy linter
lint:
    cargo clippy -- -W clippy::all

# Format code
fmt:
    cargo fmt

# Check formatting without modifying
fmt-check:
    cargo fmt --check

# Run all checks (fmt + clippy + build)
check: fmt-check lint build

# Install Vosk library system-wide (requires sudo)
install-vosk-lib:
    sudo cp "{{vendor_lib}}/libvosk.so" /usr/local/lib/
    sudo ldconfig
    @echo "libvosk.so installed to /usr/local/lib/"

# Install udev rules for uinput access (requires sudo)
install-udev:
    sudo cp dist/udev/99-vosk-dictation.rules /etc/udev/rules.d/
    sudo udevadm control --reload-rules
    sudo udevadm trigger
    @echo "udev rules installed. You may need to log out and back in."

# Add current user to the input group (requires sudo)
add-input-group:
    sudo usermod -aG input "$USER"
    @echo "User $USER added to input group. Log out and back in for it to take effect."

# Install the systemd user service
install-service:
    mkdir -p ~/.config/systemd/user
    cp dist/systemd/vosk-dictation.service ~/.config/systemd/user/
    systemctl --user daemon-reload
    @echo "Service installed. Enable with: systemctl --user enable --now vosk-dictation"

# Install the binary to /usr/local/bin (stops running daemon first)
install-bin: build
    #!/usr/bin/env bash
    set -euo pipefail
    # Stop running daemon if any
    busctl --user call com.github.voskdictation /com/github/voskdictation com.github.voskdictation Quit 2>/dev/null || true
    sleep 1
    # Force kill if still running
    pkill -f '/usr/local/bin/vosk-dictation' 2>/dev/null || true
    sleep 0.5
    sudo cp target/release/assistant /usr/local/bin/vosk-dictation
    echo "Binary installed to /usr/local/bin/vosk-dictation"

# Install and enable the GNOME Shell extension for the current user
install-extension:
    mkdir -p "{{extension_dest}}"
    cp -r "{{extension_src}}"/* "{{extension_dest}}/"
    mkdir -p ~/.local/share/dbus-1/services
    cp dist/dbus/com.github.voskdictation.service ~/.local/share/dbus-1/services/
    gnome-extensions enable {{extension_uuid}} 2>/dev/null || true
    @echo "Extension installed and enabled."
    @echo "If the icon doesn't appear, log out and back in (Wayland) or Alt+F2 -> r (X11)."

# Uninstall the GNOME Shell extension
uninstall-extension:
    rm -rf "{{extension_dest}}"
    -rm -f ~/.local/share/dbus-1/services/com.github.voskdictation.service
    @echo "Extension removed. Restart GNOME Shell to complete."

# Full install: library + binary + udev + service + extension
install: install-vosk-lib install-bin install-udev install-service install-extension
    @echo "Installation complete!"

# Uninstall everything
uninstall: uninstall-extension
    -sudo rm -f /usr/local/bin/vosk-dictation
    -sudo rm -f /usr/local/lib/libvosk.so
    -sudo rm -f /etc/udev/rules.d/99-vosk-dictation.rules
    -rm -f ~/.config/systemd/user/vosk-dictation.service
    -systemctl --user daemon-reload
    -sudo ldconfig
    @echo "Uninstalled."

# Build Debian package
deb: build
    #!/usr/bin/env bash
    set -euo pipefail
    pkg="vosk-dictation"
    ver="{{version}}"
    arch="amd64"
    stage="target/deb/${pkg}_${ver}_${arch}"

    rm -rf "$stage"
    mkdir -p "$stage/DEBIAN"
    mkdir -p "$stage/usr/bin"
    mkdir -p "$stage/usr/lib"
    mkdir -p "$stage/etc/udev/rules.d"
    mkdir -p "$stage/usr/lib/systemd/user"
    mkdir -p "$stage/usr/share/doc/${pkg}"
    mkdir -p "$stage/usr/share/vosk/model"
    mkdir -p "$stage/usr/share/gnome-shell/extensions/{{extension_uuid}}"

    # Binary
    cp target/release/assistant "$stage/usr/bin/vosk-dictation"
    strip "$stage/usr/bin/vosk-dictation"

    # Vosk shared library
    cp "{{vendor_lib}}/libvosk.so" "$stage/usr/lib/"

    # udev rules
    cp dist/udev/99-vosk-dictation.rules "$stage/etc/udev/rules.d/"

    # systemd user service
    cp dist/systemd/vosk-dictation.service "$stage/usr/lib/systemd/user/"

    # GNOME Shell extension
    cp -r {{extension_src}}/* "$stage/usr/share/gnome-shell/extensions/{{extension_uuid}}/"

    # Documentation
    cp README.md "$stage/usr/share/doc/${pkg}/"

    # Control file
    cat > "$stage/DEBIAN/control" <<EOF
    Package: ${pkg}
    Version: ${ver}
    Section: utils
    Priority: optional
    Architecture: ${arch}
    Depends: libasound2, libx11-6
    Recommends: libgtk-4-1
    Maintainer: VoskDictation Maintainers <noreply@example.com>
    Description: Offline voice-to-text daemon for Linux
     A privacy-first, fully offline voice dictation daemon that integrates
     natively with the Linux desktop. Uses Vosk for speech recognition and
     injects transcribed text via a virtual keyboard (uinput).
    EOF

    # Post-install script
    cat > "$stage/DEBIAN/postinst" <<'EOF'
    #!/bin/sh
    set -e
    ldconfig
    udevadm control --reload-rules 2>/dev/null || true
    udevadm trigger 2>/dev/null || true
    echo ""
    echo "vosk-dictation installed successfully!"
    echo ""
    echo "Next steps:"
    echo "  1. Add your user to the input group:  sudo usermod -aG input \$USER"
    echo "  2. Download a Vosk model to ~/.local/share/vosk/model/"
    echo "  3. Log out and back in, then run:  vosk-dictation"
    echo ""
    EOF
    chmod 755 "$stage/DEBIAN/postinst"

    # Post-remove script
    cat > "$stage/DEBIAN/postrm" <<'EOF'
    #!/bin/sh
    set -e
    ldconfig
    EOF
    chmod 755 "$stage/DEBIAN/postrm"

    # Build the .deb
    dpkg-deb --build --root-owner-group "$stage"
    echo ""
    echo "Package built: target/deb/${pkg}_${ver}_${arch}.deb"

# Clean build artifacts
clean:
    cargo clean
    rm -rf target/deb

# Full setup from scratch: install deps, fetch libs, build
setup: deps fetch build
    @echo "Setup complete! Run 'just run' to start the daemon."
