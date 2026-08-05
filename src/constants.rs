//! `components/constants.lua`.

/// The play area, in game units. The window is letterboxed to 4:3 around it.
pub const SCREEN_LEFT: f64 = 0.0;
pub const SCREEN_RIGHT: f64 = 1024.0;
pub const SCREEN_BOTTOM: f64 = 0.0;
pub const SCREEN_TOP: f64 = 768.0;

/// Health fractions at which a factory shows light and heavy damage art.
pub const LOW_HEALTH_THRESHOLD: f64 = 0.3;
pub const HIGH_HEALTH_THRESHOLD: f64 = 0.6;
