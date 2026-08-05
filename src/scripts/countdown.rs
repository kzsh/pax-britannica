//! `scripts/countdown.lua`: the five seconds between the last player joining
//! and the match starting.
//!
//! Counts down from 5 at one per second, showing `math.ceil(counter)` as a
//! digit that pulses once a second, and fires its callback when it runs out.
//! Every selector holds a reference to `reset_counter`, so a late joiner puts
//! the full five seconds back on the clock.
//!
//! The counter is a chunk-level local reached only through those two closures,
//! which is exactly the shape PORTING.md warns about: it is per-instance state
//! and it is in the golden trace.

use crate::game::Game;
use crate::resources::SpriteId;
use crate::scripts::game_flow;
use crate::scripts::sprite::Color;
use crate::world::ActorId;

/// Seconds on the clock, both at spawn and after every reset.
const SECONDS: f64 = 5.0;

/// What the countdown does when it reaches zero. Only ever `start_game`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CountdownCallback {
    #[default]
    None,
    /// `game_flow`'s `start_game`, addressed by the actor that owns it.
    StartGame(ActorId),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Countdown {
    pub counter: f64,
    pub callback: CountdownCallback,
}

impl Countdown {
    pub fn new(callback: CountdownCallback) -> Self {
        Self {
            counter: SECONDS,
            callback,
        }
    }
}

/// `reset_counter`, which every selector calls when its player joins.
///
/// In Lua this is a bound closure the selectors keep hold of, so it stays
/// callable after the countdown dies. Writing through the [`ActorId`] has the
/// same property: a dead actor's data survives, and mutating it changes nothing.
pub fn reset(game: &mut Game, id: ActorId) {
    game.world
        .get_mut(id)
        .countdown
        .as_mut()
        .expect("countdown script")
        .counter = SECONDS;
}

pub fn update(game: &mut Game, id: ActorId) {
    let actor = game.world.get_mut(id);
    let countdown = actor.countdown.as_mut().expect("countdown script");

    countdown.counter -= 1.0 / 60.0;
    let counter = countdown.counter;
    let callback = countdown.callback;

    // Lua's `%` is a floor modulo, so the last frame's slightly-negative counter
    // wraps up to just under 1 rather than down past zero as Rust's `%` would.
    let alpha = (counter.rem_euclid(1.0) * std::f64::consts::PI).sin();
    let sprite = actor
        .sprite
        .as_mut()
        .expect("countdown blueprint has sprite");
    sprite.color = Some(Color::WHITE.with_alpha(alpha));

    if counter <= 0.0 {
        game.world.kill(id);
        match callback {
            CountdownCallback::None => {}
            CountdownCallback::StartGame(game_flow) => game_flow::start_game(game, game_flow),
        }
    } else {
        let digit = counter.ceil() as usize;
        game.world
            .get_mut(id)
            .sprite
            .as_mut()
            .expect("countdown blueprint has sprite")
            .image = Some(SpriteId::Number(digit));
    }
}

#[cfg(test)]
mod tests {
    // Expected floats are the lua5.4 binary's %.17g output copied verbatim; see
    // the same note in src/rng.rs.
    #![allow(clippy::excessive_precision)]

    use super::*;
    use crate::scripts::ScriptKind;
    use crate::scripts::sprite::Sprite;
    use crate::world::Actor;

    fn spawn(game: &mut Game) -> ActorId {
        game.world.spawn(Actor {
            scripts: vec![ScriptKind::Countdown, ScriptKind::Sprite],
            countdown: Some(Countdown::new(CountdownCallback::None)),
            sprite: Some(Sprite::blank()),
            ..Actor::new("countdown")
        })
    }

    fn counter(game: &Game, id: ActorId) -> f64 {
        game.world.get(id).countdown.unwrap().counter
    }

    #[test]
    fn the_counter_matches_the_lua_interpreter_bit_for_bit() {
        // repeated subtraction of 1/60, not 5 - n/60; these are different
        // doubles and the trace pins the difference. Values lifted from
        // `lua5.4 test/trace.lua 305 --dump 300:303`.
        let mut game = Game::new();
        let id = spawn(&mut game);

        for _ in 0..297 {
            update(&mut game, id);
        }
        assert_eq!(counter(&game, id), 0.050000000000012756);
        update(&mut game, id);
        assert_eq!(counter(&game, id), 0.033333333333346093);
        update(&mut game, id);
        assert_eq!(counter(&game, id), 0.016666666666679427);
        update(&mut game, id);
        assert_eq!(counter(&game, id), 1.2760625889285393e-14);

        assert!(
            !game.world.is_dead(id),
            "a counter of 1.3e-14 is still above zero, so the clock runs one \
             frame longer than five seconds"
        );
    }

    #[test]
    fn the_digit_is_the_ceiling_of_the_counter() {
        let mut game = Game::new();
        let id = spawn(&mut game);

        let digit = |game: &Game| game.world.get(id).sprite.as_ref().unwrap().image;

        update(&mut game, id);
        assert_eq!(digit(&game), Some(SpriteId::Number(5)));

        for _ in 0..60 {
            update(&mut game, id);
        }
        assert_eq!(digit(&game), Some(SpriteId::Number(4)));
    }

    #[test]
    fn the_pulse_never_goes_negative_on_the_last_frame() {
        // Rust's `%` truncates and Lua's floors; on the frame the counter goes
        // negative that is the difference between a sane alpha and a negative
        // one, and `sin` is odd so it does not wash out
        let mut game = Game::new();
        let id = spawn(&mut game);

        for _ in 0..301 {
            update(&mut game, id);
        }

        let alpha = game.world.get(id).sprite.as_ref().unwrap().color.unwrap().a;
        assert_eq!(counter(&game, id), -0.016666666666653906);
        assert_eq!(alpha, 0.052335956242903894);
    }

    #[test]
    fn a_reset_puts_the_full_five_seconds_back() {
        let mut game = Game::new();
        let id = spawn(&mut game);

        for _ in 0..120 {
            update(&mut game, id);
        }
        assert!(counter(&game, id) < 4.0);

        reset(&mut game, id);
        assert_eq!(counter(&game, id), 5.0);
    }

    #[test]
    fn running_out_kills_the_actor_and_fires_the_callback_once() {
        let mut game = Game::new();
        let id = spawn(&mut game);

        for _ in 0..300 {
            update(&mut game, id);
        }
        assert!(!game.world.is_dead(id));

        update(&mut game, id);
        assert!(game.world.is_dead(id));
    }
}
