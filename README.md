# MacKeyRelay

**Forward keyboard shortcuts to macOS Screen Sharing without triggering local hotkeys.**

English | [简体中文](README.zh-CN.md)

MacKeyRelay captures local keyboard events and posts them directly to Apple's Screen Sharing app while a remote session window has focus. Switch away, and input stays local. Nothing needs to be installed on the remote Mac.

```text
Keyboard → MacKeyRelay → Screen Sharing → Remote Mac
```

It uses public macOS event APIs (`CGEventTap`, `CGEventPostToPid`) — no patching, no private APIs, no separate network protocol. Karabiner-Elements is not required.

## Compatibility

**Tested environment — not a minimum-version guarantee.**

| Component | Verified |
|---|---|
| Local macOS | Sonoma 14.8.7 (build 23J520) |
| Screen Sharing client | 4.3 (build 594.2.2) |
| Architecture | Apple Silicon (arm64) |
| Rust toolchain | 1.85.0 |

Other macOS versions, Screen Sharing builds, and Intel Macs have not been validated.

## Install

```sh
npm install -g @chasonyu/mackeyrelay
```

Or build from source (requires Rust 1.85+ and Xcode Command Line Tools):

```sh
cargo build --release
# use ./target/release/mackeyrelay instead of mackeyrelay below
```

## Usage

Run as your normal desktop user. **Do not use `sudo`** — root does not replace macOS privacy consent.

```sh
# 1. Request system permissions
mackeyrelay --request-permissions

# 2. Verify permissions
mackeyrelay --check

# 3. Run in foreground
mackeyrelay --run

# Or run as a background service (launchd)
mackeyrelay start
mackeyrelay stop
mackeyrelay status
```

After step 1, open **System Settings → Privacy & Security → Accessibility** and **Input Monitoring**, and allow the terminal or the binary itself. Then **restart the terminal** and run `--check`.

Step 2 should report `accessibility=true`, `input-monitoring=true`, `post-events=true`.

### Background service

| Command | Behavior |
|---|---|
| `start [OPTIONS]` | Start as a launchd background service |
| `stop` | Stop the background service |
| `status` | Show service state and last permission status |
| `logs` | Show last 80 log lines |
| `autostart enable` | Start automatically on login |
| `autostart disable` | Do not start on login |

The background service runs `--run` by default. Pass extra options to `start`:

```sh
mackeyrelay start --window-title 'MacBook'
```

### Foreground options

| Option | Behavior |
|---|---|
| (none) / `--check` | Check permissions and focused window |
| `--run` | Capture and forward while a remote window is focused |
| `--dry-run` | Observe routing without suppressing or injecting events |
| `--window-title TEXT` | Require focused window title to contain TEXT (case-sensitive) |
| `--duration SECONDS` | Stop after the specified interval |
| `--request-permissions` | Request system consent and exit |

**Emergency stop: `Control + Option + Shift + Escape`** — reserved locally, not forwarded. Also stop via Ctrl+C or SIGTERM.

## How it works

Installs a public `CGEventTap`, delivers events via `CGEventPostToPid`. Detects the focused window through `NSWorkspace` and Accessibility APIs. Modifiers are forwarded as state changes, including left/right flags. See [architecture notes](docs/architecture.md).

## Limitations

- Window detection is heuristic — use `--window-title` for a narrower scope.
- Focus must belong to the remote session, not just having Screen Sharing open.
- Earlier input hooks (macOS or third-party tools) can still consume events first.
- Key-release cleanup on focus changes is best-effort.
- macOS can disable an event tap — MacKeyRelay stops rather than reclaiming.

Mouse, trackpad, media, brightness, and power events are not captured.

## Privacy

Does not log typed text, key codes, clipboard contents, or remote terminal contents. Does not open network connections.

## License

[MIT](LICENSE)
