use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};

use crate::projection::{RadarProjection, RadarShape};

/// Altitude indicator of a blip relative to the player's vertical elevation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AltitudeRelation {
    /// Target is at roughly the same vertical level as the player.
    Level,
    /// Target is significantly higher than the player (draw upward triangle).
    Above,
    /// Target is significantly lower than the player (draw downward triangle).
    Below,
}

/// Category / archetype of a radar blip.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlipType {
    /// Player waypoint marker (purple by default, clamped to edge).
    Waypoint,
    /// Police law enforcement vehicle or officer (flashes red/blue).
    Cop,
    /// Active mission target or quest giver (yellow).
    Mission,
    /// Hostile target or enemy player (red).
    Enemy,
    /// Friendly or neutral NPC/entity (blue or green).
    Neutral,
    /// Vehicle marker (personal vehicle, aircraft, boat).
    Vehicle,
    /// Custom user-defined blip archetype.
    Custom(String),
}

/// RGBA color representation for blips and HUD elements (0.0 to 1.0 per channel).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BlipColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl BlipColor {
    pub const WHITE: Self = Self::new(1.0, 1.0, 1.0, 1.0);
    pub const PURPLE_WAYPOINT: Self = Self::new(0.66, 0.28, 0.98, 1.0);
    pub const COP_BLUE: Self = Self::new(0.12, 0.45, 0.95, 1.0);
    pub const COP_RED: Self = Self::new(0.95, 0.15, 0.15, 1.0);
    pub const MISSION_YELLOW: Self = Self::new(0.98, 0.85, 0.12, 1.0);
    pub const ENEMY_RED: Self = Self::new(0.92, 0.20, 0.20, 1.0);
    pub const FRIEND_GREEN: Self = Self::new(0.25, 0.85, 0.35, 1.0);
    pub const NEUTRAL_WHITE: Self = Self::new(0.90, 0.90, 0.90, 0.95);
    pub const VEHICLE_BLUE: Self = Self::new(0.20, 0.65, 0.95, 1.0);

    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Converts to RGBA float array `[r, g, b, a]`.
    #[inline]
    pub fn to_array(&self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }

    /// Formats as CSS `rgba(r, g, b, a)` string.
    pub fn to_css_rgba(&self) -> String {
        format!(
            "rgba({}, {}, {}, {:.2})",
            (self.r * 255.0).round() as u8,
            (self.g * 255.0).round() as u8,
            (self.b * 255.0).round() as u8,
            self.a
        )
    }
}

/// A blip registered in the 3D game world.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Blip {
    /// Unique identifier for this blip.
    pub id: u64,
    /// 3D World position `(x, y, z)` in meters.
    pub world_pos: Vec3,
    /// Category / Archetype of the blip.
    pub blip_type: BlipType,
    /// Render priority (higher priority rendered over lower priority).
    pub priority: i32,
    /// Scale multiplier for radar icon (default 1.0).
    pub scale: f32,
    /// Color of the blip icon.
    pub color: BlipColor,
    /// Whether this blip should clamp to the radar edge when out of bounds.
    pub clamp_to_edge: bool,
    /// Whether this blip is flashing (e.g. police alert or mission objective).
    pub flashing: bool,
    /// Optional entity yaw/heading in radians (draws orientation cone/pointer if set).
    pub heading: Option<f32>,
    /// Altitude threshold in meters to trigger above/below arrows (default 3.5m).
    pub altitude_threshold: f32,
}

impl Blip {
    /// Creates a new blip at the given 3D position with standard defaults.
    pub fn new(id: u64, world_pos: Vec3, blip_type: BlipType) -> Self {
        let (color, priority, clamp_to_edge) = match blip_type {
            BlipType::Waypoint => (BlipColor::PURPLE_WAYPOINT, 100, true),
            BlipType::Mission => (BlipColor::MISSION_YELLOW, 90, true),
            BlipType::Cop => (BlipColor::COP_BLUE, 80, true),
            BlipType::Enemy => (BlipColor::ENEMY_RED, 70, false),
            BlipType::Vehicle => (BlipColor::VEHICLE_BLUE, 50, false),
            BlipType::Neutral => (BlipColor::FRIEND_GREEN, 40, false),
            BlipType::Custom(_) => (BlipColor::WHITE, 10, false),
        };

        Self {
            id,
            world_pos,
            blip_type,
            priority,
            scale: 1.0,
            color,
            clamp_to_edge,
            flashing: false,
            heading: None,
            altitude_threshold: 3.5,
        }
    }

