//! Convex polygon collision, ported from `dokidoki-support/collision.c` (and
//! its Lua twin `test/stubs/collision/native.lua`, which is what generated the
//! golden trace) plus the wrapper in `dokidoki/collision.lua`.
//!
//! This is a transliteration, not a reimplementation, because the separating-
//! axis routine below has properties a from-scratch SAT would not reproduce.
//!
//! Axes are **not** normalised, so a correction is scaled by its edge's length
//! and "smallest correction" does not mean "shallowest penetration". This one is
//! load-bearing: normalising the axis makes the differential test in
//! `tests/collision_differential.rs` fail on its eighth case.
//!
//! Two further quirks are faithfully copied but, as far as the test vectors can
//! tell, unreachable for the shapes this game uses. Both are kept because
//! matching the original is worth more than tidiness, and neither costs
//! anything:
//!
//! - `halfwidth_along_axis` floors its running maximum at `0` instead of
//!   starting at `-inf`. Every polygon here is built around its own centroid and
//!   so contains the origin, which makes the largest projection onto any axis
//!   non-negative anyway.
//! - a zero-length correction is treated as *separated* rather than as a
//!   zero-depth contact. Reaching it needs a degenerate axis, i.e. a
//!   zero-length polygon edge.

use crate::v2::{V2, v2};

/// A convex polygon in local space, with its vertices wound counterclockwise.
#[derive(Clone, Debug)]
pub struct Polygon {
    vertices: Vec<V2>,
    /// Distance from the origin to the furthest vertex, for the broad phase.
    bounding_radius: f64,
}

impl Polygon {
    /// `collision.make_polygon(vertices)`.
    ///
    /// Vertices wound clockwise are reversed, so the outward normals computed in
    /// [`separate_by_axes`] point outward.
    pub fn new(vertices: Vec<V2>) -> Self {
        assert!(
            vertices.len() >= 3,
            "a polygon needs at least 3 vertices, got {}",
            vertices.len()
        );

        let mut vertices = vertices;
        if (vertices[1] - vertices[0]).cross(vertices[2] - vertices[1]) < 0.0 {
            vertices.reverse();
        }

        let bounding_radius = vertices
            .iter()
            .map(|v| v.mag())
            .fold(0.0f64, |acc, mag| if acc < mag { mag } else { acc });

        Self {
            vertices,
            bounding_radius,
        }
    }

    /// `collision.make_rectangle(w, h)`: a rectangle centred on the origin.
    pub fn rectangle(width: f64, height: f64) -> Self {
        let (hw, hh) = (width / 2.0, height / 2.0);
        Self::new(vec![v2(-hw, -hh), v2(hw, -hh), v2(hw, hh), v2(-hw, hh)])
    }

    pub fn vertices(&self) -> &[V2] {
        &self.vertices
    }

    pub fn bounding_radius(&self) -> f64 {
        self.bounding_radius
    }
}

/// A polygon placed in the world.
#[derive(Clone, Copy, Debug)]
pub struct Body<'a> {
    pub pos: V2,
    pub facing: V2,
    pub poly: &'a Polygon,
}

/// The largest projection of `poly`'s vertices onto `axis`, floored at zero.
///
/// The floor is inherited from the C and is not an accident to be tidied away:
/// it makes the result a halfwidth measured from the origin outward, never
/// negative, even when every vertex lies behind the axis.
fn halfwidth_along_axis(axis: V2, poly: &Polygon) -> f64 {
    let mut halfwidth = 0.0;
    for vertex in &poly.vertices {
        let candidate = vertex.dot(axis);
        if candidate > halfwidth {
            halfwidth = candidate;
        }
    }
    halfwidth
}

/// Tests one candidate separating axis.
///
/// Returns `false` as soon as the axis separates the bodies, which means "no
/// collision". Otherwise narrows `correction` to the smallest push-out seen so
/// far and returns `true`.
fn separate_by_axis(
    axis: V2,
    halfwidth1: f64,
    body1: &Body,
    body2: &Body,
    correction: &mut V2,
) -> bool {
    let overlap = body1.pos.dot(axis)
        + halfwidth1
        + halfwidth_along_axis(-axis.rotate_from(body2.facing), body2.poly)
        - body2.pos.dot(axis);

    if overlap <= 0.0 {
        return false;
    }

    let candidate = axis * (overlap / axis.dot(axis));
    // a degenerate axis yields no push-out direction; the C treats that as
    // separated rather than as a zero-depth contact
    if candidate.x == 0.0 && candidate.y == 0.0 {
        return false;
    }

    if candidate.sqrmag() < correction.sqrmag() {
        *correction = candidate;
    }
    true
}

