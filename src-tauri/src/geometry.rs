//! Only native bounds are stored; never page content, URLs or credentials.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    pub x: i32,
    pub y: i32,
    pub width: f64,
    pub height: f64,
    pub maximized: bool,
}
impl Default for Bounds {
    fn default() -> Self {
        Self {
            x: 80,
            y: 80,
            width: 1100.,
            height: 750.,
            maximized: false,
        }
    }
}
#[derive(Clone, Copy)]
pub struct WorkArea {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale: f64,
}
impl Bounds {
    pub fn encode(&self) -> String {
        format!(
            "v1 {} {} {} {} {}\n",
            self.x, self.y, self.width, self.height, self.maximized
        )
    }
    pub fn decode(s: &str) -> Option<Self> {
        let p: Vec<_> = s.split_whitespace().collect();
        if p.len() != 6 || p[0] != "v1" {
            return None;
        }
        let b = Self {
            x: p[1].parse().ok()?,
            y: p[2].parse().ok()?,
            width: p[3].parse().ok()?,
            height: p[4].parse().ok()?,
            maximized: p[5].parse().ok()?,
        };
        (b.width.is_finite()
            && b.height.is_finite()
            && (200.0..=16384.0).contains(&b.width)
            && (200.0..=16384.0).contains(&b.height))
        .then_some(b)
    }
    pub fn fit(mut self, areas: &[WorkArea]) -> Self {
        let valid: Vec<_> = areas
            .iter()
            .filter(|a| a.scale.is_finite() && a.scale > 0. && a.width > 0 && a.height > 0)
            .collect();
        let contains = |a: &&WorkArea| {
            (self.x as i64) >= a.x as i64
                && (self.y as i64) >= a.y as i64
                && (self.x as i64) < a.x as i64 + a.width as i64
                && (self.y as i64) < a.y as i64 + a.height as i64
        };
        let Some(a) = valid
            .iter()
            .copied()
            .find(contains)
            .or_else(|| valid.first().copied())
        else {
            return self;
        };
        self.width = self.width.min(a.width as f64 / a.scale);
        self.height = self.height.min(a.height as f64 / a.scale);
        let max_x = a.x as i64 + a.width as i64 - (self.width * a.scale).ceil() as i64;
        let max_y = a.y as i64 + a.height as i64 - (self.height * a.scale).ceil() as i64;
        self.x = (self.x as i64).clamp(a.x as i64, max_x.max(a.x as i64)) as i32;
        self.y = (self.y as i64).clamp(a.y as i64, max_y.max(a.y as i64)) as i32;
        self
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn persists_normal_bounds_and_maximized_state() {
        let b = Bounds {
            x: -1800,
            y: 40,
            width: 1000.5,
            height: 700.,
            maximized: true,
        };
        assert_eq!(Bounds::decode(&b.encode()), Some(b));
    }
    #[test]
    fn rejects_corrupt_unbounded_or_nonfinite_settings() {
        for s in [
            "",
            "v2 0 0 1100 750 false",
            "v1 0 0 NaN 750 false",
            "v1 0 0 inf 750 false",
            "v1 0 0 0 750 false",
            "v1 0 0 99999 750 false",
            "v1 0 0 1100 750 false extra",
        ] {
            assert_eq!(Bounds::decode(s), None, "{s}");
        }
    }
    #[test]
    fn removed_monitor_moves_window_into_remaining_work_area() {
        let b = Bounds {
            x: -1900,
            y: 40,
            ..Bounds::default()
        };
        let b = b.fit(&[WorkArea {
            x: 0,
            y: 0,
            width: 1366,
            height: 728,
            scale: 1.,
        }]);
        assert_eq!((b.x, b.y, b.width, b.height), (0, 0, 1100., 728.));
    }
    #[test]
    fn fits_logical_size_to_high_dpi_work_area() {
        let b = Bounds::default().fit(&[WorkArea {
            x: 0,
            y: 0,
            width: 1920,
            height: 1040,
            scale: 2.,
        }]);
        assert_eq!((b.width, b.height), (960., 520.));
        assert_eq!((b.x, b.y), (0, 0));
    }
    #[test]
    fn retains_valid_negative_monitor_position() {
        let b = Bounds {
            x: -1600,
            y: 100,
            width: 900.,
            height: 600.,
            maximized: false,
        };
        assert_eq!(
            b.fit(&[WorkArea {
                x: -1920,
                y: 0,
                width: 1920,
                height: 1080,
                scale: 1.
            }]),
            b
        );
    }
}
