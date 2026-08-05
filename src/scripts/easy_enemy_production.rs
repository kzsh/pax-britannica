//! `scripts/easy_enemy_production.lua`: the CPU opponent.
//!
//! There is no strategy here at all. The CPU picks one of four canned build
//! orders at random, works through it, and picks another; for each entry it
//! works out how long to wait for the resources and how long to hold the button
//! to reach that unit's quadrant, then does exactly that. It is deliberately
//! beatable -- it also takes a 20% cut to its income on creation.
//!
//! **Three draws on creation and two per action**, in this order: the build
//! order, then the hold time, then the wait time. Both of the latter are
//! `math.random() * 60`, a random extra second so the CPU does not play in
//! lockstep with itself.

use crate::game::Game;
use crate::rng::LuaRng;
use crate::scripts::production::{BUILDING_SPEED, UnitType};
use crate::scripts::resources::Resources;
use crate::world::ActorId;

/// The four canned build orders. Fighters, mostly, with something bigger at the
/// end of each.
const SCRIPTED_ACTIONS: [&[UnitType]; 4] = [
    &[
        UnitType::Fighter,
        UnitType::Fighter,
        UnitType::Fighter,
        UnitType::Fighter,
        UnitType::Bomber,
    ],
    &[
        UnitType::Fighter,
        UnitType::Fighter,
        UnitType::Fighter,
        UnitType::Bomber,
    ],
    &[UnitType::Fighter, UnitType::Fighter, UnitType::Bomber],
    &[
        UnitType::Fighter,
        UnitType::Fighter,
        UnitType::Fighter,
        UnitType::Frigate,
    ],
];

/// What the CPU's harvest rate is multiplied by on creation.
const HANDICAP: f64 = 0.8;

/// The random slack added to both the wait and the hold, in frames.
const JITTER_FRAMES: f64 = 60.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EasyEnemyProduction {
    /// 1-based into [`SCRIPTED_ACTIONS`], as in the Lua.
    script_index: usize,
    /// 1-based into the chosen build order.
    action_index: usize,
    accumulated_frames: f64,
    frames_to_wait: f64,
    frames_to_hold: f64,
}

impl EasyEnemyProduction {
    /// The script's chunk body: handicap the income, pick a build order, and
    /// line up the first action. Runs at spawn, so it takes its three draws
    /// before the factory's first update.
    pub fn new(rng: &mut LuaRng, resources: &mut Resources) -> Self {
        resources.harvest_rate *= HANDICAP;

        let mut self_ = Self {
            script_index: 0,
            action_index: 0,
            accumulated_frames: 0.0,
            frames_to_wait: 0.0,
            frames_to_hold: 0.0,
        };
        self_.next_script(rng);
        self_.next_action(rng, *resources);
        self_
    }

    fn next_script(&mut self, rng: &mut LuaRng) {
        self.script_index = rng.next_range(1, SCRIPTED_ACTIONS.len() as i64) as usize;
    }

    /// Advance to the next entry of the build order, wrapping onto a freshly
    /// chosen order, and work out the timings for it.
    fn next_action(&mut self, rng: &mut LuaRng, resources: Resources) {
        self.accumulated_frames = 0.0;

        self.action_index += 1;
        if self.action_index > SCRIPTED_ACTIONS[self.script_index - 1].len() {
            self.action_index = 1;
            self.next_script(rng);
        }

        let cost = self.current_unit().cost();

        // the dial starts at a fighter's worth the instant the button goes
        // down, so only the difference has to be held for; +2 to overshoot the
        // strictly-greater-than boundary
        self.frames_to_hold = (cost - UnitType::Fighter.cost()) / BUILDING_SPEED
            + 2.0
            + rng.next_f64() * JITTER_FRAMES;

        let missing_resources = (cost - resources.amount) / resources.harvest_rate;
        self.frames_to_wait = (missing_resources + 1.0).max(0.0) + rng.next_f64() * JITTER_FRAMES;
    }

