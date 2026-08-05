//! `components/targeting.lua`: how the AI picks something to shoot at.
//!
//! Both queries walk `game.actors.get(tag)`, which is in spawn order, and both
//! skip anything offscreen -- ships chase each other out of the play area and
//! would otherwise wander off after a target they can never reach.

use crate::constants::{SCREEN_BOTTOM, SCREEN_LEFT, SCREEN_RIGHT, SCREEN_TOP};
use crate::game::Game;
use crate::v2::V2;
use crate::world::ActorId;

pub fn on_screen(pos: V2) -> bool {
    pos.x >= SCREEN_LEFT && pos.x <= SCREEN_RIGHT && pos.y >= SCREEN_BOTTOM && pos.y <= SCREEN_TOP
}

/// The nearest enemy of the given blueprint, or `None`.
///
/// Ties go to the earlier-spawned ship, since the comparison is strictly less
/// than.
pub fn nearest_of_type(game: &Game, source: ActorId, ship_type: &str) -> Option<ActorId> {
    let source_pos = game.world.get(source).transform.expect("transform").pos;
    let source_player = game.player_of(source);

    let mut closest: Option<(ActorId, f64)> = None;

    for &candidate in game.world.tagged(ship_type) {
        let pos = game.world.get(candidate).transform.expect("transform").pos;
        let distance = (pos - source_pos).sqrmag();

        if game.player_of(candidate) == source_player || !on_screen(pos) {
            continue;
        }
        if closest.is_none_or(|(_, best)| distance < best) {
            closest = Some((candidate, distance));
        }
    }

    closest.map(|(id, _)| id)
}

/// A random enemy of the given blueprint within `range`, or `None`.
///
/// Takes one draw, and only when there is at least one candidate -- an empty
/// list returns early without touching the stream.
pub fn type_in_range(
    game: &mut Game,
    source: ActorId,
    ship_type: &str,
    range: f64,
) -> Option<ActorId> {
    let source_pos = game.world.get(source).transform.expect("transform").pos;
    let source_player = game.player_of(source);
    let range_squared = range * range;

    let in_range: Vec<ActorId> = game
        .world
        .tagged(ship_type)
        .iter()
        .copied()
        .filter(|&candidate| {
            let pos = game.world.get(candidate).transform.expect("transform").pos;
            game.player_of(candidate) != source_player
                && on_screen(pos)
                && (pos - source_pos).sqrmag() < range_squared
        })
        .collect();

    if in_range.is_empty() {
        return None;
    }

    let index = game.rng.next_range(1, in_range.len() as i64) as usize;
    Some(in_range[index - 1])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scripts::ScriptKind;
    use crate::scripts::transform::Transform;
    use crate::v2::v2;
    use crate::world::Actor;

    fn spawn(game: &mut Game, name: &'static str, pos: V2, player: usize) -> ActorId {
        game.world.spawn(Actor {
            scripts: vec![ScriptKind::Transform],
            transform: Some(Transform::at(pos)),
            player: Some(player),
            ..Actor::new(name)
        })
    }

    #[test]
    fn picks_the_nearest_enemy() {
        let mut game = Game::new();
        let source = spawn(&mut game, "fighter", v2(100.0, 100.0), 1);
        spawn(&mut game, "bomber", v2(500.0, 100.0), 2);
        let near = spawn(&mut game, "bomber", v2(150.0, 100.0), 2);

        assert_eq!(nearest_of_type(&game, source, "bomber"), Some(near));
    }

    #[test]
    fn ignores_friendly_ships() {
        let mut game = Game::new();
        let source = spawn(&mut game, "fighter", v2(100.0, 100.0), 1);
        spawn(&mut game, "bomber", v2(110.0, 100.0), 1);

        assert_eq!(nearest_of_type(&game, source, "bomber"), None);
    }

    #[test]
    fn ignores_offscreen_ships() {
        let mut game = Game::new();
        let source = spawn(&mut game, "fighter", v2(100.0, 100.0), 1);
        spawn(&mut game, "bomber", v2(-50.0, 100.0), 2);

        assert_eq!(nearest_of_type(&game, source, "bomber"), None);
    }

    #[test]
    fn ties_go_to_the_earlier_spawn() {
        let mut game = Game::new();
        let source = spawn(&mut game, "fighter", v2(100.0, 100.0), 1);
        let first = spawn(&mut game, "bomber", v2(200.0, 100.0), 2);
        spawn(&mut game, "bomber", v2(0.0, 100.0), 2);

        assert_eq!(nearest_of_type(&game, source, "bomber"), Some(first));
    }

    #[test]
    fn range_query_finds_only_what_is_close_enough() {
        let mut game = Game::new();
        let source = spawn(&mut game, "fighter", v2(100.0, 100.0), 1);
        let near = spawn(&mut game, "bomber", v2(150.0, 100.0), 2);
        spawn(&mut game, "bomber", v2(900.0, 100.0), 2);

        assert_eq!(
            type_in_range(&mut game, source, "bomber", 100.0),
            Some(near)
        );
    }

    #[test]
    fn an_empty_range_query_takes_no_draws() {
        let mut game = Game::new();
        let source = spawn(&mut game, "fighter", v2(100.0, 100.0), 1);

        let before = game.rng.clone();
        assert_eq!(type_in_range(&mut game, source, "bomber", 100.0), None);
        assert_eq!(before, game.rng, "an empty query must not touch the stream");
    }

    #[test]
    fn a_nonempty_range_query_takes_exactly_one_draw() {
        let mut game = Game::new();
        let source = spawn(&mut game, "fighter", v2(100.0, 100.0), 1);
        spawn(&mut game, "bomber", v2(150.0, 100.0), 2);

        let mut expected = game.rng.clone();
        expected.next_u64();

        type_in_range(&mut game, source, "bomber", 100.0);
        assert_eq!(expected, game.rng);
    }
}
