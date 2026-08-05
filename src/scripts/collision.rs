//! `scripts/collision.lua` (per-actor registration) and the detection half of
//! `components/collision.lua` (the singleton n^2 check).
//!
//! Detection and resolution both live here, in that order, matching the Lua's
//! single `collision_check` pass: each hit is resolved as it is found, so a ship
//! taking two bullets in one frame can die to the first and still be hit by the
//! second. That is the original's behaviour, and it matters for the stats log,
//! which caps recorded damage at the target's remaining health.

use crate::collision::{Body, Polygon, collide};
use crate::game::Game;
use crate::scripts::ship;
use crate::v2::V2;
use crate::world::ActorId;

/// `collision_type` in the Lua: bullets hit ships, and nothing hits its own
/// kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollisionType {
    Ship,
    Bullet,
}

#[derive(Clone, Debug)]
pub struct Collision {
    pub poly: Polygon,
    pub collision_type: CollisionType,
    /// Damage dealt on contact. Only meaningful for bullets.
    pub damage: f64,
}

/// A registration for this frame, snapshotted at `collision_registry` time.
///
/// The Lua caches one table per actor and overwrites `pos`/`facing` each frame;
/// the effect is a snapshot, which is what this is.
#[derive(Clone, Copy, Debug)]
struct Registration {
    actor: ActorId,
    collision_type: CollisionType,
    player: usize,
    pos: V2,
    facing: V2,
}

/// A detected contact, in the Lua's normalised order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hit {
    pub ship: ActorId,
    pub bullet: ActorId,
}

/// The registered bodies for the current frame.
#[derive(Debug, Default)]
pub struct CollisionWorld {
    bodies: Vec<Registration>,
    /// Contacts found by the last `check_all`, in detection order.
    pub hits: Vec<Hit>,
}

impl CollisionWorld {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn registered(&self) -> usize {
        self.bodies.len()
    }
}

/// `collision_registry`: hand this actor's body to the checker.
///
/// The player number comes from whichever of the ship or bullet scripts the
/// actor carries; an actor with a collision body always has exactly one.
pub fn register(game: &mut Game, id: ActorId) {
    let actor = game.world.get(id);

    let collision = actor
        .collision
        .as_ref()
        .expect("an actor running the collision script has a collision body");
    let transform = actor
        .transform
        .expect("a collision body needs a transform to place it");

    let registration = Registration {
        actor: id,
        collision_type: collision.collision_type,
        player: game.player_of(id),
        pos: transform.pos,
        facing: transform.facing,
    };

    game.collision.bodies.push(registration);
}

/// `collision_check`: every pair, once, in registration order.
///
/// Brute force, as the Lua comment cheerfully admits: "umm, looks like brute
/// force n^2 checking is fast enough". Keeping the same loop order keeps the
/// same hit order, which matters because hits are resolved in sequence and a
/// ship can die partway through.
pub fn check_all(game: &mut Game) {
    let bodies = std::mem::take(&mut game.collision.bodies);
    let mut hits = Vec::new();

    for (i, first) in bodies.iter().enumerate() {
        for second in &bodies[i + 1..] {
            // a bullet cannot hit its own team, and like never hits like
            if first.collision_type == second.collision_type || first.player == second.player {
                continue;
            }

            if !overlapping(game, first, second) {
                continue;
            }

            // the Lua swaps so that body1 is always the ship
            let (ship, bullet) = if first.collision_type == CollisionType::Bullet {
                (second.actor, first.actor)
            } else {
                (first.actor, second.actor)
            };

            let hit = Hit { ship, bullet };
            hits.push(hit);
            resolve(game, hit);
        }
    }

    game.collision.hits = hits;
    // the Lua clears the body list at the end of the check; `mem::take` above
    // already did, and leaves the allocation with us to reuse
    game.collision.bodies.clear();
}

/// Applies one hit: log it, throw sparks, damage the ship, consume the bullet.
///
/// The order is the Lua's. Logging happens before the damage lands because the
/// log caps what it records at the target's remaining health.
fn resolve(game: &mut Game, hit: Hit) {
    let damage = game
        .world
        .get(hit.bullet)
        .collision
        .as_ref()
        .expect("a bullet that hit something has a collision body")
        .damage;
    let hit_points = game
        .world
        .get(hit.ship)
        .ship
        .as_ref()
        .expect("a ship that was hit has a ship script")
        .hit_points;

    game.log.record_hit(
        game.world.get(hit.ship).blueprint,
        game.world.get(hit.bullet).blueprint,
        damage,
        hit_points,
    );

    bullet_hit(game, hit);
    ship::damage(game, hit.ship, damage);
    game.world.kill(hit.bullet);
}

