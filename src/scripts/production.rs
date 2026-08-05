//! `scripts/production.lua`: the radial build menu.
//!
//! One button, four units. Holding it commits resources at a fixed rate,
//! sweeping a needle around a dial; letting go builds whichever unit the needle
//! has swept past. The dial is the game's entire interface, so most of the
//! original file is the drawing of it -- but the state that drives the drawing
//! lives in chunk-level locals, which is why the needle and the texture scroller
//! are fields here rather than renderer scratch.
//!
//! ```text
//!   cost:  0 ----- 50 --------- 170 --------- 360 --------- 1080
//!          |  none  |  fighter   |   bomber    |   frigate   | upgrade
//!   angle: 0 ------ 0 --------- 0.25 --------- 0.5 --------- 0.75 --> 1
//! ```
//!
//! The quadrant boundaries are the costs themselves, and the release test is
//! strictly greater than: stopping exactly on 50 builds nothing.

use crate::blueprints;
use crate::game::Game;
use crate::world::ActorId;

/// Resources committed per frame while the button is held.
pub const BUILDING_SPEED: f64 = 3.0;

/// How far in front of the factory a new ship appears.
const SPAWN_OFFSET: f64 = 47.0;

/// What one upgrade adds to the factory's harvest rate.
const UPGRADE_HARVEST_BONUS: f64 = 0.25;

/// What the button can buy, cheapest first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnitType {
    Fighter,
    Bomber,
    Frigate,
    /// Not a ship: a permanent increase to this factory's income.
    Upgrade,
}

impl UnitType {
    /// `UNIT_COSTS`.
    pub const fn cost(self) -> f64 {
        match self {
            UnitType::Fighter => 50.0,
            UnitType::Bomber => 170.0,
            UnitType::Frigate => 360.0,
            UnitType::Upgrade => 1080.0,
        }
    }

