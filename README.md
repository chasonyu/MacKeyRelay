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

## Build

Requires macOS, Rust 1.85+, and Xcode Command Line Tools.

```sh
cargo build --release
```

## Usage

Run as your normal desktop user. **Do not use `sudo`** — root does not replace macOS privacy consent.

```sh
# 1. Request system permissions
./target/release/mackeyrelay --request-permissions

# 2. Verify permissions and focused window
./target/release/mackeyrelay --check

# 3. Run
./target/release/mackeyrelay --run
```

After step 1, check **System Settings → Privacy & Security → Accessibility** and **Input Monitoring**. Consent may be attributed to the launching terminal or the executable — use the name macOS shows.

Step 2 should report `accessibility=true`, `input-monitoring=true`, `post-events=true`. Full Disk Access is not required.

**Emergency stop: `Control + Option + Shift + Escape`** — reserved locally, not forwarded. Also stop via Ctrl+C or SIGTERM.

### Options

| Option | Behavior |
|---|---|
| (none) / `--check` | Check permissions and focused window; no capture |
| `--request-permissions` | Request system consent and exit |
| `--run` | Capture and forward while a remote window is focused |
| `--dry-run` | Observe routing without suppressing or injecting events |
| `--window-title TEXT` | Require focused window title to contain TEXT (case-sensitive) |
| `--duration SECONDS` | Stop after the specified interval |
| `--help` | Show command help |

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
