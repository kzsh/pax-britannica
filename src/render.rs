//! Phase 4: drawing a frame of the game with macroquad.
//!
//! The original renderer is immediate-mode OpenGL -- `glPushMatrix`, a quad per
//! sprite, a triangle fan per pie slice -- so this is a fairly direct
//! translation onto macroquad's `quad_gl`, which offers the same model-matrix
//! stack.
//!
//! **This pass is read-only.** The two draw methods that carry state or take
//! random draws (`scripts/production.lua`'s needle, `scripts/factory_damage.lua`'s
//! flicker) already ran inside [`the_game::step`](crate::the_game::step); what
//! is left here is pixels. That split is why the game can be run headless.
//!
//! The world is a fixed 1024x768 in game units, letterboxed to 4:3 inside
//! whatever the window happens to be.

use std::collections::HashMap;

use macroquad::prelude::*;

use crate::constants::{SCREEN_RIGHT, SCREEN_TOP};
use crate::game::Game;
use crate::resources::{self, Origin, SpriteId};
use crate::scripts::ScriptKind;
use crate::scripts::production::{self, UnitType};
use crate::scripts::sprite::Color as GameColor;
use crate::v2::V2;

/// Segments per pie slice, from `scripts/production.lua` and
/// `scripts/selector.lua` respectively.
const PRODUCTION_SEGMENTS: usize = 32;
const SELECTOR_SEGMENTS: usize = 16;

/// Radius of the dial's slices, shared by both.
const DIAL_RADIUS: f32 = 32.0;

/// How far behind the ship the dial sits, along its facing.
const PRODUCTION_DRAW_OFFSET: f64 = -4.0;
const SELECTOR_DRAW_OFFSET: f64 = -4.0;

pub struct Assets {
    textures: HashMap<SpriteId, Texture2D>,
}

impl Assets {
    /// Loads every sprite named in `components/resources.lua`.
    pub async fn load() -> Result<Self, macroquad::Error> {
        let mut textures = HashMap::new();

        for id in resources::all() {
            let meta = id.meta();
            let texture = load_texture(meta.path).await?;
            texture.set_filter(if meta.smooth {
                FilterMode::Linear
            } else {
                FilterMode::Nearest
            });
            textures.insert(id, texture);
        }

        Ok(Self { textures })
    }

    fn get(&self, id: SpriteId) -> Option<&Texture2D> {
        self.textures.get(&id)
    }
}

/// How much to shrink each axis to fit a 4:3 world into a window of `aspect`,
/// keeping the world centred: `kernel.set_ratio(4/3)`.
///
/// The letterbox is done in the projection rather than with a viewport because
/// `Camera2D::viewport` is in framebuffer pixels while `screen_width` is in
/// logical points, and the two differ on a HiDPI display. Normalised device
/// coordinates always span the whole framebuffer, whatever the scale factor is.
fn letterbox(aspect: f32) -> Vec2 {
    let ratio = (SCREEN_RIGHT / SCREEN_TOP) as f32;

    if aspect > ratio {
        // window wider than the game: full height, bars left and right
        vec2(ratio / aspect, 1.0)
    } else {
        vec2(1.0, aspect / ratio)
    }
}

/// A camera matching `glOrtho(0, 1024, 0, 768)`: origin bottom left, y upwards.
///
/// `zoom.y` is negative because `Camera2D::matrix` negates it again whenever the
/// camera draws to the screen rather than to a render target (macroquad 0.4.16,
/// `src/camera.rs:96`). The two cancel, and the world is y-up as the original's
/// `glOrtho` has it. Sprites are still drawn `flip_y`, since a texture's top row
/// is its first row whichever way the world runs.
fn camera() -> Camera2D {
    let fit = letterbox(screen_width() / screen_height());

    Camera2D {
        target: vec2(SCREEN_RIGHT as f32 / 2.0, SCREEN_TOP as f32 / 2.0),
        zoom: vec2(
            2.0 * fit.x / SCREEN_RIGHT as f32,
            -2.0 * fit.y / SCREEN_TOP as f32,
        ),
        ..Default::default()
    }
}

