//! `the_game.lua`: building a scene, and the kernel's one-frame loop.
//!
//! The Lua's `init` runs at the top of the scene's first update and creates
//! actors in a fixed order: one per engine component that wants a callback, then
//! the background effects, the scene machine, and the particle emitters. The
//! Lua's `background` actor has no counterpart here: the sea is a gradient the
//! renderer draws, not a sprite an actor wears. **That order is the frame's update and draw order**, so it is
//! reproduced literally here even for components whose actors do nothing outside
//! a window.
//!
//! Building the scene eagerly rather than at the first update is safe only
//! because none of it takes a draw off the shared random stream -- checked, not
//! assumed, by [`tests::building_a_scene_takes_no_draws`]. If that ever stops
//! being true the construction has to move back to the top of the first update,
//! because a restart happens mid-frame and the old scene still has a draw phase
//! to run.

use crate::blueprints;
use crate::game::Game;
use crate::rng::LuaRng;
use crate::world::{self, Actor};

/// `game.actors.new_generic(name, ...)`: a one-script actor created by an engine
/// component. In headless play these do nothing -- they poll a keyboard, set up
/// a viewport, tidy up debug tracers -- but they hold places in the spawn order
/// and they appear in the golden trace, so they are spawned rather than skipped.
fn generic(game: &mut Game, name: &'static str) {
    game.world.spawn(Actor::new(name));
}

/// A fresh scene, inheriting the random stream.
///
/// The stream is *not* reseeded on a restart: `the_game.lua` calls
/// `math.randomseed()`, and the test harness neuters that call, so the second
/// match carries on from wherever the first one left the generator. Everything
/// else -- the world, the components, the log -- is new.
pub fn make(rng: LuaRng) -> Game {
    let mut game = Game::empty(rng);

    generic(&mut game, "exit_handler");
    generic(&mut game, "key_monitor");
    generic(&mut game, "opengl_setup");
    game.world.spawn(blueprints::collision_checker());
    generic(&mut game, "tracer_cleanup");
    game.world.spawn(blueprints::log_timer());
    generic(&mut game, "fast_forward");
    game.world.spawn(blueprints::the_one_button());

    game.world.spawn(blueprints::background_fx());
    game.world.spawn(blueprints::game_flow());
    game.world.spawn(blueprints::particle_emitters());

    game
}

/// One turn of the kernel's loop: update, draw, then the scene switch that
/// `scripts/game_flow.lua` may have asked for part-way through the update.
///
/// The switch is deferred to the end of the frame because that is what
/// `dokidoki/kernel.lua` does: `switch_scene` only records the new scene, and
/// the old one still finishes its update and its draw. Sixty stacked game-over
/// fades all call it; only the first has any effect, because replacing the game
/// takes the rest with it.
/// Returns whether this frame ended with the scene being replaced.
pub fn step(game: &mut Game) -> bool {
    world::update(game);
    world::draw(game);

    if game.scene_switch_requested {
        *game = make(game.rng.clone());
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scripts::game_flow::State;
    use crate::v2::V2;

    fn blueprints_in_order(game: &Game) -> Vec<&'static str> {
        game.world
            .order()
            .iter()
            .map(|&id| game.world.get(id).blueprint)
            .collect()
    }

    #[test]
    fn the_frame_that_switches_scenes_still_runs_its_draw_phase() {
        // `scripts/factory_damage.lua` takes a draw off the shared stream per
        // factory per drawn frame, and the stream is the one thing a restart
        // carries over -- so cutting the dying scene's last draw short would
        // desynchronise every frame of the next match. Compared against the same
        // frame without the switch rather than against a predicted draw count;
        // what it pins is the ordering, not that a draw happens at all.
        let build = || {
            let mut game = make(LuaRng::new(1, 0));
            game.world
                .spawn(crate::blueprints::player_factory(1, V2::ZERO, V2::I));
            game
        };

        let mut switching = build();
        switching.scene_switch_requested = true;
        step(&mut switching);

        let mut carrying_on = build();
        step(&mut carrying_on);

        assert_eq!(switching.rng, carrying_on.rng);
    }

    #[test]
    fn the_scene_is_built_in_the_order_the_interpreter_builds_it() {
        // from `lua5.4` with the harness recording every game.actors.new call,
        // less the `background` actor: its sprite is gone, and the sea is drawn
        // as a gradient by the renderer instead
        let game = make(LuaRng::new(1, 0));
        assert_eq!(
            blueprints_in_order(&game),
            [
                "exit_handler",
                "key_monitor",
                "opengl_setup",
                "collision",
                "tracer_cleanup",
                "log",
                "fast_forward",
                "the_one_button",
                "background_fx",
                "game_flow",
                "particles",
            ]
        );
    }

    #[test]
    fn particles_are_aged_once_a_frame() {
        // `components/particles.lua`'s actor does this; without it a particle
        // never moves, never grows and never dies, it just sits there until the
        // ring buffer wraps over it
        let mut game = make(LuaRng::new(1, 0));
        game.particles
            .explode_small(&mut game.rng.clone(), V2::ZERO);
        let before = game.particles.live_count();
        assert!(before > 0);

        for _ in 0..11 {
            step(&mut game);
        }

        assert!(
            game.particles.live_count() < before,
            "the explosion emitters live 10 frames, so they should be gone"
        );
    }

    #[test]
    fn building_a_scene_takes_no_draws() {
        let rng = LuaRng::new(1, 0);
        let game = make(rng.clone());
        assert_eq!(game.rng, rng);
    }

    #[test]
    fn the_first_frame_puts_the_menu_up() {
        let mut game = make(LuaRng::new(1, 0));
        step(&mut game);

        assert_eq!(game.world.tagged("selection_factory").len(), 4);
        assert_eq!(game.world.tagged("splash").len(), 1);
        assert_eq!(game.world.tagged("fade").len(), 1);
    }

    #[test]
    fn input_reaches_the_selectors_through_the_latch() {
        let mut game = make(LuaRng::new(1, 0));
        step(&mut game);

        game.the_one_button.keys[0] = true;
        step(&mut game);

        let selector = game.world.tagged("selection_factory")[0];
        assert!(
            game.world.get(selector).selector.unwrap().picked,
            "the_one_button latches in update_setup, before the selectors update"
        );
    }

    #[test]
    fn a_restart_replaces_the_world_but_not_the_random_stream() {
        let mut game = make(LuaRng::new(1, 0));
        step(&mut game);
        let flow = game.world.tagged("game_flow")[0];
        game.world.get_mut(flow).game_flow.as_mut().unwrap().state = Some(State::InGame);

        // the match has no factories at all, so the timer starts immediately:
        // 300 frames of grace, then a fade-out that takes 61 more to land
        for _ in 0..400 {
            step(&mut game);
        }

        assert!(
            !game.scene_switch_requested,
            "the flag goes with the scene that raised it"
        );
        assert_eq!(
            game.world.tagged("selection_factory").len(),
            4,
            "the new scene should be back on the title screen"
        );
        assert!(
            game.log.frame_count < 400,
            "the log is replaced with the game, not carried over"
        );
    }
}
