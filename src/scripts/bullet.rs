//! `scripts/bullet.lua`: a projectile that flies straight and expires offscreen.
//!
//! Missiles do *not* use this -- they carry `ship` and `heatseeking_ai` instead,
//! which is why `components/particles.lua` has to ask whether a projectile has a
//! `bullet` script or a `ship` one before it can find its velocity.

use crate::game::Game;
use crate::v2::V2;
use crate::world::ActorId;

/// How far past the screen edge a bullet travels before it is reaped.
const BUFFER: f64 = 500.0;

/// The play area a bullet is culled against. `game.opengl_2d.width/height` in
/// the Lua, which `the_game.lua` sets to the screen size.
const SCREEN: (f64, f64) = (1024.0, 768.0);

#[derive(Clone, Copy, Debug)]
pub struct Bullet {
    pub velocity: V2,
}

pub fn update(game: &mut Game, id: ActorId) {
    let velocity = game.world.get(id).bullet.expect("bullet script").velocity;

    let transform = game
        .world
        .get_mut(id)
        .transform
        .as_mut()
        .expect("transform");
    transform.pos = transform.pos + velocity;
    let pos = transform.pos;

    let escaped = pos.x > SCREEN.0 + BUFFER
        || pos.y > SCREEN.1 + BUFFER
        || pos.x < -BUFFER
        || pos.y < -BUFFER;

    if escaped {
        game.log.record_miss(game.world.get(id).blueprint);
        game.world.kill(id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scripts::ScriptKind;
    use crate::scripts::transform::Transform;
    use crate::v2::v2;
    use crate::world::Actor;

    fn spawn_bullet(game: &mut Game, pos: V2, velocity: V2) -> ActorId {
        game.world.spawn(Actor {
            scripts: vec![ScriptKind::Transform, ScriptKind::Bullet],
            transform: Some(Transform::at(pos)),
            bullet: Some(Bullet { velocity }),
            player: Some(1),
            ..Actor::new("laser")
        })
    }

    #[test]
    fn a_bullet_travels_at_its_velocity() {
        let mut game = Game::new();
        let id = spawn_bullet(&mut game, V2::ZERO, v2(3.0, 4.0));
        update(&mut game, id);
        assert_eq!(game.world.get(id).transform.unwrap().pos, v2(3.0, 4.0));
        assert!(!game.world.is_dead(id));
    }

    #[test]
    fn a_bullet_well_offscreen_is_reaped_and_recorded_as_a_miss() {
        let mut game = Game::new();
        let id = spawn_bullet(&mut game, v2(SCREEN.0 + BUFFER, 0.0), v2(1.0, 0.0));
        update(&mut game, id);
        assert!(game.world.is_dead(id));
        assert_eq!(game.log.accuracy_misses.fighter, 1.0);
    }

    #[test]
    fn the_buffer_keeps_just_offscreen_bullets_alive() {
        // shots fired at the screen edge must not vanish the moment they leave
        let mut game = Game::new();
        let id = spawn_bullet(&mut game, v2(SCREEN.0 + 10.0, 0.0), V2::ZERO);
        update(&mut game, id);
        assert!(!game.world.is_dead(id));
    }
}