    /// Sets whether the blip clamps to the radar perimeter when off-screen.
    pub fn with_clamp(mut self, clamp: bool) -> Self {
        self.clamp_to_edge = clamp;
        self
    }

    /// Sets custom render priority.
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Sets custom scale factor.
    pub fn with_scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    /// Sets custom color.
    pub fn with_color(mut self, color: BlipColor) -> Self {
        self.color = color;
        self
    }

    /// Sets flashing status.
    pub fn with_flashing(mut self, flashing: bool) -> Self {
        self.flashing = flashing;
        self
    }

    /// Sets entity heading in radians.
    pub fn with_heading(mut self, heading: f32) -> Self {
        self.heading = Some(heading);
        self
    }
}

/// Calculated screen-space rendering payload for a radar blip.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenderedBlip {
    /// Blip ID.
    pub id: u64,
    /// Final 2D screen coordinate in pixels where the icon center should be drawn.
    pub screen_pos: Vec2,
    /// Unclamped relative radar coordinate `(X_r, Y_r)`.
    pub radar_rel: Vec2,
    /// Altitude relative to player (Level, Above, Below).
    pub altitude: AltitudeRelation,
    /// Elevation difference in meters `(target_z - player_z)`.
    pub delta_z: f32,
    /// True if the blip was clamped to the radar perimeter.
    pub is_clamped: bool,
    /// True if the blip is inside the normal radar interior window.
    pub is_inside: bool,
    /// Angle in radians towards off-screen target if clamped (useful for perimeter indicator arrows).
    pub border_angle: Option<f32>,
    /// Effective color (factoring in flashing timers).
    pub effective_color: BlipColor,
    /// Render priority.
    pub priority: i32,
    /// Icon scale.
    pub scale: f32,
    /// Blip category.
    pub blip_type: BlipType,
    /// Relative heading in radians (relative to player heading), if blip has heading.
    pub relative_heading: Option<f32>,
}

/// Clamping calculation result.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClampedResult {
    /// Clamped screen position.
    pub screen_pos: Vec2,
    /// Whether clamping occurred.
    pub clamped: bool,
    /// Whether the original point was inside.
    pub inside: bool,
    /// Angle in radians pointing from radar center towards the blip.
    pub border_angle: f32,
}

/// Clamps a 2D screen offset vector relative to the radar center against the radar perimeter.
pub fn clamp_to_radar_border(
    screen_offset: Vec2,
    shape: RadarShape,
    margin_pixels: f32,
) -> ClampedResult {
    let dist_sq = screen_offset.length_squared();
    let border_angle = screen_offset.y.atan2(screen_offset.x);

    match shape {
        RadarShape::Circular { radius } => {
            let r_clamp = (radius - margin_pixels).max(1.0);
            let inside = dist_sq <= r_clamp * r_clamp;

            if inside {
                ClampedResult {
                    screen_pos: screen_offset,
                    clamped: false,
                    inside: true,
                    border_angle,
                }
            } else {
                let dir = if dist_sq > 1e-6 {
                    screen_offset / dist_sq.sqrt()
                } else {
                    Vec2::new(0.0, -1.0)
                };
                ClampedResult {
                    screen_pos: dir * r_clamp,
                    clamped: true,
                    inside: false,
                    border_angle,
                }
            }
        }
        RadarShape::Rectangular { width, height } => {
            let hw = (width * 0.5 - margin_pixels).max(1.0);
            let hh = (height * 0.5 - margin_pixels).max(1.0);

            let inside = screen_offset.x.abs() <= hw && screen_offset.y.abs() <= hh;

            if inside {
                ClampedResult {
                    screen_pos: screen_offset,
                    clamped: false,
                    inside: true,
                    border_angle,
                }
            } else {
                // Ray-AABB intersection from (0,0) along direction (screen_offset.x, screen_offset.y)
                let tx = if screen_offset.x.abs() > 1e-6 {
                    hw / screen_offset.x.abs()
                } else {
                    f32::INFINITY
                };

                let ty = if screen_offset.y.abs() > 1e-6 {
                    hh / screen_offset.y.abs()
                } else {
                    f32::INFINITY
                };

                let t = tx.min(ty);
                let clamped_pos = screen_offset * t;

                ClampedResult {
                    screen_pos: clamped_pos,
                    clamped: true,
                    inside: false,
                    border_angle,
                }
            }
        }
    }
}

