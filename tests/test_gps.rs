use glam::{Vec2, Vec3};
use vhud_rs::gps::{GpsRoute, RouteNavigator, circular_clip, cohen_sutherland, liang_barsky};
use vhud_rs::projection::{RadarConfig, RadarProjection};

#[test]
fn test_cohen_sutherland_clipping() {
    let xmin = 0.0;
    let xmax = 100.0;
    let ymin = 0.0;
    let ymax = 100.0;

    // Inside segment
    let res_in = cohen_sutherland::clip_segment(
        Vec2::new(10.0, 10.0),
        Vec2::new(90.0, 90.0),
        xmin,
        xmax,
        ymin,
        ymax,
    );
    assert!(res_in.is_some());
    let (p0, p1) = res_in.unwrap();
    assert_eq!(p0, Vec2::new(10.0, 10.0));
    assert_eq!(p1, Vec2::new(90.0, 90.0));

    // Outside segment completely
    let res_out = cohen_sutherland::clip_segment(
        Vec2::new(150.0, 10.0),
        Vec2::new(180.0, 90.0),
        xmin,
        xmax,
        ymin,
        ymax,
    );
    assert!(res_out.is_none());

    // Crossing horizontal boundary
    let res_cross = cohen_sutherland::clip_segment(
        Vec2::new(-50.0, 50.0),
        Vec2::new(150.0, 50.0),
        xmin,
        xmax,
        ymin,
        ymax,
    );
    assert!(res_cross.is_some());
    let (p0, p1) = res_cross.unwrap();
    assert!((p0.x - 0.0).abs() < 1e-4);
    assert!((p0.y - 50.0).abs() < 1e-4);
    assert!((p1.x - 100.0).abs() < 1e-4);
    assert!((p1.y - 50.0).abs() < 1e-4);
}

#[test]
fn test_liang_barsky_clipping() {
    let xmin = 0.0;
    let xmax = 100.0;
    let ymin = 0.0;
    let ymax = 100.0;

    // Crossing diagonal
    let res = liang_barsky::clip_segment(
        Vec2::new(-20.0, -20.0),
        Vec2::new(120.0, 120.0),
        xmin,
        xmax,
        ymin,
        ymax,
    );
    assert!(res.is_some());
    let (p0, p1) = res.unwrap();
    assert!((p0.x - 0.0).abs() < 1e-4);
    assert!((p0.y - 0.0).abs() < 1e-4);
    assert!((p1.x - 100.0).abs() < 1e-4);
    assert!((p1.y - 100.0).abs() < 1e-4);
}

#[test]
fn test_circular_clip() {
    let center = Vec2::new(100.0, 100.0);
    let radius = 50.0;

    // Line right through center from (0, 100) to (200, 100)
    let p0 = Vec2::new(0.0, 100.0);
    let p1 = Vec2::new(200.0, 100.0);

    let res = circular_clip::clip_segment(p0, p1, center, radius);
    assert!(res.is_some());
    let (cp0, cp1) = res.unwrap();
    assert!((cp0.x - 50.0).abs() < 1e-4);
    assert!((cp0.y - 100.0).abs() < 1e-4);
    assert!((cp1.x - 150.0).abs() < 1e-4);
    assert!((cp1.y - 100.0).abs() < 1e-4);

    // Segment completely outside circle
    let out =
        circular_clip::clip_segment(Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0), center, radius);
    assert!(out.is_none());
}

#[test]
fn test_gps_route_navigation_and_arrival() {
    let wps = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(100.0, 0.0, 0.0),
        Vec3::new(100.0, 100.0, 0.0),
    ];
    let route = GpsRoute::new(wps);

    // Total length: 100 + 100 = 200m
    assert!((route.total_length() - 200.0).abs() < 1e-4);

    // Player at (50, 0, 0)
    let remaining = route.remaining_distance(Vec3::new(50.0, 0.0, 0.0));
    assert!((remaining - 150.0).abs() < 1.0);

    // Not arrived yet
    assert!(!route.has_arrived(Vec3::new(50.0, 0.0, 0.0)));

    // Arrived at destination (within arrival_threshold 15m)
    assert!(route.has_arrived(Vec3::new(95.0, 98.0, 0.0)));
}

#[test]
fn test_route_navigator_render_and_arrows() {
    let config = RadarConfig::new_circular(Vec2::new(200.0, 200.0), 100.0);
    let mut proj = RadarProjection::new(config);
    proj.zoom.set_immediate(1.0);

    let mut nav = RouteNavigator::new();
    let route = GpsRoute::new(vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(50.0, 0.0, 0.0),
        Vec3::new(50.0, 80.0, 0.0),
    ]);
    nav.set_route(route);

    let rendered = nav
        .render_route(Vec3::ZERO, 0.0, &proj)
        .expect("Should render route");

    assert!(!rendered.segments.is_empty());
    assert!(!rendered.arrived);
}
