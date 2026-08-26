//! `scripts/game_flow.lua`: the scene state machine.
//!
//! ```text
//!   init ──► player_select ──(someone joined)──► nil ──(countdown ends)──►
//!            spawn menu,      spawn countdown         start_game: fade out,
//!            fade in                                  then clear the menu,
//!                                                     place the factories,
//!                                                     fade in ──► in_game
//!
//!   in_game ──(fewer than 2 factories for 300 frames)──► fade out ──►
//!             print stats, build a whole new game
//! ```
//!
//! The Lua drives all of that through closures: the fade calls back on
//! completion, each selector calls the countdown's `reset_counter`, and the
//! countdown calls `start_game`. None of those closures capture anything a Rust
//! port could hold onto -- they close over `game_flow`'s chunk locals, which are
//! this struct.
//!
//! **The port names the callbacks instead of boxing them.** Each of the three is
//! a small `enum` ([`FadeCallback`], [`CountdownCallback`],
//! [`SelectorCallback`](crate::scripts::selector::SelectorCallback)) whose
//! payload is an [`ActorId`] and, in one case, the list of joined players.
//! Boxed `FnOnce`s would have been closer to the Lua's shape and worse in every
//! way that matters here: they would make the scripts non-comparable and
//! non-printable, which the differential trace needs; they would need interior
//! mutability to touch the game they were spawned from; and they would hide the
//! fact that there are exactly three callback sites in the whole game. The enums
//! make the machine above readable from the types alone.
//!
//! `state` is a chunk-level local, and it is in the golden trace: `nil` while a
//! transition is in flight is a distinct state from all three of the named ones.

use crate::blueprints;
use crate::constants::CENTER;
use crate::game::Game;
use crate::scripts::countdown::CountdownCallback;
use crate::scripts::fade::FadeCallback;
use crate::scripts::selector::SelectorCallback;
use crate::v2::{V2, v2};
use crate::world::ActorId;

use std::f64::consts::PI;

/// How far out from [`CENTER`] the factories start.
const RADIUS: f64 = 300.0;

/// Spacing of the four selectors across the middle of the menu screen, and how
/// far below it the countdown sits.
const SELECTOR_SPACING: f64 = 130.0;
const SELECTOR_OFFSET: V2 = v2(-75.0, 0.0);
const COUNTDOWN_DROP: f64 = 200.0;

/// Frames a match hangs on after it is down to one factory, before restarting.
const GAME_OVER_FRAMES: i64 = 300;

/// `state` in the Lua. `None` is its `nil`: a transition is in flight and
/// `update` has nothing to do until a callback moves it on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Init,
    PlayerSelect,
    InGame,
}

