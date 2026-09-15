use crate::{
    engine::{self, Route, Router},
    ffi::*,
    Options,
};
use std::{
    ffi::{c_char, c_void, CStr, CString},
    ptr,
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};
const DOWN: u32 = 10;
const UP: u32 = 11;
const FLAGS: u32 = 12;
const KEYCODE: u32 = 9;
const AUTOREPEAT: u32 = 8;
const USER_DATA: u32 = 42;
const MARK: i64 = 0x5353494e50555401;
static STOP: AtomicBool = AtomicBool::new(false);
extern "C" fn signal_stop(_: i32) {
    STOP.store(true, Ordering::Relaxed);
}
struct Owned(Ref);
impl Drop for Owned {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { CFRelease(self.0) }
        }
    }
}
struct Pool(Ref);
impl Pool {
    unsafe fn new() -> Self {
        Self(objc_autoreleasePoolPush())
    }
}
impl Drop for Pool {
    fn drop(&mut self) {
        unsafe { objc_autoreleasePoolPop(self.0) }
    }
}
unsafe fn msg(obj: Ref, sel: &CStr) -> Ref {
    let f: unsafe extern "C" fn(Ref, Ref) -> Ref = std::mem::transmute(objc_msgSend as *const ());
    f(obj, sel_registerName(sel.as_ptr()))
}
unsafe fn number(obj: Ref, sel: &CStr) -> i32 {
    let f: unsafe extern "C" fn(Ref, Ref) -> i32 = std::mem::transmute(objc_msgSend as *const ());
    f(obj, sel_registerName(sel.as_ptr()))
}
unsafe fn string(obj: Ref) -> String {
    if obj.is_null() {
        return String::new();
    }
    let p = msg(obj, c"UTF8String") as *const c_char;
    if p.is_null() {
        String::new()
    } else {
        CStr::from_ptr(p).to_string_lossy().into_owned()
    }
}
unsafe fn front() -> Option<i32> {
    let _pool = Pool::new();
    let cls = objc_getClass(c"NSWorkspace".as_ptr());
    let ws = msg(cls, c"sharedWorkspace");
    let app = msg(ws, c"frontmostApplication");
    if app.is_null() || string(msg(app, c"bundleIdentifier")) != "com.apple.ScreenSharing" {
        None
    } else {
        Some(number(app, c"processIdentifier"))
    }
}
unsafe fn attr(element: Ref, name: &str) -> Option<Owned> {
    let n = CString::new(name).ok()?;
    let cf = Owned(CFStringCreateWithCString(
        ptr::null_mut(),
        n.as_ptr(),
        0x08000100,
    ));
    let mut value = ptr::null_mut();
    if AXUIElementCopyAttributeValue(element, cf.0, &mut value) == 0 && !value.is_null() {
        Some(Owned(value))
    } else {
        None
    }
}
unsafe fn attr_string(element: Ref, name: &str) -> String {
    let Some(v) = attr(element, name) else {
        return String::new();
    };
    if CFGetTypeID(v.0) != CFStringGetTypeID() {
        return String::new();
    }
    let mut b = [0i8; 4096];
    if CFStringGetCString(v.0, b.as_mut_ptr(), 4096, 0x08000100) {
        CStr::from_ptr(b.as_ptr()).to_string_lossy().into_owned()
    } else {
        String::new()
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct Focus {
    pid: i32,
    title: String,
    eligible: bool,
    reason: &'static str,
}
unsafe fn focus(filter: Option<&str>) -> Option<Focus> {
    let pid = front()?;
    let app = Owned(AXUIElementCreateApplication(pid));
    AXUIElementSetMessagingTimeout(app.0, 0.025);
    let Some(w) = attr(app.0, "AXFocusedWindow") else {
        return Some(Focus {
            pid,
            title: String::new(),
            eligible: false,
            reason: "no accessible focused window",
        });
    };
    let title = attr_string(w.0, "AXTitle");
    let subrole = attr_string(w.0, "AXSubrole");
    let sheet = attr(w.0, "AXSheets")
        .is_some_and(|a| CFGetTypeID(a.0) == CFArrayGetTypeID() && CFArrayGetCount(a.0) > 0);
    let utility = [
        "screen sharing",
        "屏幕共享",
        "螢幕共享",
        "all connections",
        "所有连接",
        "所有連線",
        "settings",
        "设置",
        "設定",
        "preferences",
        "偏好设置",
        "connection info",
        "连接信息",
    ];
    let reason = if sheet {
        "window has a sheet"
    } else if subrole != "AXStandardWindow" {
        "not a standard session window"
    } else if title.is_empty() || utility.iter().any(|s| title.to_lowercase() == *s) {
        "connection list or utility window"
    } else if filter.is_some_and(|s| !title.contains(s)) {
        "window title does not match"
    } else {
        "eligible"
    };
    Some(Focus {
        pid,
        title,
        eligible: reason == "eligible",
        reason,
    })
}
struct State {
    options: Options,
    router: Router,
    focus: Option<Focus>,
    source: Owned,
    loop_: Ref,
    remote_flags: u64,
    started: Instant,
    forwarded: u64,
    transitions: u64,
}
impl State {
    unsafe fn post(&mut self, pid: i32, event: Ref) -> bool {
        if self.options.dry_run {
            return true;
        }
        let copied = Owned(CGEventCreateCopy(event));
        if copied.0.is_null() {
            return false;
        }
        CGEventSetIntegerValueField(copied.0, USER_DATA, MARK);
        CGEventPostToPid(pid, copied.0);
        self.forwarded += 1;
        true
    }
    unsafe fn key_up(&mut self, pid: i32, key: u16) {
        let e = Owned(CGEventCreateKeyboardEvent(self.source.0, key, false));
        if e.0.is_null() {
            return;
        }
        CGEventSetFlags(e.0, self.remote_flags);
        self.post(pid, e.0);
    }
    unsafe fn sync_flags(&mut self, pid: i32, flags: u64) -> bool {
        if flags == self.remote_flags {
            return true;
        }
        let key = engine::MODIFIERS
            .iter()
            .find(|(mask, _)| (self.remote_flags ^ flags) & mask != 0)
            .map_or(55, |(_, k)| *k);
        let e = Owned(CGEventCreateKeyboardEvent(self.source.0, key, flags != 0));
        if e.0.is_null() {
            return false;
        }
        CGEventSetType(e.0, FLAGS);
        CGEventSetFlags(e.0, flags);
        if !self.post(pid, e.0) {
            return false;
        }
        self.remote_flags = flags;
        true
    }
    unsafe fn change_focus(&mut self, next: Option<Focus>) {
        if self.focus == next {
            return;
        }
        // Even two session windows in the same PID must end the previous key sequence.
        let old = self.router.target;
        for (key, pid) in self.router.set_target(None) {
            self.key_up(pid, key);
        }
        if let Some(pid) = old {
            self.sync_flags(pid, 0);
        }
        self.remote_flags = 0;
        self.router
            .set_target(next.as_ref().filter(|f| f.eligible).map(|f| f.pid));
        self.transitions += 1;
        eprintln!(
            "routing: {}",
            if self.router.target.is_some() {
                "remote"
            } else {
                "local"
            }
        );
        self.focus = next;
    }
    unsafe fn refresh(&mut self) {
        self.change_focus(focus(self.options.window_title.as_deref()));
    }
    unsafe fn stop(&mut self) {
        self.change_focus(None);
        CFRunLoopStop(self.loop_);
    }
}
unsafe extern "C" fn event_callback(_: Ref, kind: u32, event: Ref, info: Ref) -> Ref {
    let s = &mut *(info as *mut State);
    if kind == u32::MAX || kind == u32::MAX - 1 {
        // Do not automatically grab the keyboard again after macOS disables our tap.
        eprintln!("event tap disabled; stopping and releasing tracked keys");
        s.stop();
        return event;
    }
    if event.is_null() || CGEventGetIntegerValueField(event, USER_DATA) == MARK {
        return event;
    }
    let flags = CGEventGetFlags(event);
    let key = CGEventGetIntegerValueField(event, KEYCODE) as u16;
    if kind == DOWN && engine::emergency(key, flags) {
        eprintln!("emergency stop");
        s.stop();
        return if s.options.dry_run {
            event
        } else {
            ptr::null_mut()
        };
    }
    // Fast application check for every event. Revalidate the AX window on a new press.
    if front() != s.router.target
        || (kind == DOWN && CGEventGetIntegerValueField(event, AUTOREPEAT) == 0)
    {
        s.refresh();
    }
    let route = match kind {
        DOWN => s
            .router
            .down(key, CGEventGetIntegerValueField(event, AUTOREPEAT) != 0),
        UP => s.router.up(key),
        FLAGS => s.router.target.map_or(Route::Local, Route::Remote),
        _ => Route::Local,
    };
    match route {
        Route::Local => event,
        Route::Swallow => {
            if s.options.dry_run {
                event
            } else {
                ptr::null_mut()
            }
        }
        Route::Remote(pid) => {
            // Repair state when capture begins with a modifier already physically held.
            let ok = if kind == FLAGS {
                let ok = s.post(pid, event);
                if ok {
                    s.remote_flags = flags;
                }
                ok
            } else {
                s.sync_flags(pid, flags) && s.post(pid, event)
            };
            if !ok {
                if kind == DOWN {
                    s.router.abandon_down(key);
                }
                eprintln!("event allocation failed; stopping capture");
                s.stop();
                return event;
            }
            if s.options.dry_run {
                event
            } else {
                ptr::null_mut()
            }
        }
    }
}
unsafe extern "C" fn timer_callback(_: Ref, info: Ref) {
    let s = &mut *(info as *mut State);
    if STOP.load(Ordering::Relaxed)
        || s.options
            .duration
            .is_some_and(|n| s.started.elapsed().as_secs_f64() >= n)
    {
        s.stop();
    } else {
        s.refresh();
    }
}
pub fn start(options: Options) -> Result<(), String> {
    unsafe {
        if geteuid() == 0 {
            return Err("不要使用 sudo：root 不能替代 macOS 隐私授权。请以当前桌面用户执行 --request-permissions。".into());
        }
        if options.request_permissions {
            println!("请求系统授权，不启动键盘拦截。请按系统提示授权实际启动应用（例如 Terminal）或本程序。");
            let keys = [kAXTrustedCheckOptionPrompt];
            let values = [kCFBooleanTrue];
            let dict = Owned(CFDictionaryCreate(
                ptr::null_mut(),
                keys.as_ptr(),
                values.as_ptr(),
                1,
                ptr::null_mut(),
                ptr::null_mut(),
            ));
            if dict.0.is_null() {
                return Err("Cannot allocate permission prompt options".into());
            }
            AXIsProcessTrustedWithOptions(dict.0);
            CGRequestListenEventAccess();
            CGRequestPostEventAccess();
            println!("授权是异步完成的。请检查系统设置 → 隐私与安全性 → 辅助功能、输入监控；必要时完全退出并重开启动终端，再执行 --check。无需完全磁盘访问，也不要重置整个 TCC 数据库。");
            return Ok(());
        }
        let ax = AXIsProcessTrusted();
        let listen = CGPreflightListenEventAccess();
        let post = CGPreflightPostEventAccess();
        println!("permissions: accessibility={ax} input-monitoring={listen} post-events={post}");
        if !options.run {
            println!("mode: check (no interception or injection)");
            if ax {
                match focus(options.window_title.as_deref()) {
                    Some(f) => println!(
                        "Screen Sharing PID={} title={:?} eligible={} ({})",
                        f.pid, f.title, f.eligible, f.reason
                    ),
                    None => println!("Screen Sharing is not the frontmost application"),
                }
            } else {
                println!("Window eligibility cannot be checked without Accessibility access");
            }
            return Ok(());
        }
        if !ax || !listen || (!options.dry_run && !post) {
            return Err("缺少 macOS 隐私权限。请不要用 sudo；执行 mackeyrelay --request-permissions，按系统提示授权启动应用或本程序，然后重启该终端并运行 mackeyrelay --check。本次没有请求授权或启动拦截。".into());
        }
        STOP.store(false, Ordering::Relaxed);
        let source = Owned(CGEventSourceCreate(-1));
        if source.0.is_null() {
            return Err("Cannot allocate private event source".into());
        }
        let mut s = Box::new(State {
            options,
            router: Router::default(),
            focus: None,
            source,
            loop_: CFRunLoopGetCurrent(),
            remote_flags: 0,
            started: Instant::now(),
            forwarded: 0,
            transitions: 0,
        });
        let info = &mut *s as *mut State as *mut c_void;
        let tap = Owned(CGEventTapCreate(
            0,
            0,
            u32::from(s.options.dry_run),
            (1 << DOWN) | (1 << UP) | (1 << FLAGS),
            event_callback,
            info,
        ));
        if tap.0.is_null() {
            return Err(
                "Cannot create keyboard event tap; check Accessibility and Input Monitoring".into(),
            );
        }
        let run_source = Owned(CFMachPortCreateRunLoopSource(ptr::null_mut(), tap.0, 0));
        if run_source.0.is_null() {
            CFMachPortInvalidate(tap.0);
            return Err("Cannot create run-loop source".into());
        }
        let mut context = TimerContext {
            version: 0,
            info,
            retain: ptr::null_mut(),
            release: ptr::null_mut(),
            description: ptr::null_mut(),
        };
        let timer = Owned(CFRunLoopTimerCreate(
            ptr::null_mut(),
            CFAbsoluteTimeGetCurrent(),
            0.1,
            0,
            0,
            timer_callback,
            &mut context,
        ));
        if timer.0.is_null() {
            CFMachPortInvalidate(tap.0);
            return Err("Cannot create focus timer".into());
        }
        signal(2, signal_stop);
        signal(15, signal_stop);
        CFRunLoopAddSource(s.loop_, run_source.0, kCFRunLoopCommonModes);
        CFRunLoopAddTimer(s.loop_, timer.0, kCFRunLoopCommonModes);
        CGEventTapEnable(tap.0, true);
        s.refresh();
        eprintln!(
            "{}; emergency stop: Control+Option+Shift+Escape; no key contents are logged",
            if s.options.dry_run {
                "dry-run (no forwarding)"
            } else {
                "capture enabled"
            }
        );
        CFRunLoopRun();
        s.change_focus(None);
        CGEventTapEnable(tap.0, false);
        CFRunLoopTimerInvalidate(timer.0);
        CFRunLoopRemoveSource(s.loop_, run_source.0, kCFRunLoopCommonModes);
        CFMachPortInvalidate(tap.0);
        eprintln!(
            "stopped: posted_events={} focus_transitions={}",
            s.forwarded, s.transitions
        );
        Ok(())
    }
}
