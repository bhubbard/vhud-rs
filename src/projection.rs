use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};

/// Shape of the minimap radar display.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum RadarShape {
    /// Circular radar (GTA IV, GTA V default).
    Circular {
        /// Radius of the circular radar in screen pixels.
        radius: f32,
    },
    /// Rectangular radar (GTA V expanded minimap mode).
    Rectangular {
        /// Full width of the rectangular radar in screen pixels.
        width: f32,
        /// Full height of the rectangular radar in screen pixels.
        height: f32,
    },
}

impl RadarShape {
    /// Returns the circular radius if this is a circular radar.
    #[inline]
    pub fn circle_radius(&self) -> Option<f32> {
        match *self {
            RadarShape::Circular { radius } => Some(radius),
            _ => None,
        }
    }

    /// Returns the half-extents `(width / 2, height / 2)` if rectangular, or `(radius, radius)` if circular.
    #[inline]
    pub fn half_extents(&self) -> Vec2 {
        match *self {
            RadarShape::Circular { radius } => Vec2::splat(radius),
            RadarShape::Rectangular { width, height } => Vec2::new(width * 0.5, height * 0.5),
        }
    }
}

/// Vehicle movement state for dynamic zoom computation.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum VehicleState {
    /// Player is on foot.
    OnFoot,
    /// Player is driving or riding a vehicle with a current speed in km/h.
    InVehicle { speed_kmh: f32 },
}

/// Dynamic radar zoom manager that smoothly transitions zoom based on player velocity and locomotion state.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DynamicZoom {
    /// Current interpolated zoom factor.
    pub current_zoom: f32,
    /// Target zoom factor.
    pub target_zoom: f32,
    /// Zoom factor when on foot (default ~1.2).
    pub on_foot_zoom: f32,
    /// Zoom factor when driving at normal speeds (default ~0.75).
    pub vehicle_normal_zoom: f32,
    /// Minimum zoom factor at maximum vehicle speed (default ~0.4).
    pub vehicle_high_speed_zoom: f32,
    /// Threshold speed in km/h where high-speed dynamic zoom begins (default 120.0 km/h).
    pub high_speed_threshold_kmh: f32,
    /// Reference maximum speed in km/h where zoom reaches `vehicle_high_speed_zoom` (default 240.0 km/h).
    pub max_speed_kmh: f32,
    /// Smoothing speed coefficient (higher = faster transition).
    pub transition_speed: f32,
}

impl Default for DynamicZoom {
    fn default() -> Self {
        Self {
            current_zoom: 1.2,
            target_zoom: 1.2,
            on_foot_zoom: 1.2,
            vehicle_normal_zoom: 0.75,
            vehicle_high_speed_zoom: 0.4,
            high_speed_threshold_kmh: 120.0,
            max_speed_kmh: 240.0,
            transition_speed: 4.0,
        }
    }
}

impl DynamicZoom {
    /// Constructs a new dynamic zoom manager with standard GTA V settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Computes the exact target zoom level for a given vehicle state.
    pub fn calculate_target(&self, state: VehicleState) -> f32 {
        match state {
            VehicleState::OnFoot => self.on_foot_zoom,
            VehicleState::InVehicle { speed_kmh } => {
                let speed = speed_kmh.max(0.0);
                if speed <= self.high_speed_threshold_kmh {
                    // Smooth transition from on foot to normal vehicle zoom at low speeds
                    let t = (speed / self.high_speed_threshold_kmh.max(1.0)).clamp(0.0, 1.0);
                    self.on_foot_zoom + (self.vehicle_normal_zoom - self.on_foot_zoom) * t
                } else {
                    // High-speed zoom out from normal vehicle zoom to high speed zoom
                    let speed_range = (self.max_speed_kmh - self.high_speed_threshold_kmh).max(1.0);
                    let t = ((speed - self.high_speed_threshold_kmh) / speed_range).clamp(0.0, 1.0);
                    self.vehicle_normal_zoom
                        + (self.vehicle_high_speed_zoom - self.vehicle_normal_zoom) * t
                }
            }
        }
    }

