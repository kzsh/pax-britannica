//! `blueprints.lua`: what each kind of actor is made of.
//!
//! The Lua builds these from a list of script names and default field values,
//! resolved by name at spawn time. Here each is a constructor that fills in the
//! script structs directly -- typed, and the defaults become ordinary literals.
//!
//! **Script order is load-bearing.** It is the order the scripts run within an
//! actor, so it is preserved exactly as `blueprints.lua` lists it.

use crate::collision::Polygon;
use crate::constants::PLAY_SCALE;
use crate::resources::SpriteId;
use crate::rng::LuaRng;
use crate::scripts::ScriptKind;
use crate::scripts::bomber_ai::BomberAi;
use crate::scripts::bullet::Bullet;
use crate::scripts::collision::{Collision, CollisionType};
use crate::scripts::countdown::{Countdown, CountdownCallback};
use crate::scripts::debris::Debris;
use crate::scripts::easy_enemy_production::EasyEnemyProduction;
use crate::scripts::factory_ai::FactoryAi;
use crate::scripts::factory_damage::FactoryDamage;
use crate::scripts::fade::{Fade, FadeCallback};
use crate::scripts::fighter_ai::FighterAi;
use crate::scripts::fish::Fish;
use crate::scripts::frigate_ai::FrigateAi;
use crate::scripts::game_flow::GameFlow;
use crate::scripts::heatseeking_ai::HeatseekingAi;
use crate::scripts::production::Production;
use crate::scripts::resources::Resources;
use crate::scripts::selector::Selector;
use crate::scripts::ship::{Ship, ShipSprites};
use crate::scripts::shooting::Weapon;
use crate::scripts::sprite::{Color, Sprite};
use crate::scripts::transform::Transform;
use crate::v2::V2;
use crate::world::Actor;

/// `scripts/ship.lua`'s chunk body: a ship wears its owner's colours at full
/// brightness. The blueprints leave the sprite blank and the script fills it in
/// at spawn, which is one step in Lua and one call here.
fn ship_sprite(sprites: ShipSprites, player: usize) -> Sprite {
    Sprite {
        image: Some(sprites.for_player(player)),
        color: Some(Color::WHITE),
    }
}

/// A laser: cheap, fast, fired by fighters.
pub fn laser(player: usize, pos: V2, facing: V2, velocity: V2) -> Actor {
    Actor {
        scripts: vec![
            ScriptKind::Transform,
            ScriptKind::Sprite,
            ScriptKind::Collision,
            ScriptKind::Bullet,
        ],
        transform: Some(Transform::facing(pos, facing)),
        sprite: Some(Sprite::new(SpriteId::Laser)),
        collision: Some(Collision {
            poly: Polygon::rectangle(32.0, 1.0),
            collision_type: CollisionType::Bullet,
            damage: 10.0,
        }),
        bullet: Some(Bullet { velocity }),
        player: Some(player),
        ..Actor::new("laser")
    }
}

/// A bomb: slow, enormously damaging, lobbed sideways by bombers.
pub fn bomb(player: usize, pos: V2, facing: V2, velocity: V2) -> Actor {
    Actor {
        scripts: vec![
            ScriptKind::Transform,
            ScriptKind::Sprite,
            ScriptKind::Collision,
            ScriptKind::Bullet,
        ],
        transform: Some(Transform::facing(pos, facing)),
        sprite: Some(Sprite::new(SpriteId::Bomb)),
        collision: Some(Collision {
            poly: Polygon::rectangle(4.0, 4.0),
            collision_type: CollisionType::Bullet,
            damage: 200.0,
        }),
        bullet: Some(Bullet { velocity }),
        player: Some(player),
        ..Actor::new("bomb")
    }
}

/// A missile: a projectile that steers, so it carries `ship` and an AI rather
/// than the `bullet` script. Everything that inspects projectiles has to cope
/// with that.
pub fn missile(player: usize, pos: V2, velocity: V2) -> Actor {
    let mut ship = Ship::new(0.055, 0.15, 1.0, None);
    ship.velocity = velocity;

    Actor {
        scripts: vec![
            ScriptKind::Transform,
            ScriptKind::Sprite,
            ScriptKind::Collision,
            ScriptKind::Ship,
            ScriptKind::HeatseekingAi,
        ],
        transform: Some(Transform::at(pos)),
        sprite: Some(Sprite::new(SpriteId::Missile)),
        collision: Some(Collision {
            poly: Polygon::rectangle(5.0, 2.0),
            collision_type: CollisionType::Bullet,
            damage: 40.0,
        }),
        ship: Some(ship),
        heatseeking_ai: Some(HeatseekingAi::default()),
        player: Some(player),
        ..Actor::new("missile")
    }
}

