//! `scripts/factory_ai.lua`: how a factory moves.
//!
//! It doesn't, much: full thrust and a constant left turn, against a turn speed
//! of 0.00028 and an acceleration of 0.002. The factory wanders a slow circle
//! for the whole match. Its other job is as a marker --
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
