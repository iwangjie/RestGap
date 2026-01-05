# 息间（RestGap）

[简体中文](README.zh-CN.md) | [English](README.md)

息间（RestGap）是一款纯 Rust 实现的 macOS 菜单栏休息提醒软件（事件驱动 / 非轮询），追求极低占用与足够有效的休息。

## 技术栈

- Rust（Edition 2024）
- AppKit/Foundation 绑定：`objc2`、`objc2-app-kit`、`objc2-foundation`
- 打包：`cargo-packager`（配合 `hdiutil` 生成 `.dmg`）

## 特性

- 纯 Rust 实现（仓库内无 Swift/ObjC 代码）
- Intel + Apple Silicon（universal2）兼容
- 休息间隔/休息时长可配置
- 倒计时窗口不提供“跳过”动作，保障休息有效性
- 无账号、无遥测、无联网请求
- 无独立配置文件（配置存储在系统偏好 `NSUserDefaults`）

## 运行

```bash
cargo build --release
./target/release/restgap
```

运行后仅在菜单栏显示图标与倒计时信息。菜单里可手动“现在休息 / 配置 / 关于 / 退出”。

提示：仓库提供 `./start.sh`、`./stop.sh`、`./status.sh` 便于开发时后台运行（会在仓库目录生成本地 `.pid`/`.log` 文件）。

## 配置

点击菜单栏里的“配置”，设置“每 N 分钟休息 N 秒”。配置会保存在系统偏好里（`NSUserDefaults`）。

- 默认：每 30 分钟休息 120 秒
- 范围：每 1–240 分钟休息一次；休息时长 5–3600 秒

## 编译

```bash
cargo build --release
```

Release 配置已开启体积优化（`opt-level="z" / lto / codegen-units=1 / panic=abort / strip`）。

## 打包（Cargo Packager）

> 注意：当前 `restgap` 仅在 macOS 上提供完整功能；Windows 版本目前只会提示“仅支持 macOS”。

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

## 参与贡献

欢迎提交 Issue / PR（尽量保持改动小且聚焦）。

```bash
cargo fmt
cargo clippy
cargo test
```

## 许可协议

目前尚未指定许可证；如需开源发布，请添加 `LICENSE` 并更新 `Cargo.toml`。

