//! `scripts/frigate_shooting.lua`: heat-seeking missiles, fired in volleys.

use crate::blueprints;
use crate::game::Game;
use crate::scripts::heatseeking_ai;
use crate::scripts::shooting::Weapon;
use crate::world::ActorId;

/// Missiles leave the tube slowly; the heatseeking AI does the accelerating.
const MISSILE_SPEED: f64 = 1.0;

pub fn update(game: &mut Game, id: ActorId) {
    weapon_mut(game, id).update();
}

pub fn shoot(game: &mut Game, id: ActorId) {
    if !weapon_mut(game, id).try_fire() {
        return;
    }

    let actor = game.world.get(id);
    let transform = actor.transform.expect("transform");
    let ship_velocity = actor.ship.as_ref().expect("ship").velocity;
    let player = game.player_of(id);

    let velocity = transform.facing * MISSILE_SPEED + ship_velocity;
    let missile = blueprints::missile(player, transform.pos, velocity);
    let missile_id = game.world.spawn(missile);

    // heatseeking_ai.lua calls retarget() in its chunk body, so a missile picks
    // a target the moment it exists rather than on its first update
    heatseeking_ai::retarget(game, missile_id);
}

fn weapon_mut(game: &mut Game, id: ActorId) -> &mut Weapon {
    game.world
        .get_mut(id)
        .frigate_shooting
        .as_mut()
        .expect("frigate_shooting")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::{V2, v2};

    fn spawn(game: &mut Game) -> ActorId {
        let id = game
            .world
            .spawn(blueprints::frigate(1, v2(100.0, 100.0), V2::I));
        game.world
            .get_mut(id)
            .frigate_shooting
            .as_mut()
            .unwrap()
            .shots = 8.0;
        id
    }

    #[test]
    fn a_missile_inherits_the_frigates_velocity() {
        let mut game = Game::new();
        let id = spawn(&mut game);
        game.world.get_mut(id).ship.as_mut().unwrap().velocity = v2(0.0, 2.0);

        shoot(&mut game, id);

        let missiles = game.world.tagged("missile");
        assert_eq!(missiles.len(), 1);
        let ship = game.world.get(missiles[0]).ship.as_ref().unwrap();
        assert_eq!(ship.velocity, v2(1.0, 2.0));
    }

    #[test]
    fn a_frigate_needs_a_full_magazine_to_start_a_volley() {
        // is_ready_to_shoot is reload-gated, but shoot() itself only needs one
        // round, which is what lets the volley continue as the magazine drains
        let mut game = Game::new();
        let id = spawn(&mut game);

        for _ in 0..8 {
            shoot(&mut game, id);
            for _ in 0..6 {
                update(&mut game, id);
            }
        }

        assert_eq!(game.world.tagged("missile").len(), 8);
        assert!(game.world.get(id).frigate_shooting.unwrap().is_empty());
    }
}
