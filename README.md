# RestGap (息间)
<img width="150" height="150" alt="original" src="https://github.com/user-attachments/assets/e26b58e8-2f76-43c6-9dbd-36b507d1e0a9" />

[English](README.md) | [简体中文](README.zh-CN.md)

RestGap is a lightweight, event-driven break reminder application built in pure Rust. It supports **macOS, Windows, and Linux** platforms with minimal CPU/memory usage while keeping breaks effective.

## Platform Support

- **macOS**: Full GUI support with menu bar integration using native AppKit APIs
- **Windows**: Console-based implementation with core timer functionality
- **Linux**: Console-based implementation with core timer functionality

## Tech Stack

- Rust (Edition 2024)
- **macOS**: AppKit/Foundation/WebKit bindings via `objc2`, `objc2-app-kit`, `objc2-foundation`, `objc2-web-kit`
- **Windows/Linux**: Cross-platform configuration storage via `serde` and `dirs`
- Packaging: `cargo-packager` (+ `hdiutil` for `.dmg` on macOS)

## Features

- **Cross-platform**: Runs on macOS, Windows, and Linux
- Pure Rust implementation
- Configurable work interval & break duration
- Break countdown (fullscreen on macOS, console on Windows/Linux)
- macOS break screen includes a Kegel-guided breathing animation
- No accounts, no telemetry, no network requests
- Configuration stored in platform-appropriate locations:
  - **macOS**: `NSUserDefaults` (system preferences)
  - **Windows/Linux**: JSON file in user config directory

## Requirements

- **macOS**: For full GUI functionality with menu bar and break window
- **Windows/Linux**: Basic console-based timer functionality

## Screenshot

<img width="352" height="298" alt="image" src="https://github.com/user-attachments/assets/930ac760-fb87-4452-9200-2848ecb9cbf4" />

<img width="352" height="298" alt="image" src="https://github.com/user-attachments/assets/764e9c38-3561-4144-af0c-c36a5fd96699" />

<img width="2880" height="1800" alt="3214f6f2fa29810e5c37aaff6790a49b" src="https://github.com/user-attachments/assets/eae4781b-d4d8-49d4-9bfb-ba8809786381" />

## Recent Updates (2026-02-03)

- macOS fullscreen break screen adds a Kegel animation guide
- Release preflight script `./scripts/release-preflight.sh` (fmt/clippy/test/release build)
- Toolchain pinned via `rust-toolchain.toml` to keep CI/local Clippy consistent

## Run

```bash
cargo build --release
./target/release/restgap
```

**macOS**: After launch, it shows a menu bar icon with countdown info. Menu items include: "Rest now / Settings / About / Quit".

**Windows/Linux**: The application runs in the console and prints timer updates. It will automatically trigger breaks at configured intervals.

Tip: On macOS, `./start.sh`, `./stop.sh`, and `./status.sh` are provided for running it in the background while developing (they create local `.pid`/`.log` files in this repo).

## Configuration

**macOS**: Menu bar → Settings → set "every N minutes, break for N seconds". Settings are saved in `NSUserDefaults`.

**Windows/Linux**: Edit the configuration file at:
- Windows: `%APPDATA%\restgap\config.json`
- Linux: `~/.config/restgap/config.json`

Or modify `src/common/config.rs` constants and rebuild.

- Defaults: 30 minutes / 120 seconds
- Ranges: 1–240 minutes, 5–3600 seconds

## Build

```bash
cargo build --release
```

The release profile is optimized for small size (`opt-level="z"`, `lto`, `codegen-units=1`, `panic=abort`, `strip`).

## Packaging (Cargo Packager)

1) Install Cargo Packager:

```bash
cargo install cargo-packager --locked
```

2) macOS artifacts (`.app` / `.dmg`):

```bash
./package-macos.sh
ls dist
```

The script produces a **universal2** build (Intel + Apple Silicon) and then creates a `.dmg` via `hdiutil`.

If you only want to package the current architecture:

```bash
cargo packager --release --formats default
```

3) Windows artifacts (`.exe` / `.msi`, requires NSIS/WiX):

```powershell
.\package-windows.ps1
```

Packaging config lives in `Cargo.toml` under `[package.metadata.packager]`. Before distribution, change `identifier = "com.example.restgap"` to your own reverse-domain identifier.

By default, packaging scripts run `./scripts/release-preflight.sh`. Set `RESTGAP_SKIP_PREFLIGHT=1` to bypass.

## Platform-Specific Notes

### macOS
- Provides full GUI functionality with native menu bar integration
- Break countdown shows as a fullscreen window
- Settings are stored in `NSUserDefaults`
- Requires macOS 10.13+ (High Sierra or later)

### Windows
- Console-based implementation
- Timer runs in the background and prints updates
- Configuration stored in `%APPDATA%\restgap\config.json`
- Future versions may add system tray support

### Linux
- Console-based implementation
- Timer runs in the background and prints updates  
- Configuration stored in `~/.config/restgap/config.json`
- Future versions may add system tray support for desktop environments

## Contributing

Issues and PRs are welcome.

```bash
cargo fmt
cargo clippy
cargo test
```

## License

Not specified yet. If you plan to open-source the project, add a `LICENSE` file and update `Cargo.toml`.
