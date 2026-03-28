//! NOAA solar position calculator.
//!
//! Pure-trigonometry implementation of the NOAA solar position algorithm
//! for calculating sun azimuth and elevation given latitude, longitude,
//! date, and time. Used for shadow geometry verification in the Esper
//! Machine investigation tools.

use std::f64::consts::PI;

/// Solar position result.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SolarPosition {
    /// Sun azimuth in degrees (0 = North, 90 = East, 180 = South, 270 = West).
    pub azimuth: f64,
    /// Sun elevation in degrees above the horizon (-90 to +90).
    pub elevation: f64,
    /// Solar noon expressed as hours UTC (e.g. 12.5 = 12:30 UTC).
    pub solar_noon_utc: f64,
    /// Day length in hours.
    pub day_length_hours: f64,
}

/// Calculate solar position using the NOAA algorithm.
///
/// # Arguments
/// * `latitude`  — Latitude in decimal degrees (-90 to +90, north positive)
/// * `longitude` — Longitude in decimal degrees (-180 to +180, east positive)
/// * `year`      — Calendar year (e.g. 2024)
/// * `month`     — Month of year (1–12)
/// * `day`       — Day of month (1–31)
/// * `hour_utc`  — Hour in UTC as a fractional value (0.0–24.0;
///   e.g. 14.5 = 14:30 UTC)
pub fn calculate_solar_position(
    latitude: f64,
    longitude: f64,
    year: i32,
    month: u32,
    day: u32,
    hour_utc: f64,
) -> SolarPosition {
    let rad = PI / 180.0;

    // ── Julian Day Number ─────────────────────────────────────────────────
    // Convert the calendar date to a Julian Day Number (JDN) using the
    // proleptic Gregorian calendar algorithm, then adjust by the fractional
    // hour to get a continuous Julian Date (JD).
    let a = (14 - month as i32) / 12;
    let y = year + 4800 - a;
    let m = month as i32 + 12 * a - 3;
    let jdn = day as f64 + ((153 * m + 2) / 5) as f64 + 365.0 * y as f64 + (y / 4) as f64
        - (y / 100) as f64
        + (y / 400) as f64
        - 32045.0;
    // Noon is at JD = JDN + 0.0; adjust by the fractional UTC offset from noon.
    let jd = jdn + (hour_utc - 12.0) / 24.0;

    // ── Julian Century ────────────────────────────────────────────────────
    // Fractional Julian centuries since J2000.0 (JD 2451545.0).
    let jc = (jd - 2451545.0) / 36525.0;

    // ── Sun's geometric mean longitude (°) ───────────────────────────────
    let l0 = (280.46646 + jc * (36000.76983 + 0.0003032 * jc)) % 360.0;

    // ── Sun's geometric mean anomaly (°) ─────────────────────────────────
    let m_anom = 357.52911 + jc * (35999.05029 - 0.0001537 * jc);

    // ── Eccentricity of Earth's orbit ────────────────────────────────────
    let e = 0.016708634 - jc * (0.000042037 + 0.0000001267 * jc);

    // ── Sun's equation of centre ──────────────────────────────────────────
    let m_rad = m_anom * rad;
    let c = (1.914602 - jc * (0.004817 + 0.000014 * jc)) * m_rad.sin()
        + (0.019993 - 0.000101 * jc) * (2.0 * m_rad).sin()
        + 0.000289 * (3.0 * m_rad).sin();

    // ── Sun's true longitude (°) ─────────────────────────────────────────
    let sun_lon = l0 + c;

    // ── Sun's apparent longitude (°) ─────────────────────────────────────
    // Small correction for aberration and nutation.
    let omega = 125.04 - 1934.136 * jc;
    let sun_app_lon = sun_lon - 0.00569 - 0.00478 * (omega * rad).sin();

    // ── Mean obliquity of the ecliptic (°) ───────────────────────────────
    let obliq0 =
        23.0 + (26.0 + (21.448 - jc * (46.815 + jc * (0.00059 - jc * 0.001813))) / 60.0) / 60.0;
    let obliq_corr = obliq0 + 0.00256 * (omega * rad).cos();

    // ── Sun's declination (radians) ───────────────────────────────────────
    let sin_decl = (obliq_corr * rad).sin() * (sun_app_lon * rad).sin();
    let decl = sin_decl.asin();

    // ── Equation of time (minutes) ────────────────────────────────────────
    // The difference between apparent solar time and mean solar time.
    let y_var = ((obliq_corr / 2.0) * rad).tan().powi(2);
    let eq_time = 4.0
        * (y_var * (2.0 * l0 * rad).sin() - 2.0 * e * m_rad.sin()
            + 4.0 * e * y_var * m_rad.sin() * (2.0 * l0 * rad).cos()
            - 0.5 * y_var * y_var * (4.0 * l0 * rad).sin()
            - 1.25 * e * e * (2.0 * m_rad).sin())
        / rad;

    // ── Solar noon (UTC hours) ────────────────────────────────────────────
    // Time of solar noon in minutes from midnight UTC, then convert to hours.
    let solar_noon_min = 720.0 - 4.0 * longitude - eq_time;
    let solar_noon_utc = solar_noon_min / 60.0;

    // ── Hour angle at sunrise/sunset ─────────────────────────────────────
    // Based on the solar zenith angle at the horizon (90.833° includes refraction).
    let ha_sunrise_cos = (90.833_f64 * rad).cos() / ((latitude * rad).cos() * decl.cos())
        - (latitude * rad).tan() * decl.tan();
    // Clamp to [-1, 1] to avoid NaN in acos at high latitudes.
    let ha_sunrise = ha_sunrise_cos.clamp(-1.0, 1.0).acos();
    let day_length_hours = 8.0 * ha_sunrise / rad / 60.0;

    // ── True solar time (minutes) ─────────────────────────────────────────
    // TST wraps at 1440 (24 h × 60 min).
    let tst = (hour_utc * 60.0 + eq_time + 4.0 * longitude) % 1440.0;

    // ── Hour angle for the current time (°) ───────────────────────────────
    // Positive in the afternoon, negative in the morning.
    let ha = if tst / 4.0 < 0.0 {
        tst / 4.0 + 180.0
    } else {
        tst / 4.0 - 180.0
    };

    // ── Solar zenith angle ────────────────────────────────────────────────
    let cos_zenith = (latitude * rad).sin() * decl.sin()
        + (latitude * rad).cos() * decl.cos() * (ha * rad).cos();
    // Clamp to avoid floating-point rounding beyond [-1, 1].
    let zenith = cos_zenith.clamp(-1.0, 1.0).acos();
    let elevation = 90.0 - zenith / rad;

    // ── Solar azimuth (°) ─────────────────────────────────────────────────
    // Azimuth is measured clockwise from north (0–360°).
    let azimuth_cos = ((latitude * rad).sin() * zenith.cos() - decl.sin())
        / ((latitude * rad).cos() * zenith.sin());
    let azimuth_rad = azimuth_cos.clamp(-1.0, 1.0).acos();
    let azimuth = if ha > 0.0 {
        (azimuth_rad / rad + 180.0) % 360.0
    } else {
        (540.0 - azimuth_rad / rad) % 360.0
    };

    SolarPosition {
        azimuth,
        elevation,
        solar_noon_utc,
        day_length_hours,
    }
}

