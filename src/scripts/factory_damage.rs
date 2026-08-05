//! `scripts/factory_damage.lua`: the scorch marks on a wounded factory.
//!
//! Purely cosmetic -- three frames of light damage art below 60% health, three
//! of heavy below 30%, cycling every four frames and flickering at a random
//! opacity. **It is not free, though.** That flicker is one draw off the shared
//! random stream per factory per drawn frame, unconditional, so it is part of
//! the sequence every other script's draws are interleaved with. Skip it and
//! every run diverges from the first frame a factory exists.
//!
//! See PORTING.md on `components/particles.lua` for the same trap in a louder
//! form.

use crate::game::Game;
use crate::world::ActorId;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FactoryDamage {
    /// Frames since the factory was built; `time/4 % 3` picks the art.
    pub time: i64,
}

pub fn update(game: &mut Game, id: ActorId) {
    game.world
        .get_mut(id)
        .factory_damage
        .as_mut()
        .expect("factory_damage script")
        .time += 1;
}

/// Takes the flicker's draw. The colour it computes is renderer work for phase
/// 4; the draw itself is gameplay, because the stream is shared.
pub fn draw(game: &mut Game, _id: ActorId) {
    game.rng.next_f64();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scripts::ScriptKind;
    use crate::world::Actor;

    fn spawn_factory(game: &mut Game) -> ActorId {
        game.world.spawn(Actor {
            scripts: vec![ScriptKind::FactoryDamage],
            factory_damage: Some(FactoryDamage::default()),
            ..Actor::new("factory")
        })
    }

    #[test]
    fn the_flicker_costs_exactly_one_draw_per_frame() {
        let mut game = Game::new();
        let id = spawn_factory(&mut game);

        let mut expected = game.rng.clone();
        draw(&mut game, id);
        expected.next_u64();

        assert_eq!(expected, game.rng);
    }

    #[test]
    fn the_animation_clock_runs_regardless_of_health() {
        let mut game = Game::new();
        let id = spawn_factory(&mut game);

        for _ in 0..7 {
            update(&mut game, id);
        }

        assert_eq!(game.world.get(id).factory_damage.unwrap().time, 7);
    }
}