fn to_color(color: GameColor) -> Color {
    Color::new(
        color.r as f32,
        color.g as f32,
        color.b as f32,
        color.a as f32,
    )
}

fn grey(level: f32, alpha: f32) -> Color {
    Color::new(level, level, level, alpha)
}

/// `glPushMatrix; glTranslated; glRotated; glScaled`, as one matrix.
fn push_transform(pos: V2, facing: V2, scale: (f64, f64)) {
    let matrix = Mat4::from_translation(vec3(pos.x as f32, pos.y as f32, 0.0))
        * Mat4::from_rotation_z(facing.y.atan2(facing.x) as f32)
        * Mat4::from_scale(vec3(scale.0 as f32, scale.1 as f32, 1.0));
    push_matrix(matrix);
}

fn push_matrix(matrix: Mat4) {
    unsafe { get_internal_gl() }
        .quad_gl
        .push_model_matrix(matrix);
}

fn pop_matrix() {
    unsafe { get_internal_gl() }.quad_gl.pop_model_matrix();
}

/// One sprite at the current model matrix, positioned by its own origin.
///
/// `flip_y` because the camera has y running upwards while textures are stored
/// top row first.
fn draw_sprite(assets: &Assets, id: SpriteId, color: Color) {
    let Some(texture) = assets.get(id) else {
        return;
    };

    let (w, h) = (texture.width(), texture.height());
    let (ox, oy) = match id.meta().origin {
        Origin::Center => (w / 2.0, h / 2.0),
        Origin::At(x, y) => (x, y),
    };

    draw_texture_ex(
        texture,
        -ox,
        -oy,
        color,
        DrawTextureParams {
            dest_size: Some(vec2(w, h)),
            flip_y: true,
            ..Default::default()
        },
    );
}

/// A `GL_TRIANGLE_FAN` of unit vectors around the origin, as macroquad has no
/// fan primitive. `angle_at` gives the angle of each point in radians.
fn pie(segments: usize, radius: f32, color: Color, angle_at: impl Fn(f32) -> f32) {
    let point = |i: usize| {
        let angle = angle_at(i as f32 / segments as f32);
        vec2(angle.cos() * radius, angle.sin() * radius)
    };

    for i in 0..segments {
        draw_triangle(Vec2::ZERO, point(i), point(i + 1), color);
    }
}

/// One frame, in the order `the_game.lua`'s draw phases run it.
pub fn frame(game: &Game, assets: &Assets) {
    clear_background(BLACK);
    set_camera(&camera());

    for &id in game.world.order() {
        let actor = game.world.get(id);
        if actor.hidden {
            continue;
        }

        for &kind in &actor.scripts {
            match kind {
                ScriptKind::Sprite => draw_actor_sprite(game, assets, id),
                ScriptKind::FactoryDamage => draw_factory_damage(game, assets, id),
                ScriptKind::Production => draw_production(game, assets, id),
                ScriptKind::Selector => draw_selector(game, assets, id),
                ScriptKind::Splash => draw_splash(assets),
                _ => {}
            }
        }
    }

    // the particles actor is spawned last, so its two layers land on top of
    // everything: bubbles in `draw`, explosions and sparks in `draw_foreground`
    draw_particles(game, assets, false);
    draw_particles(game, assets, true);

    for &id in game.world.tagged("fade") {
        if let Some(fade) = game.world.get(id).fade.as_ref() {
            draw_rectangle(
                0.0,
                0.0,
                SCREEN_RIGHT as f32,
                SCREEN_TOP as f32,
                Color::new(0.0, 0.0, 0.0, fade.opacity() as f32),
            );
        }
    }

    set_default_camera();
}

