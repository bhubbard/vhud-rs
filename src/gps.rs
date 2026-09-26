use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};

use crate::blip::BlipColor;
use crate::projection::{RadarProjection, RadarShape};

/// Outcode bits for Cohen-Sutherland line clipping.
pub mod cohen_sutherland {
    pub const INSIDE: u8 = 0; // 0000
    pub const LEFT: u8 = 1; // 0001
    pub const RIGHT: u8 = 2; // 0010
    pub const BOTTOM: u8 = 4; // 0100
    pub const TOP: u8 = 8; // 1000

    /// Computes the outcode for a point `(x, y)` relative to an AABB.
    #[inline]
    pub fn compute_outcode(x: f32, y: f32, xmin: f32, xmax: f32, ymin: f32, ymax: f32) -> u8 {
        let mut code = INSIDE;
        if x < xmin {
            code |= LEFT;
        } else if x > xmax {
            code |= RIGHT;
        }
        if y < ymin {
            code |= BOTTOM;
        } else if y > ymax {
            code |= TOP;
        }
        code
    }

    /// Clips line segment `(p0, p1)` against AABB `[xmin, xmax] x [ymin, ymax]` using Cohen-Sutherland.
    /// Returns `Some((clipped_p0, clipped_p1))` if any portion is visible.
    pub fn clip_segment(
        mut p0: glam::Vec2,
        mut p1: glam::Vec2,
        xmin: f32,
        xmax: f32,
        ymin: f32,
        ymax: f32,
    ) -> Option<(glam::Vec2, glam::Vec2)> {
        let mut code0 = compute_outcode(p0.x, p0.y, xmin, xmax, ymin, ymax);
        let mut code1 = compute_outcode(p1.x, p1.y, xmin, xmax, ymin, ymax);

        loop {
            if (code0 | code1) == 0 {
                // Trivial accept: both points inside
                return Some((p0, p1));
            } else if (code0 & code1) != 0 {
                // Trivial reject: both points share an outside zone
                return None;
            } else {
                // Failed both tests, calculate line intersection with edge
                let outcode = if code0 != 0 { code0 } else { code1 };
                let mut p = glam::Vec2::ZERO;

                let dx = p1.x - p0.x;
                let dy = p1.y - p0.y;

                if (outcode & TOP) != 0 {
                    p.x = p0.x + dx * (ymax - p0.y) / dy;
                    p.y = ymax;
                } else if (outcode & BOTTOM) != 0 {
                    p.x = p0.x + dx * (ymin - p0.y) / dy;
                    p.y = ymin;
                } else if (outcode & RIGHT) != 0 {
                    p.y = p0.y + dy * (xmax - p0.x) / dx;
                    p.x = xmax;
                } else if (outcode & LEFT) != 0 {
                    p.y = p0.y + dy * (xmin - p0.x) / dx;
                    p.x = xmin;
                }

                if outcode == code0 {
                    p0 = p;
                    code0 = compute_outcode(p0.x, p0.y, xmin, xmax, ymin, ymax);
                } else {
                    p1 = p;
                    code1 = compute_outcode(p1.x, p1.y, xmin, xmax, ymin, ymax);
                }
            }
        }
    }
}

/// Liang-Barsky parametric line clipping algorithm for rectangular radar windows.
pub mod liang_barsky {
    use glam::Vec2;