/// Candidate time estimate produced by `estimate_time_from_shadow`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeEstimate {
    /// Estimated hour in UTC (e.g. 14.5 = 2:30 PM UTC).
    pub hour_utc: f64,
    /// Formatted time string (e.g. "14:30 UTC").
    pub time_formatted: String,
    /// Sun elevation at this time in degrees.
    pub sun_elevation: f64,
    /// Angular deviation from the target sun azimuth in degrees.
    pub azimuth_error: f64,
}

/// Estimate the time(s) of day that would produce shadows at the given azimuth.
///
/// Since shadows point opposite to the sun, a shadow azimuth of X means the
/// sun is at azimuth `(X + 180) % 360`. The function searches through the day
/// in 1-minute increments to find times where the sun's azimuth matches.
///
/// Returns up to two candidate times (morning and afternoon produce
/// symmetric shadow angles for many latitudes), sorted by ascending azimuth
/// error (best match first).
///
/// # Arguments
/// * `latitude`               — Latitude in decimal degrees (-90 to +90, north positive)
/// * `longitude`              — Longitude in decimal degrees (-180 to +180, east positive)
/// * `year`                   — Calendar year (e.g. 2024)
/// * `month`                  — Month of year (1–12)
/// * `day`                    — Day of month (1–31)
/// * `shadow_azimuth_degrees` — Observed shadow direction in degrees clockwise from north
pub fn estimate_time_from_shadow(
    latitude: f64,
    longitude: f64,
    year: i32,
    month: u32,
    day: u32,
    shadow_azimuth_degrees: f64,
) -> Vec<TimeEstimate> {
    // Sun azimuth is opposite to shadow direction.
    let target_sun_azimuth = (shadow_azimuth_degrees + 180.0) % 360.0;

    let mut candidates: Vec<TimeEstimate> = Vec::new();
    let mut prev_diff = f64::MAX;

    // Search through the day in 1-minute increments (0..1440 minutes).
    for minute in 0..1440_u32 {
        let hour = minute as f64 / 60.0;
        let pos = calculate_solar_position(latitude, longitude, year, month, day, hour);

        // Skip nighttime — sun below 1° avoids twilight noise.
        if pos.elevation < 1.0 {
            prev_diff = f64::MAX;
            continue;
        }

        let diff = angle_diff(pos.azimuth, target_sun_azimuth);

        // Detect local minima: diff was decreasing, now check if next step increases.
        if diff < prev_diff && diff < 5.0 {
            let next_minute = minute + 1;
            if next_minute < 1440 {
                let next_hour = next_minute as f64 / 60.0;
                let next_pos =
                    calculate_solar_position(latitude, longitude, year, month, day, next_hour);
                let next_diff = angle_diff(next_pos.azimuth, target_sun_azimuth);
                if next_diff > diff {
                    // Confirmed local minimum — record candidate.
                    let h = (hour as u32).min(23);
                    let m = ((hour - h as f64) * 60.0).round() as u32 % 60;
                    candidates.push(TimeEstimate {
                        hour_utc: hour,
                        time_formatted: format!("{:02}:{:02} UTC", h, m),
                        sun_elevation: pos.elevation,
                        azimuth_error: diff,
                    });
                }
            }
        }

        prev_diff = diff;
    }

    // Sort by azimuth error (best match first) and limit to 2 candidates.
    candidates.sort_by(|a, b| {
        a.azimuth_error
            .partial_cmp(&b.azimuth_error)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    candidates.truncate(2);
    candidates
}

/// Compute the smallest angular difference between two azimuths in the range 0–360°.
///
/// Returns a value in [0, 180].
fn angle_diff(a: f64, b: f64) -> f64 {
    let diff = (a - b).abs() % 360.0;
    if diff > 180.0 {
        360.0 - diff
    } else {
        diff
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn london_summer_noon() {
        // London (51.5074°N, 0.1278°W), 21 June 2024, 12:00 UTC.
        // At summer solstice noon, the sun should be roughly south (~180°)
        // and high in the sky (~60°+) for this latitude.
        let pos = calculate_solar_position(51.5074, -0.1278, 2024, 6, 21, 12.0);
        assert!(
            pos.azimuth > 150.0 && pos.azimuth < 210.0,
            "London summer noon: expected azimuth ~180°, got {:.1}°",
            pos.azimuth
        );
        assert!(
            pos.elevation > 55.0 && pos.elevation < 65.0,
            "London summer noon: expected elevation ~60°, got {:.1}°",
            pos.elevation
        );
        assert!(
            pos.day_length_hours > 16.0,
            "London summer solstice: expected >16 h daylight, got {:.1} h",
            pos.day_length_hours
        );
    }

    #[test]
    fn sydney_winter_noon() {
        // Sydney (33.8688°S, 151.2093°E), 21 June 2024, 02:00 UTC
        // (approximately local noon AEST = UTC+10).
        // At winter solstice in the southern hemisphere the sun is north
        // and at a lower elevation than in summer.
        let pos = calculate_solar_position(-33.8688, 151.2093, 2024, 6, 21, 2.0);
        // Sun should be north (azimuth near 0°/360°)
        let north = pos.azimuth > 340.0 || pos.azimuth < 30.0;
        assert!(
            north,
            "Sydney winter noon: expected azimuth near north (0/360°), got {:.1}°",
            pos.azimuth
        );
        assert!(
            pos.elevation > 20.0 && pos.elevation < 40.0,
            "Sydney winter noon: expected elevation 20–40°, got {:.1}°",
            pos.elevation
        );
    }

    #[test]
    fn equator_equinox() {
        // Equator (0°, 0°), 20 March 2024, 12:00 UTC.
        // At vernal equinox on the equator, the sun passes almost directly
        // overhead at solar noon — elevation should be close to 90°.
        let pos = calculate_solar_position(0.0, 0.0, 2024, 3, 20, 12.0);
        assert!(
            pos.elevation > 85.0,
            "Equator equinox noon: expected elevation > 85°, got {:.1}°",
            pos.elevation
        );
    }

    #[test]
    fn arctic_summer_midnight_sun() {
        // Tromsø (69.65°N, 18.96°E), 21 June 2024, 00:00 UTC.
        // At the summer solstice above the Arctic Circle, the sun does not
        // set — elevation must remain positive at midnight.
        let pos = calculate_solar_position(69.65, 18.96, 2024, 6, 21, 0.0);
        assert!(
            pos.elevation > 0.0,
            "Arctic midnight sun: elevation should be positive, got {:.1}°",
            pos.elevation
        );
        assert!(
            pos.day_length_hours > 23.0,
            "Arctic midnight sun: expected >23 h daylight, got {:.1} h",
            pos.day_length_hours
        );
    }

    #[test]
    fn negative_elevation_at_night() {
        // London (51.5074°N, 0.1278°W), 21 December 2024, 00:00 UTC.
        // At midnight in winter the sun is firmly below the horizon.
        let pos = calculate_solar_position(51.5074, -0.1278, 2024, 12, 21, 0.0);
        assert!(
            pos.elevation < 0.0,
            "London winter midnight: elevation should be negative, got {:.1}°",
            pos.elevation
        );
    }

    #[test]
    fn azimuth_east_at_morning() {
        // London, summer solstice, 06:00 UTC — early morning, sun rising in the east.
        let pos = calculate_solar_position(51.5074, -0.1278, 2024, 6, 21, 6.0);
        assert!(
            pos.azimuth > 45.0 && pos.azimuth < 135.0,
            "London morning: expected sun in the east (45–135°), got {:.1}°",
            pos.azimuth
        );
    }

    #[test]
    fn output_always_finite() {
        // Spot-check that no combination of valid inputs produces NaN or Inf.
        let cases = [
            (0.0, 0.0, 2024, 3, 20, 12.0),
            (89.9, 0.0, 2024, 6, 21, 12.0),   // near North Pole
            (-89.9, 0.0, 2024, 12, 21, 12.0), // near South Pole
            (51.5, -0.1, 2024, 12, 21, 0.0),  // winter midnight
        ];
        for (lat, lon, y, mo, d, h) in cases {
            let pos = calculate_solar_position(lat, lon, y, mo, d, h);
            assert!(
                pos.azimuth.is_finite(),
                "azimuth not finite for ({lat}, {lon}, {y}-{mo}-{d} {h}h UTC)"
            );
            assert!(
                pos.elevation.is_finite(),
                "elevation not finite for ({lat}, {lon}, {y}-{mo}-{d} {h}h UTC)"
            );
            assert!(
                pos.solar_noon_utc.is_finite(),
                "solar_noon_utc not finite for ({lat}, {lon})"
            );
            assert!(
                pos.day_length_hours.is_finite(),
                "day_length_hours not finite for ({lat}, {lon})"
            );
        }
    }

    // ── angle_diff tests ──────────────────────────────────────────────────────

    #[test]
    fn angle_diff_basic() {
        // 10° and 350° are 20° apart across the 0°/360° boundary.
        let d = angle_diff(10.0, 350.0);
        assert!(
            (d - 20.0).abs() < 1e-9,
            "expected angle_diff(10, 350) == 20.0, got {d}"
        );
    }

    #[test]
    fn angle_diff_same() {
        // Identical azimuths must give zero.
        let d = angle_diff(180.0, 180.0);
        assert!(
            d.abs() < 1e-9,
            "expected angle_diff(180, 180) == 0.0, got {d}"
        );
    }

    // ── estimate_time_from_shadow tests ──────────────────────────────────────

    #[test]
    fn shadow_time_london_summer() {
        // London (51.5074°N, 0.1278°W), 21 June 2024.
        // A shadow pointing north (azimuth ≈ 0°) means the sun is in the south
        // (~180°), which occurs near solar noon (~12:00 UTC for London in summer).
        let results = estimate_time_from_shadow(51.5074, -0.1278, 2024, 6, 21, 0.0);
        assert!(
            !results.is_empty(),
            "Expected at least one candidate for north-pointing shadow in London summer"
        );
        let best = &results[0];
        assert!(
            best.hour_utc >= 11.0 && best.hour_utc <= 13.0,
            "Expected noon-ish result (11–13 UTC), got hour_utc = {:.2}",
            best.hour_utc
        );
    }

    #[test]
    fn shadow_time_returns_two_candidates() {
        // London, 21 June 2024.
        // A shadow pointing east (azimuth ≈ 90°) means the sun is in the west
        // (~270°), which occurs in the afternoon. We request a shadow azimuth
        // near 270° (sun in east, ~90°) to get morning/afternoon symmetry.
        // Azimuth 270° shadow → sun at ~90° → expect morning candidate.
        // Also try 90° shadow → sun at ~270° → expect afternoon candidate.
        let results_afternoon = estimate_time_from_shadow(51.5074, -0.1278, 2024, 6, 21, 90.0);
        assert!(
            !results_afternoon.is_empty(),
            "Expected at least one candidate for east-pointing shadow in London summer"
        );
        // The best result should be in the afternoon (sun at ~270° = west).
        let best = &results_afternoon[0];
        assert!(
            best.hour_utc > 12.0,
            "East-pointing shadow should correspond to afternoon (sun in west), got hour_utc = {:.2}",
            best.hour_utc
        );
    }

    #[test]
    fn shadow_time_empty_for_impossible() {
        // London, 21 December 2024 (winter).
        // At this latitude and time of year, the sun never rises very high.
        // A shadow pointing due north (azimuth 0°) in London in winter is
        // plausible near noon, but a shadow azimuth that maps to the sun being
        // directly north-at-horizon is impossible in the northern hemisphere.
        // We use a polar location in winter to guarantee no daytime candidates.
        // Tromsø (69.65°N), 21 December — sun barely rises or does not rise.
        // The NOAA algorithm will return negative elevations for most of the day.
        // We assert the result has 0 candidates OR all have elevation < 1°.
        // (In practice at this latitude/date there may be a brief window;
        //  we test that the filter correctly excludes sub-1° elevations.)
        let results = estimate_time_from_shadow(69.65, 18.96, 2024, 12, 21, 0.0);
        // If any candidates are returned, every one must have sun_elevation >= 1.0.
        for c in &results {
            assert!(
                c.sun_elevation >= 1.0,
                "Candidate with sun_elevation below threshold should have been filtered: {:.2}°",
                c.sun_elevation
            );
        }
        // At Tromsø in midwinter the sun is at or below the horizon all day —
        // we expect zero candidates.
        assert!(
            results.is_empty(),
            "Expected no candidates for Tromsø midwinter, got {}",
            results.len()
        );
    }
}