/// Tests every edge normal of `body1`'s polygon as a separating axis.
fn separate_by_axes(body1: &Body, body2: &Body, correction: &mut V2) -> bool {
    let vertices = &body1.poly.vertices;
    let mut previous = vertices.len() - 1;

    for current in 0..vertices.len() {
        // the edge normal, as a quarter turn clockwise of the edge itself
        let edge = vertices[current] - vertices[previous];
        let axis = edge.rotate_to(v2(0.0, -1.0));
        // measured in local space, before the axis is taken into world space;
        // rotation preserves dot products, so the two agree
        let halfwidth1 = vertices[current].dot(axis);
        let axis = axis.rotate_to(body1.facing);

        if !separate_by_axis(axis, halfwidth1, body1, body2, correction) {
            return false;
        }
        previous = current;
    }
    true
}

/// `collision.collide(body1, body2)`.
///
/// Returns the vector that would pull `body1` clear of `body2`, or `None` if
/// they are not touching.
pub fn collide(body1: &Body, body2: &Body) -> Option<V2> {
    let offset = body2.pos - body1.pos;

    // broad phase
    let bounding_distance = body1.poly.bounding_radius + body2.poly.bounding_radius;
    if bounding_distance * bounding_distance < offset.sqrmag() {
        return None;
    }

    let mut correction = v2(f64::INFINITY, f64::INFINITY);

    // short-circuits exactly as the C's `||` does: if the first call finds a
    // separating axis the second never runs
    if !separate_by_axes(body1, body2, &mut correction)
        || !separate_by_axes(body2, body1, &mut correction)
    {
        return None;
    }

    // point the correction away from body2
    if correction.dot(offset) > 0.0 {
        correction = -correction;
    }

    Some(correction)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body<'a>(poly: &'a Polygon, x: f64, y: f64, facing: V2) -> Body<'a> {
        Body {
            pos: v2(x, y),
            facing,
            poly,
        }
    }

    #[test]
    fn clockwise_winding_is_reversed() {
        let counterclockwise = Polygon::new(vec![v2(-1.0, -1.0), v2(1.0, -1.0), v2(1.0, 1.0)]);
        let clockwise = Polygon::new(vec![v2(1.0, 1.0), v2(1.0, -1.0), v2(-1.0, -1.0)]);
        assert_eq!(counterclockwise.vertices(), clockwise.vertices());
    }

    #[test]
    fn bounding_radius_is_the_furthest_vertex() {
        assert_eq!(Polygon::rectangle(6.0, 8.0).bounding_radius(), 5.0);
    }

    #[test]
    fn overlapping_boxes_collide() {
        let poly = Polygon::rectangle(10.0, 10.0);
        let a = body(&poly, 0.0, 0.0, V2::I);
        let b = body(&poly, 5.0, 0.0, V2::I);
        assert!(collide(&a, &b).is_some());
    }

    #[test]
    fn distant_boxes_do_not_collide() {
        let poly = Polygon::rectangle(10.0, 10.0);
        let a = body(&poly, 0.0, 0.0, V2::I);
        let b = body(&poly, 500.0, 0.0, V2::I);
        assert!(collide(&a, &b).is_none());
    }

    #[test]
    fn correction_pushes_body1_clear() {
        let poly = Polygon::rectangle(10.0, 10.0);
        let a = body(&poly, 0.0, 0.0, V2::I);
        let b = body(&poly, 8.0, 0.0, V2::I);

        let correction = collide(&a, &b).expect("boxes overlap");
        // body2 is to the right, so body1 is pushed left
        assert!(correction.x < 0.0, "{correction:?}");

        // applying it separates them
        let moved = body(&poly, correction.x, correction.y, V2::I);
        assert!(collide(&moved, &b).is_none());
    }

    #[test]
    fn just_touching_counts_as_separated() {
        // overlap of exactly 0 is `overlap <= 0` in the C, i.e. no collision
        let poly = Polygon::rectangle(10.0, 10.0);
        let a = body(&poly, 0.0, 0.0, V2::I);
        let b = body(&poly, 10.0, 0.0, V2::I);
        assert!(collide(&a, &b).is_none());
    }

    #[test]
    fn rotation_matters() {
        // a long thin box that only reaches its neighbour once turned
        let poly = Polygon::rectangle(40.0, 2.0);
        let other = Polygon::rectangle(4.0, 4.0);
        let target = body(&other, 0.0, 15.0, V2::I);

        let flat = body(&poly, 0.0, 0.0, V2::I);
        assert!(collide(&flat, &target).is_none());

        let turned = body(&poly, 0.0, 0.0, V2::unit(std::f64::consts::FRAC_PI_2));
        assert!(collide(&turned, &target).is_some());
    }

    #[test]
    fn collision_is_symmetric_in_detection() {
        let a_poly = Polygon::rectangle(54.0, 36.0);
        let b_poly = Polygon::rectangle(9.0, 6.0);
        let a = body(&a_poly, 100.0, 100.0, V2::unit(0.3));
        let b = body(&b_poly, 110.0, 105.0, V2::unit(-1.1));
        assert_eq!(collide(&a, &b).is_some(), collide(&b, &a).is_some());
    }
}
