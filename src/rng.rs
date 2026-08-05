//! Lua 5.4's `math.random`, reproduced bit for bit.
//!
//! The port is validated against the Lua original by replaying a recorded input
//! schedule and comparing per-frame state hashes (see `test/trace.lua`). That
//! only works if both sides draw the same numbers in the same order, so this is
//! a transliteration of `lmathlib.c` rather than a merely equivalent generator:
//! xoshiro256\*\*, Lua's seeding ritual, Lua's 53-bit float conversion and Lua's
//! rejection sampling for ranges.
//!
//! Do not "improve" any of it.

/// The xoshiro256\*\* generator behind Lua 5.4's `math.random`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LuaRng {
    state: [u64; 4],
}

impl LuaRng {
    /// Equivalent to `math.randomseed(n1, n2)`.
    ///
    /// `math.randomseed(n)` passes `n2 = 0`, which is what the game's fixed-seed
    /// test runs use.
    pub fn new(n1: i64, n2: i64) -> Self {
        let mut rng = Self {
            // state[1] is 0xff rather than 0 so that the state can't be all
            // zeroes, which xoshiro cannot escape from
            state: [n1 as u64, 0xff, n2 as u64, 0],
        };
        // discarded to "spread" the seed, per lmathlib.c
        for _ in 0..16 {
            rng.next_u64();
        }
        rng
    }

    /// The raw 64-bit output. `Rand64 nextrand (Rand64 *state)` in lmathlib.c.
    pub fn next_u64(&mut self) -> u64 {
        let [state0, state1, mut state2, mut state3] = self.state;
        state2 ^= state0;
        state3 ^= state1;

        let result = state1.wrapping_mul(5).rotate_left(7).wrapping_mul(9);

        self.state[0] = state0 ^ state3;
        self.state[1] = state1 ^ state2;
        self.state[2] = state2 ^ (state1 << 17);
        self.state[3] = state3.rotate_left(45);

        result
    }

    /// Equivalent to `math.random()`: a float in `[0, 1)`.
    ///
    /// Lua keeps the top 53 bits and scales by 2^-53, so the result has exactly
    /// as many significant bits as an f64 mantissa.
    pub fn next_f64(&mut self) -> f64 {
        const FIGS: u32 = 53;
        const SCALE: f64 = 1.0 / (1u64 << FIGS) as f64;
        (self.next_u64() >> (64 - FIGS)) as f64 * SCALE
    }

    /// Equivalent to `math.random(m, n)`: an integer in `[m, n]`.
    ///
    /// # Panics
    /// If `m > n`, which Lua reports as "interval is empty".
    pub fn next_range(&mut self, m: i64, n: i64) -> i64 {
        assert!(m <= n, "interval is empty");
        // the width of the interval, which can legitimately overflow i64 (the
        // full-range case is exactly u64::MAX) and so is computed unsigned
        let width = (n as u64).wrapping_sub(m as u64);
        let ran = self.next_u64();
        (m as u64).wrapping_add(self.project(ran, width)) as i64
    }

    /// `lua_Unsigned project (...)` in lmathlib.c: reduce `ran` into `[0, n]`
    /// without the modulo bias, by masking to the next power of two and
    /// redrawing until the value lands in range.
    fn project(&mut self, mut ran: u64, n: u64) -> u64 {
        if n & n.wrapping_add(1) == 0 {
            // n + 1 is a power of two, so masking alone is unbiased
            return ran & n;
        }

        // smallest 2^b - 1 that is >= n
        let mut lim = n;
        lim |= lim >> 1;
        lim |= lim >> 2;
        lim |= lim >> 4;
        lim |= lim >> 8;
        lim |= lim >> 16;
        lim |= lim >> 32;

        loop {
            ran &= lim;
            if ran <= n {
                return ran;
            }
            ran = self.next_u64();
        }
    }
}

#[cfg(test)]
mod tests {
    // The expected floats are Lua's %.17g output copied verbatim, which is the
    // shortest form guaranteed to round-trip a double. Clippy would rather they
    // were trimmed to the digits that survive parsing; leaving them exactly as
    // Lua printed them is the point, so that a vector can be diffed straight
    // against a fresh run of test/rng_vectors.lua.
    #![allow(clippy::excessive_precision)]

    use super::*;

    // Every expected value below was produced by the lua5.4 binary itself, not
    // by reasoning about the algorithm. Regenerate with test/rng_vectors.lua.

    #[test]
    fn floats_match_lua_seed_1() {
        let mut rng = LuaRng::new(1, 0);
        let expected: [f64; 8] = [
            0.81558781554723059,
            0.98657750643457565,
            0.079330719590026022,
            0.49864849323368698,
            0.59181018547898889,
            0.83396886864931397,
            0.15454780904609333,
            0.26419587508692477,
        ];
        for (i, &want) in expected.iter().enumerate() {
            let got: f64 = rng.next_f64();
            assert_eq!(got.to_bits(), want.to_bits(), "draw {i}: {got} != {want}");
        }
    }

    #[test]
    fn floats_match_lua_seed_42() {
        let mut rng = LuaRng::new(42, 0);
        let expected: [f64; 8] = [
            0.93081217803956817,
            0.45178389935924312,
            0.54688311243421495,
            0.79358935263827257,
            0.61731763595847267,
            0.0096006865506838013,
            0.96389739813422104,
            0.71097689661550945,
        ];
        for (i, &want) in expected.iter().enumerate() {
            let got: f64 = rng.next_f64();
            assert_eq!(got.to_bits(), want.to_bits(), "draw {i}: {got} != {want}");
        }
    }

    #[test]
    fn ranges_match_lua() {
        let mut rng = LuaRng::new(1, 0);
        assert_eq!(
            (0..8).map(|_| rng.next_range(1, 100)).collect::<Vec<_>>(),
            [30, 34, 45, 87, 94, 60, 94, 24]
        );
    }

    #[test]
    fn power_of_two_ranges_match_lua() {
        // exercises project()'s masking fast path
        let mut rng = LuaRng::new(1, 0);
        assert_eq!(
            (0..8).map(|_| rng.next_range(0, 7)).collect::<Vec<_>>(),
            [5, 7, 1, 7, 4, 6, 5, 0]
        );
    }

    #[test]
    fn full_range_does_not_overflow() {
        // width here is u64::MAX, the case that makes the naive `n - m` wrong
        let mut rng = LuaRng::new(1, 0);
        assert_eq!(
            (0..4)
                .map(|_| rng.next_range(i64::MIN, i64::MAX))
                .collect::<Vec<_>>(),
            [
                5821567666180820381,
                8975770733222381031,
                -7759978555394347615,
                -24930899432060937
            ]
        );
    }

    #[test]
    fn floats_stay_in_unit_interval() {
        let mut rng = LuaRng::new(7, 0);
        for _ in 0..100_000 {
            let x = rng.next_f64();
            assert!((0.0..1.0).contains(&x), "{x} out of range");
        }
    }

    #[test]
    fn ranges_stay_in_bounds() {
        let mut rng = LuaRng::new(9, 0);
        for _ in 0..100_000 {
            let x = rng.next_range(-5, 11);
            assert!((-5..=11).contains(&x), "{x} out of range");
        }
    }

    #[test]
    fn single_value_range_is_constant() {
        let mut rng = LuaRng::new(3, 0);
        for _ in 0..100 {
            assert_eq!(rng.next_range(4, 4), 4);
        }
    }
}
