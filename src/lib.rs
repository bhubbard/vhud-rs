//! # vhud-rs
//!
//! Pure Rust GTA V radar, minimap projection, blip clamping, GPS route navigation,
//! and HUD ring meter engine.
//!
//! Ported and adapted from `gennariarmando/v-hud` (C++) architecture into idiomatic,
//! safe, high-performance Rust.
//!
//! # Quickstart Example
//!
//! ```rust
//! use glam::{Vec2, Vec3};
//! use vhud_rs::{RadarConfig, RadarProjection, Blip, BlipType, BlipManager, VehicleState};
//!
//! // 1. Configure circular radar display (radius 100px at center (150, 850))
//! let config = RadarConfig::new_circular(Vec2::new(150.0, 850.0), 100.0);
//! let mut radar = RadarProjection::new(config);
//!
//! // 2. Update dynamic zoom based on player vehicle state
//! radar.zoom.update(VehicleState::InVehicle { speed_kmh: 85.0 }, 0.016);
//!
//! // 3. Register blips in the world
//! let mut blip_mgr = BlipManager::new();
//! blip_mgr.add_or_update(Blip::new(1, Vec3::new(250.0, 100.0, 15.0), BlipType::Waypoint));
//!
//! // 4. Process blips for current player position and heading
//! let player_pos = Vec3::new(200.0, 80.0, 10.0);
//! let player_heading = 0.0; // facing North
//! let rendered = blip_mgr.process_blips(player_pos, player_heading, &radar);
//! assert_eq!(rendered.len(), 1);
//! ```

pub mod blip;
pub mod error;
pub mod gps;
pub mod hud_rings;
pub mod projection;

pub use blip::{
    AltitudeRelation, Blip, BlipColor, BlipManager, BlipType, ClampedResult, RenderedBlip,
    clamp_to_radar_border,
};
pub use error::{Result, VHudError};
pub use gps::{
    GpsArrow, GpsRoute, GpsSegment, RenderedGpsRoute, RouteNavigator, circular_clip,
    cohen_sutherland, liang_barsky,
};
pub use hud_rings::{ArcMeter, HudRingLayout, HudRingSystem, MeterType, RenderedArcMeter};
pub use projection::{DynamicZoom, RadarConfig, RadarProjection, RadarShape, VehicleState};
