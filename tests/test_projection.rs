use glam::Vec2;
use std::f32::consts::PI;
use vhud_rs::projection::{DynamicZoom, RadarConfig, RadarProjection, VehicleState};

#[test]
fn test_coordinate_rotation_cardinal_directions() {
    let player_pos = Vec2::new(100.0, 100.0);

    // Heading 0: Facing North (+Y)
    let heading_north = 0.0;
    // Target 50m North of player: (100, 150)
    let target_north = Vec2::new(100.0, 150.0);
    let r_north = RadarProjection::world_to_radar_rel(target_north, player_pos, heading_north);
    assert!(
        (r_north.x - 0.0).abs() < 1e-4,
        "Xr should be 0 (straight ahead)"
    );
    assert!(
        (r_north.y - 50.0).abs() < 1e-4,
        "Yr should be +50 (forward)"
    );

    // Target 50m East of player: (150, 100)
    let target_east = Vec2::new(150.0, 100.0);
    let r_east = RadarProjection::world_to_radar_rel(target_east, player_pos, heading_north);
    assert!(
        (r_east.x - 50.0).abs() < 1e-4,
        "Xr should be +50 (to the right)"
    );
    assert!((r_east.y - 0.0).abs() < 1e-4, "Yr should be 0");

    // Heading -PI/2: Player turned to face East (+X)
    let heading_east = -PI * 0.5;
    let r_target_when_facing_east =
        RadarProjection::world_to_radar_rel(target_east, player_pos, heading_east);
    // East target is now directly in front of player
    assert!((r_target_when_facing_east.x - 0.0).abs() < 1e-4);
    assert!((r_target_when_facing_east.y - 50.0).abs() < 1e-4);

    // Heading +PI/2: Player turned to face West (-X)
    let heading_west = PI * 0.5;
    let target_west = Vec2::new(50.0, 100.0); // 50m West
    let r_target_when_facing_west =
        RadarProjection::world_to_radar_rel(target_west, player_pos, heading_west);
    assert!((r_target_when_facing_west.x - 0.0).abs() < 1e-4);
    assert!((r_target_when_facing_west.y - 50.0).abs() < 1e-4);
}

#[test]
fn test_world_to_radar_round_trip() {
    let player_pos = Vec2::new(-245.5, 1890.3);
    let heading = 1.345; // arbitrary angle in radians
    let world_point = Vec2::new(512.0, -83.4);

    let radar_rel = RadarProjection::world_to_radar_rel(world_point, player_pos, heading);
    let reconstructed_world = RadarProjection::radar_rel_to_world(radar_rel, player_pos, heading);

    assert!((reconstructed_world.x - world_point.x).abs() < 1e-4);
    assert!((reconstructed_world.y - world_point.y).abs() < 1e-4);
}

#[test]
fn test_screen_projection_and_inverse() {
    let config = RadarConfig::new_circular(Vec2::new(200.0, 700.0), 120.0);
    let mut projection = RadarProjection::new(config);
    projection.zoom.set_immediate(1.5);

    let player_pos = Vec2::new(10.0, 20.0);
    let heading = 0.5;
    let world_pt = Vec2::new(35.0, 60.0);

    let screen_pos = projection.world_to_screen(world_pt, player_pos, heading);
    let back_to_world = projection.screen_to_world(screen_pos, player_pos, heading);

    assert!((back_to_world.x - world_pt.x).abs() < 1e-4);
    assert!((back_to_world.y - world_pt.y).abs() < 1e-4);
}

#[test]
fn test_screen_coords_orientation() {
    // In screen coords, positive Yr (forward) must project UP (lower Y screen)
    let config = RadarConfig::new_circular(Vec2::new(200.0, 500.0), 100.0);
    let projection = RadarProjection::new(config);

    let forward_radar = Vec2::new(0.0, 40.0); // 40 units ahead
    let screen = projection.radar_rel_to_screen(forward_radar);
    assert_eq!(screen.x, 200.0);
    assert!(
        screen.y < 500.0,
        "Forward point on screen should be above center"
    );

    let right_radar = Vec2::new(40.0, 0.0); // 40 units right
    let screen_right = projection.radar_rel_to_screen(right_radar);
    assert!(
        screen_right.x > 200.0,
        "Right point on screen should be right of center"
    );
    assert_eq!(screen_right.y, 500.0);
}

#[test]
fn test_bounds_inside_detection() {
    let circular_config = RadarConfig::new_circular(Vec2::new(100.0, 100.0), 50.0);
    let circular_proj = RadarProjection::new(circular_config);

    assert!(circular_proj.is_screen_point_inside(Vec2::new(100.0, 100.0)));
    assert!(circular_proj.is_screen_point_inside(Vec2::new(100.0, 140.0)));
    assert!(!circular_proj.is_screen_point_inside(Vec2::new(100.0, 160.0)));

    let rect_config = RadarConfig::new_rectangular(Vec2::new(100.0, 100.0), 80.0, 60.0);
    let rect_proj = RadarProjection::new(rect_config);

    // hw = 40, hh = 30
    assert!(rect_proj.is_screen_point_inside(Vec2::new(135.0, 125.0)));
    assert!(!rect_proj.is_screen_point_inside(Vec2::new(145.0, 100.0)));
    assert!(!rect_proj.is_screen_point_inside(Vec2::new(100.0, 135.0)));
}

#[test]
fn test_dynamic_zoom_states_and_lerp() {
    let mut zoom = DynamicZoom::default();

    // On foot target zoom
    let target_foot = zoom.calculate_target(VehicleState::OnFoot);
    assert!((target_foot - 1.2).abs() < 1e-4);

    // Vehicle at 120 km/h target zoom
    let target_veh_120 = zoom.calculate_target(VehicleState::InVehicle { speed_kmh: 120.0 });
    assert!((target_veh_120 - 0.75).abs() < 1e-4);

    // Vehicle at 240 km/h (max speed)
    let target_max = zoom.calculate_target(VehicleState::InVehicle { speed_kmh: 240.0 });
    assert!((target_max - 0.4).abs() < 1e-4);

    // Update with dt smoothly steps towards target
    zoom.set_immediate(1.2);
    let new_zoom = zoom.update(VehicleState::InVehicle { speed_kmh: 240.0 }, 0.1);
    assert!(
        new_zoom < 1.2,
        "Zoom should decrease toward high speed target"
    );
    assert!(new_zoom > 0.4, "Zoom should not instantly jump to target");
}