/// `bullet_hit` from `components/particles.lua`: the visual response to a
/// strike, and a meaningful consumer of the random stream.
fn bullet_hit(game: &mut Game, hit: Hit) {
    let bullet = game.world.get(hit.bullet);
    let pos = bullet.transform.expect("transform").pos;

    // a missile is a `ship`, not a `bullet`, so its velocity lives elsewhere
    let bullet_velocity = match (bullet.bullet, bullet.ship.as_ref()) {
        (Some(b), _) => b.velocity,
        (None, Some(s)) => s.velocity,
        (None, None) => panic!("a projectile has either a bullet or a ship script"),
    };

    let bullet_direction = if bullet_velocity.sqrmag() == 0.0 {
        V2::ZERO
    } else {
        bullet_velocity.norm()
    };

    let ship_velocity = game
        .world
        .get(hit.ship)
        .ship
        .as_ref()
        .expect("ship script")
        .velocity;
    let velocity = ship_velocity - bullet_direction * 1.5;

    match game.world.get(hit.bullet).blueprint {
        "laser" => game.particles.laser_hit(&mut game.rng, pos, velocity),
        "bomb" => game.particles.explode_mid(&mut game.rng, pos),
        "missile" => game.particles.explode_tiny(&mut game.rng, pos),
        _ => {}
    }
}

