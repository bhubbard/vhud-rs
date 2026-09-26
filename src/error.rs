use thiserror::Error;

/// Errors that may occur during radar projection, blip management, or GPS calculations.
#[derive(Debug, Error, PartialEq)]
pub enum VHudError {
    #[error("Invalid radar dimensions: width {width}, height {height} must be positive")]
    InvalidDimensions { width: f32, height: f32 },

    #[error("Invalid radar radius: {0} must be strictly positive")]
    InvalidRadius(f32),

    #[error("Invalid zoom factor: {0} must be greater than zero")]
    InvalidZoom(f32),

    #[error("Route has insufficient waypoints: found {0}, minimum 2 required")]
    InsufficientWaypoints(usize),

    #[error("Waypoint index {index} out of bounds for route length {len}")]
    WaypointOutOfBounds { index: usize, len: usize },
}

pub type Result<T> = std::result::Result<T, VHudError>;
