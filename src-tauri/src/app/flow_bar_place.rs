//! Where the Flow Bar window goes on a display (BAR-05): along the bottom,
//! left or right edge of the work area (so never under the menu bar or the
//! Dock), and which edge a drop lands on. Pure geometry in physical pixels.

use crate::settings::flow_bar::Edge;

/// A display's work area.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Area {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// The bar's fixed measurements, in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spacing {
    /// The pill's height; it sits at the bottom of the window.
    pub pill: f64,
    /// Gap between the bar and the edge of the work area.
    pub margin: f64,
    /// Room kept between the pill and the ends of its edge, so what hovering
    /// shows (language chips, the menu) fits without moving the pill.
    pub room: f64,
}

/// The window's top-left corner for a `width` × `height` window whose pill
/// sits at `position` along `edge`.
pub fn origin(
    area: Area,
    edge: Edge,
    position: f64,
    width: f64,
    height: f64,
    spacing: Spacing,
) -> (f64, f64) {
    let Spacing { pill, margin, room } = spacing;
    let clamp_x = |x: f64| x.clamp(area.x, (area.x + area.width - width).max(area.x));
    let clamp_y = |y: f64| y.clamp(area.y, (area.y + area.height - height).max(area.y));
    match edge {
        Edge::Bottom => {
            let centre = (area.x + area.width * position).clamp(
                area.x + room,
                (area.x + area.width - room).max(area.x + room),
            );
            (
                clamp_x(centre - width / 2.0),
                area.y + area.height - height - margin,
            )
        }
        Edge::Left | Edge::Right => {
            // The pill's centre stays at `position`; notices grow upwards.
            let pill_centre = (area.y + area.height * position).max(area.y + room);
            let y = clamp_y(pill_centre + pill / 2.0 - height);
            let x = if edge == Edge::Left {
                area.x + margin
            } else {
                area.x + area.width - width - margin
            };
            (x, y)
        }
    }
}

/// The edge nearest to a drop at (`x`, `y`) and the place along it.
pub fn nearest_edge(area: Area, x: f64, y: f64) -> (Edge, f64) {
    let left = x - area.x;
    let right = area.x + area.width - x;
    let bottom = area.y + area.height - y;
    let along_x = ((x - area.x) / area.width).clamp(0.0, 1.0);
    let along_y = ((y - area.y) / area.height).clamp(0.0, 1.0);
    if bottom <= left && bottom <= right {
        (Edge::Bottom, along_x)
    } else if left <= right {
        (Edge::Left, along_y)
    } else {
        (Edge::Right, along_y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AREA: Area = Area {
        x: 0.0,
        y: 50.0,
        width: 2000.0,
        height: 1000.0,
    };

    const SPACING: Spacing = Spacing {
        pill: 40.0,
        margin: 20.0,
        room: 100.0,
    };

    #[test]
    fn the_bar_sits_inside_the_work_area_on_its_edge() {
        let (x, y) = origin(AREA, Edge::Bottom, 0.5, 100.0, 40.0, SPACING);
        assert_eq!((x, y), (950.0, 990.0));
        let (x, _) = origin(AREA, Edge::Bottom, 1.0, 100.0, 40.0, SPACING);
        assert_eq!(x, 1850.0, "room to grow before the right side");
        let (x, y) = origin(AREA, Edge::Left, 0.5, 100.0, 40.0, SPACING);
        assert_eq!((x, y), (20.0, 530.0));
        let (x, y) = origin(AREA, Edge::Right, 0.0, 100.0, 140.0, SPACING);
        assert_eq!((x, y), (1880.0, 50.0), "below the menu bar");
    }

    #[test]
    fn a_notice_grows_the_bar_upwards_without_moving_the_pill() {
        let (_, short) = origin(AREA, Edge::Left, 0.5, 100.0, 40.0, SPACING);
        let (_, tall) = origin(AREA, Edge::Left, 0.5, 300.0, 200.0, SPACING);
        assert_eq!(short + 40.0, tall + 200.0);
    }

    #[test]
    fn hovering_near_an_end_does_not_move_the_pill() {
        // Language chips above the pill, at the top of the left edge.
        let (_, resting) = origin(AREA, Edge::Left, 0.0, 60.0, 40.0, SPACING);
        let (_, hovered) = origin(AREA, Edge::Left, 0.0, 180.0, 80.0, SPACING);
        assert_eq!(resting + 40.0, hovered + 80.0);
        // A wider bar at the left end of the bottom edge.
        let (narrow, _) = origin(AREA, Edge::Bottom, 0.0, 60.0, 40.0, SPACING);
        let (wide, _) = origin(AREA, Edge::Bottom, 0.0, 180.0, 80.0, SPACING);
        assert_eq!(narrow + 30.0, wide + 90.0);
    }

    #[test]
    fn a_drop_lands_on_the_nearest_edge() {
        assert_eq!(nearest_edge(AREA, 500.0, 1000.0), (Edge::Bottom, 0.25));
        assert_eq!(nearest_edge(AREA, 30.0, 550.0), (Edge::Left, 0.5));
        assert_eq!(nearest_edge(AREA, 1990.0, 300.0), (Edge::Right, 0.25));
    }
}