    fn current_unit(&self) -> UnitType {
        SCRIPTED_ACTIONS[self.script_index - 1][self.action_index - 1]
    }
}

/// Hold the button over the scripted window, and move on once it closes.
pub fn update(game: &mut Game, id: ActorId) {
    let player = game.player_of(id);

    // no one left to fight: stop building and idle. Note this counts factories
    // that are mid-death -- they stay in the tag index until the cull
    let found_enemy = game
        .world
        .tagged("factory")
        .iter()
        .any(|&other| game.world.get(other).player.is_some_and(|p| p != player));
    if !found_enemy {
        production_mut(game, id).button_held = false;
        return;
    }

    let mut state = *ai(game, id);
    state.accumulated_frames += 1.0;

    let window_ends = state.frames_to_hold + state.frames_to_wait;
    let button_held =
        state.accumulated_frames > state.frames_to_wait && state.accumulated_frames < window_ends;

    if state.accumulated_frames > window_ends {
        let resources = game
            .world
            .get(id)
            .resources
            .expect("easy_enemy_production needs the resources script");
        state.next_action(&mut game.rng, resources);
    }

    *ai_mut(game, id) = state;
    production_mut(game, id).button_held = button_held;
}

fn ai(game: &Game, id: ActorId) -> &EasyEnemyProduction {
    game.world
        .get(id)
        .easy_enemy_production
        .as_ref()
        .expect("easy_enemy_production script")
}

fn ai_mut(game: &mut Game, id: ActorId) -> &mut EasyEnemyProduction {
    game.world
        .get_mut(id)
        .easy_enemy_production
        .as_mut()
        .expect("easy_enemy_production script")
}