/// `dokidoki/scripts/sprite.lua`.
fn draw_actor_sprite(game: &Game, assets: &Assets, id: crate::world::ActorId) {
    let actor = game.world.get(id);
    let (Some(transform), Some(sprite)) = (actor.transform, actor.sprite.as_ref()) else {
        return;
    };
    let Some(image) = sprite.image else {
        return;
    };

    push_transform(
        transform.pos,
        transform.facing,
        (transform.scale_x, transform.scale_y),
    );
    draw_sprite(assets, image, sprite.color.map_or(WHITE, to_color));
    pop_matrix();
}

/// `scripts/factory_damage.lua`: scorch marks over a wounded factory, flickering
/// at the opacity its draw already rolled.
fn draw_factory_damage(game: &Game, assets: &Assets, id: crate::world::ActorId) {
    let actor = game.world.get(id);
    let (Some(transform), Some(damage), Some(ship)) =
        (actor.transform, actor.factory_damage, actor.ship.as_ref())
    else {
        return;
    };

    let health = ship.health_percentage();
    let art = if health < crate::constants::LOW_HEALTH_THRESHOLD {
        SpriteId::FactoryHeavyDamage(damage.frame())
    } else if health < crate::constants::HIGH_HEALTH_THRESHOLD {
        SpriteId::FactoryLightDamage(damage.frame())
    } else {
        return;
    };

    push_transform(transform.pos, transform.facing, (1.0, 1.0));
    draw_sprite(
        assets,
        art,
        Color::new(1.0, 1.0, 1.0, damage.flicker as f32),
    );
    pop_matrix();
}

/// `scripts/production.lua`'s draw: the radial build menu, and the health bar
/// that shares its place when the button is up.
fn draw_production(game: &Game, assets: &Assets, id: crate::world::ActorId) {
    let actor = game.world.get(id);
    let (Some(transform), Some(dial), Some(resources), Some(ship)) = (
        actor.transform,
        actor.production,
        actor.resources,
        actor.ship.as_ref(),
    ) else {
        return;
    };
    if dial.halt_production {
        return;
    }

    let pos = transform.pos + transform.facing * PRODUCTION_DRAW_OFFSET;
    push_transform(pos, V2::I, (1.0, 1.0));

    draw_sprite(assets, SpriteId::ProductionLayer(1), WHITE);

    // the resources still to be earned, as a dark slice eaten anticlockwise
    let angle = production::scale_angle(resources.amount) as f32 * std::f32::consts::TAU;
    let filled = std::f32::consts::TAU - angle;
    pie(
        PRODUCTION_SEGMENTS,
        DIAL_RADIUS,
        Color::new(0.0, 0.0, 0.0, 0.6),
        |t| std::f32::consts::FRAC_PI_2 - angle - t * filled,
    );

    // the quarter being spent, highlighted, while the button is down
    if resources.amount > UnitType::Fighter.cost() && dial.button_held {
        let spent = UnitType::for_cost(dial.potential_cost).map_or(0.0, UnitType::cost);
        let base = production::scale_angle(spent) as f32 * std::f32::consts::TAU;
        pie(
            PRODUCTION_SEGMENTS,
            DIAL_RADIUS,
            Color::new(0.5, 1.0, 1.0, 1.0),
            |t| std::f32::consts::FRAC_PI_2 - (t * std::f32::consts::FRAC_PI_2 + base),
        );
    }

    push_matrix(
        Mat4::from_scale(vec3(0.5, 0.5, 1.0))
            * Mat4::from_rotation_z(-dial.needle_angle as f32 * std::f32::consts::TAU),
    );
    draw_sprite(assets, SpriteId::Needle, WHITE);
    pop_matrix();

    draw_sprite(assets, SpriteId::ProductionLayer(2), WHITE);
    draw_sprite(assets, SpriteId::ProductionLayer(3), WHITE);

    if dial.button_held {
        if let Some(unit) = UnitType::for_cost(dial.potential_cost) {
            let preview = match unit {
                UnitType::Fighter => SpriteId::FighterPreview,
                UnitType::Bomber => SpriteId::BomberPreview,
                UnitType::Frigate => SpriteId::FrigatePreview,
                UnitType::Upgrade => SpriteId::UpgradePreview,
            };
            draw_sprite(assets, preview, WHITE);
        }
    } else {
        draw_health_bar(assets, ship.health_percentage(), dial.texcoord_scroller);
    }

    pop_matrix();
}