fn overlapping(game: &Game, first: &Registration, second: &Registration) -> bool {
    let poly = |id: ActorId| {
        &game
            .world
            .get(id)
            .collision
            .as_ref()
            .expect("registered bodies keep their collision script")
            .poly
    };

    let body1 = Body {
        pos: first.pos,
        facing: first.facing,
        poly: poly(first.actor),
    };
    let body2 = Body {
        pos: second.pos,
        facing: second.facing,
        poly: poly(second.actor),
    };

    collide(&body1, &body2).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scripts::ScriptKind;
    use crate::scripts::bullet::Bullet;
    use crate::scripts::ship::Ship;
    use crate::scripts::transform::Transform;
    use crate::v2::v2;
    use crate::world::{Actor, Phase, run_phase};

    fn spawn(
        game: &mut Game,
        name: &'static str,
        pos: V2,
        collision_type: CollisionType,
        player: usize,
    ) -> ActorId {
        // ships carry a `ship` script and bullets a `bullet` one; resolution
        // reads both, so the fixture has to be as complete as a real blueprint
        let (ship, bullet) = match collision_type {
            CollisionType::Ship => (Some(Ship::new(0.025, 0.1, 100.0, None)), None),
            CollisionType::Bullet => (
                None,
                Some(Bullet {
                    velocity: v2(1.0, 0.0),
                }),
            ),
        };

        let actor = Actor {
            scripts: vec![ScriptKind::Transform, ScriptKind::Collision],
            transform: Some(Transform::at(pos)),
            collision: Some(Collision {
                poly: Polygon::rectangle(10.0, 10.0),
                collision_type,
                damage: 10.0,
            }),
            ship,
            bullet,
            ..Actor::new(name)
        };
        let id = game.world.spawn(actor);
        game.set_player(id, player);
        id
    }

    fn checker(game: &mut Game) {
        let actor = Actor {
            scripts: vec![ScriptKind::CollisionChecker],
            ..Actor::new("collision")
        };
        game.world.spawn(actor);
    }

    fn run(game: &mut Game) {
        run_phase(game, Phase::CollisionRegistry);
        run_phase(game, Phase::CollisionCheck);
    }

    #[test]
    fn bullet_hits_an_enemy_ship() {
        let mut game = Game::new();
        checker(&mut game);
        let ship = spawn(&mut game, "fighter", V2::ZERO, CollisionType::Ship, 1);
        let bullet = spawn(&mut game, "laser", v2(2.0, 0.0), CollisionType::Bullet, 2);

        run(&mut game);

        assert_eq!(game.collision.hits, [Hit { ship, bullet }]);
    }

    #[test]
    fn hit_is_normalised_regardless_of_registration_order() {
        // same pair, bullet registered first
        let mut game = Game::new();
        checker(&mut game);
        let bullet = spawn(&mut game, "laser", v2(2.0, 0.0), CollisionType::Bullet, 2);
        let ship = spawn(&mut game, "fighter", V2::ZERO, CollisionType::Ship, 1);

        run(&mut game);

        assert_eq!(game.collision.hits, [Hit { ship, bullet }]);
    }

    #[test]
    fn friendly_fire_is_ignored() {
        let mut game = Game::new();
        checker(&mut game);
        spawn(&mut game, "fighter", V2::ZERO, CollisionType::Ship, 1);
        spawn(&mut game, "laser", v2(2.0, 0.0), CollisionType::Bullet, 1);

        run(&mut game);

        assert!(game.collision.hits.is_empty());
    }

    #[test]
    fn ships_do_not_collide_with_each_other() {
        let mut game = Game::new();
        checker(&mut game);
        spawn(&mut game, "fighter", V2::ZERO, CollisionType::Ship, 1);
        spawn(&mut game, "bomber", v2(2.0, 0.0), CollisionType::Ship, 2);

        run(&mut game);

        assert!(game.collision.hits.is_empty());
    }

    #[test]
    fn distant_bodies_do_not_hit() {
        let mut game = Game::new();
        checker(&mut game);
        spawn(&mut game, "fighter", V2::ZERO, CollisionType::Ship, 1);
        spawn(&mut game, "laser", v2(900.0, 0.0), CollisionType::Bullet, 2);

        run(&mut game);

        assert!(game.collision.hits.is_empty());
    }

    #[test]
    fn registrations_are_cleared_between_frames() {
        let mut game = Game::new();
        checker(&mut game);
        spawn(&mut game, "fighter", V2::ZERO, CollisionType::Ship, 1);
        spawn(&mut game, "laser", v2(2.0, 0.0), CollisionType::Bullet, 2);

        run(&mut game);
        assert_eq!(game.collision.registered(), 0);

        // the bullet was consumed by the hit, so the second frame has nothing
        // left to register and finds nothing
        run(&mut game);
        assert_eq!(game.collision.registered(), 0);
        assert!(game.collision.hits.is_empty());
    }

    #[test]
    fn a_hit_damages_the_ship_and_consumes_the_bullet() {
        let mut game = Game::new();
        checker(&mut game);
        let ship = spawn(&mut game, "fighter", V2::ZERO, CollisionType::Ship, 1);
        let bullet = spawn(&mut game, "laser", v2(2.0, 0.0), CollisionType::Bullet, 2);

        run(&mut game);

        assert_eq!(game.world.get(ship).ship.as_ref().unwrap().hit_points, 90.0);
        assert!(game.world.is_dead(bullet));
        assert!(!game.world.is_dead(ship));
    }

    #[test]
    fn a_hit_is_recorded_in_the_stats_log() {
        let mut game = Game::new();
        checker(&mut game);
        spawn(&mut game, "fighter", V2::ZERO, CollisionType::Ship, 1);
        spawn(&mut game, "laser", v2(2.0, 0.0), CollisionType::Bullet, 2);

        run(&mut game);

        assert_eq!(game.log.accuracy_hits.fighter, 1.0);
        assert_eq!(game.log.damage_given.fighter, 10.0);
        assert_eq!(game.log.fighter_damaged_by.fighter, 10.0);
    }

    #[test]
    fn overkill_is_capped_in_the_log_but_not_in_the_damage() {
        let mut game = Game::new();
        checker(&mut game);
        let ship = spawn(&mut game, "fighter", V2::ZERO, CollisionType::Ship, 1);
        game.world.get_mut(ship).ship.as_mut().unwrap().hit_points = 4.0;
        spawn(&mut game, "laser", v2(2.0, 0.0), CollisionType::Bullet, 2);

        run(&mut game);

        assert_eq!(
            game.log.damage_given.fighter, 4.0,
            "log caps at remaining health"
        );
        assert_eq!(game.world.get(ship).ship.as_ref().unwrap().hit_points, 0.0);
    }

    #[test]
    fn a_laser_hit_takes_twenty_draws() {
        // resolution emits sparks, and those come off the shared stream
        let mut game = Game::new();
        checker(&mut game);
        spawn(&mut game, "fighter", V2::ZERO, CollisionType::Ship, 1);
        spawn(&mut game, "laser", v2(2.0, 0.0), CollisionType::Bullet, 2);

        let mut expected = game.rng.clone();
        for _ in 0..20 {
            expected.next_u64();
        }

        run(&mut game);
        assert_eq!(expected, game.rng);
    }
}
