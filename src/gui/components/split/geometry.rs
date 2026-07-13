//! Pure geometry for the split widget: clamping and divider rectangles.

use iced::{Rectangle, Size};

use super::SplitAxis;

/// Clamp the first pane's size so both panes keep their minimums.
pub(super) fn clamp_split(
    axis: SplitAxis,
    split_px: f32,
    size: Size,
    min_first: f32,
    min_second: f32,
) -> f32 {
    let extent = match axis {
        SplitAxis::Horizontal => size.width,
        SplitAxis::Vertical => size.height,
    };
    let max_first = (extent - min_second).max(min_first);
    split_px.clamp(min_first, max_first)
}

/// The (wider) invisible rectangle used for hit-testing the divider.
pub(super) fn divider_hit_rect(
    axis: SplitAxis,
    bounds: Rectangle,
    split_px: f32,
    hit: f32,
) -> Rectangle {
    match axis {
        SplitAxis::Horizontal => Rectangle {
            x: bounds.x + split_px - hit * 0.5,
            y: bounds.y,
            width: hit,
            height: bounds.height,
        },
        SplitAxis::Vertical => Rectangle {
            x: bounds.x,
            y: bounds.y + split_px - hit * 0.5,
            width: bounds.width,
            height: hit,
        },
    }
}

/// The drawn divider strip: centered on the layout gap, `draw_width` pixels wide
/// (a hover/drag highlight may be wider than the 1px gap and overdraw the panes slightly).
pub(super) fn divider_draw_rect(
    axis: SplitAxis,
    bounds: Rectangle,
    split_px: f32,
    layout_thickness: f32,
    draw_width: f32,
) -> Rectangle {
    let center = split_px + layout_thickness * 0.5;
    match axis {
        SplitAxis::Horizontal => Rectangle {
            x: bounds.x + center - draw_width * 0.5,
            y: bounds.y,
            width: draw_width,
            height: bounds.height,
        },
        SplitAxis::Vertical => Rectangle {
            x: bounds.x,
            y: bounds.y + center - draw_width * 0.5,
            width: bounds.width,
            height: draw_width,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_respects_minimums() {
        let size = Size::new(1000.0, 500.0);
        assert_eq!(
            clamp_split(SplitAxis::Horizontal, 50.0, size, 200.0, 300.0),
            200.0
        );
        assert_eq!(
            clamp_split(SplitAxis::Horizontal, 900.0, size, 200.0, 300.0),
            700.0
        );
        assert_eq!(
            clamp_split(SplitAxis::Vertical, 450.0, size, 100.0, 100.0),
            400.0
        );
    }

    #[test]
    fn clamp_prefers_first_minimum_when_too_small() {
        // Window smaller than min_first + min_second: first pane wins its minimum.
        let size = Size::new(300.0, 300.0);
        assert_eq!(
            clamp_split(SplitAxis::Horizontal, 150.0, size, 220.0, 420.0),
            220.0
        );
    }

    #[test]
    fn draw_rect_is_centered_on_gap() {
        let bounds = Rectangle {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        };
        let r = divider_draw_rect(SplitAxis::Horizontal, bounds, 50.0, 1.0, 3.0);
        assert_eq!(r.x, 49.0);
        assert_eq!(r.width, 3.0);
    }
}
