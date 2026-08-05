//! `scripts/fighter_ai.lua`: charge, empty the magazine, run away, repeat.

use crate::game::Game;
use crate::scripts::fighter_shooting;
use crate::scripts::ship;
use crate::targeting;
use crate::world::ActorId;

/// How close a fighter must be to bother shooting.
const SHOT_RANGE: f64 = 200.0;
/// How far it tries to get while reloading.
const RUN_DISTANCE: f64 = 200.0;
/// Chance per frame of picking a new target for no reason.
const RETARGET_CHANCE: f64 = 0.005;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FighterAi {
    /// True while reloading and keeping its distance.
    pub running: bool,
    pub target: Option<ActorId>,
    /// Last frame's on-screen state; leaving the screen forces a retarget.
    pub on_screen: bool,
}

impl Default for FighterAi {
    fn default() -> Self {
        Self {
            running: false,
            target: None,
            // the Lua initialises this to true, so a fighter spawned offscreen
            // retargets on its first frame
            on_screen: true,
        }
    }
}

/// Nearest bomber, else fighter, else frigate, else factory.
fn retarget(game: &mut Game, id: ActorId) {
    let target = ["bomber", "fighter", "frigate", "factory"]
        .into_iter()
        .find_map(|kind| targeting::nearest_of_type(game, id, kind));
    ai_mut(game, id).target = target;
}

pub fn update(game: &mut Game, id: ActorId) {
    let pos = game.world.get(id).transform.expect("transform").pos;
    let new_on_screen = targeting::on_screen(pos);
    let state = *ai(game, id);

    // The `or` chain short-circuits in Lua, so the random draw only happens
    // when every earlier condition is false. Draw it unconditionally and the
    // whole stream shifts.
    let left_screen = state.on_screen && !new_on_screen;
    let target_gone = state.target.is_none_or(|t| game.world.is_dead(t));
    if left_screen || target_gone || game.rng.next_f64() < RETARGET_CHANCE {
        retarget(game, id);
    }
    ai_mut(game, id).on_screen = new_on_screen;

    let Some(target) = ai(game, id).target else {
        return;
    };

    let target_pos = game.world.get(target).transform.expect("transform").pos;
    let pos = game.world.get(id).transform.expect("transform").pos;
    let to_target = target_pos - pos;
    let distance_squared = to_target.sqrmag();

    if ai(game, id).running {
        let too_close = distance_squared < RUN_DISTANCE * RUN_DISTANCE;
        if too_close {
            ship::go_away(game, id, target_pos, true);
        } else {
            ship::thrust(game, id, 1.0);
        }

        if !weapon(game, id).is_empty() && !too_close {
            ai_mut(game, id).running = false;
        }
        return;
    }

    ship::go_towards(game, id, target_pos, true);

    if weapon(game, id).is_cooled_down() && !weapon(game, id).is_empty() {
        let facing = game.world.get(id).transform.expect("transform").facing;
        let alignment = to_target.dot(facing);
        // in range, ahead, and within a narrow cone: the squared comparison is
        // a normalise-free way of asking for cos^2 > 0.97
        let aimed = distance_squared <= SHOT_RANGE * SHOT_RANGE
            && alignment > 0.0
            && alignment * alignment > 0.97 * distance_squared;
        if aimed {
            fighter_shooting::shoot(game, id);
        }
    }

    if weapon(game, id).is_empty() {
        ai_mut(game, id).running = true;
    }
}

fn ai(game: &Game, id: ActorId) -> &FighterAi {
    game.world.get(id).fighter_ai.as_ref().expect("fighter_ai")
}

fn ai_mut(game: &mut Game, id: ActorId) -> &mut FighterAi {
    game.world
        .get_mut(id)
        .fighter_ai
        .as_mut()
        .expect("fighter_ai")
}

