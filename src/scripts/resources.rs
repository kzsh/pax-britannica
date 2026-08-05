//! `scripts/resources.lua`: a factory's income.
//!
//! Tiny, but it is the clock the whole build menu runs on: `amount` ticks up by
//! `harvest_rate` every frame, `scripts/production.lua` spends it, and the one
//! upgrade the game offers buys a permanent increase to the rate.
//!
//! Not to be confused with [`crate::resources`], which is
//! `components/resources.lua` -- the art. The Lua has the same collision.

use crate::game::Game;
use crate::world::ActorId;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Resources {
    pub amount: f64,
    pub harvest_rate: f64,
}

impl Default for Resources {
    /// Enough to start with, but not enough for a fighter.
    fn default() -> Self {
        Self {
            amount: 60.0,
            harvest_rate: 0.75,
        }
    }
}

pub fn update(game: &mut Game, id: ActorId) {
    let resources = game
        .world
        .get_mut(id)
        .resources
        .as_mut()
        .expect("resources script");
    resources.amount += resources.harvest_rate;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scripts::ScriptKind;
    use crate::world::Actor;

    #[test]
    fn income_accrues_every_frame() {
        let mut game = Game::new();
        let id = game.world.spawn(Actor {
            scripts: vec![ScriptKind::Resources],
            resources: Some(Resources::default()),
            ..Actor::new("factory")
        });

        update(&mut game, id);
        update(&mut game, id);

        assert_eq!(game.world.get(id).resources.unwrap().amount, 61.5);
    }
}
