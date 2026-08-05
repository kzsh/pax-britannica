//! The actor model, ported from `dokidoki/game.lua`.
//!
//! The Lua original swaps each script's `_ENV` for a per-actor table, so a
//! script's "globals" are its instance fields and `self.ship.velocity` reaches
//! across scripts freely. None of that survives translation, but three of its
//! observable properties have to, because gameplay depends on them:
//!
//! 1. **Iteration is in spawn order.** `scripts_by_method[phase]` is appended to
//!    at spawn and culled with an order-preserving filter, so a phase visits
//!    scripts in the order their actors were created, and within an actor in
//!    blueprint order. Update order feeds AI targeting and collision
//!    resolution, and draw order *is* the z-order.
//! 2. **An actor spawned mid-phase is visited by that same phase.** Lua's
//!    `ipairs` walks a table that is being appended to, so a laser created
//!    during `update` gets its own `update` in the same frame. The phase loop
//!    below re-reads the length every iteration for exactly this reason.
//! 3. **Death is deferred, everything else is immediate.** `dead` is checked
//!    before each individual script call, so an actor that dies partway through
//!    its own script list skips the rest of it, but is only unlinked at the end
//!    of the update.
//!
//! Actor storage is a plain `Vec` that is only ever appended to. Dead actors are
//! unlinked from the iteration order and the tag index but their data is kept,
//! because the Lua holds direct table references -- an AI that stored a target
//! last frame reads `target.dead` this frame, and in Lua that read is still
//! valid. Never reusing a slot makes [`ActorId`] permanently valid, which is the
//! same guarantee at the cost of some memory.

use std::collections::HashMap;

use crate::scripts::{ScriptKind, dispatch};

/// A handle to an actor. Stays valid forever, including after death.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ActorId(pub usize);

/// The update and draw phases, in the order `the_game.lua` declares them.
///
/// Every phase is run to completion over all actors before the next one starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    UpdateSetup,
    Update,
    CollisionRegistry,
    CollisionCheck,
    UpdateCleanup,
    DrawSetup,
    Draw,
    DrawForeground,
    FadeDraw,
}

pub const UPDATE_PHASES: [Phase; 5] = [
    Phase::UpdateSetup,
    Phase::Update,
    Phase::CollisionRegistry,
    Phase::CollisionCheck,
    Phase::UpdateCleanup,
];

pub const DRAW_PHASES: [Phase; 4] = [
    Phase::DrawSetup,
    Phase::Draw,
    Phase::DrawForeground,
    Phase::FadeDraw,
];

/// One actor: a blueprint name, an ordered list of scripts, and the data for
/// each of those scripts.
///
/// The `Option` per script is the direct analogue of the Lua actor table's
/// per-script sub-table. It is not elegant, but it keeps cross-script access a
/// field access, and it makes the translation checkable line by line against the
/// original. See PORTING.md on idiomatising this once parity is reached.
#[derive(Debug, Default)]
pub struct Actor {
    pub blueprint: &'static str,
    /// Blueprint order, which is the order scripts run within this actor.
    pub scripts: Vec<ScriptKind>,
    pub dead: bool,
    pub paused: bool,
    pub hidden: bool,
    /// Which player owns this actor, where that means anything. Hoisted out of
    /// the `ship`/`bullet` scripts that hold it in Lua; see
    /// [`Game::player_of`](crate::game::Game::player_of).
    pub player: Option<usize>,

    pub transform: Option<crate::scripts::transform::Transform>,
    pub sprite: Option<crate::scripts::sprite::Sprite>,
    pub collision: Option<crate::scripts::collision::Collision>,
    pub ship: Option<crate::scripts::ship::Ship>,
    pub bullet: Option<crate::scripts::bullet::Bullet>,
    pub resources: Option<crate::scripts::resources::Resources>,
    pub production: Option<crate::scripts::production::Production>,
    pub easy_enemy_production: Option<crate::scripts::easy_enemy_production::EasyEnemyProduction>,
    pub factory_ai: Option<crate::scripts::factory_ai::FactoryAi>,
    pub factory_damage: Option<crate::scripts::factory_damage::FactoryDamage>,
    pub fighter_ai: Option<crate::scripts::fighter_ai::FighterAi>,
    pub fighter_shooting: Option<crate::scripts::shooting::Weapon>,
    pub bomber_ai: Option<crate::scripts::bomber_ai::BomberAi>,
    pub frigate_ai: Option<crate::scripts::frigate_ai::FrigateAi>,
    pub frigate_shooting: Option<crate::scripts::shooting::Weapon>,
    pub heatseeking_ai: Option<crate::scripts::heatseeking_ai::HeatseekingAi>,

