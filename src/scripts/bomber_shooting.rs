//! `scripts/bomber_shooting.lua`: bombs lobbed out of the side.

use crate::blueprints;
use crate::game::Game;
use crate::world::ActorId;

const SPEED: f64 = 3.5;

/// Fires perpendicular to the bomber's facing, to whichever side it is
/// circling. Note the bomb does *not* inherit the ship's velocity -- the Lua
/// has that addition commented out, and putting it back would change the arc.
pub fn shoot(game: &mut Game, id: ActorId, approach_sign: f64) {
    let transform = game.world.get(id).transform.expect("transform");
    let player = game.player_of(id);

    let facing = transform.facing.rotate90() * approach_sign;
    let velocity = facing * SPEED;

    let bomb = blueprints::bomb(player, transform.pos, facing, velocity);
    game.world.spawn(bomb);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::{V2, v2};

    #[test]
    fn bombs_are_thrown_sideways() {
        let mut game = Game::new();
        let id = game
            .world
            .spawn(blueprints::bomber(1, v2(100.0, 100.0), V2::I));

        shoot(&mut game, id, 1.0);

        let bombs = game.world.tagged("bomb");
        assert_eq!(bombs.len(), 1);
        // facing +x, so a bomb to port travels +y
        assert_eq!(
            game.world.get(bombs[0]).bullet.unwrap().velocity,
            v2(0.0, 3.5)
        );
    }

    #[test]
    fn the_approach_sign_flips_the_side() {
        let mut game = Game::new();
        let id = game
            .world
            .spawn(blueprints::bomber(1, v2(100.0, 100.0), V2::I));

        shoot(&mut game, id, -1.0);

        let bombs = game.world.tagged("bomb");
        assert_eq!(
            game.world.get(bombs[0]).bullet.unwrap().velocity,
            v2(0.0, -3.5)
        );
    }

    #[test]
    fn a_bomb_ignores_the_bombers_velocity() {
        let mut game = Game::new();
        let id = game
            .world
            .spawn(blueprints::bomber(1, v2(100.0, 100.0), V2::I));
        game.world.get_mut(id).ship.as_mut().unwrap().velocity = v2(50.0, 50.0);

        shoot(&mut game, id, 1.0);

        let bombs = game.world.tagged("bomb");
        assert_eq!(
            game.world.get(bombs[0]).bullet.unwrap().velocity,
            v2(0.0, 3.5)
        );
    }
}
