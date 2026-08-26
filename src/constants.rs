//! `components/constants.lua`, plus the field scale this port adds.

use crate::v2::{V2, v2};

/// The field the game was written for, in game units, and the size the sprite
/// art was drawn against.
pub const BASE_WIDTH: f64 = 1024.0;
pub const BASE_HEIGHT: f64 = 768.0;

/// How much wider and taller than [`BASE_WIDTH`] by [`BASE_HEIGHT`] the field
/// is. The art is not scaled with it and the camera still fits the whole field
/// on screen, so raising this leaves every ship its drawn size and gives it more
/// room to fly in.
///
/// Lengths that belong to the field scale with it: the ring the factories start
/// on, and the circle a factory's constant turn carries it around. Lengths that
/// belong to the art do not -- the menu's spacing, the offsets of the title and
/// the credits, weapon ranges, ship speeds.
pub const PLAY_SCALE: f64 = 1.5;

/// The play area, in game units. The window is letterboxed to 4:3 around it.
pub const SCREEN_LEFT: f64 = 0.0;
pub const SCREEN_RIGHT: f64 = BASE_WIDTH * PLAY_SCALE;
pub const SCREEN_BOTTOM: f64 = 0.0;
pub const SCREEN_TOP: f64 = BASE_HEIGHT * PLAY_SCALE;

/// The middle of the play area.
pub const CENTER: V2 = v2(
    (SCREEN_LEFT + SCREEN_RIGHT) / 2.0,
    (SCREEN_BOTTOM + SCREEN_TOP) / 2.0,
);

/// Health fractions at which a factory shows light and heavy damage art.
pub const LOW_HEALTH_THRESHOLD: f64 = 0.3;
pub const HIGH_HEALTH_THRESHOLD: f64 = 0.6;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_field_keeps_the_originals_four_by_three() {
        let aspect = SCREEN_RIGHT / SCREEN_TOP;
        assert!((aspect - BASE_WIDTH / BASE_HEIGHT).abs() < 1e-12);
        assert_eq!(CENTER, v2(SCREEN_RIGHT / 2.0, SCREEN_TOP / 2.0));
    }
}
