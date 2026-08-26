//! `scripts/factory_ai.lua`: how a factory moves.
//!
//! It doesn't, much: full thrust and a constant left turn. The factory wanders a
//! slow circle for the whole match, of radius (terminal speed) / (turn speed) --
//! and the turn speed in `blueprints.rs` is divided by the field scale so that
//! circle keeps its share of the field. Its other job is as a marker --
//! [`ship::destruct`](crate::scripts::ship::destruct) branches on it to choose
//! the drawn-out factory death over an ordinary one.

use crate::game::Game;
use crate::scripts::ship;
use crate::world::ActorId;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FactoryAi {}

pub fn update(game: &mut Game, id: ActorId) {
    ship::thrust(game, id, 1.0);
    ship::turn(game, id, 1.0, 1.0);
}

#[cfg(test)]
mod tests {
    use crate::blueprints;
    use crate::constants::{CENTER, PLAY_SCALE};
    use crate::game::Game;
    use crate::v2::V2;
    use crate::world::{Phase, run_phase};

    /// Radius of the circle a factory wanders on the original 1024x768 field:
    /// its terminal speed over its turn rate.
    ///
    /// `ship::update` runs before `factory_ai::update` in the blueprint's script
    /// list, so a frame is drag and then thrust, and the speed this samples
    /// after the update phase settles on `accel / (1 - drag)`: about 0.067 units
    /// a frame, over 0.00028 radians a frame, or a radius of some 238 units.
    const BASE_RADIUS: f64 = 0.002 / (1.0 - 0.97) / 0.00028;

    #[test]
    fn a_factory_circles_at_a_radius_that_follows_the_field() {
        let mut game = Game::new();
        let id = game
            .world
            .spawn(blueprints::player_factory(1, CENTER, V2::I));

        // long enough for the drag and the thrust to reach their balance, short
        // enough that the arc travelled is still a small part of the circle
        for _ in 0..2000 {
            run_phase(&mut game, Phase::Update);
        }

        let ship = game.world.get(id).ship.as_ref().expect("factory ship");
        let radius = ship.velocity.mag() / ship.turn_speed;
        let want = BASE_RADIUS * PLAY_SCALE;

        assert!(
            (radius - want).abs() < want * 0.01,
            "circles at {radius}, not {want}"
        );
    }
}