    /// Updates the current zoom factor towards the target using frame delta time.
    pub fn update(&mut self, state: VehicleState, dt: f32) -> f32 {
        self.target_zoom = self.calculate_target(state);
        let factor = 1.0 - (-self.transition_speed * dt.max(0.0)).exp();
        self.current_zoom += (self.target_zoom - self.current_zoom) * factor;
        self.current_zoom
    }

    /// Sets zoom immediately without smoothing.
    pub fn set_immediate(&mut self, zoom: f32) {
        self.current_zoom = zoom;
        self.target_zoom = zoom;
    }
}

/// Configuration of the Radar / Minimap viewport.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RadarConfig {
    /// Screen center position of the radar `(x, y)` in pixels.
    pub center: Vec2,
    /// Physical geometry of the radar.
    pub shape: RadarShape,
    /// Base zoom scale multiplier (default 1.0).
    pub base_scale: f32,
    /// Border thickness in pixels.
    pub border_thickness: f32,
}

impl Default for RadarConfig {
    fn default() -> Self {
        Self {
            center: Vec2::new(150.0, 850.0),
            shape: RadarShape::Circular { radius: 100.0 },
            base_scale: 1.0,
            border_thickness: 3.0,
        }
    }
}

impl RadarConfig {
    /// Creates a circular radar configuration.
    pub fn new_circular(center: Vec2, radius: f32) -> Self {
        Self {
            center,
            shape: RadarShape::Circular { radius },
            base_scale: 1.0,
            border_thickness: 3.0,
        }
    }

    /// Creates a rectangular radar configuration.
    pub fn new_rectangular(center: Vec2, width: f32, height: f32) -> Self {
        Self {
            center,
            shape: RadarShape::Rectangular { width, height },
            base_scale: 1.0,
            border_thickness: 3.0,
        }
    }
}

/// Radar Projection system responsible for transforming between 3D/2D World coordinates,
/// Player-relative radar space, and 2D Screen space.
#[derive(Debug, Clone)]
pub struct RadarProjection {
    pub config: RadarConfig,
    pub zoom: DynamicZoom,
}

impl RadarProjection {
    /// Creates a new radar projection with the specified config.
    pub fn new(config: RadarConfig) -> Self {
        Self {
            config,
            zoom: DynamicZoom::default(),
        }
    }

    /// Gets effective zoom scale $S_z = \text{base\_scale} \cdot \text{current\_zoom}$.
    #[inline]
    pub fn effective_zoom(&self) -> f32 {
        self.config.base_scale * self.zoom.current_zoom
    }

    /// Converts world coordinates to player-relative radar coordinates $(X_r, Y_r)$.
    ///
    /// Given player position $\mathbf{P} = (X_p, Y_p)$ and world point $\mathbf{W} = (X_w, Y_w)$:
    ///
    /// $$\Delta X = X_w - X_p, \quad \Delta Y = Y_w - Y_p$$
    ///
    /// Rotate by player yaw/heading $\theta$:
    ///
    /// $$X_r = \Delta X \cos \theta + \Delta Y \sin \theta$$
    /// $$Y_r = -\Delta X \sin \theta + \Delta Y \cos \theta$$
    ///
    /// Here $+Y_r$ points directly forward along player heading, and $+X_r$ points to player's right.
    #[inline]
    pub fn world_to_radar_rel(world_point: Vec2, player_pos: Vec2, player_heading: f32) -> Vec2 {
        let delta = world_point - player_pos;
        let (sin_t, cos_t) = player_heading.sin_cos();

        let xr = delta.x * cos_t + delta.y * sin_t;
        let yr = -delta.x * sin_t + delta.y * cos_t;

        Vec2::new(xr, yr)
    }

