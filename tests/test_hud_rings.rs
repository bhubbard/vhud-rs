use std::f32::consts::PI;
use vhud_rs::blip::BlipColor;
use vhud_rs::hud_rings::{ArcMeter, HudRingSystem, MeterType};
use vhud_rs::projection::RadarConfig;

#[test]
fn test_arc_meter_coverage_and_percentage() {
    let mut meter = ArcMeter::new(
        MeterType::Health,
        0.0,
        PI,
        100.0,
        110.0,
        BlipColor::new(0.0, 1.0, 0.0, 1.0),
    );

    meter.set_value(100.0);
    assert!((meter.fill_percentage() - 1.0).abs() < 1e-4);
    let (s, e) = meter.calculate_coverage_arc();
    assert_eq!(s, 0.0);
    assert!((e - PI).abs() < 1e-4);

    meter.set_value(50.0);
    assert!((meter.fill_percentage() - 0.5).abs() < 1e-4);
    let (s2, e2) = meter.calculate_coverage_arc();
    assert_eq!(s2, 0.0);
    assert!((e2 - PI * 0.5).abs() < 1e-4);

    meter.set_value(0.0);
    assert!((meter.fill_percentage() - 0.0).abs() < 1e-4);
    let (s3, e3) = meter.calculate_coverage_arc();
    assert_eq!(s3, 0.0);
    assert_eq!(e3, 0.0);
}

#[test]
fn test_low_health_pulsing() {
    let mut meter = ArcMeter::new(
        MeterType::Health,
        0.0,
        PI,
        100.0,
        110.0,
        BlipColor::new(0.0, 1.0, 0.0, 1.0),
    );
    meter.low_value_threshold = 0.25;

    meter.set_value(50.0);
    assert!(!meter.is_pulsing);

    // Below 25% activates pulsing
    meter.set_value(20.0);
    assert!(meter.is_pulsing);

    // Pulse factor oscillates
    let f1 = meter.pulse_factor(0.0);
    let f2 = meter.pulse_factor(0.25 / meter.pulse_frequency);
    assert!((f1 - f2).abs() > 0.1);
}

#[test]
fn test_hud_ring_system_defaults_and_process() {
    let mut sys = HudRingSystem::new_gta_v_circular(100.0, 6.0);
    let radar_config = RadarConfig::default();

    assert!(sys.set_health(20.0)); // low health -> returns true (pulsing)
    sys.set_armor(75.0);
    sys.tick(0.5);

    let rendered = sys.process_meters(&radar_config);
    assert_eq!(rendered.len(), 3);

    let health_rendered = rendered
        .iter()
        .find(|m| m.meter_type == MeterType::Health)
        .unwrap();
    assert!(health_rendered.is_pulsing);
    assert!((health_rendered.fill_percentage - 0.2).abs() < 1e-4);

    let armor_rendered = rendered
        .iter()
        .find(|m| m.meter_type == MeterType::Armor)
        .unwrap();
    assert!(!armor_rendered.is_pulsing);
    assert!((armor_rendered.fill_percentage - 0.75).abs() < 1e-4);
}
