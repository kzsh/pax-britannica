//! Per-actor scripts and the phase dispatcher.
//!
//! A Lua script is a function whose environment is the actor's own table; here
//! it is a data struct on [`Actor`](crate::world::Actor) plus free functions
//! taking `(&mut Game, ActorId)`. Index-based access keeps the borrow checker
//! out of the way: a script can reach any other actor through the game without
//! holding a borrow across the call.

pub mod bomber_ai;
pub mod bomber_shooting;
pub mod bullet;
pub mod collision;
pub mod easy_enemy_production;
pub mod factory_ai;
pub mod factory_damage;
pub mod fighter_ai;
pub mod fighter_shooting;
pub mod frigate_ai;
pub mod frigate_shooting;
pub mod heatseeking_ai;
pub mod player_production;
#[cfg(test)]
pub mod probe;
pub mod production;
pub mod resources;
pub mod ship;
pub mod shooting;
pub mod sprite;
pub mod transform;

use crate::game::Game;
use crate::world::{ActorId, Phase};

/// Every script a blueprint can carry.
///
/// Variants are added as they are ported, so this enum doubles as the progress
/// marker for phase 3.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScriptKind {
    Transform,
    Sprite,
    Collision,
    Ship,
    Bullet,
    FighterShooting,
    FighterAi,
    BomberShooting,
    BomberAi,
    FrigateShooting,
    FrigateAi,
    HeatseekingAi,
    FactoryDamage,
    FactoryAi,
    Resources,
    Production,
    PlayerProduction,
    EasyEnemyProduction,
    /// The singleton that latches input, from `components/the_one_button.lua`.
    TheOneButton,
    /// The singleton that runs the broad/narrow phase, from
    /// `components/collision.lua`.
    CollisionChecker,
    #[cfg(test)]
    Probe,
}

/// Routes one phase call to one script.
///
/// A script that has nothing to do in a phase falls through, which is the
/// equivalent of the Lua script simply not defining that method and therefore
/// never being registered for it.
pub fn dispatch(game: &mut Game, id: ActorId, kind: ScriptKind, phase: Phase) {
    match (kind, phase) {
        (ScriptKind::Collision, Phase::CollisionRegistry) => collision::register(game, id),
        (ScriptKind::Ship, Phase::Update) => ship::update(game, id),
        (ScriptKind::Bullet, Phase::Update) => bullet::update(game, id),
        (ScriptKind::FighterShooting, Phase::Update) => fighter_shooting::update(game, id),
        (ScriptKind::FighterAi, Phase::Update) => fighter_ai::update(game, id),
        (ScriptKind::FrigateShooting, Phase::Update) => frigate_shooting::update(game, id),
        (ScriptKind::FrigateAi, Phase::Update) => frigate_ai::update(game, id),
        (ScriptKind::BomberAi, Phase::Update) => bomber_ai::update(game, id),
        (ScriptKind::HeatseekingAi, Phase::Update) => heatseeking_ai::update(game, id),
        (ScriptKind::FactoryDamage, Phase::Update) => factory_damage::update(game, id),
        (ScriptKind::FactoryAi, Phase::Update) => factory_ai::update(game, id),
        (ScriptKind::Resources, Phase::Update) => resources::update(game, id),
        (ScriptKind::Production, Phase::Update) => production::update(game, id),
        (ScriptKind::PlayerProduction, Phase::Update) => player_production::update(game, id),
        (ScriptKind::EasyEnemyProduction, Phase::Update) => easy_enemy_production::update(game, id),
        (ScriptKind::TheOneButton, Phase::UpdateSetup) => game.the_one_button.latch(),
        (ScriptKind::CollisionChecker, Phase::CollisionCheck) => collision::check_all(game),

        // The two draws that are not purely pixels: the dial carries the
        // needle's state across frames, and the damage flicker takes a draw off
        // the shared random stream. See each module's doc comment.
        (ScriptKind::Production, Phase::Draw) => production::draw(game, id),
        (ScriptKind::FactoryDamage, Phase::Draw) => factory_damage::draw(game, id),

        #[cfg(test)]
        (ScriptKind::Probe, Phase::Update) => probe::update(game, id),

        // Transform is pure data, and Sprite's draw lands with the renderer in
        // phase 4; both still occupy their place in the script order.
        _ => {}
    }
}
