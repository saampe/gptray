//! Native initial configuration, selected before WebView2 constructs its viewport.
use crate::{geometry::Bounds, lifecycle::Mode};
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindowMode {
    pub width: f64,
    pub height: f64,
    pub full: bool,
    pub maximized: bool,
}
impl WindowMode {
    pub fn new(mode: Mode, bounds: Bounds) -> Self {
        match mode {
            Mode::Quick => Self {
                width: 400.,
                height: 600.,
                full: false,
                maximized: false,
            },
            Mode::Full => Self {
                width: bounds.width,
                height: bounds.height,
                full: true,
                maximized: bounds.maximized,
            },
        }
    }
}
pub fn requires_reconfigure(current: Mode, requested: Mode) -> bool {
    current != requested || requested == Mode::Quick
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn first_full_uses_full_dimensions_and_native_chrome() {
        let s = WindowMode::new(Mode::Full, Bounds::default());
        assert_eq!((s.width, s.height, s.full), (1100., 750., true));
    }
    #[test]
    fn full_recreation_honors_saved_normal_size_and_maximize() {
        let b = Bounds {
            width: 980.,
            height: 640.,
            maximized: true,
            ..Bounds::default()
        };
        let s = WindowMode::new(Mode::Full, b);
        assert_eq!((s.width, s.height, s.maximized), (980., 640., true));
    }
    #[test]
    fn quick_ignores_saved_full_maximization() {
        let s = WindowMode::new(
            Mode::Quick,
            Bounds {
                maximized: true,
                ..Bounds::default()
            },
        );
        assert_eq!(
            (s.width, s.height, s.full, s.maximized),
            (400., 600., false, false)
        );
    }
    #[test]
    fn reopening_full_does_not_unmaximize_or_relayout_it() {
        assert!(!requires_reconfigure(Mode::Full, Mode::Full));
        assert!(requires_reconfigure(Mode::Quick, Mode::Full));
        assert!(requires_reconfigure(Mode::Full, Mode::Quick));
        assert!(requires_reconfigure(Mode::Quick, Mode::Quick));
    }
}
