//! `scripts/ship.lua`: movement, health, and dying.
//!
//! Every combatant carries this, from a 9x6 fighter to the 170x100 factory, and
//! the AI scripts steer through `turn`/`thrust`/`go_towards`.

use crate::game::Game;
use crate::particles::Particles;
use crate::resources::SpriteId;
use crate::rng::LuaRng;
use crate::scripts::sprite::Color;
use crate::v2::V2;
use crate::world::ActorId;

/// Velocity retained per frame. Everything drifts to a halt without thrust.
const DRAG: f64 = 0.97;

/// Frames a factory spends dying before it is finally removed.
const DEATH_COUNTER: i32 = 100;

#[derive(Clone, Debug)]
pub struct Ship {
    pub turn_speed: f64,
    pub accel: f64,
    pub hit_points: f64,
    pub max_hit_points: f64,
    pub velocity: V2,
    /// Which per-player sprite set to wear, if any.
    pub sprites: Option<ShipSprites>,

    // `factory_destruct`'s chunk-level locals, which are per-instance state
    death_counter: i32,
    next_explosion: i32,
    opacity: f64,

    /// `scripts/ship.lua` assigns its scratch vector to an undeclared name in
    /// `factory_destruct`, so it lands in the script's own table and shows up in
    /// the state trace. Kept so the Rust trace can match.
    pub random_scratch: Option<V2>,
}

/// Which sprite family a ship wears. `sprites_table` in the Lua.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShipSprites {
    Fighter,
    Bomber,
    Frigate,
    Factory,
}

impl ShipSprites {
    pub fn for_player(self, player: usize) -> SpriteId {
        match self {
            ShipSprites::Fighter => SpriteId::Fighter(player),
            ShipSprites::Bomber => SpriteId::Bomber(player),
            ShipSprites::Frigate => SpriteId::Frigate(player),
            ShipSprites::Factory => SpriteId::Factory(player),
        }
    }
}

impl Ship {
    pub fn new(turn_speed: f64, accel: f64, hit_points: f64, sprites: Option<ShipSprites>) -> Self {
        Self {
            turn_speed,
            accel,
            hit_points,
            max_hit_points: hit_points,
            velocity: V2::ZERO,
            sprites,
            death_counter: DEATH_COUNTER,
            next_explosion: 10,
            opacity: 0.6,
            random_scratch: None,
        }
    }

    /// `health_percentage()`, floored at zero.
    pub fn health_percentage(&self) -> f64 {
        (self.hit_points / self.max_hit_points).max(0.0)
    }
}

/// `damage(amount)`. Never drives hit points below zero.
pub fn damage(game: &mut Game, id: ActorId, amount: f64) {
    let ship = ship_mut(game, id);
    ship.hit_points = (ship.hit_points - amount).max(0.0);
}

/// `turn(direction, amount)`. `direction` is 1 for left, -1 for right.
pub fn turn(game: &mut Game, id: ActorId, direction: f64, amount: f64) {
    let turn_speed = ship(game, id).turn_speed;
    let transform = game
        .world
        .get_mut(id)
        .transform
        .as_mut()
        .expect("transform");
    transform.facing = transform
        .facing
        .rotate(turn_speed * direction * amount)
        .norm();
}

/// `thrust(amount)`: accelerate along the facing.
pub fn thrust(game: &mut Game, id: ActorId, amount: f64) {
    let facing = game.world.get(id).transform.expect("transform").facing;
    let ship = ship_mut(game, id);
    ship.velocity = ship.velocity + facing * ship.accel * amount;
}

/// `go_towards` / `go_away`: turn to face the target (or away), and thrust when
/// already pointing roughly the right way.
pub fn go_towards_or_away(
    game: &mut Game,
    id: ActorId,
    target_pos: V2,
    force_thrust: bool,
    is_away: bool,
) {
    let transform = game.world.get(id).transform.expect("transform");
    let mut target_direction = target_pos - transform.pos;
    if is_away {
        target_direction = -target_direction;
    }

    let direction = if transform.facing.cross(target_direction) > 0.0 {
        1.0
    } else {
        -1.0
    };
    turn(game, id, direction, 1.0);

    if force_thrust || transform.facing.dot(target_direction) > 0.0 {
        thrust(game, id, 1.0);
    }
}

