//! A test-only script for pinning down the phase loop's semantics.
//!
//! The two behaviours checked here are the ones the whole port rests on, and
//! both are easy to get subtly wrong in a way no gameplay test would catch until
//! thousands of frames later:
//!
//! - an actor spawned during a phase is visited by that same phase;
//! - a script that kills its own actor stops the actor's remaining scripts.

use crate::game::Game;
use crate::world::{Actor, ActorId};

use super::ScriptKind;

/// What a probe does when its update runs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Probe {
    /// Spawn one more probe, once.
    pub spawn_once: bool,
    /// Mark this actor dead.
    pub kill_self: bool,
    /// An identifying number, written to the game's probe log when this runs.
    pub label: usize,
}

pub fn update(game: &mut Game, id: ActorId) {
    let probe = game
        .world
        .get(id)
        .probe
        .expect("probe script needs a probe");
    game.probe_log.push(probe.label);

    if probe.spawn_once {
        game.world.get_mut(id).probe.as_mut().unwrap().spawn_once = false;
        spawn(
            game,
            Probe {
                label: probe.label + 100,
                ..Default::default()
            },
        );
    }

    if probe.kill_self {
        game.world.kill(id);
    }
}

/// A probe that runs `update` twice, so a kill in the first can be seen to stop
/// the second.
pub fn spawn_double(game: &mut Game, probe: Probe) -> ActorId {
    game.world.spawn(Actor {
        scripts: vec![ScriptKind::Probe, ScriptKind::Probe],
        probe: Some(probe),
        ..Actor::new("probe")
    })
}

pub fn spawn(game: &mut Game, probe: Probe) -> ActorId {
    game.world.spawn(Actor {
        scripts: vec![ScriptKind::Probe],
        probe: Some(probe),
        ..Actor::new("probe")
    })
}
