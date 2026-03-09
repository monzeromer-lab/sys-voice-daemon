# VoskDictation

Offline voice-to-text daemon for Linux. A privacy-first, fully offline voice dictation utility that integrates natively with the Linux desktop environment.

## Features

- **100% Offline** - All speech recognition runs locally via [Vosk](https://alphacephei.com/vosk/). No audio data ever leaves your machine.
- **Context-Aware Focus Detection** - Automatically activates when a text field is focused. Supports both X11 (via `x11rb`) and Wayland (via AT-SPI2/D-Bus).
- **Virtual Keyboard Injection** - Injects transcribed text directly into the active window using a kernel-level virtual keyboard (`/dev/uinput`). No clipboard needed.
- **Low Latency** - Sub-500ms transcription using multithreaded architecture with dedicated audio capture, STT processing, and injection threads.
- **GNOME Panel Integration** - Top panel indicator with enable/disable toggle, status display, and daemon control via D-Bus.
- **GNOME HUD Overlay** - Optional floating GTK4 window showing real-time partial transcriptions (feature-gated).

## Architecture

```
┌─────────────┐    ┌──────────┐    ┌────────────┐    ┌──────────┐
│ Audio Capture│───>│ Vosk STT │───>│   Daemon   │───>│  uinput  │
│   (rodio)   │    │          │    │   State    │    │ Virtual  │
│             │    │          │    │  Machine   │    │ Keyboard │
└─────────────┘    └──────────┘    └────────────┘    └──────────┘
                                     ▲         ▲
                                     │         │
                               ┌─────┴───┐ ┌───┴──────────┐
                               │  Focus  │ │ D-Bus Server │
                               │Detection│ │ com.github.  │
                               │x11rb/   │ │ voskdictation│
                               │zbus     │ └──────┬───────┘
                               └─────────┘        │
                                              ┌───┴───────────┐
                                              │  GNOME Shell  │
                                              │  Extension    │
                                              │  (top panel)  │
                                              └───────────────┘
```

Each subsystem runs in its own thread, communicating via `crossbeam-channel`. The GNOME Shell extension communicates with the daemon over the session D-Bus.

## Requirements

- Ubuntu 24.04 LTS (or compatible distribution)
- GNOME Shell 46 (for the panel extension)
- Rust 1.85+ (2024 edition)
- [just](https://github.com/casey/just) command runner
- System libraries: `libasound2-dev`, `libx11-dev`
- Optional: `libgtk-4-dev` (for HUD overlay)

## Quick Start

```bash
# Install system dependencies and fetch Vosk library + model
just setup

# Run the daemon
just run

# Install the GNOME panel extension
just install-extension
# Then restart GNOME Shell: log out/in (Wayland) or Alt+F2 -> r (X11)
```

## Installation

### From Source

```bash
# 1. Install system dependencies (requires sudo)
just deps

# 2. Download Vosk native library and speech model
just fetch

# 3. Build
just build

# 4. Install system-wide (binary + library + udev rules + systemd service + GNOME extension)
just install

# 5. Add your user to the input group for uinput access
just add-input-group

# 6. Log out and back in, then start the daemon
vosk-dictation
```

### Debian Package

```bash
# Build the .deb package
just deb

# Install it
sudo dpkg -i target/deb/vosk-dictation_0.1.0_amd64.deb

# Follow the post-install instructions printed by the package
```

The `.deb` package includes:
- Binary (`/usr/bin/vosk-dictation`)
- Vosk shared library (`/usr/lib/libvosk.so`)
- udev rules (`/etc/udev/rules.d/99-vosk-dictation.rules`)
- systemd user service (`/usr/lib/systemd/user/vosk-dictation.service`)
- GNOME Shell extension (`/usr/share/gnome-shell/extensions/vosk-dictation@vosk-dictation/`)

## GNOME Panel Extension

The panel extension adds a microphone icon to the GNOME top bar with a dropdown menu:

- **Enable Dictation** toggle - Enable or disable the daemon globally
- **Status indicator** - Shows current state (Idle / Listening / Paused)
- **Quit Daemon** - Gracefully shut down the daemon

### Installing the Extension

```bash
# Install for the current user
just install-extension

# Or install system-wide (done automatically by `just install` or the .deb package)
sudo cp -r gnome-extension/vosk-dictation@vosk-dictation \
    /usr/share/gnome-shell/extensions/

# Restart GNOME Shell to activate:
#   Wayland: log out and back in
#   X11: Alt+F2 -> type 'r' -> Enter

# Enable the extension
gnome-extensions enable vosk-dictation@vosk-dictation
```

### D-Bus Interface

The daemon registers on the session bus as `com.github.voskdictation`. You can control it from the command line:

```bash
# Enable the daemon
busctl --user call com.github.voskdictation \
    /com/github/voskdictation com.github.voskdictation Enable

# Disable (pause) the daemon
busctl --user call com.github.voskdictation \
    /com/github/voskdictation com.github.voskdictation Disable

# Check current state
busctl --user get-property com.github.voskdictation \
    /com/github/voskdictation com.github.voskdictation State

# Quit the daemon
busctl --user call com.github.voskdictation \
    /com/github/voskdictation com.github.voskdictation Quit
```

## Usage

### Running Manually

```bash
# Run with default model path (~/.local/share/vosk/model)
just run

# Run with a custom model path
just run /path/to/vosk/model

# Run with verbose debug logging
just run-debug
```

### Running as a Systemd Service

```bash
# Install the user service
just install-service

# Enable and start
systemctl --user enable --now vosk-dictation

# Check status
systemctl --user status vosk-dictation

# View logs
journalctl --user -u vosk-dictation -f
```

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `RUST_LOG` | `info` | Log level (`debug`, `info`, `warn`, `error`) |
| `XDG_DATA_HOME` | `~/.local/share` | Base path for the default model directory |

## Vosk Models

The daemon requires a Vosk language model. The default location is `~/.local/share/vosk/model/`.

```bash
# Download the small English model (~40MB)
just fetch-model
```

For other languages or larger models, visit the [Vosk Models page](https://alphacephei.com/vosk/models) and extract the model to the path of your choice. Then run:

```bash
vosk-dictation /path/to/your/model
```

### Recommended Models

| Model | Size | Use Case |
|-------|------|----------|
| `vosk-model-small-en-us-0.15` | 40MB | Fast, lightweight, good for general dictation |
| `vosk-model-en-us-0.22` | 1.8GB | High accuracy, requires more RAM |

## uinput Access

The daemon creates a virtual keyboard device at `/dev/uinput` to inject transcribed text. This requires your user to be in the `input` group.

```bash
# Add yourself to the input group
sudo usermod -aG input $USER

# Install the udev rule (done automatically by `just install`)
sudo cp dist/udev/99-vosk-dictation.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules && sudo udevadm trigger

# Log out and back in for group changes to take effect
```

If uinput access is unavailable, the daemon falls back to printing transcriptions to stdout.

## Project Structure

```
.
├── src/
│   ├── main.rs         # Entry point, wires all subsystems
│   ├── audio.rs        # Continuous microphone capture (rodio) -> i16 PCM
│   ├── stt.rs          # Vosk speech-to-text engine wrapper
│   ├── injector.rs     # uinput virtual keyboard text injection
│   ├── focus.rs        # Focus detection (X11 + Wayland/AT-SPI2)
│   ├── daemon.rs       # Central state machine (Idle/Listening/Paused)
│   ├── dbus.rs         # D-Bus control interface (com.github.voskdictation)
│   ├── config.rs       # Configuration (model path, sample rate)
│   └── hud.rs          # GTK4 floating HUD overlay (feature-gated)
├── gnome-extension/
│   └── vosk-dictation@vosk-dictation/
│       ├── metadata.json    # Extension metadata (GNOME 46)
│       ├── extension.js     # Panel indicator + D-Bus client
│       └── stylesheet.css   # Extension styles
├── dist/
│   ├── udev/           # udev rules for /dev/uinput access
│   └── systemd/        # systemd user service file
├── debian/             # Debian packaging files
├── vendor/             # Vosk native library (downloaded, gitignored)
├── build.rs            # Links vendor Vosk library at compile time
├── justfile            # Command runner recipes
└── Cargo.toml
```

## Just Commands

Run `just` to see all available recipes:

| Command | Description |
|---------|-------------|
| `just setup` | Full setup: install deps, fetch libs, build |
| `just deps` | Install system dependencies (sudo) |
| `just fetch` | Download Vosk library and speech model |
| `just build` | Build in release mode |
| `just build-hud` | Build with GTK4 HUD support |
| `just run` | Run the daemon |
| `just run-debug` | Run with debug logging |
| `just lint` | Run clippy |
| `just fmt` | Format code |
| `just check` | Run fmt + lint + build |
| `just install` | Install everything system-wide |
| `just install-extension` | Install GNOME Shell extension |
| `just uninstall-extension` | Remove GNOME Shell extension |
| `just deb` | Build a `.deb` package |
| `just clean` | Clean build artifacts |

## Roadmap

- [ ] Context-aware vocabularies (swap Vosk model per focused application)
- [ ] Command mode ("Scratch that", "Press Enter", "New line")
- [ ] Wayland clipboard fallback for compositors that block virtual keyboards
- [ ] Audio buffer replay (last 10s for debugging misheard transcriptions)
- [ ] Audio device selection from GNOME extension menu
- [ ] GPU-accelerated models

## License

MIT
