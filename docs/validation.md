# 验证记录

2026-09-15，本机 macOS 14.8.7，Rust 1.85.0。

已完成：

- `cargo test`：6 个按键状态测试通过。
- `cargo clippy --all-targets -- -D warnings`：通过。
- `cargo fmt --check`：通过。
- `cargo build --release`：通过。
- 沙箱外 `--check`：辅助功能、输入监控、事件发送权限均为 true。
- `--dry-run --duration 10`：成功建立只监听事件 tap，正常到时退出，posted_events=0。

- `--run --duration 60`：成功识别真实远程窗口，累计 5 次焦点状态变化、64 次定向事件投递，正常到时退出，退出码 0。
- 用户确认 Rust 常驻模式下"文字正常，快捷键只触发远端"。本次测试组合键为两端相同的 Command+P。

仍待验证：
- 同时运行 Karabiner、Keyboard Maestro 时的全局热键竞争。
- 焦点切换、长按、断线和紧急退出的真实桌面行为。

此前 Swift 单次投递探针的人工验收见 `native-screen-sharing.md`，Rust 的文字输入及 Command+P 核心链路已在本次单独验收通过，其余边界测试仍未完成。
