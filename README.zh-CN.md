# 息间（RestGap）

<img width="150" height="150" alt="original" src="https://github.com/user-attachments/assets/e26b58e8-2f76-43c6-9dbd-36b507d1e0a9" />

[简体中文](README.zh-CN.md) | [English](README.md)

息间（RestGap）是一款纯 Rust 实现的跨平台休息提醒软件（事件驱动 / 非轮询），支持 **macOS、Windows 和 Linux**，追求极低占用与足够有效的休息。

## 平台支持

- **macOS**: 完整 GUI 支持，使用原生 AppKit API 实现菜单栏集成
- **Windows**: 基于控制台的实现，提供核心计时功能
- **Linux**: 基于控制台的实现，提供核心计时功能

## 技术栈

- Rust（Edition 2024）
- **macOS**: AppKit/Foundation/WebKit 绑定，使用 `objc2`、`objc2-app-kit`、`objc2-foundation`、`objc2-web-kit`
- **Windows/Linux**: 跨平台配置存储，使用 `serde` 和 `dirs`
- 打包：`cargo-packager`（配合 `hdiutil` 生成 `.dmg`，仅 macOS）

## 特性

- **跨平台**：支持 macOS、Windows 和 Linux
- 纯 Rust 实现
- 休息间隔/休息时长可配置
- 休息倒计时（macOS 全屏窗口，Windows/Linux 控制台）
- macOS 休息界面内置提肛呼吸引导动画
- 无账号、无遥测、无联网请求
- 配置存储在平台适当的位置：
  - **macOS**: `NSUserDefaults`（系统偏好设置）
  - **Windows/Linux**: 用户配置目录中的 JSON 文件

## 截屏

<img width="300" height="300" alt="image" src="https://github.com/user-attachments/assets/930ac760-fb87-4452-9200-2848ecb9cbf4" />

<img width="300" height="300" alt="image" src="https://github.com/user-attachments/assets/764e9c38-3561-4144-af0c-c36a5fd96699" />

<img width="1880" height="800" alt="3214f6f2fa29810e5c37aaff6790a49b" src="https://github.com/user-attachments/assets/eae4781b-d4d8-49d4-9bfb-ba8809786381" />

## 最近更新（2026-02-03）

- macOS 全屏休息界面新增提肛动画引导
- 发版前置检查脚本 `./scripts/release-preflight.sh`（fmt/clippy/test/release build）
- 通过 `rust-toolchain.toml` 固定 Rust/Clippy 版本，避免 CI 漂移

## 运行

```bash
cargo build --release
./target/release/restgap
```

**macOS**: 运行后仅在菜单栏显示图标与倒计时信息。菜单里可手动"现在休息 / 配置 / 关于 / 退出"。

**Windows/Linux**: 应用在控制台中运行并打印计时器更新。它将在配置的间隔时间自动触发休息。

提示：在 macOS 上，仓库提供 `./start.sh`、`./stop.sh`、`./status.sh` 便于开发时后台运行（会在仓库目录生成本地 `.pid`/`.log` 文件）。

## 配置

**macOS**: 点击菜单栏里的"配置"，设置"每 N 分钟休息 N 秒"。配置会保存在系统偏好里（`NSUserDefaults`）。

**Windows/Linux**: 编辑配置文件：
- Windows: `%APPDATA%\restgap\config.json`
- Linux: `~/.config/restgap/config.json`

或修改 `src/common/config.rs` 中的常量并重新编译。

- 默认：每 30 分钟休息 120 秒
- 范围：每 1–240 分钟休息一次；休息时长 5–3600 秒

## 编译

```bash
cargo build --release
```

Release 配置已开启体积优化（`opt-level="z" / lto / codegen-units=1 / panic=abort / strip`）。

## 打包（Cargo Packager）

1) 安装 Cargo Packager：

```bash
cargo install cargo-packager --locked
```

2) macOS 产物（`.app` / `.dmg`）：

```bash
./package-macos.sh
ls dist
```

脚本默认产出 **universal2**（同时支持 Intel + Apple Silicon），并使用 `hdiutil` 生成 `.dmg`。

如果你只想打包当前架构，可直接运行：

```bash
cargo packager --release --formats default
```

3) Windows 产物（`.exe` / `.msi`，需要 NSIS/WiX）：

```powershell
.\package-windows.ps1
```

打包配置在 `Cargo.toml` 的 `[package.metadata.packager]`；正式分发前建议把 `identifier = "com.example.restgap"` 改成你自己的反向域名标识。

默认情况下，打包脚本会先执行 `./scripts/release-preflight.sh`。如需跳过，设置 `RESTGAP_SKIP_PREFLIGHT=1`。

## 平台特定说明

### macOS
- 提供完整的 GUI 功能，原生菜单栏集成
- 休息倒计时显示为全屏窗口
- 设置存储在 `NSUserDefaults` 中
- 需要 macOS 10.13+ (High Sierra 或更高版本)

### Windows
- 基于控制台的实现
- 计时器在后台运行并打印更新
- 配置存储在 `%APPDATA%\restgap\config.json`
- 未来版本可能会添加系统托盘支持

### Linux
- 基于控制台的实现
- 计时器在后台运行并打印更新
- 配置存储在 `~/.config/restgap/config.json`
- 未来版本可能会为桌面环境添加系统托盘支持

## 参与贡献

欢迎提交 Issue / PR（尽量保持改动小且聚焦）。

```bash
cargo fmt
cargo clippy
cargo test
```

## 许可协议

目前尚未指定许可证；如需开源发布，请添加 `LICENSE` 并更新 `Cargo.toml`。
