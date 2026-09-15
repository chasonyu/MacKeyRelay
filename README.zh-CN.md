# MacKeyRelay

**将键盘快捷键转发到 macOS 屏幕共享，不触发本地热键。**

[English](README.md) | 简体中文

MacKeyRelay 捕获本地键盘事件，在远程会话窗口聚焦时直接投递给苹果屏幕共享。切走焦点，输入回到本地。远端无需安装任何东西。

```text
键盘 → MacKeyRelay → 屏幕共享 → 远端 Mac
```

使用公开 macOS 事件 API（`CGEventTap`、`CGEventPostToPid`）——不 patch、不调私有 API、不实现独立网络协议。不需要 Karabiner-Elements。

## 兼容性

**实测环境，非最低版本声明。**

| 组件 | 已验证 |
|---|---|
| 本机 macOS | Sonoma 14.8.7（构建 23J520） |
| 屏幕共享客户端 | 4.3（构建 594.2.2） |
| 架构 | Apple Silicon（arm64） |
| Rust 工具链 | 1.85.0 |

其他 macOS 版本、屏幕共享构建和 Intel Mac 未经验证。

## 构建

需要 macOS、Rust 1.85+ 和 Xcode Command Line Tools。

```sh
cargo build --release
```

## 用法

以普通桌面用户运行。**不要用 `sudo`**——管理员身份不替代 macOS 隐私授权。

```sh
# 1. 请求系统权限
./target/release/mackeyrelay --request-permissions

# 2. 验证权限和当前窗口
./target/release/mackeyrelay --check

# 3. 运行
./target/release/mackeyrelay --run
```

第 1 步后，在 **系统设置 → 隐私与安全性** 的 **辅助功能** 和 **输入监控** 中允许对应应用。授权可能归属启动终端或程序本身，以系统显示为准。

第 2 步应显示 `accessibility=true`、`input-monitoring=true`、`post-events=true`。不需要完全磁盘访问。

**紧急停止：`Control + Option + Shift + Escape`**——本地保留，不转发。也可用 Ctrl+C 或 SIGTERM。

### 选项

| 选项 | 行为 |
|---|---|
| （无）/ `--check` | 检查权限和当前窗口，不捕获 |
| `--request-permissions` | 请求系统授权后退出 |
| `--run` | 远程窗口聚焦时捕获并转发 |
| `--dry-run` | 只观察路由，不拦截不注入 |
| `--window-title TEXT` | 要求焦点窗口标题包含 TEXT（区分大小写） |
| `--duration SECONDS` | 到时自动停止 |
| `--help` | 显示帮助 |

## 原理

安装公开 `CGEventTap`，通过 `CGEventPostToPid` 定向投递。通过 `NSWorkspace` 和辅助功能 API 检测聚焦窗口。修饰键以状态变化方式转发，保留左右区分。详见 [架构说明](docs/architecture.md)。

## 限制

- 窗口检测是启发式的——用 `--window-title` 收窄范围。
- 焦点必须属于远程会话，仅打开屏幕共享不够。
- 更早的输入钩子（macOS 或第三方工具）可能抢先消费事件。
- 焦点切换时按键释放清理是尽力而为。
- macOS 可能禁用事件 tap——届时停止捕获，不反复抢回。

鼠标、触控板、媒体、亮度和电源事件不在捕获范围内。

## 隐私

不记录按键文本、键码、剪贴板内容或远端终端内容。不发起网络连接。

## 许可证

[MIT](LICENSE)