    /// Clips line segment `(p0, p1)` against AABB `[xmin, xmax] x [ymin, ymax]`.
    /// Returns `Some((clipped_p0, clipped_p1))` if any part of the segment is visible.
    pub fn clip_segment(
        p0: Vec2,
        p1: Vec2,
        xmin: f32,
        xmax: f32,
        ymin: f32,
        ymax: f32,
    ) -> Option<(Vec2, Vec2)> {
        let dx = p1.x - p0.x;
        let dy = p1.y - p0.y;

        let mut t0 = 0.0f32;
        let mut t1 = 1.0f32;

        let p = [-dx, dx, -dy, dy];
        let q = [p0.x - xmin, xmax - p0.x, p0.y - ymin, ymax - p0.y];

        for i in 0..4 {
            let pi = p[i];
            let qi = q[i];

            if pi.abs() < 1e-8 {
                // Line is parallel to this boundary
                if qi < 0.0 {
                    return None; // Completely outside
                }
            } else {
                let r = qi / pi;
                if pi < 0.0 {
                    // Line proceeds from outside to inside
                    if r > t1 {
                        return None;
                    }
                    if r > t0 {
                        t0 = r;
                    }
                } else {
                    // Line proceeds from inside to outside
                    if r < t0 {
                        return None;
                    }
                    if r < t1 {
                        t1 = r;
                    }
                }
            }
        }

        if t0 <= t1 {
            let clipped_0 = Vec2::new(p0.x + t0 * dx, p0.y + t0 * dy);
            let clipped_1 = Vec2::new(p0.x + t1 * dx, p0.y + t1 * dy);
            Some((clipped_0, clipped_1))
        } else {
            None
        }
    }
}

/// Circular disk line segment clipping algorithm.
pub mod circular_clip {
    use glam::Vec2;

    /// Clips a line segment `(p0, p1)` against a circle centered at `center` with `radius`.
    /// Returns `Some((clipped_p0, clipped_p1))` if any portion of the segment falls within the circle.
    pub fn clip_segment(p0: Vec2, p1: Vec2, center: Vec2, radius: f32) -> Option<(Vec2, Vec2)> {
        let d = p1 - p0;
        let delta = p0 - center;

        let a = d.length_squared();
        if a < 1e-8 {
            // Segment is a single point
            if delta.length_squared() <= radius * radius {
                return Some((p0, p1));
            } else {
                return None;
            }
        }

        let b = 2.0 * delta.dot(d);
        let c = delta.length_squared() - radius * radius;

        let discriminant = b * b - 4.0 * a * c;

        if discriminant < 0.0 {
            // No real intersection with circle boundary
            return None;
        }

        let sqrt_disc = discriminant.sqrt();
        let mut t0 = (-b - sqrt_disc) / (2.0 * a);
        let mut t1 = (-b + sqrt_disc) / (2.0 * a);

        if t0 > t1 {
            std::mem::swap(&mut t0, &mut t1);
        }

        // Clamp parametric interval to [0, 1]
        let t_start = t0.max(0.0);
        let t_end = t1.min(1.0);

        if t_start <= t_end && t_end >= 0.0 && t_start <= 1.0 {
            let clipped_0 = p0 + d * t_start;
            let clipped_1 = p0 + d * t_end;
            Some((clipped_0, clipped_1))
        } else {
            None
        }
    }
}

/// Directional arrow/chevron displayed along the GPS route.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GpsArrow {
    /// Screen position of arrow center.
    pub screen_pos: Vec2,
    /// Angle in radians indicating direction of travel along route.
    pub angle_rad: f32,
}

/// A clipped 2D screen line segment belonging to the GPS route.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GpsSegment {
    /// Start coordinate on screen.
    pub start: Vec2,
    /// End coordinate on screen.
    pub end: Vec2,
    /// Index of origin waypoint segment in route.
    pub waypoint_index: usize,
}

/// A GPS navigation route consisting of a sequence of 3D or 2D waypoints.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GpsRoute {
    /// World positions along the route polyline.
    pub waypoints: Vec<Vec3>,
    /// Color of the GPS path line (default GTA purple/magenta).
    pub color: BlipColor,
    /// Screen line width in pixels (default 5.0).
    pub line_width: f32,
    /// Distance threshold in meters to register arrival at the destination.
    pub arrival_threshold: f32,
    /// Spacing between directional arrows along the path in screen pixels (default 32.0).
    pub arrow_spacing_pixels: f32,
}

