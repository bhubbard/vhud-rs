use glam::{Vec2, Vec3};
use std::f32::consts::PI;
use vhud_rs::blip::{AltitudeRelation, Blip, BlipManager, BlipType, clamp_to_radar_border};
use vhud_rs::projection::{RadarConfig, RadarProjection, RadarShape};

#[test]
fn test_circular_edge_clamping() {
    let shape = RadarShape::Circular { radius: 100.0 };
    let margin = 5.0; // clamp radius = 95.0

    // Point inside circle (e.g. 50 units away)
    let inside_pt = Vec2::new(30.0, 40.0); // length = 50
    let res_inside = clamp_to_radar_border(inside_pt, shape, margin);
    assert!(!res_inside.clamped);
    assert!(res_inside.inside);
    assert_eq!(res_inside.screen_pos, inside_pt);

    // Point far outside circle (e.g. 200 units to the right)
    let outside_pt = Vec2::new(200.0, 0.0);
    let res_outside = clamp_to_radar_border(outside_pt, shape, margin);
    assert!(res_outside.clamped);
    assert!(!res_outside.inside);
    assert!((res_outside.screen_pos.x - 95.0).abs() < 1e-4);
    assert!((res_outside.screen_pos.y - 0.0).abs() < 1e-4);
    assert!((res_outside.border_angle - 0.0).abs() < 1e-4);

    // Diagonal outside point (direction 45 deg)
    let diag_pt = Vec2::new(100.0, 100.0);
    let res_diag = clamp_to_radar_border(diag_pt, shape, margin);
    assert!(res_diag.clamped);
    assert!((res_diag.screen_pos.length() - 95.0).abs() < 1e-3);
    assert!((res_diag.border_angle - PI * 0.25).abs() < 1e-4);
}

#[test]
fn test_rectangular_edge_clamping() {
    let shape = RadarShape::Rectangular {
        width: 200.0,
        height: 100.0,
    };
    let margin = 0.0;
    // hw = 100, hh = 50

    // Point inside
    let inside = Vec2::new(50.0, 20.0);
    let res_in = clamp_to_radar_border(inside, shape, margin);
    assert!(!res_in.clamped);
    assert!(res_in.inside);

    // Point beyond right edge
    let out_right = Vec2::new(300.0, 0.0);
    let res_r = clamp_to_radar_border(out_right, shape, margin);
    assert!(res_r.clamped);
    assert!((res_r.screen_pos.x - 100.0).abs() < 1e-4);
    assert!((res_r.screen_pos.y - 0.0).abs() < 1e-4);

    // Point beyond top edge
    let out_top = Vec2::new(0.0, -200.0);
    let res_t = clamp_to_radar_border(out_top, shape, margin);
    assert!(res_t.clamped);
    assert!((res_t.screen_pos.x - 0.0).abs() < 1e-4);
    assert!((res_t.screen_pos.y - (-50.0)).abs() < 1e-4);

    // Ray at 45 degrees: (100, 100) -> box has hw=100, hh=50
    // Will hit the vertical boundary y = 50 first!
    let diag = Vec2::new(100.0, 100.0);
    let res_diag = clamp_to_radar_border(diag, shape, margin);
    assert!(res_diag.clamped);
    assert!((res_diag.screen_pos.y - 50.0).abs() < 1e-4);
    assert!((res_diag.screen_pos.x - 50.0).abs() < 1e-4);
}

#[test]
fn test_altitude_thresholds() {
    let config = RadarConfig::new_circular(Vec2::new(100.0, 100.0), 100.0);
    let proj = RadarProjection::new(config);
    let mut manager = BlipManager::new();

    let player_pos = Vec3::new(0.0, 0.0, 50.0);

    // Above blip (> 3.5m higher)
    manager.add_or_update(Blip::new(1, Vec3::new(10.0, 0.0, 60.0), BlipType::Enemy));
    // Below blip (> 3.5m lower)
    manager.add_or_update(Blip::new(2, Vec3::new(20.0, 0.0, 40.0), BlipType::Enemy));
    // Level blip (within 3.5m)
    manager.add_or_update(Blip::new(3, Vec3::new(30.0, 0.0, 51.0), BlipType::Enemy));

    let rendered = manager.process_blips(player_pos, 0.0, &proj);

    let blip1 = rendered.iter().find(|b| b.id == 1).unwrap();
    assert_eq!(blip1.altitude, AltitudeRelation::Above);

    let blip2 = rendered.iter().find(|b| b.id == 2).unwrap();
    assert_eq!(blip2.altitude, AltitudeRelation::Below);

    let blip3 = rendered.iter().find(|b| b.id == 3).unwrap();
    assert_eq!(blip3.altitude, AltitudeRelation::Level);
}

#[test]
fn test_blip_manager_priority_and_filtering() {
    let config = RadarConfig::new_circular(Vec2::new(100.0, 100.0), 100.0);
    let mut proj = RadarProjection::new(config);
    proj.zoom.set_immediate(1.0);

    let mut manager = BlipManager::new();
    let player = Vec3::ZERO;

    // Waypoint far away (clamp_to_edge = true)
    let wp = Blip::new(10, Vec3::new(1000.0, 0.0, 0.0), BlipType::Waypoint);
    // Enemy far away (clamp_to_edge = false)
    let enemy = Blip::new(20, Vec3::new(1000.0, 0.0, 0.0), BlipType::Enemy);

    manager.add_or_update(wp);
    manager.add_or_update(enemy);

    let rendered = manager.process_blips(player, 0.0, &proj);

    // The enemy without clamp_to_edge should be omitted
    assert_eq!(rendered.len(), 1);
    assert_eq!(rendered[0].id, 10);
    assert!(rendered[0].is_clamped);
    assert!(rendered[0].border_angle.is_some());
}