pub fn fighter(player: usize, pos: V2, facing: V2) -> Actor {
    Actor {
        scripts: vec![
            ScriptKind::Transform,
            ScriptKind::Sprite,
            ScriptKind::Collision,
            ScriptKind::Ship,
            ScriptKind::FighterShooting,
            ScriptKind::FighterAi,
        ],
        transform: Some(Transform::facing(pos, facing)),
        sprite: Some(ship_sprite(ShipSprites::Fighter, player)),
        collision: Some(Collision {
            poly: Polygon::rectangle(9.0, 6.0),
            collision_type: CollisionType::Ship,
            damage: 0.0,
        }),
        ship: Some(Ship::new(0.025, 0.1, 40.0, Some(ShipSprites::Fighter))),
        fighter_shooting: Some(Weapon::fighter()),
        fighter_ai: Some(FighterAi::default()),
        player: Some(player),
        ..Actor::new("fighter")
    }
}

pub fn bomber(player: usize, pos: V2, facing: V2) -> Actor {
    Actor {
        scripts: vec![
            ScriptKind::Transform,
            ScriptKind::Sprite,
            ScriptKind::Collision,
            ScriptKind::Ship,
            ScriptKind::BomberAi,
        ],
        transform: Some(Transform::facing(pos, facing)),
        sprite: Some(ship_sprite(ShipSprites::Bomber, player)),
        collision: Some(Collision {
            poly: Polygon::rectangle(22.0, 14.0),
            collision_type: CollisionType::Ship,
            damage: 0.0,
        }),
        ship: Some(Ship::new(0.03, 0.05, 250.0, Some(ShipSprites::Bomber))),
        bomber_ai: Some(BomberAi::default()),
        player: Some(player),
        ..Actor::new("bomber")
    }
}

pub fn frigate(player: usize, pos: V2, facing: V2) -> Actor {
    Actor {
        scripts: vec![
            ScriptKind::Transform,
            ScriptKind::Sprite,
            ScriptKind::Collision,
            ScriptKind::Ship,
            ScriptKind::FrigateShooting,
            ScriptKind::FrigateAi,
        ],
        transform: Some(Transform::facing(pos, facing)),
        sprite: Some(ship_sprite(ShipSprites::Frigate, player)),
        collision: Some(Collision {
            poly: Polygon::rectangle(54.0, 36.0),
            collision_type: CollisionType::Ship,
            damage: 0.0,
        }),
        ship: Some(Ship::new(0.01, 0.01, 1400.0, Some(ShipSprites::Frigate))),
        frigate_shooting: Some(Weapon::frigate()),
        frigate_ai: Some(FrigateAi::default()),
        player: Some(player),
        ..Actor::new("frigate")
    }
}

/// A factory's constant left turn, from `blueprints.lua`.
///
/// The circle it wanders is its terminal speed over this rate, so dividing by
/// the field scale makes that circle grow with the field instead of leaving the
/// factories crowded into the middle of a bigger sea.
const FACTORY_TURN_SPEED: f64 = 0.00028 / PLAY_SCALE;
const FACTORY_ACCEL: f64 = 0.002;

/// The shared half of the two factory blueprints: everything down to the
/// production dial. They differ only in who works the button.
fn factory(player: usize, pos: V2, facing: V2) -> Actor {
    Actor {
        scripts: vec![
            ScriptKind::Transform,
            ScriptKind::Sprite,
            ScriptKind::Collision,
            ScriptKind::Ship,
            ScriptKind::FactoryDamage,
            ScriptKind::FactoryAi,
            ScriptKind::Resources,
            ScriptKind::Production,
        ],
        transform: Some(Transform::facing(pos, facing)),
        sprite: Some(ship_sprite(ShipSprites::Factory, player)),
        collision: Some(Collision {
            poly: Polygon::rectangle(170.0, 100.0),
            collision_type: CollisionType::Ship,
            damage: 0.0,
        }),
        ship: Some(Ship::new(
            FACTORY_TURN_SPEED,
            FACTORY_ACCEL,
            20000.0,
            Some(ShipSprites::Factory),
        )),
        factory_damage: Some(FactoryDamage::default()),
        factory_ai: Some(FactoryAi::default()),
        resources: Some(Resources::default()),
        production: Some(Production::default()),
        player: Some(player),
        ..Actor::new("factory")
    }
}