    /// The dearest unit this many resources has swept past, if any. This is the
    /// `if/elseif` chain that both the release branch and the preview outline
    /// run, in the same order.
    pub fn for_cost(cost: f64) -> Option<UnitType> {
        [
            UnitType::Upgrade,
            UnitType::Frigate,
            UnitType::Bomber,
            UnitType::Fighter,
        ]
        .into_iter()
        .find(|unit| cost > unit.cost())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Production {
    /// Whether the owner is holding the button this frame, written by
    /// `player_production` or `easy_enemy_production` earlier in the update.
    pub button_held: bool,
    /// Set when a factory starts dying: it stops accepting build input, and the
    /// dial stops being drawn, while the actor lives on for another hundred
    /// frames of explosions.
    pub halt_production: bool,
    /// Resources committed so far this hold. Reset to zero on release.
    pub potential_cost: f64,

    // The needle's state. Only the draw touches these, but they persist across
    // frames, so they are state rather than scratch.
    pub needle_angle: f64,
    pub needle_velocity: f64,
    /// Scrolls the health bar's texture. Advances only while the button is *not*
    /// held, since that is the only time the bar is on screen.
    pub texcoord_scroller: f64,
}

impl Default for Production {
    fn default() -> Self {
        Self {
            button_held: false,
            halt_production: false,
            potential_cost: 0.0,
            needle_angle: 0.0,
            needle_velocity: 0.0,
            texcoord_scroller: -0.25,
        }
    }
}

/// Where on the dial a given number of resources sits, as a fraction of a turn.
///
/// Each unit gets a quarter turn, and the leading quarter is subtracted at the
/// end so that anything below a fighter reads as exactly zero -- which is what
/// the needle's fall-back-to-rest test keys off.
pub fn scale_angle(cost: f64) -> f64 {
    let (fighter, bomber, frigate, upgrade) = (
        UnitType::Fighter.cost(),
        UnitType::Bomber.cost(),
        UnitType::Frigate.cost(),
        UnitType::Upgrade.cost(),
    );

    let angle = if cost < fighter {
        0.25
    } else if cost < bomber {
        (cost - fighter) / (bomber - fighter) * 0.25 + 0.25
    } else if cost < frigate {
        (cost - bomber) / (frigate - bomber) * 0.25 + 0.5
    } else {
        (cost - frigate) / (upgrade - frigate) * 0.25 + 0.75
    };

    (angle - 0.25).min(1.0)
}

/// Buy one thing: pay for it, then either raise the harvest rate or put a ship
/// in the water in front of the factory.
fn spawn(game: &mut Game, id: ActorId, unit: UnitType) {
    let resources = game
        .world
        .get_mut(id)
        .resources
        .as_mut()
        .expect("production needs the resources script");
    resources.amount -= unit.cost();

    if unit == UnitType::Upgrade {
        resources.harvest_rate += UPGRADE_HARVEST_BONUS;
        return;
    }

    let transform = game.world.get(id).transform.expect("transform");
    let pos = transform.pos + transform.facing * SPAWN_OFFSET;
    let player = game.player_of(id);

    let actor = match unit {
        UnitType::Fighter => blueprints::fighter(player, pos, transform.facing),
        UnitType::Bomber => blueprints::bomber(player, pos, transform.facing),
        UnitType::Frigate => blueprints::frigate(player, pos, transform.facing),
        UnitType::Upgrade => unreachable!("handled above"),
    };

    game.log.record_spawn(actor.blueprint);
    game.world.spawn(actor);
}

/// While held, commit resources; on release, build.
///
/// The original also reads four debug keys here to spawn units directly. They
/// are dead in every build: `components/debug_keys.lua` gates them on a
/// `--debug` flag that nothing in the shipped game passes, so they are not
/// ported.
pub fn update(game: &mut Game, id: ActorId) {
    let (button_held, mut potential_cost) = {
        let production = production(game, id);
        (production.button_held, production.potential_cost)
    };
    let amount = game
        .world
        .get(id)
        .resources
        .expect("production needs the resources script")
        .amount;

    if button_held {
        // the first frame of a hold jumps straight to a fighter, so the needle
        // never sits in the dead quarter while the button is down
        if potential_cost == 0.0 && amount >= potential_cost {
            potential_cost = UnitType::Fighter.cost();
        }
        if amount > potential_cost + BUILDING_SPEED - 1.0 {
            potential_cost += BUILDING_SPEED;
        }
        production_mut(game, id).potential_cost = potential_cost;
    } else {
        if let Some(unit) = UnitType::for_cost(potential_cost) {
            spawn(game, id, unit);
        }
        production_mut(game, id).potential_cost = 0.0;
    }
}

/// The part of the dial's draw that is state rather than pixels.
///
/// The needle snaps to the committed cost while the button is held; on release
/// it falls back towards rest under a constant acceleration and bounces, losing
/// just over half its speed each time. The texture scroller creeps the health
/// bar along. Everything else in the Lua `draw` -- the pie slices, the preview
/// outlines, the health bar's colours -- is renderer work for phase 4, but these
/// two carry frame-to-frame state and so have to run now.
pub fn draw(game: &mut Game, id: ActorId) {
    let production = production(game, id);
    if production.halt_production {
        return;
    }

    let (button_held, potential_cost) = (production.button_held, production.potential_cost);
    let (mut needle_angle, mut needle_velocity) =
        (production.needle_angle, production.needle_velocity);
    let mut texcoord_scroller = production.texcoord_scroller;

    let angle = scale_angle(potential_cost);
    if angle == 0.0 {
        // `needle_velocity < 0` is the bounce still travelling upwards; without
        // it the needle would stick the instant it first touched zero
        if needle_angle > 0.0 || needle_velocity < 0.0 {
            needle_velocity = (needle_velocity + 0.002).min(0.025);
            needle_angle = (needle_angle - needle_velocity).max(0.0);
            if needle_angle == 0.0 {
                needle_velocity *= -0.475;
            }
        }
    } else {
        needle_velocity = 0.0;
        needle_angle = angle;
    }

    if !button_held {
        texcoord_scroller += 0.01;
        if texcoord_scroller > 0.25 {
            texcoord_scroller -= 0.5;
        }
    }

    let production = production_mut(game, id);
    production.needle_angle = needle_angle;
    production.needle_velocity = needle_velocity;
    production.texcoord_scroller = texcoord_scroller;
}

fn production(game: &Game, id: ActorId) -> &Production {
    game.world
        .get(id)
        .production
        .as_ref()
        .expect("production script")
}

fn production_mut(game: &mut Game, id: ActorId) -> &mut Production {
    game.world
        .get_mut(id)
        .production
        .as_mut()
        .expect("production script")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scripts::ScriptKind;
    use crate::scripts::resources::Resources;
    use crate::scripts::transform::Transform;
    use crate::v2::{V2, v2};
    use crate::world::Actor;

    fn spawn_factory(game: &mut Game, amount: f64) -> ActorId {
        game.world.spawn(Actor {
            scripts: vec![
                ScriptKind::Transform,
                ScriptKind::Resources,
                ScriptKind::Production,
            ],
            transform: Some(Transform::facing(v2(100.0, 200.0), V2::I)),
            resources: Some(Resources {
                amount,
                ..Resources::default()
            }),
            production: Some(Production::default()),
            player: Some(1),
            ..Actor::new("factory")
        })
    }

    fn hold(game: &mut Game, id: ActorId, held: bool) {
        production_mut(game, id).button_held = held;
    }

    #[test]
    fn a_hold_commits_a_fighters_worth_immediately_then_creeps() {
        let mut game = Game::new();
        let id = spawn_factory(&mut game, 500.0);
        hold(&mut game, id, true);

        update(&mut game, id);
        assert_eq!(
            production(&game, id).potential_cost,
            UnitType::Fighter.cost() + BUILDING_SPEED
        );

        update(&mut game, id);
        assert_eq!(
            production(&game, id).potential_cost,
            UnitType::Fighter.cost() + 2.0 * BUILDING_SPEED
        );
    }

    #[test]
    fn the_commitment_stops_climbing_when_the_resources_run_out() {
        // the creep is gated on having more than the cost, so a poor factory
        // parks at exactly a fighter no matter how long the button is held
        let mut game = Game::new();
        let id = spawn_factory(&mut game, 51.0);
        hold(&mut game, id, true);

        for _ in 0..10 {
            update(&mut game, id);
        }

        assert_eq!(
            production(&game, id).potential_cost,
            UnitType::Fighter.cost()
        );
    }

    #[test]
    fn releasing_builds_the_unit_the_needle_swept_past() {
        let mut game = Game::new();
        let id = spawn_factory(&mut game, 500.0);
        production_mut(&mut game, id).potential_cost = UnitType::Bomber.cost() + 1.0;

        update(&mut game, id);

        let spawned = game.world.tagged("bomber").to_vec();
        assert_eq!(spawned.len(), 1);
        assert_eq!(game.world.get(spawned[0]).player, Some(1));
        assert_eq!(
            game.world.get(spawned[0]).transform.unwrap().pos,
            v2(100.0 + SPAWN_OFFSET, 200.0),
            "new ships appear in front of the factory"
        );
        assert_eq!(
            game.world.get(id).resources.unwrap().amount,
            500.0 - UnitType::Bomber.cost()
        );
        assert_eq!(game.log.spawn.bomber, 1.0);
        assert_eq!(production(&game, id).potential_cost, 0.0);
    }

    #[test]
    fn releasing_exactly_on_a_boundary_builds_nothing() {
        let mut game = Game::new();
        let id = spawn_factory(&mut game, 500.0);
        production_mut(&mut game, id).potential_cost = UnitType::Fighter.cost();

        update(&mut game, id);

        assert_eq!(game.world.tagged("fighter"), []);
        assert_eq!(game.world.get(id).resources.unwrap().amount, 500.0);
    }

    #[test]
    fn an_upgrade_buys_income_rather_than_a_ship() {
        let mut game = Game::new();
        let id = spawn_factory(&mut game, 2000.0);
        production_mut(&mut game, id).potential_cost = UnitType::Upgrade.cost() + 1.0;

        update(&mut game, id);

        let resources = game.world.get(id).resources.unwrap();
        assert_eq!(resources.amount, 2000.0 - UnitType::Upgrade.cost());
        assert_eq!(resources.harvest_rate, 0.75 + UPGRADE_HARVEST_BONUS);
        assert_eq!(game.world.live_count(), 1, "an upgrade spawns no actor");
    }

    #[test]
    fn the_dial_takes_no_draws_from_the_shared_stream() {
        // if this ever stops being true, every run after the first build
        // diverges from the trace
        let mut game = Game::new();
        let id = spawn_factory(&mut game, 2000.0);
        production_mut(&mut game, id).potential_cost = UnitType::Frigate.cost() + 1.0;

        let before = game.rng.clone();
        update(&mut game, id);
        draw(&mut game, id);

        assert_eq!(before, game.rng);
    }

    #[test]
    fn each_quadrant_maps_to_its_unit() {
        assert_eq!(UnitType::for_cost(0.0), None);
        assert_eq!(UnitType::for_cost(50.0), None);
        assert_eq!(UnitType::for_cost(51.0), Some(UnitType::Fighter));
        assert_eq!(UnitType::for_cost(171.0), Some(UnitType::Bomber));
        assert_eq!(UnitType::for_cost(361.0), Some(UnitType::Frigate));
        assert_eq!(UnitType::for_cost(1081.0), Some(UnitType::Upgrade));
    }

    #[test]
    fn the_dial_gives_each_unit_a_quarter_turn() {
        assert_eq!(scale_angle(0.0), 0.0);
        assert_eq!(scale_angle(49.0), 0.0);
        assert_eq!(scale_angle(50.0), 0.0);
        assert_eq!(scale_angle(170.0), 0.25);
        assert_eq!(scale_angle(360.0), 0.5);
        assert_eq!(scale_angle(1080.0), 0.75);
        assert_eq!(scale_angle(10_000.0), 1.0, "the dial does not wrap round");
    }

    #[test]
    fn the_needle_snaps_to_the_hold_then_falls_back_on_release() {
        let mut game = Game::new();
        let id = spawn_factory(&mut game, 500.0);
        production_mut(&mut game, id).potential_cost = UnitType::Bomber.cost();

        draw(&mut game, id);
        assert_eq!(production(&game, id).needle_angle, 0.25);
        assert_eq!(production(&game, id).needle_velocity, 0.0);

        production_mut(&mut game, id).potential_cost = 0.0;
        draw(&mut game, id);
        assert_eq!(production(&game, id).needle_angle, 0.25 - 0.002);
    }

    #[test]
    fn the_settled_needle_keeps_bouncing_infinitesimally_forever() {
        // A faithful quirk: the bounce loses 52.5% of its speed but gains a
        // constant 0.002 first, so the velocity converges on -0.002*0.475/1.475
        // rather than on zero, and the `velocity < 0` arm keeps running. Costs
        // nothing -- the needle is pinned at zero -- but it means "at rest" is
        // never a state you can test for.
        let mut game = Game::new();
        let id = spawn_factory(&mut game, 500.0);
        production_mut(&mut game, id).needle_angle = 0.25;

        for _ in 0..2000 {
            draw(&mut game, id);
        }

        assert_eq!(production(&game, id).needle_angle, 0.0);
        let velocity = production(&game, id).needle_velocity;
        assert!(
            (velocity - (-0.002 * 0.475 / 1.475)).abs() < 1e-12,
            "{velocity}"
        );
    }

    #[test]
    fn a_halted_factory_freezes_its_dial() {
        let mut game = Game::new();
        let id = spawn_factory(&mut game, 500.0);
        production_mut(&mut game, id).halt_production = true;
        let before = *production(&game, id);

        draw(&mut game, id);

        assert_eq!(*production(&game, id), before);
    }

    #[test]
    fn the_health_bar_scrolls_only_while_the_button_is_up() {
        let mut game = Game::new();
        let id = spawn_factory(&mut game, 500.0);

        hold(&mut game, id, true);
        draw(&mut game, id);
        assert_eq!(production(&game, id).texcoord_scroller, -0.25);

        hold(&mut game, id, false);
        draw(&mut game, id);
        assert_eq!(production(&game, id).texcoord_scroller, -0.24);

        // and it wraps rather than running away
        for _ in 0..200 {
            draw(&mut game, id);
        }
        assert!(production(&game, id).texcoord_scroller <= 0.25);
    }
}
