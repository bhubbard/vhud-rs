use serde::{Deserialize, Serialize};
use std::f32::consts::PI;

use crate::blip::BlipColor;
use crate::projection::RadarConfig;

/// Category of HUD ring meter.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MeterType {
    /// Player health meter (green, pulses red when critical).
    Health,
    /// Body armor protection meter (blue).
    Armor,
    /// Physical stamina / lung capacity meter (yellow).
    Stamina,
    /// Special character ability gauge (yellow / orange).
    SpecialAbility,
    /// Custom user-defined HUD ring meter.
    Custom(String),
}

/// HUD meter layout mode around the radar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HudRingLayout {
    /// Radial arcs encircling the circular radar perimeter.
    CircularArcs,
    /// Horizontal segmented bars underneath rectangular radar.
    RectangularBars,
}

/// A circular arc meter encircling the radar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArcMeter {
    /// Meter category.
    pub meter_type: MeterType,
    /// Starting angle in radians (screen space: 0 = East/Right, PI/2 = South/Down).
    pub start_angle_rad: f32,
    /// Ending angle in radians.
    pub end_angle_rad: f32,
    /// Inner radius from radar center in pixels.
    pub inner_radius: f32,
    /// Outer radius from radar center in pixels.
    pub outer_radius: f32,
    /// Current meter value.
    pub current_value: f32,
    /// Maximum meter value.
    pub max_value: f32,
    /// Foreground fill color.
    pub color: BlipColor,
    /// Background bar fill color (semi-transparent backdrop).
    pub background_color: BlipColor,
    /// Low value threshold percentage (0.0 to 1.0) below which warning pulse activates.
    pub low_value_threshold: f32,
    /// Whether this meter is pulsing.
    pub is_pulsing: bool,
    /// Frequency of flashing/pulsing in Hz.
    pub pulse_frequency: f32,
    /// Optional critical warning color during pulsing (e.g. Red for critical health).
    pub critical_color: Option<BlipColor>,
}

impl ArcMeter {
    /// Creates a new arc meter.
    pub fn new(
        meter_type: MeterType,
        start_angle_rad: f32,
        end_angle_rad: f32,
        inner_radius: f32,
        outer_radius: f32,
        color: BlipColor,
    ) -> Self {
        Self {
            meter_type,
            start_angle_rad,
            end_angle_rad,
            inner_radius,
            outer_radius,
            current_value: 100.0,
            max_value: 100.0,
            color,
            background_color: BlipColor::new(0.08, 0.08, 0.10, 0.55),
            low_value_threshold: 0.25,
            is_pulsing: false,
            pulse_frequency: 3.5,
            critical_color: None,
        }
    }

    /// Fill fraction between 0.0 and 1.0.
    #[inline]
    pub fn fill_percentage(&self) -> f32 {
        if self.max_value <= 1e-6 {
            0.0
        } else {
            (self.current_value / self.max_value).clamp(0.0, 1.0)
        }
    }

    /// Sets the current value, automatically triggering pulsing if below low threshold.
    pub fn set_value(&mut self, value: f32) {
        self.current_value = value.clamp(0.0, self.max_value);
        let pct = self.fill_percentage();
        self.is_pulsing = pct <= self.low_value_threshold && pct > 0.0;
    }

    /// Computes the start and end angles for the active fill arc.
    pub fn calculate_coverage_arc(&self) -> (f32, f32) {
        let pct = self.fill_percentage();
        let total_sweep = self.end_angle_rad - self.start_angle_rad;
        let fill_end = self.start_angle_rad + total_sweep * pct;
        (self.start_angle_rad, fill_end)
    }

    /// Computes the pulsing alpha / flash multiplier for a given time in seconds.
    pub fn pulse_factor(&self, time_sec: f32) -> f32 {
        if !self.is_pulsing {
            return 1.0;
        }
        // Sine pulsation between 0.3 and 1.0
        let phase = (time_sec * self.pulse_frequency * 2.0 * PI).sin();
        0.65 + 0.35 * phase
    }

    /// Returns the effective color at `time_sec` factoring in warning flashes.
    pub fn effective_color(&self, time_sec: f32) -> BlipColor {
        if !self.is_pulsing {
            return self.color;
        }

        let factor = self.pulse_factor(time_sec);
        if let Some(crit) = self.critical_color {
            // Flash between main color and critical color
            let flash_switch = (time_sec * self.pulse_frequency * 2.0 * PI).sin() > 0.0;
            if flash_switch { crit } else { self.color }
        } else {
            BlipColor::new(
                self.color.r,
                self.color.g,
                self.color.b,
                self.color.a * factor,
            )
        }
    }
}