pub fn go_towards(game: &mut Game, id: ActorId, target_pos: V2, force_thrust: bool) {
    go_towards_or_away(game, id, target_pos, force_thrust, false);
}

pub fn go_away(game: &mut Game, id: ActorId, target_pos: V2, force_thrust: bool) {
    go_towards_or_away(game, id, target_pos, force_thrust, true);
}

/// A uniformly random point on the ship's rectangular hull, in world space.
///
/// Two draws, and only taken when the caller has already decided to emit -- the
/// bubble check in `update` guards it, so the draw count depends on the ship's
/// speed. Assumes a rectangle, as the Lua comment says.
fn random_point_on_ship(game: &mut Game, id: ActorId) -> V2 {
    let actor = game.world.get(id);
    let transform = actor.transform.expect("transform");
    let vertices = actor
        .collision
        .as_ref()
        .expect("random_point_on_ship needs a hull")
        .poly
        .vertices();

    let a = vertices[0];
    let ab = vertices[1] - a;
    let ac = vertices[3] - a;

    let along_ab = game.rng.next_f64();
    let along_ac = game.rng.next_f64();
    let offset = a + ab * along_ab + ac * along_ac;

    transform.pos + transform.facing.rotate_to(offset)
}

/// `update()`: drift, drag, die, and occasionally trail a bubble.
pub fn update(game: &mut Game, id: ActorId) {
    let velocity = {
        let ship = ship_mut(game, id);
        ship.velocity = ship.velocity * DRAG;
        ship.velocity
    };

    let transform = game
        .world
        .get_mut(id)
        .transform
        .as_mut()
        .expect("transform");
    transform.pos = transform.pos + velocity;

    if ship(game, id).hit_points <= 0.0 {
        destruct(game, id);
    }

    // faster ships bubble more often; one draw every frame for every ship
    if game.rng.next_f64() < velocity.mag() / 10.0 {
        let pos = random_point_on_ship(game, id);
        game.particles.add_bubble(&mut game.rng, pos);
    }
}

/// `destruct()`: factories die slowly and theatrically, everything else at once.
pub fn destruct(game: &mut Game, id: ActorId) {
    if game.world.get(id).factory_ai.is_some() {
        factory_destruct(game, id);
        return;
    }

    let pos = game.world.get(id).transform.expect("transform").pos;
    explode_for(
        &mut game.particles,
        &mut game.rng,
        game.world.get(id).blueprint,
        pos,
    );
    game.log.record_death(game.world.get(id).blueprint);
    game.world.kill(id);
}

/// The drawn-out factory death: a hundred frames of secondary explosions and
/// fading, then a final flurry.
fn factory_destruct(game: &mut Game, id: ActorId) {
    let (death_counter, next_explosion, opacity) = {
        let ship = ship(game, id);
        (ship.death_counter, ship.next_explosion, ship.opacity)
    };

    if death_counter > 0 {
        if let Some(production) = game.world.get_mut(id).production.as_mut() {
            production.halt_production = true;
        }

        let sprite = game.world.get_mut(id).sprite.as_mut().expect("sprite");
        let base = sprite.color.unwrap_or(Color::WHITE);
        sprite.color = Some(base.with_alpha(opacity.max(0.0)));

        ship_mut(game, id).opacity = opacity - 0.006;

        if death_counter % next_explosion == 0 {
            let pos = random_point_on_ship(game, id);
            explode_for(&mut game.particles, &mut game.rng, "factory", pos);
            ship_mut(game, id).next_explosion = game.rng.next_range(6, 15) as i32;
        }

        ship_mut(game, id).death_counter = death_counter - 1;
        return;
    }

    let pos = game.world.get(id).transform.expect("transform").pos;
    for _ in 0..5 {
        let scatter = (V2::random(&mut game.rng) + V2::random(&mut game.rng)) * 20.0;
        ship_mut(game, id).random_scratch = Some(scatter);
        explode_for(&mut game.particles, &mut game.rng, "factory", pos + scatter);
    }

    game.log.record_death(game.world.get(id).blueprint);
    game.world.kill(id);
}

/// Which explosion a blueprint gets, from `components/particles.lua`'s
/// `explode`.
pub fn explode_for(particles: &mut Particles, rng: &mut LuaRng, blueprint: &str, pos: V2) {
    match blueprint {
        "factory" | "frigate" => particles.explode_big(rng, pos),
        "bomber" => particles.explode_mid(rng, pos),
        _ => particles.explode_small(rng, pos),
    }
}

