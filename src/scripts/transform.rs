//! `dokidoki/scripts/transform.lua`: where an actor is and which way it faces.
//!
//! Pure data. The Lua script body is four defaulted assignments and nothing
//! else, so there is no behaviour to port.

use crate::v2::V2;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    pub pos: V2,
    pub facing: V2,
    pub scale_x: f64,
    pub scale_y: f64,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            pos: V2::ZERO,
            facing: V2::I,
            scale_x: 1.0,
            scale_y: 1.0,
        }
    }
}

impl Transform {
    pub fn at(pos: V2) -> Self {
        Self {
            pos,
            ..Default::default()
        }
    }

    pub fn facing(pos: V2, facing: V2) -> Self {
        Self {
            pos,
            facing,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::v2;

    #[test]
    fn defaults_match_the_lua() {
        let t = Transform::default();
        assert_eq!(t.pos, V2::ZERO);
        assert_eq!(t.facing, V2::I);
        assert_eq!((t.scale_x, t.scale_y), (1.0, 1.0));
    }

    #[test]
    fn at_sets_position_only() {
        let t = Transform::at(v2(3.0, 4.0));
        assert_eq!(t.pos, v2(3.0, 4.0));
        assert_eq!(t.facing, V2::I);
    }
}
