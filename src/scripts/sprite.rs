//! `dokidoki/scripts/sprite.lua`: what an actor looks like.
//!
//! Data only for now. The Lua `draw` pushes a matrix, translates by the
//! transform, rotates by `atan2` of the facing, scales, optionally sets a
//! colour, and draws the image; all of that is renderer work and lands in phase
//! 4. What matters before then is that the fields exist, since gameplay writes
//! to them -- `scripts/ship.lua` sets `self.sprite.color` on spawn and fades its
//! alpha out as a factory dies.

use crate::resources::SpriteId;

/// An RGBA colour. Lua stores `{r, g, b}` or `{r, g, b, a}`, defaulting alpha to
/// 1 at draw time; the alpha is explicit here.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl Color {
    pub const WHITE: Color = Color::rgb(1.0, 1.0, 1.0);

    pub const fn rgb(r: f64, g: f64, b: f64) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    pub const fn rgba(r: f64, g: f64, b: f64, a: f64) -> Self {
        Self { r, g, b, a }
    }

    pub fn with_alpha(self, a: f64) -> Self {
        Self { a, ..self }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Sprite {
    /// Which image to draw. `None` is the Lua's dummy 1x1 quad, used by actors
    /// that have a sprite slot but no art.
    pub image: Option<SpriteId>,
    /// `false` in Lua, meaning "don't touch the current colour".
    pub color: Option<Color>,
}

impl Sprite {
    pub fn blank() -> Self {
        Self::default()
    }

    pub fn new(image: SpriteId) -> Self {
        Self {
            image: Some(image),
            color: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_sprite_has_no_image_or_colour() {
        let s = Sprite::blank();
        assert_eq!(s.image, None);
        assert_eq!(s.color, None);
    }

    #[test]
    fn alpha_defaults_to_opaque() {
        assert_eq!(Color::rgb(1.0, 0.0, 0.0).a, 1.0);
        assert_eq!(Color::WHITE.with_alpha(0.25).a, 0.25);
    }
}
