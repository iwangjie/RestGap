# RestGap (息间) 
<img width="150" height="150" alt="original" src="https://github.com/user-attachments/assets/e26b58e8-2f76-43c6-9dbd-36b507d1e0a9" />

[English](README.md) | [简体中文](README.zh-CN.md)

RestGap is a lightweight, event-driven macOS menu bar break reminder built in pure Rust (AppKit/Foundation via `objc2`). It aims for minimal CPU/memory usage while keeping breaks effective.

## Tech Stack

- Rust (Edition 2024)
- AppKit/Foundation bindings: `objc2`, `objc2-app-kit`, `objc2-foundation`
- Packaging: `cargo-packager` (+ `hdiutil` for `.dmg`)

## Features

- Pure Rust implementation (no Swift/ObjC code in this repo)
- Universal packaging (Intel + Apple Silicon)
- Configurable work interval & break duration
- Break countdown window has no “skip” action (to encourage real breaks)
- No accounts, no telemetry, no network requests
- No standalone config file (settings are stored in macOS user defaults via `NSUserDefaults`)

## Screenshot

<img width="352" height="298" alt="image" src="https://github.com/user-attachments/assets/930ac760-fb87-4452-9200-2848ecb9cbf4" />

<img width="352" height="298" alt="image" src="https://github.com/user-attachments/assets/764e9c38-3561-4144-af0c-c36a5fd96699" />

<img width="2880" height="1800" alt="3214f6f2fa29810e5c37aaff6790a49b" src="https://github.com/user-attachments/assets/eae4781b-d4d8-49d4-9bfb-ba8809786381" />


## Requirements

- macOS for full functionality
- Other platforms: the binary prints a message and exits

## Run

```bash
cargo build --release
./target/release/restgap
```

After launch, it only shows a menu bar icon with countdown info. Menu items include: “Rest now / Settings / About / Quit”.

## Configuration

Menu bar → Settings → set “every N minutes, break for N seconds”. Settings are saved in `NSUserDefaults`.

- Defaults: 30 minutes / 120 seconds
- Ranges: 1–240 minutes, 5–3600 seconds

## Build

```bash
cargo build --release
```

The release profile is optimized for small size (`opt-level="z"`, `lto`, `codegen-units=1`, `panic=abort`, `strip`).

## Packaging (Cargo Packager)

> Note: `restgap` only provides full functionality on macOS. The Windows build currently only prints “macOS only”.

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

## Contributing

Issues and PRs are welcome.

```bash
cargo fmt
cargo clippy
cargo test
```

## License

Not specified yet. If you plan to open-source the project, add a `LICENSE` file and update `Cargo.toml`.
