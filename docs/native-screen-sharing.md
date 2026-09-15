# 苹果屏幕共享键盘转发：本机反汇编记录

## 范围与方法

- 分析日期：2026-09-15。
- 本机 macOS 14.8.7（23J520），Screen Sharing 4.3，分析 arm64 执行路径。
- 主程序：`/System/Applications/Utilities/Screen Sharing.app/Contents/MacOS/Screen Sharing`。
- 主程序 SHA-256：`0c1d147b68461b474274554b5829401155ab0443519a33fa5d41955b095f6b8b`。
- 核心方法在系统共享缓存中的 `ScreenSharing.framework`。在独立检查进程内加载框架，通过 Objective-C runtime 定位 IMP，使用 Xcode libLTO 的 LLVM 反汇编器解析；未附加、注入或修改正在运行的屏幕共享进程。
- 这是部分反汇编与等价逻辑整理，不是恢复苹果原始源码。未调用私有方法控制连接。
- 地址均为框架镜像相对偏移，仅适用于本次版本；不应硬编码到转发工具。

## 发现 1：普通键和修饰键走不同分支

`-[SSApplication sendEvent:]`（0x2274）在 input event consumer 存在时：

- 事件类型 10（keyDown）：从 NSEvent 取 keyCode，构造 SSKeyboardEvent，state=0，送到 consumer 的 `ssInputEvent:`。
- 类型 11（keyUp）：同样处理，state=1。
- 类型 12（flagsChanged）：从 NSEvent 取 modifierFlags，调用 `sendChangedModifierFlags:`。
- 普通 keyDown/keyUp 分支没有调用修饰键差异同步函数。
- consumer 不存在时回退父类事件处理。缓冲区视图自身还有 keyDown/keyUp 转发入口。

因此，给 A 的 keyDown 加一个 Command 标志，不等于发送了 Command 的按下事件。

## 发现 2：修饰键同步使用设备级左右键标志

`-[SSApplication sendChangedModifierFlags:]`（0x16ccc）调用 `SSSendChangedModifierFlags`（0x16d1c），传入旧 flags、新 flags、consumer，然后保存新 flags。

以下是由汇编整理的伪代码，不是苹果源码：

```text
changed = oldFlags XOR newFlags
for (mask, keyCode) in modifierTable:
    if changed AND mask:
        state = 0 if newFlags AND mask else 1
        consumer.ssInputEvent(keyboardEvent(keyCode, state))
lastModifierFlags = newFlags
```

从函数引用的常量表读出的全部 10 项：

| flag mask | macOS keyCode | 键 |
|---|---:|---|
| 0x1 | 59 | 左 Control |
| 0x2 | 56 | 左 Shift |
| 0x4 | 60 | 右 Shift |
| 0x8 | 55 | 左 Command |
| 0x10 | 54 | 右 Command |
| 0x20 | 58 | 左 Option |
| 0x40 | 61 | 右 Option |
| 0x2000 | 62 | 右 Control |
| 0x10000 | 57 | Caps Lock |
| 0x800000 | 63 | Fn |

左右修饰键 mask 名称已与本机 SDK `IOKit/hidsystem/IOLLEvent.h` 核对。

关键：通用 Command mask（0x100000）不是表中的 Command 项。转发时必须保留设备级左右标志，不能只用通用的 `.maskCommand` 重建事件。本次成功的探针同时设置 `.maskCommand | 0x8`，并发送 flagsChanged。

## 发现 3：真正的条件是远程会话窗口的键盘焦点

`-[SSSessionView configureInputEventConsumer]`（0x45458）依次判断：

```text
if window.isKeyWindow:
    consumer = session if isConnected AND isControlling else nil
    coordinator.setActiveConsumer(consumer)
else:
    coordinator.deactivateConsumer(session)
```

`windowDidBecomeKey:`（0x2785c）和 `windowDidResignKey:`（0x279e0）均调用此方法。

这比“Screen Sharing App 在前台”更严格：连接列表、其他窗口或弹窗处于前台，不自动等于正确远程窗口可接收完整组合键。

`-[SSApplication ssSetInputEventConsumer:]`（0x16b94）在接收者改变前先以 flags=0 同步旧接收者，随后更换接收者；设置新接收者后读取 `[NSEvent modifierFlags]` 并同步。这是原生的修饰键释放／重新同步行为。

## 发现 4：后台普通字母有另一条可能路径

`-[SSFrameBufferView keyDown:]`（0x632d4）和 `keyUp:`（0x63388）也会读取 keyCode、创建 SSKeyboardEvent 并发送给视图自己的 consumer，未见修饰键差异同步。

这与测试中“后台 p/a 可到达，但组合键变成字母”相符。不过没有对正在运行的进程做调用追踪，不能断言每次后台事件一定走了这个入口。

## 发现 5：后续发送与特殊键捕获

- `SSEventSession.stSendKeyboardEvent:`（0xbd368）和 `SSSession.stSendKeyboardEvent:`（0x71608）读取 keyCode、keyState、timestamp，并调用 `RFBPostKeyEvent`。这里只确认所检查的发送入口，不据此断言所有高性能会话底层传输细节相同。
- `SSEventHelperManager.captureSpecialKeys:`（0x16f7c）调用 `EventHelperGrabKeys_rpc`；consumer 启停对应 capture／stopKeyCapture。
- 发现特殊键捕获机制，不等于证明它能压过 Keyboard Maestro 或其他所有全局热键。没有验证存在可配置的“全部键盘独占”开关。

## 对转发程序的直接指导

1. 使用用户态公开的 CGEvent 定向投递入口，不依赖上述私有类、偏移或 RPC。
2. 目标应为持有键盘焦点的远程控制窗口；焦点离开时停止普通事件转发。
3. 保留原始 keyDown、keyUp、flagsChanged 及左右修饰键标志。不要只转发字符，也不要把修饰键简化为 Command/Shift 等通用集合。
4. 维持按键状态、顺序、重复事件；发送时避免再经过全局热键入口。
5. 焦点切换和连接断开时处理释放状态。苹果已处理部分修饰键清理，但本地拦截器仍要管理自己吞掉的 keyUp，不能假定原生会清理所有普通键。
6. 实际拦截层尚未接入。Karabiner 的每键 shell_command 并非适合连续事件流的最终结构，应另行验证常驻拦截器或与 Karabiner 配合的事件通道。
7. 防止自身注入事件被再次捕获；设置事件来源标记并明确排除，或让拦截层与定向投递通路分离。

## 已完成的人工实测与边界

- 定向投递普通 p：远端显示 p。
- 后台携带通用 Command flags 的 A：显示 a。
- 后台发送带左右位的完整 Command 序列：仍显示 a。
- 物理按住 Command，前台定向投递 a：全选。
- 松开物理按键，前台完整人工 Command+A：全选。
- 前台完整人工 Command+P：远端应用菜单弹出，本机未同时弹出。

前后台与事件格式曾在早期测试中共同变化，不能把所有失败归为单一因素；反汇编现在明确显示两者都有作用。尚未验证所有快捷键、长按、焦点边界、Karabiner 实际吞键及与其他输入工具的兼容性。

## 文件

- `native-disassembly.txt`：所检查方法的汇编及解析出的 modifier 常量表。
- `disassemble.m`：本次自编写的方法定位／LLVM 反汇编工具。
- `methods.txt`：框架相关 Objective-C 方法定位结果。
- 原有单键定向投递探针位于 `/private/tmp/screen-sharing-key-probe/`。
