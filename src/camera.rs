//! A camera that follows Mario and stays inside the level.
//!
//! The camera is the top-left pixel of the visible 160x144 window over a larger
//! level. It keeps its focus (usually Mario) centered, but clamps at the level
//! edges so we never scroll past the world into blank space.

use crate::{SCREEN_HEIGHT, SCREEN_WIDTH};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Camera {
    pub x: i32,
    pub y: i32,
}

impl Camera {
    pub fn new() -> Self {
        Self::default()
    }

    /// Center the view on a focus point, clamped so the view stays within a
    /// level of the given pixel size. If the level is smaller than the screen on
    /// an axis, that axis stays at 0.
    pub fn follow(&mut self, focus_x: i32, focus_y: i32, level_w: i32, level_h: i32) {
        self.x = center_clamped(focus_x, SCREEN_WIDTH as i32, level_w);
        self.y = center_clamped(focus_y, SCREEN_HEIGHT as i32, level_h);
    }
}

fn center_clamped(focus: i32, view: i32, level: i32) -> i32 {
    let target = focus - view / 2;
    let max = (level - view).max(0);
    target.clamp(0, max)
}

#[cfg(test)]
mod tests {
    use super::*;

    const W: i32 = SCREEN_WIDTH as i32; // 160
    const H: i32 = SCREEN_HEIGHT as i32; // 144

    #[test]
    fn centers_on_focus_in_open_space() {
        let mut cam = Camera::new();
        cam.follow(500, 400, 2000, 2000);
        assert_eq!(cam.x, 500 - W / 2);
        assert_eq!(cam.y, 400 - H / 2);
    }

    #[test]
    fn clamps_at_the_left_and_top_edges() {
        let mut cam = Camera::new();
        cam.follow(10, 5, 2000, 2000);
        assert_eq!(cam.x, 0);
        assert_eq!(cam.y, 0);
    }

    #[test]
    fn clamps_at_the_right_and_bottom_edges() {
        let mut cam = Camera::new();
        cam.follow(1990, 1990, 2000, 2000);
        assert_eq!(cam.x, 2000 - W);
        assert_eq!(cam.y, 2000 - H);
    }

    #[test]
    fn small_level_does_not_scroll() {
        let mut cam = Camera::new();
        // Level smaller than the screen on both axes.
        cam.follow(50, 50, 100, 100);
        assert_eq!(cam.x, 0);
        assert_eq!(cam.y, 0);
    }
}
