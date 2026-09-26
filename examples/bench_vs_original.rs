//! Benchmark comparing `vhud-rs` (Rust) vs original v-hud (C++ / FiveM Lua CEF minimap).

use glam::{Vec2, Vec3};
use std::time::Instant;
use vhud_rs::gps::{circular_clip, liang_barsky};
use vhud_rs::hud_rings::HudRingSystem;
use vhud_rs::projection::{RadarConfig, RadarProjection, RadarShape};
use vhud_rs::{clamp_to_radar_border, Blip, BlipManager, BlipType};

fn main() {
    println!("============================================================");
    println!("    vhud-rs (Rust) vs Original v-hud / FiveM NUI Minimap   ");
    println!("============================================================");

    let config = RadarConfig::new_circular(Vec2::new(150.0, 850.0), 100.0);
    let radar = RadarProjection::new(config.clone());

    // 1. Full Radar Blip Processing Pipeline (500 Blips)
    println!("\n--- 1. Multi-Blip Radar Projection & Border Clamping (500 Blips) ---");
    {
        let mut blip_mgr = BlipManager::new();
        for i in 0..500 {
            let angle = (i as f32) * 0.1;
            let dist = 50.0 + (i as f32) * 2.0;
            let pos = Vec3::new(angle.cos() * dist, angle.sin() * dist, 10.0);
            let blip_type = match i % 5 {
                0 => BlipType::Waypoint,
                1 => BlipType::Cop,
                2 => BlipType::Mission,
                3 => BlipType::Enemy,
                _ => BlipType::Neutral,
            };
            blip_mgr.add_or_update(Blip::new(i as u64, pos, blip_type));
        }

        let iterations = 100_000;
        let start = Instant::now();
        let mut total_rendered = 0;

        for i in 0..iterations {
            let t = (i as f32) * 0.01;
            let player_pos = Vec3::new(t.sin() * 100.0, t.cos() * 100.0, 10.0);
            let player_heading = (i % 360) as f32 * 0.01745;

            let rendered = blip_mgr.process_blips(player_pos, player_heading, &radar);
            total_rendered += rendered.len();
        }

        std::hint::black_box(total_rendered);
        let elapsed = start.elapsed();
        let ns_per_frame = elapsed.as_nanos() as f64 / iterations as f64;
        let frames_per_sec = iterations as f64 / elapsed.as_secs_f64();
        let ns_per_blip = ns_per_frame / 500.0;

        println!(
            "Frames (500 Blips): {} | Time: {:.2?} | Latency: {:.2} µs/frame ({:.2} ns/blip) | {:>10.0} frames/s",
            iterations, elapsed, ns_per_frame / 1000.0, ns_per_blip, frames_per_sec
        );
    }

    // 2. GPS Route Segment Clipping (Cohen-Sutherland & Liang-Barsky)
    println!("\n--- 2. GPS Vector Route Clipping (Circular & Liang-Barsky) ---");
    {
        let iterations = 5_000_000;
        let start = Instant::now();
        let mut clipped_segments = 0;

        for i in 0..iterations {
            let offset = (i % 200) as f32 - 100.0;
            let p0 = Vec2::new(offset, -150.0);
            let p1 = Vec2::new(offset * 0.5, 150.0);

            // Liang-Barsky rectangular clipping
            if let Some((_c0, _c1)) = liang_barsky::clip_segment(p0, p1, -100.0, 100.0, -100.0, 100.0) {
                clipped_segments += 1;
            }

            // Circular radar border clipping
            if let Some((_c0, _c1)) = circular_clip::clip_segment(p0, p1, Vec2::ZERO, 100.0) {
                clipped_segments += 1;
            }
        }

        std::hint::black_box(clipped_segments);
        let elapsed = start.elapsed();
        let ns_per_clip = elapsed.as_nanos() as f64 / (iterations * 2) as f64;
        let clips_per_sec = (iterations * 2) as f64 / elapsed.as_secs_f64();

        println!(
            "GPS Segment Clips: {} | Time: {:.2?} | Latency: {:.2} ns/clip | {:>10.0} clips/s",
            iterations * 2, elapsed, ns_per_clip, clips_per_sec
        );
    }

    // 3. Isolated Radar Border Clamping Throughput
    println!("\n--- 3. Radar Edge/Border Clamping Mathematics ---");
    {
        let iterations = 10_000_000;
        let shape = RadarShape::Circular { radius: 100.0 };
        let start = Instant::now();
        let mut clamped_count = 0;

        for i in 0..iterations {
            let raw = Vec2::new((i % 400) as f32 - 200.0, ((i / 400) % 400) as f32 - 200.0);
            let res = clamp_to_radar_border(raw, shape, 0.0);
            if res.clamped {
                clamped_count += 1;
            }
        }

        std::hint::black_box(clamped_count);
        let elapsed = start.elapsed();
        let ns_per_clamp = elapsed.as_nanos() as f64 / iterations as f64;
        let clamps_per_sec = iterations as f64 / elapsed.as_secs_f64();

        println!(
            "Border Clamps: {} | Time: {:.2?} | Latency: {:.2} ns/clamp | {:>10.0} clamps/s | Clamped: {}",
            iterations, elapsed, ns_per_clamp, clamps_per_sec, clamped_count
        );
    }

    // 4. Circular HUD Arc Rings Geometry Calculation
    println!("\n--- 4. HUD Arc Ring Meter Geometry Synthesis ---");
    {
        let mut rings = HudRingSystem::new_gta_v_circular(100.0, 6.0);
        let iterations = 2_000_000;
        let start = Instant::now();
        let mut total_meters = 0;

        for i in 0..iterations {
            let hp = ((i % 100) as f32) / 100.0 * 100.0;
            let armor = (((i + 25) % 100) as f32) / 100.0 * 100.0;

            rings.set_health(hp);
            rings.set_armor(armor);
            rings.tick(0.016);

            let rendered = rings.process_meters(&config);
            total_meters += rendered.len();
        }

        std::hint::black_box(total_meters);
        let elapsed = start.elapsed();
        let ns_per_ring_step = elapsed.as_nanos() as f64 / iterations as f64;
        let rings_per_sec = iterations as f64 / elapsed.as_secs_f64();

        println!(
            "HUD Ring Frames: {} | Time: {:.2?} | Latency: {:.2} ns/frame | {:>10.0} frames/s",
            iterations, elapsed, ns_per_ring_step, rings_per_sec
        );
    }

    println!("\n============================================================");
    println!("                      Benchmark Complete                    ");
    println!("============================================================");
}
