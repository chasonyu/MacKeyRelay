//! Minimal public macOS APIs. No private ScreenSharing framework calls.
#![allow(non_snake_case)]
use std::ffi::{c_char, c_void};
pub type Ref = *mut c_void;
pub type EventCallback = unsafe extern "C" fn(Ref, u32, Ref, Ref) -> Ref;
#[repr(C)]
pub struct TimerContext {
    pub version: isize,
    pub info: Ref,
    pub retain: Ref,
    pub release: Ref,
    pub description: Ref,
}
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    pub fn CGPreflightListenEventAccess() -> bool;
    pub fn CGRequestListenEventAccess() -> bool;
    pub fn CGRequestPostEventAccess() -> bool;
    pub fn CGPreflightPostEventAccess() -> bool;
    pub fn CGEventTapCreate(
        tap: u32,
        place: u32,
        options: u32,
        mask: u64,
        callback: EventCallback,
        info: Ref,
    ) -> Ref;
    pub fn CGEventTapEnable(tap: Ref, enable: bool);
    pub fn CGEventCreateCopy(event: Ref) -> Ref;
    pub fn CGEventCreateKeyboardEvent(source: Ref, key: u16, down: bool) -> Ref;
    pub fn CGEventSourceCreate(state: i32) -> Ref;
    pub fn CGEventGetFlags(event: Ref) -> u64;
    pub fn CGEventSetFlags(event: Ref, flags: u64);
    pub fn CGEventGetIntegerValueField(event: Ref, field: u32) -> i64;
    pub fn CGEventSetIntegerValueField(event: Ref, field: u32, value: i64);
    pub fn CGEventSetType(event: Ref, kind: u32);
    pub fn CGEventPostToPid(pid: i32, event: Ref);
}
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    pub fn AXIsProcessTrusted() -> bool;
    pub fn AXIsProcessTrustedWithOptions(options: Ref) -> bool;
    pub static kAXTrustedCheckOptionPrompt: Ref;
    pub fn AXUIElementCreateApplication(pid: i32) -> Ref;
    pub fn AXUIElementCopyAttributeValue(element: Ref, attribute: Ref, value: *mut Ref) -> i32;
    pub fn AXUIElementSetMessagingTimeout(element: Ref, seconds: f32) -> i32;
}
#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    pub static kCFRunLoopCommonModes: Ref;
    pub static kCFBooleanTrue: Ref;
    pub fn CFDictionaryCreate(
        allocator: Ref,
        keys: *const Ref,
        values: *const Ref,
        count: isize,
        key_callbacks: Ref,
        value_callbacks: Ref,
    ) -> Ref;
    pub fn CFRelease(value: Ref);
    pub fn CFGetTypeID(value: Ref) -> usize;
    pub fn CFStringGetTypeID() -> usize;
    pub fn CFArrayGetTypeID() -> usize;
    pub fn CFArrayGetCount(array: Ref) -> isize;
    pub fn CFStringCreateWithCString(allocator: Ref, text: *const c_char, encoding: u32) -> Ref;
    pub fn CFStringGetCString(value: Ref, buffer: *mut c_char, size: isize, encoding: u32) -> bool;
    pub fn CFMachPortCreateRunLoopSource(allocator: Ref, port: Ref, order: isize) -> Ref;
    pub fn CFMachPortInvalidate(port: Ref);
    pub fn CFRunLoopGetCurrent() -> Ref;
    pub fn CFRunLoopAddSource(loop_: Ref, source: Ref, mode: Ref);
    pub fn CFRunLoopRemoveSource(loop_: Ref, source: Ref, mode: Ref);
    pub fn CFRunLoopRun();
    pub fn CFRunLoopStop(loop_: Ref);
    pub fn CFAbsoluteTimeGetCurrent() -> f64;
    pub fn CFRunLoopTimerCreate(
        allocator: Ref,
        fire: f64,
        interval: f64,
        flags: usize,
        order: isize,
        callback: unsafe extern "C" fn(Ref, Ref),
        context: *mut TimerContext,
    ) -> Ref;
    pub fn CFRunLoopAddTimer(loop_: Ref, timer: Ref, mode: Ref);
    pub fn CFRunLoopTimerInvalidate(timer: Ref);
}
#[link(name = "AppKit", kind = "framework")]
extern "C" {}
#[link(name = "objc")]
extern "C" {
    pub fn objc_getClass(name: *const c_char) -> Ref;
    pub fn sel_registerName(name: *const c_char) -> Ref;
    pub fn objc_msgSend();
    pub fn objc_autoreleasePoolPush() -> Ref;
    pub fn objc_autoreleasePoolPop(pool: Ref);
}
extern "C" {
    pub fn geteuid() -> u32;
    pub fn signal(sig: i32, handler: extern "C" fn(i32)) -> usize;
}
