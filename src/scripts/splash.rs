//! `scripts/splash.lua`: the title and the credits on the menu screen.
//!
//! Two sprites at fixed positions and nothing else -- no state, no `update`, and
//! a `draw` that takes no draws off the shared random stream. Checked rather
//! than assumed, per PORTING.md's rule about `draw` methods.
//!
//! It still needs to exist as a script, because the actor carrying it is spawned
//! by `scripts/game_flow.lua`, occupies a place in the spawn order, and is
//! killed by name when the match starts.

/// Where the two sprites go, for phase 4. In game units, origin bottom left.
pub const TITLE_POS: crate::v2::V2 = crate::v2::v2(1024.0 / 2.0, 768.0 / 2.0 + 768.0 / 4.0);
pub const CREDITS_POS: crate::v2::V2 = crate::v2::v2(1024.0 / 2.0 + 265.0, 768.0 / 2.0);