/// The scrolling ring around the dial. Its colour, vertical squash and image all
/// change with health, and its texture creeps sideways.
fn draw_health_bar(assets: &Assets, health: f64, scroller: f64) {
    let (low, high) = (
        crate::constants::LOW_HEALTH_THRESHOLD,
        crate::constants::HIGH_HEALTH_THRESHOLD,
    );

    let (image, squash, color) = if health < low {
        let factor = (health / low) as f32;
        (
            SpriteId::HealthNone,
            (0.5f32).max(factor),
            Color::new(1.0, factor * 0.3, factor * 0.3, 1.0),
        )
    } else if health < high {
        let factor = ((health - low) / (high - low)) as f32;
        (
            SpriteId::HealthSome,
            factor * 0.5 + 0.5,
            Color::new(1.0, factor * 0.7 + 0.3, factor * 0.2 + 0.3, 1.0),
        )
    } else {
        let factor = ((health - high) / (1.0 - high)) as f32;
        (
            SpriteId::HealthFull,
            health as f32,
            Color::new((1.0 - factor) * 0.3 + 0.7, 1.0, factor * 0.4 + 0.6, 1.0),
        )
    };

    push_matrix(Mat4::from_scale(vec3(1.2, squash, 1.0)));
    draw_scrolled(assets, image, color, scroller);
    pop_matrix();
}

/// The Lua scrolls this sprite by translating the texture matrix with the
/// texture set to `GL_REPEAT`. macroquad has no wrap mode to set, so the quad is
/// split in two at the wrap point instead.
fn draw_scrolled(assets: &Assets, id: SpriteId, color: Color, scroller: f64) {
    let Some(texture) = assets.get(id) else {
        return;
    };

    let (w, h) = (texture.width(), texture.height());
    let (ox, oy) = (w / 2.0, h / 2.0);
    let offset = (scroller as f32).rem_euclid(1.0);
    let split = w * (1.0 - offset);

    let piece = |source: Rect, x: f32, width: f32| {
        draw_texture_ex(
            texture,
            x,
            -oy,
            color,
            DrawTextureParams {
                dest_size: Some(vec2(width, h)),
                source: Some(source),
                flip_y: true,
                ..Default::default()
            },
        );
    };

    piece(Rect::new(w * offset, 0.0, split, h), -ox, split);
    piece(Rect::new(0.0, 0.0, w * offset, h), -ox + split, w * offset);
}

/// `scripts/selector.lua`'s draw: the join prompt on the title screen.
fn draw_selector(game: &Game, assets: &Assets, id: crate::world::ActorId) {
    let actor = game.world.get(id);
    let (Some(transform), Some(selector)) = (actor.transform, actor.selector.as_ref()) else {
        return;
    };

    let pos = transform.pos + transform.facing * SELECTOR_DRAW_OFFSET;
    let pulse = (1.0 + (selector.pulse_time as f32 / 180.0 * std::f32::consts::TAU).cos()) / 2.0;
    let level = selector.fade as f32 * pulse + (1.0 - pulse);

    push_transform(pos, V2::I, (1.0, 1.0));

    pie(
        SELECTOR_SEGMENTS,
        DIAL_RADIUS,
        Color::new(0.0, 0.0, 0.0, 0.6),
        |t| std::f32::consts::FRAC_PI_2 - t * std::f32::consts::TAU,
    );

    if selector.picked {
        let base = selector.player as f32 * std::f32::consts::FRAC_PI_2 - std::f32::consts::PI;
        pie(
            SELECTOR_SEGMENTS,
            DIAL_RADIUS,
            Color::new(0.5, 1.0, 1.0, 0.7),
            |t| std::f32::consts::FRAC_PI_2 - (t * std::f32::consts::FRAC_PI_2 + base),
        );
    }

    draw_sprite(assets, SpriteId::AButton, grey(level, 1.0));
    pop_matrix();
}

