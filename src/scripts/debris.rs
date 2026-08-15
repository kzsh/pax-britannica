//! `scripts/debris.lua`: a scrap of junk drifting across the sea floor.
//!
//! Purely decorative. Everything that makes one debris mote differ from the
//! next is drawn once, when it spawns, and kept in chunk-level locals -- so they
//! are per-instance fields here. It drifts in a fixed direction, wobbling
//! slightly, fades in over two seconds, fades out over the last two, and dies at
//! eight.

use crate::game::Game;
use crate::v2::V2;
use crate::world::ActorId;

const SPEED: f64 = 0.2;
const LIFETIME: f64 = 8.0;
const FADE_TIME: f64 = 2.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Debris {
    pub direction: V2,
    pub scale: f64,
    pub speed: f64,
    pub opacity: f64,
    pub since_alive: f64,
}

impl Debris {
    /// Four draws, in the order the Lua's chunk body takes them.
    pub fn new(rng: &mut crate::rng::LuaRng) -> Self {
        Self {
            direction: V2::unit(rng.next_f64() * 2.0 * std::f64::consts::PI),
            scale: rng.next_f64() * 0.75 + 0.5,
            speed: rng.next_f64() + 0.5,
            opacity: rng.next_f64() * 0.25 + 0.4,
            since_alive: 0.0,
        }
    }
}

pub fn update(game: &mut Game, id: ActorId) {
    let actor = game.world.get_mut(id);
    let debris = actor.debris.as_mut().expect("debris script");

    debris.since_alive += 1.0 / 60.0;
    let debris = *debris;

    let transform = actor.transform.as_mut().expect("debris has a transform");
    transform.scale_x = debris.scale;
    transform.scale_y = debris.scale;
    transform.facing = debris.direction;
    transform.pos = transform.pos
        + debris.direction.rotate(debris.since_alive.sin() * 0.5) * SPEED * debris.speed;

    let sprite = actor.sprite.as_mut().expect("debris has a sprite");
    if let Some(color) = sprite.color.as_mut() {
        color.a = alpha(debris.since_alive) * debris.opacity;
    }

    if debris.since_alive > LIFETIME {
        game.world.kill(id);
    }
}

/// The fade envelope, as a fraction of the mote's own peak opacity.
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
    use crate::v2::v2;

    fn spawn(game: &mut Game) -> ActorId {
        let debris = Debris::new(&mut game.rng);
        game.world
            .spawn(blueprints::debris(v2(100.0, 100.0), 1, debris))
    }

    #[test]
    fn a_mote_fades_in_then_out_and_dies_at_eight_seconds() {
        let mut game = Game::new();
        let id = spawn(&mut game);
        let peak = game.world.get(id).debris.unwrap().opacity;
        let alpha_now = |game: &Game| game.world.get(id).sprite.as_ref().unwrap().color.unwrap().a;

        for _ in 0..60 {
            update(&mut game, id);
        }
        let after_a_second = alpha_now(&game);
        assert!(after_a_second > 0.0 && after_a_second < peak);

        for _ in 60..120 {
            update(&mut game, id);
        }
        assert!((alpha_now(&game) - peak).abs() < 1e-9, "fully faded in");

        for _ in 120..7 * 60 {
            update(&mut game, id);
        }
        assert!(alpha_now(&game) < peak, "fading out again");
        assert!(!game.world.is_dead(id));

        for _ in 7 * 60..8 * 60 + 1 {
            update(&mut game, id);
        }
        assert!(game.world.is_dead(id));
    }

    #[test]
    fn a_mote_drifts_and_takes_no_draws_while_it_does() {
        let mut game = Game::new();
        let id = spawn(&mut game);
        let start = game.world.get(id).transform.unwrap().pos;
        let rng = game.rng.clone();

        for _ in 0..60 {
            update(&mut game, id);
        }

        assert_ne!(game.world.get(id).transform.unwrap().pos, start);
        assert_eq!(game.rng, rng, "drifting is deterministic once spawned");
    }
}
