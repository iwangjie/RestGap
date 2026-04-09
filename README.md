# RestGap（息间）

<img width="150" height="150" alt="RestGap 图标" src="https://github.com/user-attachments/assets/e26b58e8-2f76-43c6-9dbd-36b507d1e0a9" />

RestGap 是一个仅支持 macOS 的休息提醒软件，使用纯 Rust 编写，基于原生 AppKit / WebKit 实现菜单栏、设置窗口和全屏休息界面。项目目标很直接：提醒休息这件事要足够稳定、足够轻、足够不打扰。

## 当前定位

- 仅支持 macOS
- 菜单栏常驻
- 原生设置与全屏休息倒计时
- 无账号、无遥测、默认离线运行
- 提供 universal2 打包产物，兼容 Intel 与 Apple Silicon

## 主要功能

- 可配置工作时长与休息时长
- 支持立即开始休息
- 休息界面包含倒计时与呼吸/提肛引导动画
- 可选开启“允许跳过休息”
- 默认关闭跳过功能，关闭时不会展示跳过输入区域，界面更简洁
- 基于系统空闲时长做“几乎整轮未使用”的自动跳过判断

## 运行要求

- macOS 13 及以上版本更稳妥
- Rust 1.85+
- 若要打包 `.dmg`，需要本机可用 `hdiutil`

## 本地运行

```bash
cargo run
```

或构建 release 后运行：

```bash
cargo build --release
./target/release/restgap
```

开发时也可以使用仓库内脚本：

```bash
./start.sh
./status.sh
./stop.sh
```

## 配置说明

配置通过 macOS 的 `NSUserDefaults` 保存，不写独立 JSON 文件。

在菜单栏中打开“配置”后可设置：

- 每隔多少分钟休息一次
- 每次休息多少秒
- 是否允许跳过休息
- 界面语言

默认值：

- 工作间隔：30 分钟
- 休息时长：120 秒
- 允许跳过休息：关闭

数值范围：

- 工作间隔：1 到 240 分钟
- 休息时长：5 到 3600 秒

## 构建与检查

日常检查：

```bash
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

Release 构建：

```bash
cargo build --release
```

## 打包

先安装 `cargo-packager`：

```bash
cargo install cargo-packager --locked
```

然后执行：

```bash
./package-macos.sh
```

脚本会做这些事：

1. 运行发布前检查
2. 构建 universal2 二进制
3. 生成 `.app`
4. 生成 `.dmg`

产物位于 `dist/` 目录。

如果你只想跳过预检查：

```bash
RESTGAP_SKIP_PREFLIGHT=1 ./package-macos.sh
```

## 项目结构

- `src/main.rs`：程序入口，仅保留 macOS 平台分发
- `src/macos/`：菜单栏、倒计时窗口、配置、日志与状态管理
- `src/idle.rs`：系统空闲时长判定
- `src/skip_challenge.rs`：跳过休息的英文输入挑战
- `assets/`：图标与打包资源
- `scripts/`：universal2 构建与 DMG 打包辅助脚本
- `.github/workflows/ci.yml`：macOS 专用 CI / Release 流程

## 发布流程

本仓库发布只保留 macOS：

- 推送到 `main` 会跑 macOS CI
- 打 `vX.Y.Z` tag 会生成 `.dmg` 并上传到 GitHub Release

版本号必须与 `Cargo.toml` 一致。

## 截图

<img width="352" height="298" alt="菜单栏截图" src="https://github.com/user-attachments/assets/930ac760-fb87-4452-9200-2848ecb9cbf4" />

<img width="352" height="298" alt="设置截图" src="https://github.com/user-attachments/assets/764e9c38-3561-4144-af0c-c36a5fd96699" />

<img width="2880" height="1800" alt="全屏休息界面截图" src="https://github.com/user-attachments/assets/eae4781b-d4d8-49d4-9bfb-ba8809786381" />

## License

当前 `Cargo.toml` 标记为 `MIT`。如需正式开源发布，建议补齐仓库根目录 `LICENSE` 文件。