fn ship(game: &Game, id: ActorId) -> &Ship {
    game.world.get(id).ship.as_ref().expect("ship script")
}

fn ship_mut(game: &mut Game, id: ActorId) -> &mut Ship {
    game.world.get_mut(id).ship.as_mut().expect("ship script")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collision::Polygon;
    use crate::scripts::ScriptKind;
    use crate::scripts::collision::{Collision, CollisionType};
    use crate::scripts::sprite::Sprite;
    use crate::scripts::transform::Transform;
    use crate::v2::v2;
    use crate::world::Actor;

    fn spawn_ship(game: &mut Game, hit_points: f64) -> ActorId {
        let actor = Actor {
            scripts: vec![ScriptKind::Transform, ScriptKind::Sprite, ScriptKind::Ship],
            transform: Some(Transform::default()),
            sprite: Some(Sprite::blank()),
            collision: Some(Collision {
                poly: Polygon::rectangle(10.0, 6.0),
                collision_type: CollisionType::Ship,
                damage: 0.0,
            }),
            ship: Some(Ship::new(0.025, 0.1, hit_points, None)),
            player: Some(1),
            ..Actor::new("fighter")
        };
        game.world.spawn(actor)
    }

    #[test]
    fn drag_slows_a_drifting_ship() {
        let mut game = Game::new();
        let id = spawn_ship(&mut game, 40.0);
        ship_mut(&mut game, id).velocity = v2(10.0, 0.0);

        update(&mut game, id);

        assert_eq!(ship(&game, id).velocity.x, 10.0 * DRAG);
        assert_eq!(game.world.get(id).transform.unwrap().pos.x, 10.0 * DRAG);
    }

    #[test]
    fn thrust_accelerates_along_the_facing() {
        let mut game = Game::new();
        let id = spawn_ship(&mut game, 40.0);
        thrust(&mut game, id, 1.0);
        assert_eq!(ship(&game, id).velocity, v2(0.1, 0.0));
    }

    #[test]
    fn turning_keeps_the_facing_normalised() {
        let mut game = Game::new();
        let id = spawn_ship(&mut game, 40.0);
        for _ in 0..500 {
            turn(&mut game, id, 1.0, 1.0);
        }
        let facing = game.world.get(id).transform.unwrap().facing;
        assert!((facing.mag() - 1.0).abs() < 1e-12, "{facing:?}");
    }

    #[test]
    fn damage_floors_at_zero() {
        let mut game = Game::new();
        let id = spawn_ship(&mut game, 40.0);
        damage(&mut game, id, 100.0);
        assert_eq!(ship(&game, id).hit_points, 0.0);
        assert_eq!(ship(&game, id).health_percentage(), 0.0);
    }

    #[test]
    fn a_ship_at_zero_health_dies_on_update() {
        let mut game = Game::new();
        let id = spawn_ship(&mut game, 40.0);
        damage(&mut game, id, 40.0);
        update(&mut game, id);
        assert!(game.world.is_dead(id));
    }

    #[test]
    fn a_stationary_ship_takes_exactly_one_draw_per_update() {
        // the bubble check is one draw; the two draws for the emission point are
        // only taken when it passes, which a motionless ship never does
        let mut game = Game::new();
        let id = spawn_ship(&mut game, 40.0);

        let before = game.rng.clone();
        update(&mut game, id);

        let mut counter = before;
        counter.next_u64();
        assert_eq!(counter, game.rng, "expected exactly one draw");
    }

    #[test]
    fn go_towards_turns_toward_the_target() {
        let mut game = Game::new();
        let id = spawn_ship(&mut game, 40.0);
        // facing +x, target above: should turn left (counterclockwise)
        go_towards(&mut game, id, v2(0.0, 100.0), false);
        assert!(game.world.get(id).transform.unwrap().facing.y > 0.0);
    }

    #[test]
    fn go_away_turns_from_the_target() {
        let mut game = Game::new();
        let id = spawn_ship(&mut game, 40.0);
        go_away(&mut game, id, v2(0.0, 100.0), false);
        assert!(game.world.get(id).transform.unwrap().facing.y < 0.0);
    }
}
