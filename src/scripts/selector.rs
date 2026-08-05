//! `scripts/selector.lua`: one of the four "press A to join" factories on the
//! title screen.
//!
//! Latches `picked` the first time its player holds the button, then brightens
//! over ten frames. Everything else in the file is the pulsing ring drawn around
//! it, which is renderer work -- and, importantly, renderer work that takes *no*
//! draws off the shared random stream and writes nothing back: `pulse_time` is
//! advanced in `update`, and the ring's brightness is recomputed from it every
//! frame. See PORTING.md on reading every `draw` before writing it off.

use crate::game::Game;
use crate::scripts::countdown;
use crate::scripts::sprite::Color;
use crate::world::ActorId;

/// How fast a picked selector brightens, and the ceiling it brightens to.
const FADE_STEP: f64 = 0.1;
const FADE_MAX: f64 = 1.0;

/// What a selector does the moment its player joins. Only ever the countdown's
/// `reset_counter`, and only once the countdown exists -- the first player to
/// join is what creates it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SelectorCallback {
    #[default]
    None,
    ResetCountdown(ActorId),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Selector {
    pub player: usize,
    pub picked: bool,
    /// Chunk-level locals in the Lua, so per-instance state rather than scratch.
    pub fade: f64,
    pub pulse_time: i64,
    pub callback: SelectorCallback,
}

impl Selector {
    pub fn new(player: usize) -> Self {
        Self {
            player,
            picked: false,
            fade: 0.2,
            pulse_time: 0,
            callback: SelectorCallback::None,
        }
    }
}

pub fn update(game: &mut Game, id: ActorId) {
    let before = *game
        .world
        .get(id)
        .selector
        .as_ref()
        .expect("selector script");

    let joining = !before.picked && game.the_one_button.held(before.player);

    let actor = game.world.get_mut(id);
    let selector = actor.selector.as_mut().expect("selector script");
    selector.pulse_time += 1;
    if joining {
        selector.picked = true;
    }

    if selector.picked {
        selector.fade = (selector.fade + FADE_STEP).min(FADE_MAX);
        let fade = selector.fade;
        actor
            .sprite
            .as_mut()
            .expect("selection_factory blueprint has a sprite")
            .color = Some(Color::rgb(fade, fade, fade));
    }

    // the Lua fires the callback between latching `picked` and the fade, but the
    // callback only touches the countdown, so the two are independent; this is
    // the order the original runs them in
    if joining {
        match before.callback {
            SelectorCallback::None => {}
            SelectorCallback::ResetCountdown(countdown) => countdown::reset(game, countdown),
        }
    }
}

#[cfg(test)]
mod tests {
    // Expected floats are the lua5.4 binary's %.17g output copied verbatim; see
    // the same note in src/rng.rs.
    #![allow(clippy::excessive_precision)]

    use super::*;
    use crate::scripts::ScriptKind;
    use crate::scripts::countdown::{Countdown, CountdownCallback};
    use crate::scripts::sprite::Sprite;
    use crate::world::Actor;

    fn spawn(game: &mut Game, player: usize) -> ActorId {
        game.world.spawn(Actor {
            scripts: vec![ScriptKind::Sprite, ScriptKind::Selector],
            selector: Some(Selector::new(player)),
            sprite: Some(Sprite::blank()),
            ..Actor::new("selection_factory")
        })
    }

    fn selector(game: &Game, id: ActorId) -> Selector {
        game.world.get(id).selector.unwrap()
    }

    #[test]
    fn joining_and_the_first_fade_step_land_on_the_same_frame() {
        // the trace's first two frames: pulse_time 1 then 2, fade 0.2 then 0.3,
        // with `picked` already true on frame 2. Values from
        // `lua5.4 test/trace.lua 8 --dump 1:2`.
        let mut game = Game::new();
        let id = spawn(&mut game, 1);

        update(&mut game, id);
        assert_eq!(selector(&game, id).pulse_time, 1);
        assert_eq!(selector(&game, id).fade, 0.2);
        assert!(!selector(&game, id).picked);

        game.the_one_button.keys[0] = true;
        game.the_one_button.latch();
        update(&mut game, id);

        assert!(selector(&game, id).picked);
        assert_eq!(selector(&game, id).pulse_time, 2);
        assert_eq!(selector(&game, id).fade, 0.30000000000000004);
    }

    #[test]
    fn the_pulse_clock_runs_whether_or_not_the_player_joined() {
        let mut game = Game::new();
        let id = spawn(&mut game, 3);

        for _ in 0..17 {
            update(&mut game, id);
        }

        assert!(!selector(&game, id).picked);
        assert_eq!(selector(&game, id).pulse_time, 17);
        assert_eq!(selector(&game, id).fade, 0.2, "an unpicked selector is dim");
    }

    #[test]
    fn the_fade_stops_at_one() {
        let mut game = Game::new();
        let id = spawn(&mut game, 1);
        game.the_one_button.keys[0] = true;
        game.the_one_button.latch();

        for _ in 0..50 {
            update(&mut game, id);
        }

        assert_eq!(selector(&game, id).fade, 1.0);
        assert_eq!(
            game.world.get(id).sprite.as_ref().unwrap().color,
            Some(Color::rgb(1.0, 1.0, 1.0))
        );
    }

    #[test]
    fn a_late_joiner_puts_the_full_clock_back() {
        let mut game = Game::new();
        let countdown_id = game.world.spawn(Actor {
            scripts: vec![ScriptKind::Countdown],
            countdown: Some(Countdown::new(CountdownCallback::None)),
            sprite: Some(Sprite::blank()),
            ..Actor::new("countdown")
        });
        let id = spawn(&mut game, 2);
        game.world.get_mut(id).selector.as_mut().unwrap().callback =
            SelectorCallback::ResetCountdown(countdown_id);

        for _ in 0..90 {
            countdown::update(&mut game, countdown_id);
        }
        assert!(game.world.get(countdown_id).countdown.unwrap().counter < 4.0);

        game.the_one_button.keys[1] = true;
        game.the_one_button.latch();
        update(&mut game, id);

        assert_eq!(game.world.get(countdown_id).countdown.unwrap().counter, 5.0);
    }

    #[test]
    fn joining_only_fires_the_callback_once() {
        let mut game = Game::new();
        let countdown_id = game.world.spawn(Actor {
            scripts: vec![ScriptKind::Countdown],
            countdown: Some(Countdown::new(CountdownCallback::None)),
            sprite: Some(Sprite::blank()),
            ..Actor::new("countdown")
        });
        let id = spawn(&mut game, 1);
        game.world.get_mut(id).selector.as_mut().unwrap().callback =
            SelectorCallback::ResetCountdown(countdown_id);

        game.the_one_button.keys[0] = true;
        game.the_one_button.latch();
        update(&mut game, id);
        countdown::update(&mut game, countdown_id);
        let after_first = game.world.get(countdown_id).countdown.unwrap().counter;

        update(&mut game, id);
        countdown::update(&mut game, countdown_id);

        assert!(
            game.world.get(countdown_id).countdown.unwrap().counter < after_first,
            "holding the button must not keep resetting the clock"
        );
    }

    #[test]
    fn a_selector_takes_no_draws() {
        let mut game = Game::new();
        let id = spawn(&mut game, 4);
        let before = game.rng.clone();

        for _ in 0..30 {
            update(&mut game, id);
        }

        assert_eq!(before, game.rng);
    }
}
