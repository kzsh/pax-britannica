//! `scripts/bomber_ai.lua`: close in, turn broadside, unload four bombs, leave.

use crate::game::Game;
use crate::scripts::bomber_shooting;
use crate::scripts::ship;
use crate::targeting;
use crate::v2::V2;
use crate::world::ActorId;

const APPROACH_DISTANCE: f64 = 275.0;
const COOLDOWN_DURATION: i32 = 40;
const MAX_SHOTS: i32 = 4;
const RETARGET_CHANCE: f64 = 0.005;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BomberState {
    Approach,
    Turn,
    Shoot,
    MoveAway,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BomberAi {
    pub state: BomberState,
    pub cooldown_timer: i32,
    pub shots_counter: i32,
    pub target: Option<ActorId>,
    /// Which way round the target it circles: 1 or -1.
    pub approach_sign: f64,

    /// `bomber_ai.lua` assigns these two without `local`, so in Lua they land in
    /// the script's table and appear in the state trace. Kept for that reason.
    pub target_distance: Option<f64>,
    pub target_direction: Option<V2>,
}

impl Default for BomberAi {
    fn default() -> Self {
        Self {
            state: BomberState::Approach,
            cooldown_timer: COOLDOWN_DURATION,
            shots_counter: MAX_SHOTS,
            target: None,
            approach_sign: 1.0,
            target_distance: None,
            target_direction: None,
        }
    }
}

/// Nearest frigate, else factory, else bomber, else fighter -- bombers go for
/// the big slow things.
fn retarget(game: &mut Game, id: ActorId) {
    let target = ["frigate", "factory", "bomber", "fighter"]
        .into_iter()
        .find_map(|kind| targeting::nearest_of_type(game, id, kind));
    ai_mut(game, id).target = target;
}

/// One draw, to pick a side to circle from.
fn revise_approach(game: &mut Game, id: ActorId) {
    let sign = if game.rng.next_f64() < 0.5 { 1.0 } else { -1.0 };
    ai_mut(game, id).approach_sign = sign;
}

pub fn update(game: &mut Game, id: ActorId) {
    let previous_target = ai(game, id).target;

    // short-circuits: the roll only happens when there *is* a live target
    let no_target = previous_target.is_none();
    if no_target || game.rng.next_f64() < RETARGET_CHANCE || is_dead(game, previous_target) {
        retarget(game, id);
        if ai(game, id).target != previous_target {
            revise_approach(game, id);
        }
    }

    let Some(target) = ai(game, id).target else {
        return;
    };

    let pos = game.world.get(id).transform.expect("transform").pos;
    let target_pos = game.world.get(target).transform.expect("transform").pos;
    let target_distance = (target_pos - pos).mag();
    let target_direction = (target_pos - pos).norm();

    {
        let ai = ai_mut(game, id);
        ai.target_distance = Some(target_distance);
        ai.target_direction = Some(target_direction);
    }

    // frigates are big enough to warrant a tighter orbit
    let unit_factor = if game.world.get(target).blueprint == "frigate" {
        0.6
    } else {
        1.0
    };

    if target_distance > (APPROACH_DISTANCE + 50.0) * unit_factor {
        ai_mut(game, id).state = BomberState::Approach;
    }

    match ai(game, id).state {
        BomberState::Approach => {
            ship::go_towards(game, id, target_pos, true);
            if target_distance < APPROACH_DISTANCE * unit_factor {
                revise_approach(game, id);
                ai_mut(game, id).state = BomberState::Turn;
            }
        }

        BomberState::Turn => {
            let sign = ai(game, id).approach_sign;
            ship::turn(game, id, -sign, 1.0);
            ship::thrust(game, id, unit_factor * 0.75);

            let facing = game.world.get(id).transform.expect("transform").facing;
            if target_direction.dot(facing) < 0.5 {
                ai_mut(game, id).state = BomberState::Shoot;
            }
        }

        BomberState::Shoot => {
            let sign = ai(game, id).approach_sign;
            ship::turn(game, id, sign * 0.05, 1.0);
            ship::thrust(game, id, unit_factor * 0.75);

            ai_mut(game, id).cooldown_timer -= 1;
            if ai(game, id).cooldown_timer == 0 {
                bomber_shooting::shoot(game, id, sign);

                let ai = ai_mut(game, id);
                ai.cooldown_timer = COOLDOWN_DURATION;
                ai.shots_counter -= 1;

                if ai.shots_counter == 0 {
                    ai.shots_counter = MAX_SHOTS;
                    ai.state = BomberState::MoveAway;
                }
            }
        }

        BomberState::MoveAway => ship::go_away(game, id, target_pos, true),
    }
}

fn is_dead(game: &Game, target: Option<ActorId>) -> bool {
    target.is_some_and(|t| game.world.is_dead(t))
}

fn ai(game: &Game, id: ActorId) -> &BomberAi {
    game.world.get(id).bomber_ai.as_ref().expect("bomber_ai")
}

fn ai_mut(game: &mut Game, id: ActorId) -> &mut BomberAi {
    game.world
        .get_mut(id)
        .bomber_ai
        .as_mut()
        .expect("bomber_ai")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blueprints;
    use crate::v2::v2;

    fn spawn_bomber(game: &mut Game, pos: V2) -> ActorId {
        game.world.spawn(blueprints::bomber(1, pos, V2::I))
    }

    #[test]
    fn a_bomber_prefers_frigates() {
        let mut game = Game::new();
        let id = spawn_bomber(&mut game, v2(100.0, 100.0));
        game.world
            .spawn(blueprints::fighter(2, v2(110.0, 100.0), V2::I));
        let frigate = game
            .world
            .spawn(blueprints::frigate(2, v2(600.0, 100.0), V2::I));

        update(&mut game, id);

        assert_eq!(ai(&game, id).target, Some(frigate));
    }

    #[test]
    fn closing_on_a_target_switches_from_approach_to_turn() {
        let mut game = Game::new();
        let id = spawn_bomber(&mut game, v2(100.0, 100.0));
        game.world
            .spawn(blueprints::fighter(2, v2(300.0, 100.0), V2::I));

        update(&mut game, id);

        assert_eq!(ai(&game, id).state, BomberState::Turn);
    }

    #[test]
    fn a_distant_target_keeps_the_bomber_approaching() {
        let mut game = Game::new();
        let id = spawn_bomber(&mut game, v2(100.0, 100.0));
        game.world
            .spawn(blueprints::fighter(2, v2(800.0, 400.0), V2::I));

        update(&mut game, id);

        assert_eq!(ai(&game, id).state, BomberState::Approach);
    }

    #[test]
    fn four_bombs_then_it_leaves() {
        let mut game = Game::new();
        let id = spawn_bomber(&mut game, v2(100.0, 100.0));
        game.world
            .spawn(blueprints::fighter(2, v2(300.0, 100.0), V2::I));

        update(&mut game, id);
        ai_mut(&mut game, id).state = BomberState::Shoot;

        // 4 shots at one per 40 frames
        for _ in 0..(COOLDOWN_DURATION * MAX_SHOTS) {
            update(&mut game, id);
        }

        assert_eq!(game.world.tagged("bomb").len(), MAX_SHOTS as usize);
        assert_eq!(ai(&game, id).state, BomberState::MoveAway);
    }

    #[test]
    fn a_bomber_with_no_target_takes_no_draws() {
        let mut game = Game::new();
        let id = spawn_bomber(&mut game, v2(100.0, 100.0));

        let before = game.rng.clone();
        update(&mut game, id);

        // `not target` short-circuits the roll, retargeting finds nothing, and
        // the target is unchanged so no approach is rolled either
        assert_eq!(before, game.rng);
    }
}