fn weapon(game: &Game, id: ActorId) -> crate::scripts::shooting::Weapon {
    game.world
        .get(id)
        .fighter_shooting
        .expect("fighter_shooting")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blueprints;
    use crate::v2::{V2, v2};

    fn spawn_fighter(game: &mut Game, pos: V2, player: usize) -> ActorId {
        game.world.spawn(blueprints::fighter(player, pos, V2::I))
    }

    #[test]
    fn a_fighter_with_no_target_takes_exactly_one_draw() {
        // `not target` is true, so the `or` chain short-circuits before the
        // random call, and the retarget itself finds nothing and draws nothing
        let mut game = Game::new();
        let id = spawn_fighter(&mut game, v2(100.0, 100.0), 1);

        let before = game.rng.clone();
        update(&mut game, id);
        assert_eq!(before, game.rng, "no draw should have been taken");
    }

    #[test]
    fn a_fighter_with_a_live_target_takes_one_draw() {
        let mut game = Game::new();
        let id = spawn_fighter(&mut game, v2(100.0, 100.0), 1);
        game.world
            .spawn(blueprints::bomber(2, v2(300.0, 100.0), V2::I));

        update(&mut game, id); // acquires the target
        let mut expected = game.rng.clone();
        expected.next_u64();

        update(&mut game, id);
        assert_eq!(expected, game.rng, "exactly one retarget roll");
    }

    #[test]
    fn a_fighter_prefers_bombers_over_everything() {
        let mut game = Game::new();
        let id = spawn_fighter(&mut game, v2(100.0, 100.0), 1);
        // a much closer fighter, and a distant bomber
        game.world
            .spawn(blueprints::fighter(2, v2(110.0, 100.0), V2::I));
        let bomber = game
            .world
            .spawn(blueprints::bomber(2, v2(600.0, 100.0), V2::I));

        update(&mut game, id);

        assert_eq!(ai(&game, id).target, Some(bomber));
    }

    #[test]
    fn a_fighter_retargets_onto_a_corpse_for_one_frame_then_moves_on() {
        // `components/targeting.lua` never checks `dead`, and the tag index is
        // only culled at the *end* of an update. So on the frame a target dies,
        // the retarget finds the same corpse again; only once the cull has run
        // does the fighter pick something living. Faithful to the original, and
        // worth knowing about when a trace divergence lands near a kill.
        let mut game = Game::new();
        let id = spawn_fighter(&mut game, v2(100.0, 100.0), 1);
        let first = game
            .world
            .spawn(blueprints::bomber(2, v2(300.0, 100.0), V2::I));

        crate::world::update(&mut game);
        assert_eq!(ai(&game, id).target, Some(first));

        game.world.kill(first);
        let second = game
            .world
            .spawn(blueprints::bomber(2, v2(400.0, 100.0), V2::I));

        // the corpse is still in the tag index for the rest of this frame
        crate::world::update(&mut game);
        assert_eq!(
            ai(&game, id).target,
            Some(first),
            "should still be aiming at the dead one"
        );

        // now that it has been culled, the next retarget finds the live one
        crate::world::update(&mut game);
        assert_eq!(ai(&game, id).target, Some(second));
    }

    #[test]
    fn an_empty_fighter_starts_running() {
        let mut game = Game::new();
        let id = spawn_fighter(&mut game, v2(100.0, 100.0), 1);
        game.world
            .spawn(blueprints::bomber(2, v2(300.0, 100.0), V2::I));

        game.world
            .get_mut(id)
            .fighter_shooting
            .as_mut()
            .unwrap()
            .shots = 0.0;
        update(&mut game, id);

        assert!(ai(&game, id).running);
    }

    #[test]
    fn a_running_fighter_stops_once_reloaded_and_clear() {
        let mut game = Game::new();
        let id = spawn_fighter(&mut game, v2(100.0, 100.0), 1);
        // far enough away to count as clear
        game.world
            .spawn(blueprints::bomber(2, v2(900.0, 100.0), V2::I));

        update(&mut game, id);
        ai_mut(&mut game, id).running = true;
        update(&mut game, id);

        assert!(!ai(&game, id).running);
    }

    #[test]
    fn a_fighter_shoots_a_target_dead_ahead_and_in_range() {
        let mut game = Game::new();
        let id = spawn_fighter(&mut game, v2(100.0, 100.0), 1);
        game.world
            .spawn(blueprints::bomber(2, v2(200.0, 100.0), V2::I));

        update(&mut game, id);

        assert_eq!(game.world.tagged("laser").len(), 1);
    }

    #[test]
    fn a_fighter_holds_fire_on_a_target_out_of_range() {
        let mut game = Game::new();
        let id = spawn_fighter(&mut game, v2(100.0, 100.0), 1);
        game.world
            .spawn(blueprints::bomber(2, v2(700.0, 100.0), V2::I));

        update(&mut game, id);

        assert!(game.world.tagged("laser").is_empty());
    }
}
