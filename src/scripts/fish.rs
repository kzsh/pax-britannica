//! `scripts/fish.lua`: a fish swimming across the background.
//!
//! The same shape as [`debris`](crate::scripts::debris) with three differences:
//! it swims dead left or dead right rather than in any direction, it does not
//! wobble, and it is fainter and lives two seconds longer.

use crate::game::Game;
use crate::v2::{V2, v2};
use crate::world::ActorId;

const SPEED: f64 = 0.2;
const LIFETIME: f64 = 10.0;
const FADE_TIME: f64 = 2.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fish {
    pub direction: V2,
    pub scale: f64,
    pub speed: f64,
    pub opacity: f64,
    pub since_alive: f64,
}

impl Fish {
    /// Four draws, in the order the Lua's chunk body takes them.
    pub fn new(rng: &mut crate::rng::LuaRng) -> Self {
        Self {
            direction: v2(if rng.next_f64() < 0.5 { 1.0 } else { -1.0 }, 0.0),
            scale: rng.next_f64() * 0.75 + 0.5,
            speed: rng.next_f64() + 0.5,
            opacity: rng.next_f64() * 0.1 + 0.1,
            since_alive: 0.0,
        }
    }
}

pub fn update(game: &mut Game, id: ActorId) {
    let actor = game.world.get_mut(id);
    let fish = actor.fish.as_mut().expect("fish script");

    fish.since_alive += 1.0 / 60.0;
    let fish = *fish;

    let transform = actor.transform.as_mut().expect("fish has a transform");
    transform.scale_x = fish.scale;
    transform.scale_y = fish.scale;
    transform.facing = fish.direction;
    transform.pos = transform.pos + fish.direction * SPEED * fish.speed;

    let sprite = actor.sprite.as_mut().expect("fish has a sprite");
    if let Some(color) = sprite.color.as_mut() {
        color.a = alpha(fish.since_alive) * fish.opacity;
    }

    if fish.since_alive > LIFETIME {
        game.world.kill(id);
    }
}

fn alpha(since_alive: f64) -> f64 {
    if since_alive < FADE_TIME {
        since_alive / FADE_TIME
    } else {
        (1.0 - (since_alive - LIFETIME + FADE_TIME) / FADE_TIME).min(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blueprints;

    fn spawn(game: &mut Game) -> ActorId {
        let fish = Fish::new(&mut game.rng);
        game.world.spawn(blueprints::fish(v2(0.0, 0.0), 1, fish))
    }

    #[test]
    fn a_fish_swims_horizontally_at_a_constant_speed() {
        let mut game = Game::new();
        let id = spawn(&mut game);
        let fish = game.world.get(id).fish.unwrap();
        assert!(fish.direction == v2(1.0, 0.0) || fish.direction == v2(-1.0, 0.0));

        update(&mut game, id);
        let after_one = game.world.get(id).transform.unwrap().pos;
        update(&mut game, id);
        let after_two = game.world.get(id).transform.unwrap().pos;

        assert_eq!(after_one.y, 0.0);
        assert_eq!(after_two.y, 0.0);
        assert_eq!(after_two.x - after_one.x, after_one.x);
    }

    #[test]
    fn a_fish_dies_at_ten_seconds() {
        let mut game = Game::new();
        let id = spawn(&mut game);

        // repeated addition of 1/60 does not land exactly on 10, so the frame
        // it dies on is 600 give or take one
        for _ in 0..9 * 60 {
            update(&mut game, id);
        }
        assert!(!game.world.is_dead(id));

        for _ in 9 * 60..10 * 60 + 2 {
            update(&mut game, id);
        }
        assert!(game.world.is_dead(id));
    }

    #[test]
    fn a_fish_is_fainter_than_debris() {
        let mut game = Game::new();
        for _ in 0..20 {
            let fish = Fish::new(&mut game.rng);
            assert!((0.1..=0.2).contains(&fish.opacity));
        }
    }
}