    #[cfg(test)]
    pub probe: Option<crate::scripts::probe::Probe>,
}

impl Actor {
    pub fn new(blueprint: &'static str) -> Self {
        Self {
            blueprint,
            ..Default::default()
        }
    }

    /// Whether this actor runs `kind`. Cheap: the list is a handful of entries.
    pub fn has(&self, kind: ScriptKind) -> bool {
        self.scripts.contains(&kind)
    }
}

/// All the actors, the order they run in, and the tag index.
#[derive(Debug, Default)]
pub struct World {
    /// Append-only. Indexed by [`ActorId`]; entries are never removed or reused.
    actors: Vec<Actor>,
    /// Live actors in spawn order. This is what the phase loop walks.
    order: Vec<ActorId>,
    /// `game.actors.get(tag)`. Values stay in spawn order.
    by_tag: HashMap<&'static str, Vec<ActorId>>,
}

impl World {
    pub fn new() -> Self {
        Self::default()
    }

    /// `game.actors.new(blueprint)`.
    ///
    /// Appending to `order` is what makes an actor spawned mid-phase visible to
    /// the rest of that phase.
    pub fn spawn(&mut self, actor: Actor) -> ActorId {
        let id = ActorId(self.actors.len());
        let tag = actor.blueprint;

        self.actors.push(actor);
        self.order.push(id);
        self.by_tag.entry(tag).or_default().push(id);

        id
    }

    pub fn get(&self, id: ActorId) -> &Actor {
        &self.actors[id.0]
    }

    pub fn get_mut(&mut self, id: ActorId) -> &mut Actor {
        &mut self.actors[id.0]
    }

    /// `game.actors.get(tag)`: live actors with this tag, in spawn order.
    pub fn tagged(&self, tag: &str) -> &[ActorId] {
        self.by_tag.get(tag).map_or(&[], Vec::as_slice)
    }

    /// Live actors in spawn order.
    pub fn order(&self) -> &[ActorId] {
        &self.order
    }

    pub fn live_count(&self) -> usize {
        self.order.len()
    }

    /// Total ever spawned, dead included. Diagnostic only.
    pub fn total_spawned(&self) -> usize {
        self.actors.len()
    }

    pub fn kill(&mut self, id: ActorId) {
        self.actors[id.0].dead = true;
    }

    pub fn is_dead(&self, id: ActorId) -> bool {
        self.actors[id.0].dead
    }

    /// Unlinks dead actors, preserving the order of the survivors.
    ///
    /// Runs once at the end of the update, never mid-phase, matching the Lua.
    fn cull(&mut self) {
        let actors = &self.actors;
        self.order.retain(|id| !actors[id.0].dead);
        for ids in self.by_tag.values_mut() {
            ids.retain(|id| !actors[id.0].dead);
        }
    }
}

/// Runs one phase over every live actor.
///
/// Split out from [`Game`](crate::game::Game) so the borrow of the world stays
/// visibly scoped to the index bookkeeping, with the game handed whole to each
/// script.
pub fn run_phase(game: &mut crate::game::Game, phase: Phase) {
    let mut position = 0;
    // NOT a `for` loop over a snapshot: the length is re-read every iteration so
    // that actors spawned during this phase are visited by it
    while position < game.world.order.len() {
        let id = game.world.order[position];
        position += 1;

        // an actor's own script may kill it partway through the list, so the
        // check is per script rather than per actor
        for index in 0..game.world.get(id).scripts.len() {
            let actor = game.world.get(id);

            let skip = match phase {
                Phase::DrawSetup | Phase::Draw | Phase::DrawForeground | Phase::FadeDraw => {
                    actor.hidden
                }
                _ => actor.dead || actor.paused,
            };
            if skip {
                break;
            }

            let kind = actor.scripts[index];
            dispatch(game, id, kind, phase);
        }
    }
}

/// One full update: every update phase in order, then the cull.
pub fn update(game: &mut crate::game::Game) {
    for phase in UPDATE_PHASES {
        run_phase(game, phase);
    }
    game.world.cull();
}

