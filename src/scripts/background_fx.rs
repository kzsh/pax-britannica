//! `scripts/background_fx.lua`: the spawner that keeps the sea busy.
//!
//! One actor per scene, alive for the whole scene. Every frame it rolls for a
//! piece of debris (15%) and a fish (3%), each dropped at a uniformly random
//! point over the whole field and given a random one of its sprites.
//!
//! Both spawns start fully transparent; the drifting script fades them in.

use crate::blueprints;
use crate::constants::{SCREEN_RIGHT, SCREEN_TOP};
use crate::game::Game;
use crate::scripts::debris::Debris;
use crate::scripts::fish::Fish;
use crate::v2::v2;
use crate::world::ActorId;

const DEBRIS_CHANCE: f64 = 0.15;
const FISH_CHANCE: f64 = 0.03;

const DEBRIS_SPRITES: i64 = 3;
const FISH_SPRITES: i64 = 8;

pub fn update(game: &mut Game, _id: ActorId) {
    if game.rng.next_f64() < DEBRIS_CHANCE {
        let pos = v2(
            game.rng.next_f64() * SCREEN_RIGHT,
            game.rng.next_f64() * SCREEN_TOP,
        );
        let sprite = game.rng.next_range(1, DEBRIS_SPRITES) as usize;
        let debris = Debris::new(&mut game.rng);
        game.world.spawn(blueprints::debris(pos, sprite, debris));
    }

    if game.rng.next_f64() < FISH_CHANCE {
        let pos = v2(
            game.rng.next_f64() * SCREEN_RIGHT,
            game.rng.next_f64() * SCREEN_TOP,
        );
        let sprite = game.rng.next_range(1, FISH_SPRITES) as usize;
        let fish = Fish::new(&mut game.rng);
        game.world.spawn(blueprints::fish(pos, sprite, fish));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::SpriteId;

    fn run(frames: usize) -> Game {
        let mut game = Game::new();
        let id = game.world.spawn(blueprints::background_fx());
        for _ in 0..frames {
            update(&mut game, id);
        }
        game
    }

    #[test]
    fn debris_is_commoner_than_fish_and_both_appear() {
        let game = run(1000);
        let debris = game.world.tagged("debris").len();
        let fish = game.world.tagged("fish").len();

        assert!(debris > 100, "roughly 15% of 1000 frames, got {debris}");
        assert!(fish > 10, "roughly 3% of 1000 frames, got {fish}");
        assert!(debris > fish);
    }

    #[test]
    fn spawns_are_spread_over_the_field_and_start_invisible() {
        let game = run(1000);

        for &id in game.world.tagged("debris") {
            let actor = game.world.get(id);
            let pos = actor.transform.unwrap().pos;
            assert!((0.0..SCREEN_RIGHT).contains(&pos.x));
            assert!((0.0..SCREEN_TOP).contains(&pos.y));
            assert_eq!(actor.sprite.as_ref().unwrap().color.unwrap().a, 0.0);
        }
    }

    #[test]
    fn every_sprite_in_both_sets_can_come_up() {
        let game = run(20000);
        let images: Vec<SpriteId> = game
            .world
            .tagged("debris")
            .iter()
            .chain(game.world.tagged("fish"))
            .filter_map(|&id| game.world.get(id).sprite.as_ref().and_then(|s| s.image))
            .collect();

        for i in 1..=DEBRIS_SPRITES as usize {
            assert!(
                images.contains(&SpriteId::Debris(i)),
                "debris {i} never used"
            );
        }
        for i in 1..=FISH_SPRITES as usize {
            assert!(images.contains(&SpriteId::Fish(i)), "fish {i} never used");
        }
    }
}
