//! `scripts/player_production.lua`: a human on the other end of the button.
//!
//! The entire player interface, in one line. Note it runs *after* `production`
//! in the blueprint, so the dial always acts on the previous frame's input. That
//! one-frame lag is in the original and is load-bearing for the trace.

use crate::game::Game;
use crate::world::ActorId;

pub fn update(game: &mut Game, id: ActorId) {
    let held = game.the_one_button.held(game.player_of(id));
    game.world
        .get_mut(id)
        .production
        .as_mut()
        .expect("player_production needs the production script")
        .button_held = held;
}