impl State {
    /// The string the Lua holds, which is what the trace compares.
    pub fn name(self) -> &'static str {
        match self {
            State::Init => "init",
            State::PlayerSelect => "player_select",
            State::InGame => "in_game",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GameFlow {
    pub state: Option<State>,
    /// The four menu factories, in player order, so index + 1 is the player
    /// number. Kept after they die, exactly as the Lua's table is.
    pub selectors: Vec<ActorId>,
    pub splash: Option<ActorId>,
    pub game_over_timer: i64,
}

impl Default for GameFlow {
    fn default() -> Self {
        Self {
            state: Some(State::Init),
            selectors: Vec::new(),
            splash: None,
            game_over_timer: 0,
        }
    }
}

/// Where the factories start, by player count.
///
/// `POSITIONS` in the Lua, whose first slot is `false` and would error if it
/// were ever indexed; a lone picker always gains a CPU opponent before this is
/// reached, so 2..=4 is the whole domain.
///
/// The arithmetic is kept in the original's exact shape (`CENTER + unit(a) *
/// RADIUS`, and `pi * 5 / 6` rather than a folded constant) because these
/// positions are the first thing every match's simulation is built on.
fn start_positions(players: usize) -> Vec<V2> {
    let at = |angle: f64| CENTER + V2::unit(angle) * RADIUS;
    match players {
        2 => vec![at(PI), at(0.0)],
        3 => vec![at(PI * 5.0 / 6.0), at(PI / 6.0), at(PI * 9.0 / 6.0)],
        4 => vec![
            at(PI * 3.0 / 4.0),
            at(PI / 4.0),
            at(PI * 5.0 / 4.0),
            at(PI * 7.0 / 4.0),
        ],
        _ => Vec::new(),
    }
}

fn flow(game: &Game, id: ActorId) -> GameFlow {
    game.world
        .get(id)
        .game_flow
        .as_ref()
        .expect("game_flow script")
        .clone()
}

fn set_state(game: &mut Game, id: ActorId, state: Option<State>) {
    game.world
        .get_mut(id)
        .game_flow
        .as_mut()
        .expect("game_flow script")
        .state = state;
}

pub fn update(game: &mut Game, id: ActorId) {
    match flow(game, id).state {
        Some(State::Init) => init(game, id),
        Some(State::PlayerSelect) => player_select(game, id),
        Some(State::InGame) => in_game(game, id),
        None => {}
    }
}

/// Build the menu: four selectors left to right, the title, and a fade in.
fn init(game: &mut Game, id: ActorId) {
    let dist = SELECTOR_SPACING;
    let offsets = [
        -dist / 2.0 - dist,
        -dist / 2.0,
        dist / 2.0,
        dist + dist / 2.0,
    ];

    let selectors: Vec<ActorId> = offsets
        .into_iter()
        .enumerate()
        .map(|(index, x)| {
            let pos = CENTER + SELECTOR_OFFSET + v2(x, 0.0);
            game.world
                .spawn(blueprints::selection_factory(index + 1, pos))
        })
        .collect();

    let splash = game.world.spawn(blueprints::splash());
    game.world.spawn(blueprints::fade_in());

    let game_flow = game
        .world
        .get_mut(id)
        .game_flow
        .as_mut()
        .expect("game_flow script");
    game_flow.selectors = selectors;
    game_flow.splash = Some(splash);
    game_flow.state = Some(State::PlayerSelect);
}

/// Wait for the first player to join, then put five seconds on the clock and
/// hand every selector the means to reset it.
fn player_select(game: &mut Game, id: ActorId) {
    let selectors = flow(game, id).selectors;

    let anyone_joined = selectors.iter().any(|&s| {
        game.world
            .get(s)
            .selector
            .as_ref()
            .is_some_and(|selector| selector.picked)
    });
    if !anyone_joined {
        return;
    }

    let countdown = game.world.spawn(blueprints::countdown(
        CENTER - v2(0.0, COUNTDOWN_DROP),
        CountdownCallback::StartGame(id),
    ));
    for selector in selectors {
        game.world
            .get_mut(selector)
            .selector
            .as_mut()
            .expect("a selector actor carries the selector script")
            .callback = SelectorCallback::ResetCountdown(countdown);
    }

    set_state(game, id, None);
}

/// A match with fewer than two factories left is over, but not immediately:
/// the loser's wreck spends 300 frames exploding first.
fn in_game(game: &mut Game, id: ActorId) {
    if game.world.tagged("factory").len() >= 2 {
        return;
    }

    let timer = flow(game, id).game_over_timer + 1;
    game.world
        .get_mut(id)
        .game_flow
        .as_mut()
        .expect("game_flow script")
        .game_over_timer = timer;

    // No guard and no state change, so this spawns a *fresh* fade-out every
    // frame from here on -- sixty of them before the first one lands. Copied
    // deliberately: they are all in the trace, and the first to finish switches
    // the scene out from under the rest.
    if timer > GAME_OVER_FRAMES {
        game.world
            .spawn(blueprints::fade_out(FadeCallback::Restart));
    }
}

/// `start_game`: the countdown ran out. Freeze the roster and fade to black.
///
/// Does nothing at all if nobody joined, which cannot happen -- the countdown
/// only exists because somebody did -- but the guard is the Lua's and costs
/// nothing.
pub fn start_game(game: &mut Game, id: ActorId) {
    let selectors = flow(game, id).selectors;

    // the Lua collects the *index* into `selectors`, which is the player number
    // by construction
    let players: Vec<usize> = selectors
        .iter()
        .enumerate()
        .filter(|&(_, &s)| {
            game.world
                .get(s)
                .selector
                .as_ref()
                .is_some_and(|selector| selector.picked)
        })
        .map(|(index, _)| index + 1)
        .collect();

    if players.is_empty() {
        return;
    }

    game.world
        .spawn(blueprints::fade_out(FadeCallback::BeginMatch {
            game_flow: id,
            players,
        }));
    set_state(game, id, None);
}

/// The fade-out's callback: tear the menu down and put the factories in the
/// water. Spawn order here is z-order, update order and AI targeting order, so
/// it is the literal order of the Lua's loop.
pub fn begin_match(game: &mut Game, id: ActorId, players: &[usize]) {
    let flow_state = flow(game, id);
    for selector in &flow_state.selectors {
        game.world.kill(*selector);
    }
    if let Some(splash) = flow_state.splash {
        game.world.kill(splash);
    }

    // one human gets an opponent: whichever of players 1 and 2 they are not
    let mut players = players.to_vec();
    let cpu_player = if players.len() == 1 {
        let cpu = if players[0] == 1 { 2 } else { 1 };
        players.push(cpu);
        Some(cpu)
    } else {
        None
    };

    let positions = start_positions(players.len());
    for (&player, &pos) in players.iter().zip(positions.iter()) {
        let facing = (pos - CENTER).rotate90().norm();
        let actor = if Some(player) == cpu_player {
            blueprints::easy_enemy_factory(&mut game.rng, player, pos, facing)
        } else {
            blueprints::player_factory(player, pos, facing)
        };
        game.world.spawn(actor);
    }

    set_state(game, id, Some(State::InGame));
    game.world.spawn(blueprints::fade_in());
}

/// The game-over fade's callback. The kernel defers the switch to the end of the
/// frame, so this only raises the flag; see [`crate::the_game::step`].
pub fn restart(game: &mut Game) {
    game.log.print_stats();
    game.scene_switch_requested = true;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{Phase, run_phase};

    fn spawn_flow(game: &mut Game) -> ActorId {
        game.world.spawn(blueprints::game_flow())
    }

    fn flow_of(game: &Game, id: ActorId) -> GameFlow {
        flow(game, id)
    }

    #[test]
    fn the_menu_is_built_in_the_order_the_lua_builds_it() {
        // four selectors, then the splash, then the fade: spawn order is
        // iteration order, so this is also the z-order of the title screen
        let mut game = Game::new();
        let id = spawn_flow(&mut game);

        update(&mut game, id);

        let names: Vec<&str> = game
            .world
            .order()
            .iter()
            .map(|&a| game.world.get(a).blueprint)
            .collect();
        assert_eq!(
            names,
            [
                "game_flow",
                "selection_factory",
                "selection_factory",
                "selection_factory",
                "selection_factory",
                "splash",
                "fade",
            ]
        );
        assert_eq!(flow_of(&game, id).state, Some(State::PlayerSelect));
    }

    #[test]
    fn the_selectors_sit_evenly_about_the_middle_of_the_menu() {
        // from `lua5.4 test/trace.lua 8 --dump 1:1`, which put them at 242, 372,
        // 502 and 632 on the original 1024-wide field, whose middle was 512. The
        // menu does not scale with the field, so the offsets from the middle are
        // what survive.
        let mut game = Game::new();
        let id = spawn_flow(&mut game);
        update(&mut game, id);

        let xs: Vec<f64> = flow_of(&game, id)
            .selectors
            .iter()
            .map(|&s| game.world.get(s).transform.unwrap().pos.x - CENTER.x)
            .collect();
        assert_eq!(
            xs,
            [242.0 - 512.0, 372.0 - 512.0, 502.0 - 512.0, 632.0 - 512.0]
        );

        for &s in &flow_of(&game, id).selectors {
            let transform = game.world.get(s).transform.unwrap();
            assert_eq!(transform.pos.y, CENTER.y);
            assert_eq!(transform.facing, V2::J);
        }
    }

    #[test]
    fn selectors_are_numbered_in_spawn_order() {
        let mut game = Game::new();
        let id = spawn_flow(&mut game);
        update(&mut game, id);

        let players: Vec<usize> = flow_of(&game, id)
            .selectors
            .iter()
            .map(|&s| game.world.get(s).selector.unwrap().player)
            .collect();
        assert_eq!(players, [1, 2, 3, 4]);
    }

    #[test]
    fn the_countdown_only_appears_once_somebody_joins() {
        let mut game = Game::new();
        let id = spawn_flow(&mut game);
        update(&mut game, id);

        update(&mut game, id);
        assert_eq!(game.world.tagged("countdown"), []);
        assert_eq!(flow_of(&game, id).state, Some(State::PlayerSelect));

        let first = flow_of(&game, id).selectors[0];
        game.world.get_mut(first).selector.as_mut().unwrap().picked = true;
        update(&mut game, id);

        assert_eq!(game.world.tagged("countdown").len(), 1);
        assert_eq!(
            flow_of(&game, id).state,
            None,
            "the machine has no state while the countdown runs"
        );
    }

    #[test]
    fn every_selector_can_reset_the_countdown_including_the_one_that_started_it() {
        let mut game = Game::new();
        let id = spawn_flow(&mut game);
        update(&mut game, id);

        let selectors = flow_of(&game, id).selectors;
        game.world
            .get_mut(selectors[2])
            .selector
            .as_mut()
            .unwrap()
            .picked = true;
        update(&mut game, id);

        let countdown = game.world.tagged("countdown")[0];
        for &s in &selectors {
            assert_eq!(
                game.world.get(s).selector.unwrap().callback,
                SelectorCallback::ResetCountdown(countdown)
            );
        }
    }

    #[test]
    fn a_lone_player_is_given_the_other_of_the_first_two_as_a_cpu() {
        for (human, expected_cpu) in [(1, 2), (2, 1), (3, 1), (4, 1)] {
            let mut game = Game::new();
            let id = spawn_flow(&mut game);
            update(&mut game, id);

            begin_match(&mut game, id, &[human]);

            let factories = game.world.tagged("factory");
            assert_eq!(factories.len(), 2);
            let players: Vec<usize> = factories
                .iter()
                .map(|&f| game.world.get(f).player.unwrap())
                .collect();
            assert_eq!(players, [human, expected_cpu]);

            let cpu = factories[1];
            assert!(
                game.world.get(cpu).easy_enemy_production.is_some(),
                "the added player is the CPU, and only it"
            );
            assert!(game.world.get(factories[0]).easy_enemy_production.is_none());
        }
    }

    #[test]
    fn four_factories_land_on_the_ring_facing_along_it() {
        // the Lua put them on the diagonals of a 300-unit ring around the middle
        // of the field, each facing along the circle; the ring is a field
        // distance, so it is RADIUS -- 300 scaled -- that they sit on here
        let mut game = Game::new();
        let id = spawn_flow(&mut game);
        update(&mut game, id);

        begin_match(&mut game, id, &[1, 2, 3, 4]);

        let placements: Vec<(V2, V2)> = game
            .world
            .tagged("factory")
            .iter()
            .map(|&f| {
                let t = game.world.get(f).transform.unwrap();
                (t.pos, t.facing)
            })
            .collect();

        let quarter = PI / 4.0;
        let angles = [3.0 * quarter, quarter, 5.0 * quarter, 7.0 * quarter];

        for (&(pos, facing), angle) in placements.iter().zip(angles) {
            let spoke = pos - CENTER;
            assert!(
                (spoke.mag() - RADIUS).abs() < 1e-9,
                "{pos:?} is {} from the middle, not {RADIUS}",
                spoke.mag()
            );
            assert!(
                (spoke.norm() - V2::unit(angle)).mag() < 1e-9,
                "{pos:?} is not at {angle} radians"
            );
            assert!(
                spoke.norm().dot(facing).abs() < 1e-9,
                "{facing:?} is not tangent to the ring at {pos:?}"
            );
        }
    }

    #[test]
    fn beginning_a_match_clears_the_menu_away() {
        let mut game = Game::new();
        let id = spawn_flow(&mut game);
        update(&mut game, id);
        let before = flow_of(&game, id);

        begin_match(&mut game, id, &[1, 2]);

        for &s in &before.selectors {
            assert!(game.world.is_dead(s));
        }
        assert!(game.world.is_dead(before.splash.unwrap()));
        assert_eq!(flow_of(&game, id).state, Some(State::InGame));
        assert_eq!(
            game.world
                .get(*game.world.order().last().unwrap())
                .blueprint,
            "fade",
            "the fade in is spawned after the factories, so it draws over them"
        );
    }

    #[test]
    fn a_match_with_two_factories_standing_never_times_out() {
        let mut game = Game::new();
        let id = spawn_flow(&mut game);
        update(&mut game, id);
        begin_match(&mut game, id, &[1, 2]);

        for _ in 0..GAME_OVER_FRAMES + 10 {
            in_game(&mut game, id);
        }

        assert_eq!(flow_of(&game, id).game_over_timer, 0);
        assert!(!game.scene_switch_requested);
    }

    #[test]
    fn the_game_over_fade_is_respawned_every_frame_after_the_timer_expires() {
        // deliberate quirk: the Lua has no guard here, so sixty fades pile up
        // before the first one lands and switches the scene
        let mut game = Game::new();
        let id = spawn_flow(&mut game);
        set_state(&mut game, id, Some(State::InGame));

        for _ in 0..GAME_OVER_FRAMES {
            in_game(&mut game, id);
        }
        assert_eq!(flow_of(&game, id).game_over_timer, GAME_OVER_FRAMES);
        let fades = game.world.tagged("fade").len();

        in_game(&mut game, id);
        in_game(&mut game, id);
        assert_eq!(game.world.tagged("fade").len(), fades + 2);
    }

    #[test]
    fn the_scene_machine_takes_no_draws_of_its_own() {
        // the factories it spawns do -- easy_enemy_production takes three on
        // creation -- so this checks the all-human path
        let mut game = Game::new();
        let id = spawn_flow(&mut game);
        let before = game.rng.clone();

        update(&mut game, id);
        run_phase(&mut game, Phase::Draw);
        begin_match(&mut game, id, &[1, 2, 3, 4]);

        assert_eq!(before, game.rng);
    }
}
