//! 单独按住右Ctrl的判定；重复键不重置计时，任何组合键或取消都作废。
use std::cell::Cell;
use std::time::{Duration, Instant};

#[derive(Default)]
pub(crate) struct SchemeHold {
    down: Cell<bool>,

    started: Cell<Option<Instant>>,
}

impl SchemeHold {
    pub(crate) fn key_down(&self, right: bool, repeat: bool, eligible: bool, now: Instant) -> bool {
        if !right {
            self.cancel();
            return false;
        }
        if repeat || self.down.replace(true) {
            return false;
        }
        if eligible {
            self.started.set(Some(now));
            true
        } else {
            self.cancel();
            false
        }
    }

    pub(crate) fn key_up(&self, right: bool, now: Instant, threshold: Duration) -> bool {
        if !right {
            self.cancel();
            return false;
        }
        self.down.set(false);
        self.started
            .take()
            .is_some_and(|start| now.duration_since(start) >= threshold)
    }

    pub(crate) fn cancel(&self) {
        self.started.set(None);
    }

    pub(crate) fn reset(&self) {
        self.cancel();
        self.down.set(false);
    }
}

pub(crate) fn is_right_control(vk: u32, lparam: isize) -> bool {
    vk == 0xa3 || (vk == 0x11 && ((lparam >> 24) & 1) != 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn short_press_ignored_long_release_fires_once() {
        let h = SchemeHold::default();
        let t = Instant::now();
        let duration = Duration::from_millis(600);
        h.key_down(true, false, true, t);
        assert!(!h.key_up(true, t + Duration::from_millis(599), duration));
        h.key_down(true, false, true, t);
        assert!(h.key_up(true, t + duration, duration));
        assert!(!h.key_up(true, t + duration, duration));
    }
    #[test]
    fn chords_and_mouse_cancel_do_not_toggle_even_after_long_wait() {
        for key in [false, true] {
            let h = SchemeHold::default();
            let t = Instant::now();
            h.key_down(true, false, true, t);
            if key {
                h.key_down(false, false, true, t);
            } else {
                h.cancel();
            }
            assert!(!h.key_up(true, t + Duration::from_secs(1), Duration::from_millis(600)));
        }
    }
    #[test]
    fn duplicate_test_callback_and_repeat_do_not_restart_hold() {
        let h = SchemeHold::default();
        let t = Instant::now();
        h.key_down(true, false, true, t);
        assert!(!h.key_down(true, false, true, t + Duration::from_millis(500)));
        h.key_down(true, true, true, t + Duration::from_millis(550));
        assert!(h.key_up(
            true,
            t + Duration::from_millis(650),
            Duration::from_millis(600)
        ));
    }
    #[test]
    fn preheld_modifier_and_focus_loss_cancel() {
        let h = SchemeHold::default();
        let t = Instant::now();
        h.key_down(true, false, false, t);
        assert!(!h.key_up(true, t + Duration::from_secs(1), Duration::from_millis(600)));
        h.key_down(true, false, true, t);
        h.reset();
        assert!(!h.key_up(true, t + Duration::from_secs(1), Duration::from_millis(600)));
    }
    #[test]
    fn only_right_control_or_extended_generic_control_matches() {
        assert!(is_right_control(0xa3, 0));
        assert!(is_right_control(0x11, 1 << 24));
        assert!(!is_right_control(0x11, 0));
        assert!(!is_right_control(0xa2, 1 << 24));
    }
}
