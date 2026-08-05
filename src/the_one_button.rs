//! `components/the_one_button.lua`: the whole of the game's input.
//!
//! Each of the four players has exactly one button. The Lua reads the keyboard
//! and the four joysticks in `update_setup` and latches the result, so every
//! script sees a single consistent view of the input for the frame, and
//! `pressed`/`released` can be derived by comparing against the previous frame.
//!
//! Here the polling is split from the latching: whoever is driving the game
//! (the headless runner now, the platform layer in phase 4) writes [`keys`]
//! before the update, and [`latch`] moves it into place as the `update_setup`
//! phase runs. That keeps the component free of any dependency on a window.
//!
//! [`keys`]: TheOneButton::keys
//! [`latch`]: TheOneButton::latch

/// Players are 1-based throughout the game, matching the Lua.
pub const PLAYERS: usize = 4;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TheOneButton {
    /// The raw input for the coming frame, written by the driver.
    pub keys: [bool; PLAYERS],
    states: [bool; PLAYERS],
    old_states: [bool; PLAYERS],
}

impl TheOneButton {
    pub fn new() -> Self {
        Self::default()
    }

    /// `update_setup`: yesterday's news becomes history, today's input becomes
    /// the state every script will read this frame.
    pub fn latch(&mut self) {
        self.old_states = self.states;
        self.states = self.keys;
    }

    /// `held(i)`, for a 1-based player number.
    pub fn held(&self, player: usize) -> bool {
        self.states[Self::index(player)]
    }

    /// `pressed(i)`: held now, not held last frame.
    pub fn pressed(&self, player: usize) -> bool {
        let i = Self::index(player);
        self.states[i] && !self.old_states[i]
    }

    /// `released(i)`: not held now, held last frame.
    pub fn released(&self, player: usize) -> bool {
        let i = Self::index(player);
        !self.states[i] && self.old_states[i]
    }

    /// The Lua asserts `1 <= i and i <= 4`; a bad player number is a bug in the
    /// caller either way.
    fn index(player: usize) -> usize {
        assert!(
            (1..=PLAYERS).contains(&player),
            "player {player} is not one of the four"
        );
        player - 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_is_only_visible_after_it_is_latched() {
        let mut button = TheOneButton::new();
        button.keys[0] = true;
        assert!(
            !button.held(1),
            "the frame's view of input is fixed at latch"
        );

        button.latch();
        assert!(button.held(1));
    }

    #[test]
    fn pressed_and_released_are_edges() {
        let mut button = TheOneButton::new();

        button.keys[1] = true;
        button.latch();
        assert!(button.pressed(2));
        assert!(!button.released(2));

        button.latch();
        assert!(!button.pressed(2), "still held is not pressed again");

        button.keys[1] = false;
        button.latch();
        assert!(button.released(2));
    }

    #[test]
    fn players_are_independent() {
        let mut button = TheOneButton::new();
        button.keys[2] = true;
        button.latch();

        assert!(button.held(3));
        assert!(!button.held(1));
        assert!(!button.held(4));
    }
}
