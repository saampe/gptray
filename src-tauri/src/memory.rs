//! Best-effort native memory hints, never page disposal or JavaScript inspection.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Target {
    #[default]
    Normal,
    Low,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Request {
    pub generation: u64,
    pub revision: u64,
    pub target: Target,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Applied,
    Unsupported,
    Failed(i32),
}
#[derive(Debug, Default)]
pub struct Memory {
    generation: u64,
    revision: u64,
    desired: Target,
    applied: Option<Target>,
    unsupported: bool,
}
impl Memory {
    pub fn new_page(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.applied = None;
        self.unsupported = false;
    }
    pub fn request(&mut self, target: Target) -> Option<Request> {
        self.desired = target;
        self.revision = self.revision.wrapping_add(1);
        if self.unsupported || self.applied == Some(target) {
            return None;
        }
        Some(Request {
            generation: self.generation,
            revision: self.revision,
            target,
        })
    }
    pub fn is_current(&self, r: Request) -> bool {
        !self.unsupported
            && r.generation == self.generation
            && r.revision == self.revision
            && r.target == self.desired
    }
    pub fn finish(&mut self, r: Request, outcome: Outcome) -> bool {
        if !self.is_current(r) {
            return false;
        }
        match outcome {
            Outcome::Applied => self.applied = Some(r.target),
            Outcome::Unsupported => {
                self.unsupported = true;
                self.applied = None;
            }
            Outcome::Failed(_) => self.applied = None,
        }
        true
    }
}
pub fn target_for(visible: bool, minimized: bool, foreground: Option<bool>) -> Target {
    if !visible || minimized || foreground == Some(false) {
        Target::Low
    } else {
        Target::Normal
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inactive_is_low_active_and_unknown_foreground_are_normal() {
        assert_eq!(target_for(false, false, None), Target::Low);
        assert_eq!(target_for(true, true, Some(true)), Target::Low);
        assert_eq!(target_for(true, false, Some(false)), Target::Low);
        assert_eq!(target_for(true, false, Some(true)), Target::Normal);
        assert_eq!(target_for(true, false, None), Target::Normal);
    }
    #[test]
    fn low_callback_cannot_override_reactivation() {
        let mut m = Memory::default();
        m.new_page();
        let low = m.request(Target::Low).unwrap();
        let normal = m.request(Target::Normal).unwrap();
        assert!(!m.is_current(low));
        assert!(!m.finish(low, Outcome::Applied));
        assert!(m.finish(normal, Outcome::Applied));
        assert_eq!(m.request(Target::Normal), None);
    }
    #[test]
    fn destroying_and_recreating_invalidates_old_callbacks() {
        let mut m = Memory::default();
        m.new_page();
        let old = m.request(Target::Low).unwrap();
        m.new_page();
        assert!(!m.is_current(old));
        assert!(!m.finish(old, Outcome::Unsupported));
        assert!(m.request(Target::Normal).is_some());
    }
    #[test]
    fn unsupported_interface_is_not_retried_until_new_page() {
        let mut m = Memory::default();
        m.new_page();
        let r = m.request(Target::Low).unwrap();
        assert!(m.finish(r, Outcome::Unsupported));
        assert_eq!(m.request(Target::Normal), None);
        m.new_page();
        assert!(m.request(Target::Normal).is_some());
    }
    #[test]
    fn transient_failure_keeps_reactivation_retryable() {
        let mut m = Memory::default();
        m.new_page();
        let r = m.request(Target::Normal).unwrap();
        assert!(m.finish(r, Outcome::Failed(-1)));
        let retry = m.request(Target::Normal).unwrap();
        assert!(m.finish(retry, Outcome::Applied));
        assert_eq!(m.request(Target::Normal), None);
    }
    #[test]
    fn repeated_inactive_notifications_deduplicate_successful_hint() {
        let mut m = Memory::default();
        m.new_page();
        let r = m.request(Target::Low).unwrap();
        m.finish(r, Outcome::Applied);
        assert_eq!(m.request(Target::Low), None);
        assert!(m.request(Target::Normal).is_some());
    }
}