impl Default for GpsRoute {
    fn default() -> Self {
        Self {
            waypoints: Vec::new(),
            color: BlipColor::new(0.63, 0.25, 0.95, 0.85),
            line_width: 5.0,
            arrival_threshold: 15.0,
            arrow_spacing_pixels: 32.0,
        }
    }
}

impl GpsRoute {
    /// Creates a new GPS route from a slice of 3D world waypoints.
    pub fn new(waypoints: Vec<Vec3>) -> Self {
        Self {
            waypoints,
            ..Default::default()
        }
    }

    /// Appends a waypoint to the route.
    pub fn add_waypoint(&mut self, wp: Vec3) {
        self.waypoints.push(wp);
    }

    /// Number of waypoints.
    pub fn len(&self) -> usize {
        self.waypoints.len()
    }

    /// Checks if route is empty.
    pub fn is_empty(&self) -> bool {
        self.waypoints.is_empty()
    }

    /// Total world length of the route polyline in meters.
    pub fn total_length(&self) -> f32 {
        if self.waypoints.len() < 2 {
            return 0.0;
        }
        let mut dist = 0.0;
        for i in 0..self.waypoints.len() - 1 {
            dist += (self.waypoints[i + 1] - self.waypoints[i]).length();
        }
        dist
    }

    /// Calculates remaining distance from player to final destination along the route.
    pub fn remaining_distance(&self, player_pos: Vec3) -> f32 {
        if self.waypoints.is_empty() {
            return 0.0;
        }
        if self.waypoints.len() == 1 {
            return (self.waypoints[0] - player_pos).length();
        }

        // Find closest segment to player
        let (closest_seg_idx, closest_pt) = self.closest_point_on_route(player_pos);
        let mut dist = (closest_pt - player_pos).length();

        // Distance from closest point to end of that segment
        dist += (self.waypoints[closest_seg_idx + 1] - closest_pt).length();

        // Sum subsequent segments
        for i in (closest_seg_idx + 1)..(self.waypoints.len() - 1) {
            dist += (self.waypoints[i + 1] - self.waypoints[i]).length();
        }

        dist
    }

    /// Finds the closest segment index and point on the polyline to the given position.
    pub fn closest_point_on_route(&self, pos: Vec3) -> (usize, Vec3) {
        if self.waypoints.len() < 2 {
            return (0, self.waypoints.first().copied().unwrap_or(pos));
        }

        let mut min_dist_sq = f32::INFINITY;
        let mut best_idx = 0;
        let mut best_pt = self.waypoints[0];

        for i in 0..self.waypoints.len() - 1 {
            let a = self.waypoints[i];
            let b = self.waypoints[i + 1];
            let ab = b - a;
            let ab_len_sq = ab.length_squared();

            let pt = if ab_len_sq < 1e-6 {
                a
            } else {
                let t = ((pos - a).dot(ab) / ab_len_sq).clamp(0.0, 1.0);
                a + ab * t
            };

            let dist_sq = (pos - pt).length_squared();
            if dist_sq < min_dist_sq {
                min_dist_sq = dist_sq;
                best_idx = i;
                best_pt = pt;
            }
        }

        (best_idx, best_pt)
    }

    /// Checks if the player has arrived at the final destination waypoint.
    pub fn has_arrived(&self, player_pos: Vec3) -> bool {
        if let Some(&dest) = self.waypoints.last() {
            (dest - player_pos).length() <= self.arrival_threshold
        } else {
            true
        }
    }
}

/// Result of projecting and clipping a GPS route against the radar viewport.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenderedGpsRoute {
    /// Clipped visible segments ready to be drawn on screen.
    pub segments: Vec<GpsSegment>,
    /// Directional chevrons along the visible route segments.
    pub arrows: Vec<GpsArrow>,
    /// Color of the route line.
    pub color: BlipColor,
    /// Screen line width.
    pub line_width: f32,
    /// Estimated distance remaining to destination in meters.
    pub remaining_distance: f32,
    /// Whether player has reached the final destination.
    pub arrived: bool,
}