    /// Inverse transformation: converts player-relative radar coordinates $(X_r, Y_r)$
    /// back to 2D world coordinates.
    ///
    /// $$\Delta X = X_r \cos \theta - Y_r \sin \theta$$
    /// $$\Delta Y = X_r \sin \theta + Y_r \cos \theta$$
    ///
    /// $$\mathbf{W} = \mathbf{P} + (\Delta X, \Delta Y)$$
    #[inline]
    pub fn radar_rel_to_world(radar_rel: Vec2, player_pos: Vec2, player_heading: f32) -> Vec2 {
        let (sin_t, cos_t) = player_heading.sin_cos();
        let dx = radar_rel.x * cos_t - radar_rel.y * sin_t;
        let dy = radar_rel.x * sin_t + radar_rel.y * cos_t;

        player_pos + Vec2::new(dx, dy)
    }

    /// Converts radar relative coordinates $(X_r, Y_r)$ to screen pixel coordinates $(x_{screen}, y_{screen})$.
    ///
    /// $$x_{screen} = x_{center} + X_r \cdot S_z$$
    /// $$y_{screen} = y_{center} - Y_r \cdot S_z$$
    #[inline]
    pub fn radar_rel_to_screen(&self, radar_rel: Vec2) -> Vec2 {
        let sz = self.effective_zoom();
        Vec2::new(
            self.config.center.x + radar_rel.x * sz,
            self.config.center.y - radar_rel.y * sz,
        )
    }

    /// Converts screen pixel coordinates back to radar relative coordinates.
    #[inline]
    pub fn screen_to_radar_rel(&self, screen_pos: Vec2) -> Vec2 {
        let sz = self.effective_zoom();
        let inv_sz = if sz.abs() > 1e-6 { 1.0 / sz } else { 0.0 };
        Vec2::new(
            (screen_pos.x - self.config.center.x) * inv_sz,
            (self.config.center.y - screen_pos.y) * inv_sz,
        )
    }

    /// Direct full transformation from 3D/2D world coordinates to screen pixel coordinates.
    #[inline]
    pub fn world_to_screen(
        &self,
        world_point: Vec2,
        player_pos: Vec2,
        player_heading: f32,
    ) -> Vec2 {
        let radar_rel = Self::world_to_radar_rel(world_point, player_pos, player_heading);
        self.radar_rel_to_screen(radar_rel)
    }

    /// Direct full transformation from 3D world coordinates to screen pixel coordinates.
    #[inline]
    pub fn world3d_to_screen(
        &self,
        world_point: Vec3,
        player_pos: Vec3,
        player_heading: f32,
    ) -> Vec2 {
        self.world_to_screen(
            Vec2::new(world_point.x, world_point.y),
            Vec2::new(player_pos.x, player_pos.y),
            player_heading,
        )
    }

    /// Inverse transformation: converts screen pixel coordinates to 2D world coordinates.
    #[inline]
    pub fn screen_to_world(&self, screen_pos: Vec2, player_pos: Vec2, player_heading: f32) -> Vec2 {
        let radar_rel = self.screen_to_radar_rel(screen_pos);
        Self::radar_rel_to_world(radar_rel, player_pos, player_heading)
    }

    /// Checks if a screen position lies within the radar boundaries.
    pub fn is_screen_point_inside(&self, screen_pos: Vec2) -> bool {
        let offset = screen_pos - self.config.center;
        match self.config.shape {
            RadarShape::Circular { radius } => offset.length_squared() <= radius * radius,
            RadarShape::Rectangular { width, height } => {
                let hw = width * 0.5;
                let hh = height * 0.5;
                offset.x.abs() <= hw && offset.y.abs() <= hh
            }
        }
    }

    /// Checks if a relative radar point lies within the radar boundaries.
    pub fn is_radar_rel_inside(&self, radar_rel: Vec2) -> bool {
        let screen_pos = self.radar_rel_to_screen(radar_rel);
        self.is_screen_point_inside(screen_pos)
    }

    /// Returns the maximum visible radius / distance in world units at current zoom level.
    pub fn visible_world_radius(&self) -> f32 {
        let half_extents = self.config.shape.half_extents();
        let max_screen_extent = half_extents.x.max(half_extents.y);
        let sz = self.effective_zoom();
        if sz > 1e-6 {
            max_screen_extent / sz
        } else {
            f32::INFINITY
        }
    }
}
