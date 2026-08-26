//! Checks `src/collision.rs` against the Lua implementation that recorded the
//! golden trace, case for case and bit for bit.
//!
//! Unit tests can only say the port is plausible. This says it agrees with the
//! thing it has to agree with. `traces/collision.txt` is frozen: the Lua that
//! emitted it is no longer in this repo, so the vectors are evidence, not
//! something to regenerate.
//!
//! The vectors have been mutation-checked: normalising the separating axis in
//! `separate_by_axes` makes this test fail on case 8. Worth redoing if the
//! vector file is ever regenerated with a different shape mix, since a test this
//! uniform is easy to render vacuous without noticing.

use pax_britannica::collision::{Body, Polygon, collide};
use pax_britannica::v2::{V2, v2};

struct Case {
    poly1: Polygon,
    pos1: V2,
    facing1: V2,
    poly2: Polygon,
    pos2: V2,
    facing2: V2,
    expected: Option<V2>,
}

/// Pulls fields off a whitespace-separated line in the order the generator
/// wrote them.
struct Fields<'a> {
    tokens: std::str::SplitWhitespace<'a>,
}

impl Fields<'_> {
    fn f64(&mut self) -> f64 {
        let token = self.tokens.next().expect("truncated case");
        token
            .parse()
            .unwrap_or_else(|_| panic!("not a number: {token}"))
    }

    fn usize(&mut self) -> usize {
        let token = self.tokens.next().expect("truncated case");
        token
            .parse()
            .unwrap_or_else(|_| panic!("not an integer: {token}"))
    }

    fn v2(&mut self) -> V2 {
        v2(self.f64(), self.f64())
    }

    /// A vertex count followed by that many pairs, then a position and facing.
    fn body(&mut self) -> (Polygon, V2, V2) {
        let vertex_count = self.usize();
        let vertices: Vec<V2> = (0..vertex_count).map(|_| self.v2()).collect();
        (Polygon::new(vertices), self.v2(), self.v2())
    }
}

fn parse(line: &str) -> Case {
    let mut fields = Fields {
        tokens: line.split_whitespace(),
    };

    let (poly1, pos1, facing1) = fields.body();
    let (poly2, pos2, facing2) = fields.body();

    let expected = match fields.usize() {
        0 => None,
        1 => Some(fields.v2()),
        other => panic!("expected a 0/1 hit flag, got {other}"),
    };

    Case {
        poly1,
        pos1,
        facing1,
        poly2,
        pos2,
        facing2,
        expected,
    }
}

#[test]
fn matches_the_lua_implementation_exactly() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/traces/collision.txt");
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("{path}: {e}"));

    let mut checked = 0;
    let mut hits = 0;

    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let case = parse(line);

        let body1 = Body {
            pos: case.pos1,
            facing: case.facing1,
            poly: &case.poly1,
        };
        let body2 = Body {
            pos: case.pos2,
            facing: case.facing2,
            poly: &case.poly2,
        };

        let line_number = index + 1;
        match (collide(&body1, &body2), case.expected) {
            (None, None) => {}
            (Some(got), Some(want)) => {
                // bit-exact, not approximate: a correction that is merely close
                // would drift the simulation apart over 12,000 frames
                assert_eq!(
                    (got.x.to_bits(), got.y.to_bits()),
                    (want.x.to_bits(), want.y.to_bits()),
                    "line {line_number}: correction {got:?} != {want:?}"
                );
                hits += 1;
            }
            (got, want) => panic!("line {line_number}: detection disagrees, {got:?} != {want:?}"),
        }
        checked += 1;
    }

    assert!(checked > 1000, "only {checked} cases in the vector file");
    // guards against a file of pure near-misses, which would exercise nothing
    // but the broad phase and still pass
    assert!(
        hits > checked / 5,
        "only {hits} of {checked} cases collided; the vectors are not exercising the fine phase"
    );
    println!("{checked} cases, {hits} collisions, all bit-exact");
}