/// One full draw: every draw phase in order. Never mutates liveness.
pub fn draw(game: &mut crate::game::Game) {
    for phase in DRAW_PHASES {
        run_phase(game, phase);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Game;
    use crate::scripts::transform::Transform;
    use crate::v2::v2;

    fn actor_with_transform(name: &'static str) -> Actor {
        Actor {
            scripts: vec![ScriptKind::Transform],
            transform: Some(Transform::default()),
            ..Actor::new(name)
        }
    }

    #[test]
    fn spawn_order_is_preserved() {
        let mut world = World::new();
        let a = world.spawn(Actor::new("a"));
        let b = world.spawn(Actor::new("b"));
        let c = world.spawn(Actor::new("c"));
        assert_eq!(world.order(), [a, b, c]);
    }

    #[test]
    fn tags_index_by_blueprint_name_in_spawn_order() {
        let mut world = World::new();
        let first = world.spawn(Actor::new("fighter"));
        world.spawn(Actor::new("bomber"));
        let second = world.spawn(Actor::new("fighter"));

        assert_eq!(world.tagged("fighter"), [first, second]);
        assert_eq!(world.tagged("nothing"), []);
    }

    #[test]
    fn cull_unlinks_the_dead_and_keeps_the_order() {
        let mut game = Game::new();
        let a = game.world.spawn(Actor::new("a"));
        let b = game.world.spawn(Actor::new("b"));
        let c = game.world.spawn(Actor::new("c"));

        game.world.kill(b);
        update(&mut game);

        assert_eq!(game.world.order(), [a, c]);
        assert_eq!(game.world.tagged("b"), []);
    }

    #[test]
    fn a_dead_actor_is_still_readable_through_its_id() {
        // an AI that stored a target last frame reads target.dead this frame;
        // in Lua that read is still valid, so it has to be here too
        let mut game = Game::new();
        let id = game.world.spawn(actor_with_transform("fighter"));
        game.world.get_mut(id).transform.as_mut().unwrap().pos = v2(7.0, 9.0);

        game.world.kill(id);
        update(&mut game);

        assert!(game.world.is_dead(id));
        assert_eq!(
            game.world.get(id).transform.unwrap().pos,
            v2(7.0, 9.0),
            "a culled actor's data must survive for stale references"
        );
    }

    #[test]
    fn ids_are_never_reused() {
        let mut game = Game::new();
        let first = game.world.spawn(Actor::new("a"));
        game.world.kill(first);
        update(&mut game);
        let second = game.world.spawn(Actor::new("b"));

        assert_ne!(first, second);
        assert_eq!(game.world.get(first).blueprint, "a");
        assert_eq!(game.world.get(second).blueprint, "b");
    }

    #[test]
    fn an_actor_spawned_mid_phase_runs_in_that_phase() {
        // Lua's ipairs walks a table that is being appended to, so a laser
        // created during `update` gets its own `update` the same frame. Miss
        // this and every projectile is a frame late.
        use crate::scripts::probe::{Probe, spawn};

        let mut game = Game::new();
        spawn(
            &mut game,
            Probe {
                label: 1,
                spawn_once: true,
                ..Default::default()
            },
        );

        run_phase(&mut game, Phase::Update);

        assert_eq!(
            game.probe_log,
            [1, 101],
            "the spawned probe should have run in the same phase"
        );
    }

    #[test]
    fn a_script_killing_its_actor_stops_that_actors_later_scripts() {
        use crate::scripts::probe::{Probe, spawn_double};

        let mut game = Game::new();
        spawn_double(
            &mut game,
            Probe {
                label: 7,
                kill_self: true,
                ..Default::default()
            },
        );

        run_phase(&mut game, Phase::Update);

        assert_eq!(
            game.probe_log,
            [7],
            "the second script on a dead actor should not have run"
        );
    }

    #[test]
    fn a_dead_actor_is_skipped_by_later_phases_before_the_cull() {
        use crate::scripts::probe::{Probe, spawn};

        let mut game = Game::new();
        let id = spawn(
            &mut game,
            Probe {
                label: 3,
                ..Default::default()
            },
        );
        game.world.kill(id);

        run_phase(&mut game, Phase::Update);

        assert!(game.probe_log.is_empty());
    }

    #[test]
    fn live_count_ignores_the_dead() {
        let mut game = Game::new();
        game.world.spawn(Actor::new("a"));
        let b = game.world.spawn(Actor::new("b"));
        game.world.kill(b);
        update(&mut game);

        assert_eq!(game.world.live_count(), 1);
        assert_eq!(game.world.total_spawned(), 2);
    }
}
