//! Immutable 2D vectors, ported from `dokidoki/v2.lua`.
//!
//! `f64` throughout, deliberately: the Lua original computes everything in
//! doubles and the port is checked against it bit for bit. Narrowing any of this
//! to `f32` would be an invisible behaviour change.

use crate::rng::LuaRng;
use std::ops::{Add, Div, Mul, Neg, Sub};

/// A 2D vector. `Copy`, so passing it around never borrows.
///
/// `Default` is the zero vector, matching `v2.zero`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct V2 {
    pub x: f64,
    pub y: f64,
}

/// `v2(x, y)` in Lua.
pub const fn v2(x: f64, y: f64) -> V2 {
    V2 { x, y }
}

impl V2 {
    pub const ZERO: V2 = v2(0.0, 0.0);
    pub const I: V2 = v2(1.0, 0.0);
    pub const J: V2 = v2(0.0, 1.0);

    /// The unit vector at `angle` radians.
    pub fn unit(angle: f64) -> V2 {
        v2(angle.cos(), angle.sin())
    }

    /// A uniformly distributed point in the unit disc.
    ///
    /// The two draws happen angle-first. Lua does not specify argument
    /// evaluation order, but PUC-Rio Lua evaluates left to right and that is
    /// what produced the golden trace, so the order here is load-bearing rather
    /// than arbitrary.
    pub fn random(rng: &mut LuaRng) -> V2 {
        let angle = rng.next_f64();
        let radius = rng.next_f64();
        V2::unit(angle * std::f64::consts::PI * 2.0) * radius.sqrt()
    }

    pub fn dot(self, other: V2) -> f64 {
        self.x * other.x + self.y * other.y
    }

    pub fn cross(self, other: V2) -> f64 {
        self.x * other.y - self.y * other.x
    }

    /// Squared magnitude. Prefer this to `mag()` when comparing distances.
    pub fn sqrmag(self) -> f64 {
        self.dot(self)
    }

    pub fn mag(self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    pub fn angle(self) -> f64 {
        self.y.atan2(self.x)
    }

    /// The unit vector in the same direction.
    ///
    /// Follows Lua in not special-casing the zero vector: the result is NaN,
    /// and callers that care check `sqrmag() == 0.0` first, as
    /// `components/particles.lua` does.
    pub fn norm(self) -> V2 {
        self / self.mag()
    }

    pub fn project(self, onto: V2) -> V2 {
        onto * (self.dot(onto) / onto.sqrmag())
    }

    pub fn rotate(self, angle: f64) -> V2 {
        let (sin_a, cos_a) = (angle.sin(), angle.cos());
        v2(
            self.x * cos_a - self.y * sin_a,
            self.y * cos_a + self.x * sin_a,
        )
    }

    /// A quarter turn counterclockwise.
    pub fn rotate90(self) -> V2 {
        v2(-self.y, self.x)
    }

    /// Rotates from local space into the frame whose x axis is `facing`.
    pub fn rotate_to(self, facing: V2) -> V2 {
        v2(
            facing.x * self.x - facing.y * self.y,
            facing.y * self.x + facing.x * self.y,
        )
    }

    /// The inverse of [`rotate_to`](V2::rotate_to).
    pub fn rotate_from(self, facing: V2) -> V2 {
        v2(
            facing.x * self.x + facing.y * self.y,
            -facing.y * self.x + facing.x * self.y,
        )
    }

    pub fn coords(self) -> (f64, f64) {
        (self.x, self.y)
    }
}

impl Add for V2 {
    type Output = V2;
    fn add(self, other: V2) -> V2 {
        v2(self.x + other.x, self.y + other.y)
    }
}

impl Sub for V2 {
    type Output = V2;
    fn sub(self, other: V2) -> V2 {
        v2(self.x - other.x, self.y - other.y)
    }
}

impl Neg for V2 {
    type Output = V2;
    fn neg(self) -> V2 {
        v2(-self.x, -self.y)
    }
}

impl Mul<f64> for V2 {
    type Output = V2;
    /// Scalar multiply. Lua's `v2.mul` computes `s * v.x`, and float
    /// multiplication is commutative in IEEE 754 (including for signed zero and
    /// NaN payloads), so the operand order here is not significant.
    fn mul(self, scalar: f64) -> V2 {
        v2(scalar * self.x, scalar * self.y)
    }
}

impl Mul<V2> for f64 {
    type Output = V2;
    fn mul(self, v: V2) -> V2 {
        v * self
    }
}

impl Div<f64> for V2 {
    type Output = V2;
    fn div(self, scalar: f64) -> V2 {
        v2(self.x / scalar, self.y / scalar)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-12
    }

    #[test]
    fn arithmetic() {
        let a = v2(1.0, 2.0);
        let b = v2(3.0, 5.0);
        assert_eq!(a + b, v2(4.0, 7.0));
        assert_eq!(a - b, v2(-2.0, -3.0));
        assert_eq!(-a, v2(-1.0, -2.0));
        assert_eq!(a * 3.0, v2(3.0, 6.0));
        assert_eq!(3.0 * a, v2(3.0, 6.0));
        assert_eq!(b / 2.0, v2(1.5, 2.5));
        assert_eq!(a.dot(b), 13.0);
        assert_eq!(a.cross(b), -1.0);
        assert_eq!(a.sqrmag(), 5.0);
    }

    #[test]
    fn magnitude_and_angle() {
        assert_eq!(v2(3.0, 4.0).mag(), 5.0);
        assert!(close(v2(0.0, 1.0).angle(), std::f64::consts::FRAC_PI_2));
        let n = v2(3.0, 4.0).norm();
        assert!(close(n.x, 0.6) && close(n.y, 0.8));
    }

    #[test]
    fn zero_vector_norm_is_nan() {
        // matching Lua, which divides by zero rather than guarding
        assert!(V2::ZERO.norm().x.is_nan());
    }

    #[test]
    fn rotate90_is_counterclockwise() {
        assert_eq!(V2::I.rotate90(), V2::J);
        assert_eq!(V2::J.rotate90(), -V2::I);
    }

    #[test]
    fn rotate_from_inverts_rotate_to() {
        let facing = V2::unit(0.9);
        let v = v2(2.0, -3.0);
        let round_tripped = v.rotate_to(facing).rotate_from(facing);
        assert!(close(round_tripped.x, v.x) && close(round_tripped.y, v.y));
    }

    #[test]
    fn rotate_to_identity_facing_is_identity() {
        let v = v2(2.0, -3.0);
        assert_eq!(v.rotate_to(V2::I), v);
    }

    #[test]
    fn project_onto_axis() {
        assert_eq!(v2(3.0, 4.0).project(V2::I), v2(3.0, 0.0));
    }

    #[test]
    fn random_is_inside_the_unit_disc() {
        let mut rng = LuaRng::new(1, 0);
        for _ in 0..10_000 {
            assert!(V2::random(&mut rng).mag() <= 1.0);
        }
    }

    #[test]
    fn random_draws_angle_before_radius() {
        // verified against the Lua original; see the note on V2::random
        let mut rng = LuaRng::new(1, 0);
        let got = V2::random(&mut rng);

        let mut rng = LuaRng::new(1, 0);
        let angle = rng.next_f64();
        let radius = rng.next_f64();
        let want = V2::unit(angle * std::f64::consts::PI * 2.0) * radius.sqrt();

        assert_eq!(got, want);
    }
}
