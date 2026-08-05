//! `scripts/fighter_shooting.lua`: the laser and its magazine.

use crate::blueprints;
use crate::game::Game;
use crate::scripts::shooting::Weapon;
use crate::world::ActorId;

const BULLET_SPEED: f64 = 20.0;

pub fn update(game: &mut Game, id: ActorId) {
    weapon_mut(game, id).update();
}

/// Fires forward, inheriting the ship's own velocity so a charging fighter's
/// shots fly faster than a retreating one's.
pub fn shoot(game: &mut Game, id: ActorId) {
    if !weapon_mut(game, id).try_fire() {
        return;
    }

    let actor = game.world.get(id);
    let transform = actor.transform.expect("transform");
    let ship_velocity = actor.ship.as_ref().expect("ship").velocity;
    let player = game.player_of(id);

    let velocity = transform.facing * BULLET_SPEED + ship_velocity;
    let laser = blueprints::laser(player, transform.pos, transform.facing, velocity);
    game.world.spawn(laser);
}

fn weapon_mut(game: &mut Game, id: ActorId) -> &mut Weapon {
    game.world
        .get_mut(id)
        .fighter_shooting
        .as_mut()
        .expect("fighter_shooting")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::{V2, v2};

    fn spawn(game: &mut Game) -> ActorId {
        game.world
            .spawn(blueprints::fighter(1, v2(100.0, 100.0), V2::I))
    }

    #[test]
    fn shooting_spawns_a_laser_carrying_the_ships_velocity() {
        let mut game = Game::new();
        let id = spawn(&mut game);
        game.world.get_mut(id).ship.as_mut().unwrap().velocity = v2(2.0, 0.0);

        shoot(&mut game, id);

        let lasers = game.world.tagged("laser");
        assert_eq!(lasers.len(), 1);
        let laser = game.world.get(lasers[0]);
        assert_eq!(laser.bullet.unwrap().velocity, v2(22.0, 0.0));
        assert_eq!(laser.player, Some(1));
    }

    #[test]
    fn a_fighter_cannot_fire_twice_in_a_row() {
        let mut game = Game::new();
        let id = spawn(&mut game);
        shoot(&mut game, id);
        shoot(&mut game, id);
        assert_eq!(game.world.tagged("laser").len(), 1);
    }

    #[test]
    fn five_shots_empty_the_magazine() {
        let mut game = Game::new();
        let id = spawn(&mut game);
        for _ in 0..5 {
            shoot(&mut game, id);
            for _ in 0..6 {
                update(&mut game, id);
            }
        }
        // six frames of reload per shot is well under a full round
        assert!(game.world.get(id).fighter_shooting.unwrap().is_empty());
        assert_eq!(game.world.tagged("laser").len(), 5);
    }
}
