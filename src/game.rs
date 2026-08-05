//! The game object: the world plus the singleton services that `the_game.lua`
//! installs with `game.init_component`.
//!
//! In Lua these are tables hung off `game` (`game.resources`, `game.targeting`,
//! `game.log`, ...) whose functions close over module-level locals. Here they
//! are plain fields, so there is no global state and every script is handed the
//! game it belongs to.

use crate::log::Log;
use crate::particles::Particles;
use crate::rng::LuaRng;
use crate::scripts::collision::CollisionWorld;
use crate::the_one_button::TheOneButton;
use crate::world::{ActorId, World};

pub struct Game {
    pub world: World,
    /// The one shared random stream. Every draw taken anywhere advances it, and
    /// the order of those draws is part of the observable behaviour -- see
    /// PORTING.md on `components/particles.lua`.
    pub rng: LuaRng,
    pub collision: CollisionWorld,
    pub particles: Particles,
    pub log: Log,
    /// Every player's entire input, latched once per frame.
    pub the_one_button: TheOneButton,
    /// Raised by `scripts/game_flow.lua`'s game-over fade. The kernel does not
    /// switch scenes where it is asked to: the current frame finishes its update
    /// *and* its draw first. See [`crate::the_game::step`].
    pub scene_switch_requested: bool,

    /// Labels of the probe scripts that have run, in order.
    #[cfg(test)]
    pub probe_log: Vec<usize>,
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

impl Game {
    /// A game seeded the way the test runs seed it: `math.randomseed(1)`.
    pub fn new() -> Self {
        Self::with_seed(1)
    }

    pub fn with_seed(seed: i64) -> Self {
        Self::empty(LuaRng::new(seed, 0))
    }

    /// An empty game around an existing random stream.
    ///
    /// A restart builds one of these: the world, the components and the log are
    /// all new, but the generator carries on from where the last match left it.
    /// [`crate::the_game::make`] is what fills it in.
    pub fn empty(rng: LuaRng) -> Self {
        Self {
            world: World::new(),
            rng,
            collision: CollisionWorld::new(),
            particles: Particles::new(),
            log: Log::new(),
            the_one_button: TheOneButton::new(),
            scene_switch_requested: false,
            #[cfg(test)]
            probe_log: Vec::new(),
        }
    }

    /// Which player owns an actor.
    ///
    /// The Lua reads `self.ship and self.ship.player or self.bullet.player`,
    /// i.e. it asks whichever of the two scripts happens to be present. Both are
    /// per-actor and an actor never has both, so the field is hoisted onto the
    /// actor here and the awkward fallback disappears.
    pub fn player_of(&self, id: ActorId) -> usize {
        self.world
            .get(id)
            .player
            .expect("this actor should belong to a player")
    }

    pub fn set_player(&mut self, id: ActorId, player: usize) {
        self.world.get_mut(id).player = Some(player);
    }
}