/// Manages active world blips, updates flashing states, and computes screen-space rendering data.
#[derive(Debug, Clone, Default)]
pub struct BlipManager {
    blips: Vec<Blip>,
    /// Internal timer for pulsing/flashing blips.
    elapsed_time: f32,
    /// Distance margin from radar perimeter border for clamped icons (default 8.0 pixels).
    pub border_margin: f32,
}

impl BlipManager {
    /// Creates a new empty blip manager.
    pub fn new() -> Self {
        Self {
            blips: Vec::new(),
            elapsed_time: 0.0,
            border_margin: 8.0,
        }
    }

    /// Adds or updates a blip.
    pub fn add_or_update(&mut self, blip: Blip) {
        if let Some(existing) = self.blips.iter_mut().find(|b| b.id == blip.id) {
            *existing = blip;
        } else {
            self.blips.push(blip);
        }
    }

    /// Removes a blip by ID.
    pub fn remove(&mut self, id: u64) -> bool {
        if let Some(idx) = self.blips.iter().position(|b| b.id == id) {
            self.blips.remove(idx);
            true
        } else {
            false
        }
    }

    /// Clears all blips.
    pub fn clear(&mut self) {
        self.blips.clear();
    }

    /// Number of managed blips.
    pub fn len(&self) -> usize {
        self.blips.len()
    }

    /// Checks if empty.
    pub fn is_empty(&self) -> bool {
        self.blips.is_empty()
    }

    /// Borrows the internal list of blips.
    pub fn blips(&self) -> &[Blip] {
        &self.blips
    }

    /// Updates animation timers by `dt` seconds.
    pub fn tick(&mut self, dt: f32) {
        self.elapsed_time += dt;
    }

    /// Processes and projects all blips for current player position, heading, and radar configuration.
    ///
    /// Returns rendered blips sorted by priority (lowest first, highest last).
    pub fn process_blips(
        &self,
        player_pos: Vec3,
        player_heading: f32,
        radar: &RadarProjection,
    ) -> Vec<RenderedBlip> {
        let player_pos_2d = Vec2::new(player_pos.x, player_pos.y);
        let mut rendered = Vec::with_capacity(self.blips.len());

        for blip in &self.blips {
            let blip_pos_2d = Vec2::new(blip.world_pos.x, blip.world_pos.y);
            let radar_rel =
                RadarProjection::world_to_radar_rel(blip_pos_2d, player_pos_2d, player_heading);
            let raw_screen_pos = radar.radar_rel_to_screen(radar_rel);
            let screen_offset = raw_screen_pos - radar.config.center;

            let clamp_res =
                clamp_to_radar_border(screen_offset, radar.config.shape, self.border_margin);

            // If off-screen and blip is not configured to clamp, skip rendering
            if !clamp_res.inside && !blip.clamp_to_edge {
                continue;
            }

            let final_screen_pos = radar.config.center + clamp_res.screen_pos;

            // Compute relative altitude
            let delta_z = blip.world_pos.z - player_pos.z;
            let altitude = if delta_z > blip.altitude_threshold {
                AltitudeRelation::Above
            } else if delta_z < -blip.altitude_threshold {
                AltitudeRelation::Below
            } else {
                AltitudeRelation::Level
            };

            // Calculate flashing color if enabled
            let effective_color = if blip.flashing {
                let flash_cycle = (self.elapsed_time * 5.0).sin();
                if blip.blip_type == BlipType::Cop {
                    if flash_cycle > 0.0 {
                        BlipColor::COP_BLUE
                    } else {
                        BlipColor::COP_RED
                    }
                } else {
                    let alpha = if flash_cycle > 0.0 { 1.0 } else { 0.2 };
                    BlipColor::new(
                        blip.color.r,
                        blip.color.g,
                        blip.color.b,
                        blip.color.a * alpha,
                    )
                }
            } else {
                blip.color
            };

            // Relative heading
            let relative_heading = blip.heading.map(|h| h - player_heading);

            rendered.push(RenderedBlip {
                id: blip.id,
                screen_pos: final_screen_pos,
                radar_rel,
                altitude,
                delta_z,
                is_clamped: clamp_res.clamped,
                is_inside: clamp_res.inside,
                border_angle: if clamp_res.clamped {
                    Some(clamp_res.border_angle)
                } else {
                    None
                },
                effective_color,
                priority: blip.priority,
                scale: blip.scale,
                blip_type: blip.blip_type.clone(),
                relative_heading,
            });
        }

        // Sort by render priority ascending so higher priorities render on top
        rendered.sort_by_key(|b| b.priority);
        rendered
    }
}