/// A factory driven by a human on a button.
pub fn player_factory(player: usize, pos: V2, facing: V2) -> Actor {
    let mut actor = factory(player, pos, facing);
    actor.scripts.push(ScriptKind::PlayerProduction);
    actor
}

/// A factory driven by the canned build orders in
/// `scripts/easy_enemy_production.lua`.
///
/// Takes the rng because that script's chunk body runs at spawn: three draws,
/// and a 20% cut to the factory's income. See its doc comment.
pub fn easy_enemy_factory(rng: &mut LuaRng, player: usize, pos: V2, facing: V2) -> Actor {
    let mut actor = factory(player, pos, facing);
    actor.scripts.push(ScriptKind::EasyEnemyProduction);

    let resources = actor
        .resources
        .as_mut()
        .expect("the factory blueprint carries resources");
    actor.easy_enemy_production = Some(EasyEnemyProduction::new(rng, resources));

    actor
}

/// `blueprints.background_fx`: the spawner that drops debris and fish.
pub fn background_fx() -> Actor {
    Actor {
        scripts: vec![ScriptKind::BackgroundFx],
        ..Actor::new("background_fx")
    }
}

/// `blueprints.debris`. The position and the sprite are picked by
/// `scripts/background_fx.lua`; the rest of the mote's character is in
/// [`Debris`].
pub fn debris(pos: V2, sprite: usize, debris: Debris) -> Actor {
    Actor {
        scripts: vec![
            ScriptKind::Transform,
            ScriptKind::Debris,
            ScriptKind::Sprite,
        ],
        transform: Some(Transform::at(pos)),
        debris: Some(debris),
        sprite: Some(Sprite {
            image: Some(SpriteId::Debris(sprite)),
            color: Some(Color::rgba(1.0, 1.0, 1.0, 0.0)),
        }),
        ..Actor::new("debris")
    }
}

/// `blueprints.fish`.
pub fn fish(pos: V2, sprite: usize, fish: Fish) -> Actor {
    Actor {
        scripts: vec![ScriptKind::Transform, ScriptKind::Fish, ScriptKind::Sprite],
        transform: Some(Transform::at(pos)),
        fish: Some(fish),
        sprite: Some(Sprite {
            image: Some(SpriteId::Fish(sprite)),
            color: Some(Color::rgba(1.0, 1.0, 1.0, 0.0)),
        }),
        ..Actor::new("fish")
    }
}

/// `blueprints.game_flow`: the scene state machine, one per scene.
pub fn game_flow() -> Actor {
    Actor {
        scripts: vec![ScriptKind::GameFlow],
        game_flow: Some(GameFlow::default()),
        ..Actor::new("game_flow")
    }
}

/// `blueprints.splash`: the title and credits on the menu screen.
pub fn splash() -> Actor {
    Actor {
        scripts: vec![ScriptKind::Splash],
        ..Actor::new("splash")
    }
}

/// `blueprints.selection_factory`: one of the four "press to join" factories.
///
/// The sprite is set twice, as in the Lua: the blueprint dims it to 0.2 grey,
/// and the script body then picks the player's own factory art.
pub fn selection_factory(player: usize, pos: V2) -> Actor {
    Actor {
        scripts: vec![
            ScriptKind::Transform,
            ScriptKind::Sprite,
            ScriptKind::Selector,
        ],
        transform: Some(Transform::facing(pos, V2::J)),
        sprite: Some(Sprite {
            image: Some(SpriteId::Factory(player)),
            color: Some(Color::rgb(0.2, 0.2, 0.2)),
        }),
        selector: Some(Selector::new(player)),
        ..Actor::new("selection_factory")
    }
}

/// `blueprints.countdown`. The blueprint's own position is the middle of the
/// screen; `scripts/game_flow.lua` always overrides it, and so does this.
pub fn countdown(pos: V2, callback: CountdownCallback) -> Actor {
    Actor {
        scripts: vec![
            ScriptKind::Countdown,
            ScriptKind::Sprite,
            ScriptKind::Transform,
        ],
        countdown: Some(Countdown::new(callback)),
        sprite: Some(Sprite::blank()),
        transform: Some(Transform::at(pos)),
        ..Actor::new("countdown")
    }
}

/// `blueprints.fade_in`: black to clear over one second.
pub fn fade_in() -> Actor {
    fade(Fade::fade_in())
}

/// `blueprints.fade_out`: clear to black over one second, then the callback.
pub fn fade_out(callback: FadeCallback) -> Actor {
    fade(Fade::fade_out(callback))
}

fn fade(fade: Fade) -> Actor {
    Actor {
        scripts: vec![ScriptKind::Fade],
        fade: Some(fade),
        ..Actor::new("fade")
    }
}

