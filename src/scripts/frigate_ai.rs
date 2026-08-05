//! `scripts/frigate_ai.lua`: lumber into position, stop dead, fire a volley.
//!
//! The frigate is the only ship that has to come to a complete halt to shoot,
//! which is why it aims at a *fuzzed* target position -- without the scatter,
//! frigates converge on the same point and pile up.

use crate::game::Game;
use crate::scripts::frigate_shooting;
use crate::scripts::ship;
use crate::targeting;
use crate::v2::V2;
use crate::world::ActorId;

/// Chance per frame of re-picking a target. Lower than the other ships': a
/// frigate that keeps changing its mind never finishes stopping.
const RETARGET_CHANCE: f64 = 0.001;
/// How far the aim point is scattered from the real target.
const FUZZ_RADIUS: f64 = 250.0;
/// Closer than this and it backs off instead of closing.
const TOO_CLOSE: f64 = 200.0;
/// Squared speed below which it counts as stopped.
const STOPPED_SPEED_SQUARED: f64 = 0.01;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FrigateAi {
    /// True while it is trying to come to a halt to fire.
    pub stopping: bool,
    pub target: Option<ActorId>,
    pub target_fuzzy_pos: Option<V2>,

    /// Assigned without `local` in the Lua, so these live in the script's table
    /// and show up in the state trace.
    pub target_distance: Option<f64>,
    pub speed_square: Option<f64>,
}

/// Nearest fighter, else frigate, else factory. Note there is no bomber case:
/// frigates simply ignore them.
///
/// Two draws when a target is found, for the fuzz offset.
fn retarget(game: &mut Game, id: ActorId) {
    let target = ["fighter", "frigate", "factory"]
        .into_iter()
        .find_map(|kind| targeting::nearest_of_type(game, id, kind));

    ai_mut(game, id).target = target;

    if let Some(target) = target {
        let target_pos = game.world.get(target).transform.expect("transform").pos;
        let fuzzy = target_pos + V2::random(&mut game.rng) * FUZZ_RADIUS;
        ai_mut(game, id).target_fuzzy_pos = Some(fuzzy);
    }
}

pub fn update(game: &mut Game, id: ActorId) {
    let target = ai(game, id).target;

    // short-circuits before the roll when there is no target or it is dead
    let gone = target.is_none_or(|t| game.world.is_dead(t));
    if gone || game.rng.next_f64() < RETARGET_CHANCE {
        retarget(game, id);
    }

    let Some(target) = ai(game, id).target else {
        return;
    };

    let pos = game.world.get(id).transform.expect("transform").pos;
    let target_pos = game.world.get(target).transform.expect("transform").pos;
    let velocity = game.world.get(id).ship.as_ref().expect("ship").velocity;

    let target_distance = (target_pos - pos).mag();
    let speed_square = velocity.sqrmag();
    {
        let ai = ai_mut(game, id);
        ai.target_distance = Some(target_distance);
        ai.speed_square = Some(speed_square);
    }

    let ready = weapon(game, id);
    if ready.is_cooled_down() && ready.is_reloaded() && speed_square > 0.0 {
        ai_mut(game, id).stopping = true;
    } else if ready.is_empty() {
        ai_mut(game, id).stopping = false;
    }

    if !ai(game, id).stopping {
        let aim = ai(game, id).target_fuzzy_pos.unwrap_or(target_pos);
        if target_distance < TOO_CLOSE {
            ship::go_away(game, id, aim, true);
        } else {
            ship::go_towards(game, id, aim, true);
        }
    }

    // it fires once drifting has bled off nearly all its speed
    if !weapon(game, id).is_empty() && speed_square < STOPPED_SPEED_SQUARED {
        frigate_shooting::shoot(game, id);
    }
}

fn ai(game: &Game, id: ActorId) -> &FrigateAi {
    game.world.get(id).frigate_ai.as_ref().expect("frigate_ai")
}

fn ai_mut(game: &mut Game, id: ActorId) -> &mut FrigateAi {
    game.world
        .get_mut(id)
        .frigate_ai
        .as_mut()
        .expect("frigate_ai")
}

fn weapon(game: &Game, id: ActorId) -> crate::scripts::shooting::Weapon {
    game.world
        .get(id)
        .frigate_shooting
        .expect("frigate_shooting")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blueprints;
    use crate::v2::v2;

    fn spawn_frigate(game: &mut Game, pos: V2) -> ActorId {
        game.world.spawn(blueprints::frigate(1, pos, V2::I))
    }

    #[test]
    fn a_frigate_ignores_bombers() {
        let mut game = Game::new();
        let id = spawn_frigate(&mut game, v2(100.0, 100.0));
        game.world
            .spawn(blueprints::bomber(2, v2(150.0, 100.0), V2::I));

        update(&mut game, id);

        assert_eq!(ai(&game, id).target, None);
    }

    #[test]
    fn acquiring_a_target_fuzzes_the_aim_point() {
        let mut game = Game::new();
        let id = spawn_frigate(&mut game, v2(100.0, 100.0));
        let enemy = game
            .world
            .spawn(blueprints::fighter(2, v2(500.0, 400.0), V2::I));

        update(&mut game, id);

        let target_pos = game.world.get(enemy).transform.unwrap().pos;
        let aim = ai(&game, id).target_fuzzy_pos.expect("fuzzed aim point");
        assert_ne!(aim, target_pos);
        assert!((aim - target_pos).mag() <= FUZZ_RADIUS);
    }

    #[test]
    fn acquiring_a_target_takes_two_draws_for_the_fuzz() {
        let mut game = Game::new();
        let id = spawn_frigate(&mut game, v2(100.0, 100.0));
        game.world
            .spawn(blueprints::fighter(2, v2(500.0, 400.0), V2::I));

        let mut expected = game.rng.clone();
        expected.next_u64();
        expected.next_u64();

        update(&mut game, id);
        assert_eq!(expected, game.rng);
    }

    #[test]
    fn a_frigate_without_a_target_takes_no_draws() {
        let mut game = Game::new();
        let id = spawn_frigate(&mut game, v2(100.0, 100.0));

        let before = game.rng.clone();
        update(&mut game, id);
        assert_eq!(before, game.rng);
    }

    #[test]
    fn a_stopped_and_loaded_frigate_fires() {
        let mut game = Game::new();
        let id = spawn_frigate(&mut game, v2(100.0, 100.0));
        game.world
            .spawn(blueprints::fighter(2, v2(500.0, 100.0), V2::I));

        game.world
            .get_mut(id)
            .frigate_shooting
            .as_mut()
            .unwrap()
            .shots = 8.0;
        update(&mut game, id);

        assert_eq!(game.world.tagged("missile").len(), 1);
    }

    #[test]
    fn a_moving_frigate_holds_fire() {
        let mut game = Game::new();
        let id = spawn_frigate(&mut game, v2(100.0, 100.0));
        game.world
            .spawn(blueprints::fighter(2, v2(500.0, 100.0), V2::I));

        game.world
            .get_mut(id)
            .frigate_shooting
            .as_mut()
            .unwrap()
            .shots = 8.0;
        game.world.get_mut(id).ship.as_mut().unwrap().velocity = v2(5.0, 0.0);
        update(&mut game, id);

        assert!(game.world.tagged("missile").is_empty());
    }
}