/// Rendered representation of an arc meter for screen drawing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenderedArcMeter {
    pub meter_type: MeterType,
    pub inner_radius: f32,
    pub outer_radius: f32,
    pub total_arc: (f32, f32),
    pub fill_arc: (f32, f32),
    pub fill_percentage: f32,
    pub active_color: BlipColor,
    pub background_color: BlipColor,
    pub is_pulsing: bool,
}

/// Complete HUD ring meter management system.
#[derive(Debug, Clone)]
pub struct HudRingSystem {
    pub meters: Vec<ArcMeter>,
    pub layout: HudRingLayout,
    pub elapsed_time: f32,
}

impl Default for HudRingSystem {
    fn default() -> Self {
        Self::new_gta_v_circular(100.0, 6.0)
    }
}

impl HudRingSystem {
    /// Creates a GTA V circular ring meter setup encircling a circular radar.
    ///
    /// - Health: Green arc spanning bottom-left quadrant (PI to PI/2).
    /// - Armor: Blue arc spanning bottom-right quadrant (PI/2 to 0).
    /// - Stamina: Yellow arc inner ring.
    /// - Special Ability: Orange arc outer ring.
    pub fn new_gta_v_circular(radar_radius: f32, meter_thickness: f32) -> Self {
        let r_in = radar_radius + 4.0;
        let r_out = r_in + meter_thickness;

        // Health arc: bottom-left quadrant from PI (left) down to PI * 0.5 (bottom)
        // With small angular gap in the center bottom
        let angle_gap = 0.06; // radians gap between health and armor
        let health_start = PI - 0.02;
        let health_end = PI * 0.5 + angle_gap;

        let mut health = ArcMeter::new(
            MeterType::Health,
            health_start,
            health_end,
            r_in,
            r_out,
            BlipColor::new(0.20, 0.72, 0.28, 0.95), // GTA V Green
        );
        health.critical_color = Some(BlipColor::new(0.95, 0.15, 0.15, 1.0)); // Red flash on low health
        health.low_value_threshold = 0.25;

        // Armor arc: bottom-right quadrant from PI * 0.5 - gap to 0.02
        let armor_start = PI * 0.5 - angle_gap;
        let armor_end = 0.02;

        let armor = ArcMeter::new(
            MeterType::Armor,
            armor_start,
            armor_end,
            r_in,
            r_out,
            BlipColor::new(0.22, 0.52, 0.88, 0.95), // GTA V Blue
        );

        // Special Ability arc: outer thin ring
        let r_spec_in = r_out + 3.0;
        let r_spec_out = r_spec_in + (meter_thickness * 0.75);
        let special = ArcMeter::new(
            MeterType::SpecialAbility,
            health_start,
            health_end,
            r_spec_in,
            r_spec_out,
            BlipColor::new(0.98, 0.75, 0.15, 0.90), // GTA V Yellow/Orange
        );

        Self {
            meters: vec![health, armor, special],
            layout: HudRingLayout::CircularArcs,
            elapsed_time: 0.0,
        }
    }

    /// Advances the meter system animation timers by `dt` seconds.
    pub fn tick(&mut self, dt: f32) {
        self.elapsed_time += dt;
    }

    /// Finds a meter by type.
    pub fn get_meter_mut(&mut self, meter_type: &MeterType) -> Option<&mut ArcMeter> {
        self.meters.iter_mut().find(|m| &m.meter_type == meter_type)
    }

    /// Sets the health value and returns whether it is in critical low health state.
    pub fn set_health(&mut self, value: f32) -> bool {
        if let Some(m) = self.get_meter_mut(&MeterType::Health) {
            m.set_value(value);
            m.is_pulsing
        } else {
            false
        }
    }

    /// Sets the armor value.
    pub fn set_armor(&mut self, value: f32) {
        if let Some(m) = self.get_meter_mut(&MeterType::Armor) {
            m.set_value(value);
        }
    }

    /// Sets the special ability value.
    pub fn set_special_ability(&mut self, value: f32) {
        if let Some(m) = self.get_meter_mut(&MeterType::SpecialAbility) {
            m.set_value(value);
        }
    }

    /// Computes screen rendering payloads for all active meters.
    pub fn process_meters(&self, _radar_config: &RadarConfig) -> Vec<RenderedArcMeter> {
        self.meters
            .iter()
            .map(|m| {
                let fill_arc = m.calculate_coverage_arc();
                let active_color = m.effective_color(self.elapsed_time);
                RenderedArcMeter {
                    meter_type: m.meter_type.clone(),
                    inner_radius: m.inner_radius,
                    outer_radius: m.outer_radius,
                    total_arc: (m.start_angle_rad, m.end_angle_rad),
                    fill_arc,
                    fill_percentage: m.fill_percentage(),
                    active_color,
                    background_color: m.background_color,
                    is_pulsing: m.is_pulsing,
                }
            })
            .collect()
    }
}