/// The generic actor `components/collision.lua` creates to get its
/// `collision_check` callback.
pub fn collision_checker() -> Actor {
    Actor {
        scripts: vec![ScriptKind::CollisionChecker],
        ..Actor::new("collision")
    }
}

/// The generic actor `components/log.lua` creates to count elapsed frames.
pub fn log_timer() -> Actor {
    Actor {
        scripts: vec![ScriptKind::LogTimer],
        ..Actor::new("log")
    }
}

/// The generic actor `components/particles.lua` creates to age its emitters.
///
/// Spawned last, so the ageing happens after every script that might have
/// emitted this frame, and the two draw layers land on top of everything.
pub fn particle_emitters() -> Actor {
    Actor {
        scripts: vec![ScriptKind::ParticleEmitters],
        ..Actor::new("particles")
    }
}

/// The generic actor `components/the_one_button.lua` creates to latch input in
/// `update_setup`.
pub fn the_one_button() -> Actor {
    Actor {
        scripts: vec![ScriptKind::TheOneButton],
        ..Actor::new("the_one_button")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::v2;

    #[test]
    fn a_bombers_script_order_matches_the_blueprint() {
        // the ship moves before the AI steers it, every frame
        let actor = bomber(1, V2::ZERO, V2::I);
        assert_eq!(
            actor.scripts,
            [
                ScriptKind::Transform,
                ScriptKind::Sprite,
                ScriptKind::Collision,
                ScriptKind::Ship,
                ScriptKind::BomberAi,
            ]
        );
    }

    #[test]
    fn a_missile_is_a_ship_not_a_bullet() {
        let actor = missile(1, V2::ZERO, v2(1.0, 0.0));
        assert!(actor.bullet.is_none());
        assert!(actor.ship.is_some());
        assert_eq!(actor.ship.as_ref().unwrap().velocity, v2(1.0, 0.0));
    }

    #[test]
    fn projectile_damage_matches_the_blueprints() {
        let damage = |a: Actor| a.collision.as_ref().unwrap().damage;
        assert_eq!(damage(laser(1, V2::ZERO, V2::I, V2::ZERO)), 10.0);
        assert_eq!(damage(bomb(1, V2::ZERO, V2::I, V2::ZERO)), 200.0);
        assert_eq!(damage(missile(1, V2::ZERO, V2::ZERO)), 40.0);
    }

    #[test]
    fn every_ship_wears_its_owners_colours() {
        // `scripts/ship.lua` sets this at spawn; without it a ship is invisible
        let image = |a: Actor| a.sprite.as_ref().and_then(|s| s.image);
        assert_eq!(
            image(fighter(2, V2::ZERO, V2::I)),
            Some(SpriteId::Fighter(2))
        );
        assert_eq!(image(bomber(3, V2::ZERO, V2::I)), Some(SpriteId::Bomber(3)));
        assert_eq!(
            image(frigate(4, V2::ZERO, V2::I)),
            Some(SpriteId::Frigate(4))
        );
        assert_eq!(
            image(player_factory(1, V2::ZERO, V2::I)),
            Some(SpriteId::Factory(1))
        );
    }

    #[test]
    fn every_drawable_blueprint_has_an_image() {
        // anything carrying the sprite script and no image draws nothing at all
        let mut rng = LuaRng::new(1, 0);
        let actors = [
            laser(1, V2::ZERO, V2::I, V2::ZERO),
            bomb(1, V2::ZERO, V2::I, V2::ZERO),
            missile(1, V2::ZERO, V2::ZERO),
            fighter(1, V2::ZERO, V2::I),
            bomber(1, V2::ZERO, V2::I),
            frigate(1, V2::ZERO, V2::I),
            player_factory(1, V2::ZERO, V2::I),
            easy_enemy_factory(&mut rng, 2, V2::ZERO, V2::I),
            selection_factory(1, V2::ZERO),
        ];

        for actor in actors {
            if !actor.has(ScriptKind::Sprite) {
                continue;
            }
            assert!(
                actor.sprite.as_ref().and_then(|s| s.image).is_some(),
                "{} has the sprite script but no image",
                actor.blueprint
            );
        }
    }

    #[test]
    fn ships_start_at_full_health() {
        for actor in [
            fighter(1, V2::ZERO, V2::I),
            bomber(1, V2::ZERO, V2::I),
            frigate(1, V2::ZERO, V2::I),
        ] {
            let ship = actor.ship.as_ref().unwrap();
            assert_eq!(ship.hit_points, ship.max_hit_points);
        }
    }
}