/// GPS Route Navigator that handles real-time projection and window clipping.
#[derive(Debug, Clone, Default)]
pub struct RouteNavigator {
    pub active_route: Option<GpsRoute>,
}

impl RouteNavigator {
    /// Creates a new route navigator with no active route.
    pub fn new() -> Self {
        Self { active_route: None }
    }

    /// Sets the active navigation route.
    pub fn set_route(&mut self, route: GpsRoute) {
        self.active_route = Some(route);
    }

    /// Clears the active navigation route.
    pub fn clear_route(&mut self) {
        self.active_route = None;
    }

    /// Generates rendered screen segments and arrows clipped to the radar boundary.
    pub fn render_route(
        &self,
        player_pos: Vec3,
        player_heading: f32,
        radar: &RadarProjection,
    ) -> Option<RenderedGpsRoute> {
        let route = self.active_route.as_ref()?;
        if route.waypoints.len() < 2 {
            return None;
        }

        let remaining_dist = route.remaining_distance(player_pos);
        let arrived = route.has_arrived(player_pos);

        // Find where the player is currently projected relative to route
        let (current_seg_idx, closest_pt) = route.closest_point_on_route(player_pos);

        // Build active polyline: starting from player's current position to closest point, then onward
        let mut active_points = Vec::with_capacity(route.waypoints.len() - current_seg_idx + 1);
        active_points.push(player_pos);
        if (closest_pt - player_pos).length() > 2.0 {
            active_points.push(closest_pt);
        }
        for wp in &route.waypoints[(current_seg_idx + 1)..] {
            active_points.push(*wp);
        }

        // Project world points to screen coordinates
        let screen_pts: Vec<Vec2> = active_points
            .iter()
            .map(|&p| radar.world3d_to_screen(p, player_pos, player_heading))
            .collect();

        let mut visible_segments = Vec::new();

        // Clip each segment against radar viewport
        for i in 0..screen_pts.len() - 1 {
            let p0 = screen_pts[i];
            let p1 = screen_pts[i + 1];

            let clipped = match radar.config.shape {
                RadarShape::Circular { radius } => {
                    circular_clip::clip_segment(p0, p1, radar.config.center, radius)
                }
                RadarShape::Rectangular { width, height } => {
                    let hw = width * 0.5;
                    let hh = height * 0.5;
                    let xmin = radar.config.center.x - hw;
                    let xmax = radar.config.center.x + hw;
                    let ymin = radar.config.center.y - hh;
                    let ymax = radar.config.center.y + hh;
                    liang_barsky::clip_segment(p0, p1, xmin, xmax, ymin, ymax)
                }
            };

            if let Some((cp0, cp1)) = clipped
                && (cp1 - cp0).length_squared() > 1.0
            {
                visible_segments.push(GpsSegment {
                    start: cp0,
                    end: cp1,
                    waypoint_index: i,
                });
            }
        }

        // Generate directional chevrons along visible segments
        let mut arrows = Vec::new();
        let spacing = route.arrow_spacing_pixels.max(10.0);

        for seg in &visible_segments {
            let delta = seg.end - seg.start;
            let seg_len = delta.length();
            if seg_len >= spacing * 0.75 {
                let angle_rad = delta.y.atan2(delta.x);
                let num_arrows = (seg_len / spacing).floor() as usize;
                let step = seg_len / ((num_arrows + 1) as f32);

                for a in 1..=num_arrows {
                    let t = (a as f32 * step) / seg_len;
                    let arrow_pos = seg.start + delta * t;
                    arrows.push(GpsArrow {
                        screen_pos: arrow_pos,
                        angle_rad,
                    });
                }
            }
        }

        Some(RenderedGpsRoute {
            segments: visible_segments,
            arrows,
            color: route.color,
            line_width: route.line_width,
            remaining_distance: remaining_dist,
            arrived,
        })
    }
}
