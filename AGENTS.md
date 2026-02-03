# Repository Guidelines

## Project Structure & Module Organization

- `src/main.rs`: platform dispatcher (`macos` / `windows` / `linux` modules).
- `src/macos/`: AppKit-based menu bar UI, fullscreen break countdown, macOS config/logging.
- `src/windows/`: Windows UI (tray, dialogs, countdown) plus timer/state/wndproc helpers.
- `src/linux/`: console-based timer implementation.
- `src/common/`: shared config types/constants for Windows/Linux.
- `assets/`: icons and packaging assets (see `assets/icons/...`).
- `scripts/` + `package-macos.sh` / `package-windows.ps1`: packaging helpers; outputs go to `dist/`.
- `target/`: Cargo build artifacts (do not commit).

## Build, Test, and Development Commands

- `cargo build` / `cargo build --release`: compile (release is size-optimized).
- `cargo run` or `./target/release/restgap`: run locally.
- `cargo fmt`: format code (uses `rustfmt.toml`).
- `cargo clippy`: lint (project enables strict Clippy groups; keep warnings at zero).
- `cargo test`: run unit tests (`#[cfg(test)] mod tests` inside modules).
- `./scripts/release-preflight.sh`: release preflight (fmt check + clippy + tests + release build). Set `RESTGAP_SKIP_PREFLIGHT=1` to bypass.
- Packaging:
  - macOS: `./package-macos.sh` (creates universal2 `.app`, then `.dmg` in `dist/`).
  - Windows: `./package-windows.ps1` (builds NSIS/WiX installers when tools are installed).

## Coding Style & Naming Conventions

- Rust Edition 2024; requires Rust `1.85+` (see `Cargo.toml`).
- Formatting: 4 spaces, max width 100; run `cargo fmt` before pushing.
- Naming: `snake_case` for modules/functions, `CamelCase` for types, `SCREAMING_SNAKE_CASE` for constants.
- Keep platform-specific code behind `cfg(target_os = "...")` and avoid adding new `unsafe` unless necessary.

## Commit & Pull Request Guidelines

- Commits follow a Conventional Commits-like style seen in history: `feat: ...`, `fix: ...`, `refactor: ...` (optional scope: `feat(windows): ...`).
- PRs should state impacted platforms (macOS/Windows/Linux), include UI screenshots for tray/menu/countdown changes, and mention any packaging changes (`Cargo.toml` packager metadata or scripts).

## Security & Configuration Tips

- RestGap is intentionally offline (no telemetry/network); avoid adding network dependencies by default.
- Config is platform-specific: macOS uses `NSUserDefaults`; Windows/Linux use a user config JSON file (paths documented in `README.md`).

## Release Guardrails

- Toolchain is pinned via `rust-toolchain.toml` to keep CI/local Clippy consistent.
- Packaging scripts run release preflight unless `RESTGAP_SKIP_PREFLIGHT=1` is set.
