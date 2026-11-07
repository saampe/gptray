//! Window/page policy, independent of Tauri and wall-clock time.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Quick,
    Full,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Deadline {
    pub epoch: u64,
    pub at_ms: u64,
}
#[derive(Debug, Default)]
pub struct Lifecycle {
    pub mode: Mode,
    pub transitioning: bool,
    pub releasing: bool,
    auto_free: bool,
    hidden_since: Option<u64>,
    epoch: u64,
}
impl Lifecycle {
    pub fn show(&mut self, mode: Mode) {
        self.mode = mode;
        self.hidden_since = None;
        self.epoch = self.epoch.wrapping_add(1);
    }
    pub fn hide(&mut self, now: u64) {
        // Repeated focus notifications must not extend an existing idle period.
        if self.hidden_since.is_none() {
            self.hidden_since = Some(now);
            self.epoch = self.epoch.wrapping_add(1);
        }
    }
    pub fn blur_hides(&self) -> bool {
        self.mode == Mode::Quick && !self.transitioning && !self.releasing
    }
    pub fn blur_should_hide(&self, native_foreground: Option<bool>) -> bool {
        self.blur_hides() && native_foreground == Some(false)
    }
    pub fn toggle_should_hide(&self, visible: bool) -> bool {
        self.mode == Mode::Quick && visible && !self.releasing
    }
    pub fn set_auto_free(&mut self, enabled: bool, now: u64) {
        self.auto_free = enabled;
        self.epoch = self.epoch.wrapping_add(1);
        if self.hidden_since.is_some() {
            self.hidden_since = Some(now);
        }
    }
    pub fn deadline(&self) -> Option<Deadline> {
        if !self.auto_free || self.releasing || self.transitioning {
            return None;
        }
        self.hidden_since.map(|since| Deadline {
            epoch: self.epoch,
            at_ms: since.saturating_add(120_000),
        })
    }
    pub fn may_auto_release(&self, ticket: Deadline, now: u64) -> bool {
        self.deadline() == Some(ticket) && now >= ticket.at_ms
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_never_discards_unknown_page_state() {
        let mut s = Lifecycle::default();
        s.hide(0);
        assert_eq!(s.deadline(), None);
        assert!(!s.may_auto_release(
            Deadline {
                epoch: 1,
                at_ms: 120_000
            },
            999_999
        ));
    }
    #[test]
    fn only_quick_mode_hides_on_blur() {
        let mut s = Lifecycle::default();
        assert!(s.blur_hides());
        s.show(Mode::Full);
        assert!(!s.blur_hides());
        s.show(Mode::Quick);
        s.transitioning = true;
        assert!(!s.blur_hides());
    }
    #[test]
    fn opt_in_waits_two_minutes_from_hide() {
        let mut s = Lifecycle::default();
        s.set_auto_free(true, 0);
        s.hide(10);
        let d = s.deadline().unwrap();
        assert_eq!(d.at_ms, 120_010);
        assert!(!s.may_auto_release(d, 120_009));
        assert!(s.may_auto_release(d, 120_010));
    }
    #[test]
    fn reopen_and_rehide_invalidates_queued_timer() {
        let mut s = Lifecycle::default();
        s.set_auto_free(true, 0);
        s.hide(10);
        let old = s.deadline().unwrap();
        s.show(Mode::Full);
        assert_eq!(s.deadline(), None);
        assert!(!s.may_auto_release(old, 999_999));
        s.hide(100);
        let new = s.deadline().unwrap();
        assert_ne!(new.epoch, old.epoch);
        assert_eq!(new.at_ms, 120_100);
        assert!(!s.may_auto_release(old, 999_999));
    }
    #[test]
    fn opt_out_invalidates_queued_timer() {
        let mut s = Lifecycle::default();
        s.set_auto_free(true, 0);
        s.hide(0);
        let old = s.deadline().unwrap();
        s.set_auto_free(false, 50);
        assert_eq!(s.deadline(), None);
        assert!(!s.may_auto_release(old, 999_999));
        s.set_auto_free(true, 80);
        assert_eq!(s.deadline().unwrap().at_ms, 120_080);
    }
    #[test]
    fn no_release_during_transition_or_destruction() {
        let mut s = Lifecycle::default();
        s.set_auto_free(true, 0);
        s.hide(0);
        let d = s.deadline().unwrap();
        s.transitioning = true;
        assert!(!s.may_auto_release(d, 120_000));
        s.transitioning = false;
        s.releasing = true;
        assert!(!s.may_auto_release(d, 120_000));
    }
    #[test]
    fn repeat_mode_changes_cancel_idle_deadline() {
        let mut s = Lifecycle::default();
        s.set_auto_free(true, 0);
        for i in 0..100 {
            s.hide(i);
            s.show(Mode::Full);
            assert!(!s.blur_hides());
            s.show(Mode::Quick);
            assert!(s.blur_hides());
            assert_eq!(s.deadline(), None);
        }
    }
    #[test]
    fn queued_blur_never_hides_current_foreground_or_unknown_focus() {
        let s = Lifecycle::default();
        assert!(!s.blur_should_hide(Some(true)));
        assert!(!s.blur_should_hide(None));
        assert!(s.blur_should_hide(Some(false)));
    }
    #[test]
    fn hotkey_during_release_requests_reopen_instead_of_hiding_old_window() {
        let mut s = Lifecycle::default();
        assert!(s.toggle_should_hide(true));
        s.releasing = true;
        assert!(!s.toggle_should_hide(true));
    }
}
