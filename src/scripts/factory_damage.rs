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

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FactoryDamage {
    /// Frames since the factory was built; `time/4 % 3` picks the art.
    pub time: i64,
    /// The opacity the scorch marks were last drawn at. Written by the draw,
    /// read by the renderer.
    pub flicker: f64,
}

impl FactoryDamage {
    /// Which of the three frames of damage art is showing.
    pub fn frame(&self) -> usize {
        (self.time / 4).rem_euclid(3) as usize + 1
    }
}

pub fn update(game: &mut Game, id: ActorId) {
    game.world
        .get_mut(id)
        .factory_damage
        .as_mut()
        .expect("factory_damage script")
        .time += 1;
}

/// Takes the flicker's draw and records the opacity it produces.
///
/// The draw is the part that is gameplay, because the stream is shared; the
/// opacity is what the renderer paints the scorch marks with.
pub fn draw(game: &mut Game, id: ActorId) {
    let alpha = game
        .world
        .get(id)
        .sprite
        .as_ref()
        .and_then(|sprite| sprite.color.map(|color| color.a))
        .unwrap_or(1.0);
    let flicker = alpha * game.rng.next_f64();

    game.world
        .get_mut(id)
        .factory_damage
        .as_mut()
        .expect("factory_damage script")
        .flicker = flicker;
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

    #[test]
    fn the_art_cycles_every_four_frames_over_three_frames() {
        let mut damage = FactoryDamage::default();
        let frames: Vec<usize> = (0..13)
            .map(|_| {
                let frame = damage.frame();
                damage.time += 1;
                frame
            })
            .collect();

        assert_eq!(frames, [1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 1]);
    }
}
