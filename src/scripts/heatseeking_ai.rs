//! `scripts/heatseeking_ai.lua`: a missile that leads its target.

use crate::game::Game;
use crate::scripts::ship;
use crate::targeting;
use crate::v2::V2;
use crate::world::ActorId;

/// Five seconds, then it gives up and counts as a miss.
const MAX_LIFETIME: i32 = 60 * 5;
/// How far a missile will look for something worth chasing.
const SEEK_RANGE: f64 = 600.0;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HeatseekingAi {
    pub age: i32,
    pub target: Option<ActorId>,
}

/// Something nearby at random, else the nearest fighter, else the nearest
/// factory.
///
/// The three range queries each take one draw when they find candidates, so the
/// cost here varies with the battlefield.
pub fn retarget(game: &mut Game, id: ActorId) {
    let target = ["fighter", "bomber", "frigate"]
        .into_iter()
        .find_map(|kind| targeting::type_in_range(game, id, kind, SEEK_RANGE))
        .or_else(|| targeting::nearest_of_type(game, id, "fighter"))
        .or_else(|| targeting::nearest_of_type(game, id, "factory"));

    ai_mut(game, id).target = target;
}

/// Where to aim so as to meet the target rather than chase it.
///
/// Faithful to a quirk in the original: the guard tests the missile's own
/// velocity against the bearing, but the division uses the *relative* velocity.
/// When the two disagree this yields an infinite or negative time, which the
/// `max(0, ..)` then flattens to aiming straight at the target. Rewriting it to
/// be "correct" would change the flight paths.
fn predict(game: &Game, id: ActorId, target: ActorId) -> V2 {
    let velocity = game.world.get(id).ship.as_ref().expect("ship").velocity;
    let target_velocity = game
        .world
        .get(target)
        .ship
        .as_ref()
        .expect("targets always have a ship")
        .velocity;
    let target_pos = game.world.get(target).transform.expect("transform").pos;
    let pos = game.world.get(id).transform.expect("transform").pos;

    let relative_velocity = velocity - target_velocity;
    let to_target = target_pos - pos;

    if velocity.dot(to_target) != 0.0 {
        let time_to_target = to_target.sqrmag() / relative_velocity.dot(to_target);
        target_pos - relative_velocity * time_to_target.max(0.0)
    } else {
        target_pos
    }
}

pub fn update(game: &mut Game, id: ActorId) {
    ai_mut(game, id).age += 1;
    let ai = *ai(game, id);

    if ai.target.is_none() || ai.age > MAX_LIFETIME {
        self_destruct(game, id);
        return;
    }

    let target = ai.target.expect("checked above");
    if game.world.is_dead(target) {
        retarget(game, id);
        return;
    }

    let aim = predict(game, id, target);
    ship::go_towards(game, id, aim, true);
}

/// Expiring counts against the firing frigate's accuracy, same as flying off
/// the screen does for a dumb bullet.
fn self_destruct(game: &mut Game, id: ActorId) {
    game.log.record_miss(game.world.get(id).blueprint);
    game.world.kill(id);
}

fn ai(game: &Game, id: ActorId) -> &HeatseekingAi {
    game.world
        .get(id)
        .heatseeking_ai
        .as_ref()
        .expect("heatseeking_ai")
}

fn ai_mut(game: &mut Game, id: ActorId) -> &mut HeatseekingAi {
    game.world
        .get_mut(id)
        .heatseeking_ai
        .as_mut()
        .expect("heatseeking_ai")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blueprints;
    use crate::v2::v2;

    fn spawn_missile(game: &mut Game, pos: V2) -> ActorId {
        game.world.spawn(blueprints::missile(1, pos, v2(1.0, 0.0)))
    }

    #[test]
    fn a_missile_with_no_target_destroys_itself_and_counts_as_a_miss() {
        let mut game = Game::new();
        let id = spawn_missile(&mut game, v2(100.0, 100.0));

        update(&mut game, id);

        assert!(game.world.is_dead(id));
        assert_eq!(game.log.accuracy_misses.frigate, 1.0);
    }

    #[test]
    fn a_missile_expires_after_five_seconds() {
        let mut game = Game::new();
        let id = spawn_missile(&mut game, v2(100.0, 100.0));
        game.world
            .spawn(blueprints::fighter(2, v2(200.0, 100.0), V2::I));
        retarget(&mut game, id);

        for _ in 0..MAX_LIFETIME {
            update(&mut game, id);
            assert!(!game.world.is_dead(id));
        }
        update(&mut game, id);

        assert!(game.world.is_dead(id));
    }

    #[test]
    fn a_missile_retargets_when_its_quarry_dies() {
        let mut game = Game::new();
        let id = spawn_missile(&mut game, v2(100.0, 100.0));
        let first = game
            .world
            .spawn(blueprints::fighter(2, v2(200.0, 100.0), V2::I));
        retarget(&mut game, id);
        assert_eq!(ai(&game, id).target, Some(first));

        game.world.kill(first);
        let second = game
            .world
            .spawn(blueprints::fighter(2, v2(300.0, 100.0), V2::I));
        update(&mut game, id);

        assert_eq!(ai(&game, id).target, Some(second));
        assert!(!game.world.is_dead(id));
    }

    #[test]
    fn a_missile_steers_toward_its_target() {
        let mut game = Game::new();
        let id = spawn_missile(&mut game, v2(100.0, 100.0));
        game.world
            .spawn(blueprints::fighter(2, v2(100.0, 400.0), V2::I));
        retarget(&mut game, id);

        update(&mut game, id);

        // it started facing +x and the target is straight up
        assert!(game.world.get(id).transform.unwrap().facing.y > 0.0);
    }

    #[test]
    fn prediction_leads_a_crossing_target() {
        let mut game = Game::new();
        let id = spawn_missile(&mut game, v2(0.0, 0.0));
        let target = game
            .world
            .spawn(blueprints::fighter(2, v2(200.0, 0.0), V2::I));
        game.world.get_mut(target).ship.as_mut().unwrap().velocity = v2(0.0, 10.0);

        let aim = predict(&game, id, target);
        let target_pos = game.world.get(target).transform.unwrap().pos;

        assert_ne!(aim, target_pos, "the aim point should lead the target");
    }
}