/// `scripts/splash.lua`: the title and the credits, at fixed points.
fn draw_splash(assets: &Assets) {
    let width = SCREEN_RIGHT as f32;
    let height = SCREEN_TOP as f32;

    push_matrix(Mat4::from_translation(vec3(
        width / 2.0,
        height / 2.0 + height / 4.0,
        0.0,
    )));
    draw_sprite(assets, SpriteId::Title, WHITE);
    pop_matrix();

    push_matrix(Mat4::from_translation(vec3(
        width / 2.0 + 265.0,
        height / 2.0,
        0.0,
    )));
    draw_sprite(assets, SpriteId::Credits, WHITE);
    pop_matrix();
}

/// `particles.c`'s draw: one quad per live particle, fading out with its life.
fn draw_particles(game: &Game, assets: &Assets, foreground: bool) {
    let particles = &game.particles;
    let layers: Vec<(&crate::particles::Emitter, SpriteId)> = if foreground {
        vec![
            (&particles.small_explosion, SpriteId::Explosion),
            (&particles.mid_explosion, SpriteId::Explosion),
            (&particles.big_explosion, SpriteId::Explosion),
            (&particles.spark, SpriteId::Spark),
        ]
    } else {
        vec![
            (&particles.big_bubble, SpriteId::BigBubble),
            (&particles.bubble, SpriteId::Bubble),
        ]
    };

    for (emitter, image) in layers {
        let Some(texture) = assets.get(image) else {
            continue;
        };

        for particle in emitter.particles() {
            let dx = (emitter.width / 2.0 * particle.scale) as f32;
            let dy = (emitter.height / 2.0 * particle.scale) as f32;
            let alpha = emitter.life_fraction(particle) as f32;

            draw_texture_ex(
                texture,
                particle.pos.x as f32 - dx,
                particle.pos.y as f32 - dy,
                Color::new(1.0, 1.0, 1.0, alpha),
                DrawTextureParams {
                    dest_size: Some(vec2(dx * 2.0, dy * 2.0)),
                    flip_y: true,
                    ..Default::default()
                },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_four_by_three_window_needs_no_letterbox() {
        let fit = letterbox(4.0 / 3.0);
        assert!((fit.x - 1.0).abs() < 1e-6 && (fit.y - 1.0).abs() < 1e-6);
    }

    #[test]
    fn a_wide_window_gets_bars_at_the_sides_and_a_tall_one_top_and_bottom() {
        let wide = letterbox(16.0 / 9.0);
        assert_eq!(wide.y, 1.0);
        assert!(wide.x < 1.0);

        let tall = letterbox(1.0);
        assert_eq!(tall.x, 1.0);
        assert!(tall.y < 1.0);
    }

    #[test]
    fn a_pixel_stays_square_whatever_the_window_is() {
        // the whole point of the letterbox: one game unit covers the same
        // fraction of the screen horizontally and vertically
        for aspect in [0.5, 1.0, 4.0 / 3.0, 16.0 / 9.0, 3.0] {
            let fit = letterbox(aspect);
            let per_unit_x = fit.x / SCREEN_RIGHT as f32;
            let per_unit_y = fit.y / SCREEN_TOP as f32 / aspect;
            assert!(
                (per_unit_x - per_unit_y).abs() < 1e-6,
                "aspect {aspect}: {per_unit_x} by {per_unit_y}"
            );
        }
    }

    #[test]
    fn the_world_always_fills_one_axis_completely() {
        for aspect in [0.5, 1.0, 4.0 / 3.0, 16.0 / 9.0, 3.0] {
            let fit = letterbox(aspect);
            assert!(
                fit.x == 1.0 || fit.y == 1.0,
                "aspect {aspect} wastes space on both axes: {fit:?}"
            );
            assert!(fit.x <= 1.0 && fit.y <= 1.0);
        }
    }
}