fn production_mut(game: &mut Game, id: ActorId) -> &mut crate::scripts::production::Production {
    game.world
        .get_mut(id)
        .production
        .as_mut()
        .expect("easy_enemy_production needs the production script")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blueprints;
    use crate::scripts::production::Production;
    use crate::v2::{V2, v2};

    fn spawn_cpu(game: &mut Game, player: usize) -> ActorId {
        let actor = blueprints::easy_enemy_factory(&mut game.rng, player, v2(500.0, 400.0), V2::I);
        game.world.spawn(actor)
    }

    #[test]
    fn creation_costs_exactly_three_draws() {
        // build order, hold jitter, wait jitter -- and nothing else, or every
        // frame after the CPU joins is off
        let mut game = Game::new();
        let mut expected = game.rng.clone();

        spawn_cpu(&mut game, 2);

        for _ in 0..3 {
            expected.next_u64();
        }
        assert_eq!(expected, game.rng);
    }

    // The literals below are the interpreter's own `%.17g` output, kept exactly
    // as it printed them so they can be checked against a fresh Lua run by eye.
    // Clippy would rather they were written to f64's shortest round-trip form;
    // both parse to the same bits, and provenance is worth more here.
    #[expect(clippy::excessive_precision)]
    #[test]
    fn creation_matches_the_lua_interpreter_bit_for_bit() {
        // Taken from lua5.4 itself after math.randomseed(1), replaying the
        // chunk body's three draws in order. Derived numbers would only prove
        // the port agrees with my reading of it; these prove it agrees with the
        // interpreter that produced traces/golden.txt.
        let mut game = Game::new();
        let id = spawn_cpu(&mut game, 2);
        let state = ai(&game, id);

        assert_eq!(state.script_index, 2);
        assert_eq!(state.action_index, 1);
        assert_eq!(state.frames_to_hold, 61.194650386074542);
        assert_eq!(state.frames_to_wait, 4.7598431754015618);
        assert_eq!(
            game.world.get(id).resources.unwrap().harvest_rate,
            0.60000000000000009
        );
    }

    #[test]
    fn the_cpu_takes_a_cut_to_its_income() {
        let mut game = Game::new();
        let id = spawn_cpu(&mut game, 2);
        assert_eq!(
            game.world.get(id).resources.unwrap().harvest_rate,
            Resources::default().harvest_rate * HANDICAP
        );
    }

    #[test]
    fn an_idle_update_takes_no_draws() {
        let mut game = Game::new();
        spawn_cpu(&mut game, 1);
        let id = spawn_cpu(&mut game, 2);

        let before = game.rng.clone();
        update(&mut game, id);

        assert_eq!(before, game.rng, "only rolling a new action costs draws");
    }

    #[test]
    fn a_new_action_costs_two_draws() {
        let mut game = Game::new();
        spawn_cpu(&mut game, 1);
        let id = spawn_cpu(&mut game, 2);
        // park the run right at the end of the current action's window
        let state = ai_mut(&mut game, id);
        state.accumulated_frames = state.frames_to_hold + state.frames_to_wait;

        let mut expected = game.rng.clone();
        update(&mut game, id);
        expected.next_u64();
        expected.next_u64();

        assert_eq!(expected, game.rng);
        assert_eq!(ai(&game, id).accumulated_frames, 0.0);
        assert_eq!(ai(&game, id).action_index, 2);
    }

    #[test]
    fn the_button_is_held_only_inside_the_scripted_window() {
        let mut game = Game::new();
        spawn_cpu(&mut game, 1);
        let id = spawn_cpu(&mut game, 2);

        let (wait, hold) = {
            let state = ai(&game, id);
            (state.frames_to_wait, state.frames_to_hold)
        };

        ai_mut(&mut game, id).accumulated_frames = wait - 1.0;
        update(&mut game, id);
        assert!(!game.world.get(id).production.unwrap().button_held);

        ai_mut(&mut game, id).accumulated_frames = wait;
        update(&mut game, id);
        assert!(game.world.get(id).production.unwrap().button_held);

        ai_mut(&mut game, id).accumulated_frames = wait + hold;
        update(&mut game, id);
        assert!(!game.world.get(id).production.unwrap().button_held);
    }

    #[test]
    fn a_cpu_with_no_opponent_lets_go_of_the_button() {
        let mut game = Game::new();
        let id = spawn_cpu(&mut game, 2);
        production_mut(&mut game, id).button_held = true;

        update(&mut game, id);

        assert!(!game.world.get(id).production.unwrap().button_held);
        assert_eq!(
            ai(&game, id).accumulated_frames,
            0.0,
            "the clock does not run while there is nothing to build for"
        );
    }

    #[test]
    fn a_build_order_wraps_onto_a_freshly_chosen_one() {
        let mut game = Game::new();
        let mut resources = Resources::default();
        let mut state = EasyEnemyProduction::new(&mut game.rng, &mut resources);

        let order_length = SCRIPTED_ACTIONS[state.script_index - 1].len();
        for _ in 1..order_length {
            state.next_action(&mut game.rng, resources);
        }
        assert_eq!(state.action_index, order_length);

        state.next_action(&mut game.rng, resources);
        assert_eq!(state.action_index, 1);
    }

    #[test]
    fn the_hold_reaches_the_unit_it_was_computed_for() {
        // the timings are the CPU's whole competence: hold this long and the
        // dial must land in the right quadrant
        let mut game = Game::new();
        let mut resources = Resources {
            amount: 10_000.0,
            ..Resources::default()
        };
        let state = EasyEnemyProduction::new(&mut game.rng, &mut resources);

        let mut production = Production {
            button_held: true,
            ..Production::default()
        };
        // frames_to_hold carries up to a second of jitter; the floor is what
        // the arithmetic guarantees
        let frames = (state.frames_to_hold - JITTER_FRAMES).floor() as i64;
        for _ in 0..frames {
            if production.potential_cost == 0.0 {
                production.potential_cost = UnitType::Fighter.cost();
            }
            production.potential_cost += BUILDING_SPEED;
        }

        assert_eq!(
            UnitType::for_cost(production.potential_cost),
            Some(state.current_unit())
        );
    }
}
