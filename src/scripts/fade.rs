//! `scripts/fade.lua`: the black rectangle that wipes between scenes.
//!
//! A counter, a duration, and a callback fired once when the counter runs past
//! the duration. The rectangle itself is renderer work for phase 4; what is
//! gameplay is *when* the callback lands, because everything the scene machine
//! does -- starting a match, restarting the game -- hangs off one of these.
//!
//! The Lua callback is a closure over `scripts/game_flow.lua`'s chunk locals.
//! Rust has nothing to lean on there, so the two closures the game actually
//! creates are spelled out as [`FadeCallback`] variants instead of boxed. That
//! keeps the fade's state comparable, printable and `PartialEq`, and it makes
//! the whole set of things a fade can trigger visible in one place -- which is
//! the point, since the scene machine is otherwise scattered across four files.

use crate::game::Game;
use crate::scripts::game_flow;
use crate::world::ActorId;

/// What a fade does when it finishes. The two non-trivial variants are the only
/// two callbacks `scripts/game_flow.lua` ever passes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum FadeCallback {
    /// `callback = callback or function () end`.
    #[default]
    None,
    /// The fade-out at the end of player select: clear the menu away and put the
    /// factories in the water. `players` is frozen at the moment the countdown
    /// expired, not re-read when the fade lands.
    BeginMatch {
        game_flow: ActorId,
        players: Vec<usize>,
    },
    /// The fade-out after a match ends: print the stats and build a whole new
    /// game.
    Restart,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Fade {
    pub from: f64,
    pub to: f64,
    pub duration: f64,
    /// Frames elapsed. A chunk-level local in the Lua, so it is per-instance
    /// state and the trace captures it.
    pub counter: f64,
    pub callback: FadeCallback,
}

impl Fade {
    /// `blueprints.fade_in`: opaque to clear over one second.
    pub fn fade_in() -> Self {
        Self::new(1.0, 0.0, FadeCallback::None)
    }

    /// `blueprints.fade_out`: clear to opaque over one second.
    pub fn fade_out(callback: FadeCallback) -> Self {
        Self::new(0.0, 1.0, callback)
    }

    fn new(from: f64, to: f64, callback: FadeCallback) -> Self {
        Self {
            from,
            to,
            duration: 60.0,
            counter: 0.0,
            callback,
        }
    }

    /// The opacity the rectangle would be drawn at this frame. Phase 4 wants
    /// this; nothing else reads it.
    pub fn opacity(&self) -> f64 {
        let t = self.counter / self.duration;
        self.from * (1.0 - t) + self.to * t
    }
}

pub fn update(game: &mut Game, id: ActorId) {
    let fade = game
        .world
        .get_mut(id)
        .fade
        .as_mut()
        .expect("fade script")
        .clone();

    let counter = fade.counter + 1.0;
    game.world
        .get_mut(id)
        .fade
        .as_mut()
        .expect("fade script")
        .counter = counter;

    if counter > fade.duration {
        // the Lua fires the callback and only then marks itself dead, and the
        // callback spawns actors, so the order is observable in the trace
        run(game, fade.callback);
        game.world.kill(id);
    }
}

fn run(game: &mut Game, callback: FadeCallback) {
    match callback {
        FadeCallback::None => {}
        FadeCallback::BeginMatch { game_flow, players } => {
            game_flow::begin_match(game, game_flow, &players);
        }
        FadeCallback::Restart => game_flow::restart(game),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scripts::ScriptKind;
    use crate::world::{Actor, Phase, run_phase};

    fn spawn(game: &mut Game, fade: Fade) -> ActorId {
        game.world.spawn(Actor {
            scripts: vec![ScriptKind::Fade],
            fade: Some(fade),
            ..Actor::new("fade")
        })
    }

    #[test]
    fn a_fade_lives_for_one_frame_longer_than_its_duration() {
        // `counter > duration`, not `>=`: the frame that reaches 60 still draws
        let mut game = Game::new();
        let id = spawn(&mut game, Fade::fade_in());

        for _ in 0..60 {
            update(&mut game, id);
        }
        assert!(!game.world.is_dead(id));
        assert_eq!(game.world.get(id).fade.as_ref().unwrap().counter, 60.0);

        update(&mut game, id);
        assert!(game.world.is_dead(id));
    }

    #[test]
    fn opacity_runs_from_from_to_to() {
        let mut fade = Fade::fade_out(FadeCallback::None);
        assert_eq!(fade.opacity(), 0.0);
        fade.counter = 30.0;
        assert_eq!(fade.opacity(), 0.5);
        fade.counter = 60.0;
        assert_eq!(fade.opacity(), 1.0);
    }

    #[test]
    fn a_fade_takes_no_draws_off_the_shared_stream() {
        // fade_draw is a single black quad; if this ever stops being true the
        // whole run downstream of a scene change moves
        let mut game = Game::new();
        let id = spawn(&mut game, Fade::fade_in());
        let before = game.rng.clone();

        for _ in 0..61 {
            update(&mut game, id);
            run_phase(&mut game, Phase::FadeDraw);
        }

        assert_eq!(before, game.rng);
    }
}
