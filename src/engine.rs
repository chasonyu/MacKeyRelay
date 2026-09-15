//! Platform-independent ownership of key-down/key-up pairs.
use std::collections::BTreeMap;

pub const SHIFT: u64 = 1 << 17;
pub const CONTROL: u64 = 1 << 18;
pub const OPTION: u64 = 1 << 19;
pub const COMMAND: u64 = 1 << 20;
pub const MODIFIERS: &[(u64, u16)] = &[
    (0x1, 59),
    (0x2, 56),
    (0x4, 60),
    (0x8, 55),
    (0x10, 54),
    (0x20, 58),
    (0x40, 61),
    (0x2000, 62),
    (0x10000, 57),
    (0x800000, 63),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route {
    Local,
    Remote(i32),
    Swallow,
}

#[derive(Default)]
pub struct Router {
    // None means a key was released remotely on focus loss; swallow its later physical up/repeats.
    held: BTreeMap<u16, Option<i32>>,
    local_held: std::collections::BTreeSet<u16>,
    pub target: Option<i32>,
}

impl Router {
    pub fn set_target(&mut self, target: Option<i32>) -> Vec<(u16, i32)> {
        if self.target == target {
            return vec![];
        }
        let mut releases = vec![];
        for (&key, owner) in &mut self.held {
            if let Some(pid) = owner.take() {
                releases.push((key, pid));
            }
        }
        self.target = target;
        releases
    }
    pub fn down(&mut self, key: u16, repeat: bool) -> Route {
        if self.local_held.contains(&key) {
            return Route::Local;
        }
        if let Some(owner) = self.held.get(&key) {
            return owner.map_or(Route::Swallow, Route::Remote);
        }
        // A repeat first observed after startup belongs to the original local press.
        if repeat || self.target.is_none() {
            self.local_held.insert(key);
            return Route::Local;
        }
        let pid = self.target.unwrap();
        self.held.insert(key, Some(pid));
        Route::Remote(pid)
    }
    pub fn up(&mut self, key: u16) -> Route {
        if self.local_held.remove(&key) {
            return Route::Local;
        }
        self.held
            .remove(&key)
            .map_or(Route::Local, |p| p.map_or(Route::Swallow, Route::Remote))
    }
    pub fn abandon_down(&mut self, key: u16) {
        self.held.remove(&key);
        self.local_held.insert(key);
    }
}

pub fn emergency(key: u16, flags: u64) -> bool {
    key == 53 && flags & (SHIFT | CONTROL | OPTION | COMMAND) == SHIFT | CONTROL | OPTION
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn focus_loss_releases_once_and_swallows_physical_up() {
        let mut r = Router::default();
        r.set_target(Some(12));
        assert_eq!(r.down(35, false), Route::Remote(12));
        assert_eq!(r.set_target(None), vec![(35, 12)]);
        assert!(r.set_target(None).is_empty());
        assert_eq!(r.down(35, true), Route::Swallow);
        assert_eq!(r.up(35), Route::Swallow);
        assert_eq!(r.down(35, false), Route::Local);
    }
    #[test]
    fn local_press_cannot_be_migrated_mid_hold() {
        let mut r = Router::default();
        assert_eq!(r.down(0, false), Route::Local);
        r.set_target(Some(9));
        assert_eq!(r.down(0, true), Route::Local);
        assert_eq!(r.up(0), Route::Local);
        assert_eq!(r.down(0, false), Route::Remote(9));
    }
    #[test]
    fn repeats_preserve_owner_and_release_on_target_change() {
        let mut r = Router::default();
        r.set_target(Some(1));
        r.down(0, false);
        assert_eq!(r.down(0, true), Route::Remote(1));
        assert_eq!(r.set_target(Some(2)), vec![(0, 1)]);
        assert_eq!(r.down(0, true), Route::Swallow);
        r.up(0);
        assert_eq!(r.down(0, false), Route::Remote(2));
    }
    #[test]
    fn inherited_repeat_is_not_captured() {
        let mut r = Router::default();
        r.set_target(Some(3));
        assert_eq!(r.down(0, true), Route::Local);
        assert_eq!(r.up(0), Route::Local);
    }
    #[test]
    fn failed_injection_returns_ownership_to_local() {
        let mut r = Router::default();
        r.set_target(Some(3));
        r.down(0, false);
        r.abandon_down(0);
        assert_eq!(r.up(0), Route::Local);
        assert!(r.set_target(None).is_empty());
    }
    #[test]
    fn emergency_requires_exact_modifiers() {
        assert!(emergency(53, CONTROL | OPTION | SHIFT | 0x23));
        assert!(!emergency(53, CONTROL | OPTION | SHIFT | COMMAND));
        assert!(!emergency(35, CONTROL | OPTION | SHIFT));
    }
}
