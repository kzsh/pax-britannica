//! `components/particles.lua` and `particles.c`.
//!
//! Purely decorative on screen, and load-bearing everywhere else: every
//! explosion pulls dozens to hundreds of draws off the shared random stream, so
//! the *number and order* of those draws is part of the simulation. Emitting one
//! spark too few here desynchronises every later frame.
//!
//! Each `V2::random()` is two draws. The loop bounds below are therefore copied
//! from the Lua exactly, including the places where it looks like they were
//! meant to be something else -- `explode_tiny` routes to the small explosion
//! emitter, and the nested spark loops in `explode_*` recompute a velocity that
//! shadows the outer one. Both are faithful.

use crate::rng::LuaRng;
use crate::v2::{V2, v2};

/// Ring size from `particles.c`.
const PARTICLE_COUNT: usize = 2000;

#[derive(Clone, Copy, Debug, Default)]
pub struct Particle {
    pub life: i32,
    pub pos: V2,
    pub velocity: V2,
    pub scale: f64,
}

/// One pool of particles sharing an image and a decay rule.
#[derive(Clone, Debug)]
pub struct Emitter {
    pub width: f64,
    pub height: f64,
    life: i32,
    damping: f64,
    delta_scale: f64,
    particles: Vec<Particle>,
    next: usize,
}

impl Emitter {
    fn new(width: f64, height: f64, life: i32, damping: f64, delta_scale: f64) -> Self {
        Self {
            width,
            height,
            life,
            damping,
            delta_scale,
            particles: vec![Particle::default(); PARTICLE_COUNT],
            next: 0,
        }
    }

    /// Overwrites the oldest slot, as the C ring does.
    pub fn add(&mut self, pos: V2, velocity: V2) {
        self.next %= PARTICLE_COUNT;
        self.particles[self.next] = Particle {
            life: self.life,
            pos,
            velocity,
            scale: 1.0,
        };
        self.next += 1;
    }

    pub fn update(&mut self) {
        for particle in &mut self.particles {
            if particle.life > 0 {
                particle.life -= 1;
                particle.pos = particle.pos + particle.velocity;
                particle.velocity = particle.velocity * self.damping;
                particle.scale += self.delta_scale;
            }
        }
    }

    pub fn live_count(&self) -> usize {
        self.particles.iter().filter(|p| p.life > 0).count()
    }

    pub fn particles(&self) -> impl Iterator<Item = &Particle> {
        self.particles.iter().filter(|p| p.life > 0)
    }

    /// How much life a particle has left, as a fraction of a full one. The
    /// opacity `particles.c` draws it at.
    pub fn life_fraction(&self, particle: &Particle) -> f64 {
        particle.life as f64 / self.life as f64
    }
}

/// The shape of one explosion, which is all that differs between
/// `explode_big`/`_mid`/`_small`/`_tiny`.
#[derive(Clone, Copy, Debug)]
struct Blast {
    /// Outer passes, each drawing one shared spark velocity.
    blast_loops: usize,
    /// Sparks per pass, fanned out along that velocity.
    spark_loops: usize,
    /// Extra sparks scattered in every direction afterwards.
    scatter_sparks: usize,
    scatter_speed: f64,
    bubbles: usize,
    bubble_spread: f64,
}

/// All six emitters, split into the two draw layers the Lua uses.
#[derive(Clone, Debug)]
pub struct Particles {
    pub bubble: Emitter,
    pub big_bubble: Emitter,
    pub big_explosion: Emitter,
    pub mid_explosion: Emitter,
    pub small_explosion: Emitter,
    pub spark: Emitter,
}

impl Default for Particles {
    fn default() -> Self {
        Self::new()
    }
}

impl Particles {
    /// Sizes come from the sprite dimensions in `components/particles.lua`; the
    /// divisors there (`/ 64`, `/ 96`, `/ 128`) scale the shared explosion image
    /// down per blast size.
    pub fn new() -> Self {
        // sprite pixel sizes, from sprites/*.png
        const BUBBLE: (f64, f64) = (8.0, 8.0);
        const BIG_BUBBLE: (f64, f64) = (16.0, 16.0);
        const EXPLOSION: (f64, f64) = (64.0, 64.0);
        const SPARK: (f64, f64) = (8.0, 8.0);

        Self {
            bubble: Emitter::new(BUBBLE.0, BUBBLE.1, 300, 1.0, 0.0),
            big_bubble: Emitter::new(BIG_BUBBLE.0, BIG_BUBBLE.1, 300, 1.0, 0.0),
            big_explosion: Emitter::new(EXPLOSION.0 / 64.0, EXPLOSION.1 / 64.0, 10, 1.0, 30.0),
            mid_explosion: Emitter::new(EXPLOSION.0 / 64.0, EXPLOSION.1 / 64.0, 10, 1.0, 15.0),
            small_explosion: Emitter::new(EXPLOSION.0 / 96.0, EXPLOSION.1 / 96.0, 10, 1.0, 10.0),
            spark: Emitter::new(SPARK.0 * 1.75, SPARK.1 * 1.75, 40, 0.95, 0.0),
        }
    }

    /// Live particles across every emitter.
    pub fn live_count(&self) -> usize {
        [
            &self.bubble,
            &self.big_bubble,
            &self.big_explosion,
            &self.mid_explosion,
            &self.small_explosion,
            &self.spark,
        ]
        .iter()
        .map(|emitter| emitter.live_count())
        .sum()
    }

    pub fn update(&mut self) {
        // foreground first, then background, as the Lua's update does
        for emitter in [
            &mut self.small_explosion,
            &mut self.mid_explosion,
            &mut self.big_explosion,
            &mut self.spark,
        ] {
            emitter.update();
        }
        for emitter in [&mut self.big_bubble, &mut self.bubble] {
            emitter.update();
        }
    }

    /// Two draws, in this order.
    pub fn add_bubble(&mut self, rng: &mut LuaRng, pos: V2) {
        let x_velocity = rng.next_f64() * 0.1 - 0.05;
        let y_velocity = 0.01 + rng.next_f64() * 0.05;
        self.bubble.add(pos, v2(x_velocity, y_velocity));
    }

    /// The shared shape of `explode_big`/`_mid`/`_small`/`_tiny`.
    ///
    /// `blast_loops` x `spark_loops` sparks in the nested pass, then
    /// `scatter_sparks` more, then `bubbles` bubbles. The inner spark loop
    /// always divides by 20 regardless of its own bound, which is why the
    /// smaller explosions produce slower sparks rather than merely fewer.
    fn explode_shape(&mut self, rng: &mut LuaRng, pos: V2, blast: Blast) {
        let Blast {
            blast_loops,
            spark_loops,
            scatter_sparks,
            scatter_speed,
            bubbles,
            bubble_spread,
        } = blast;

        for _ in 0..blast_loops {
            let base_velocity = V2::random(rng) + V2::random(rng);
            for i in 1..=spark_loops {
                let velocity = base_velocity * (i as f64 / 20.0 * 2.0);
                let offset = V2::random(rng) * 10.0;
                self.spark.add(pos + offset, velocity);
            }
        }

        for _ in 0..scatter_sparks {
            let velocity =
                v2(rng.next_f64() * 2.0 - 1.0, rng.next_f64() * 2.0 - 1.0) * scatter_speed;
            let offset = V2::random(rng) * 3.0;
            self.spark.add(pos + offset, velocity);
        }

        for _ in 0..bubbles {
            let velocity = v2(rng.next_f64() * 2.0 - 1.0, rng.next_f64() * 2.0 - 1.0) * 0.2;
            let offset = V2::random(rng) * bubble_spread;
            self.big_bubble.add(pos + offset, velocity);
        }
    }

    pub fn explode_big(&mut self, rng: &mut LuaRng, pos: V2) {
        self.big_explosion.add(pos, V2::ZERO);
        self.explode_shape(
            rng,
            pos,
            Blast {
                blast_loops: 20,
                spark_loops: 20,
                scatter_sparks: 50,
                scatter_speed: 5.0,
                bubbles: 50,
                bubble_spread: 17.0,
            },
        );
    }

    pub fn explode_mid(&mut self, rng: &mut LuaRng, pos: V2) {
        self.mid_explosion.add(pos, V2::ZERO);
        self.explode_shape(
            rng,
            pos,
            Blast {
                blast_loops: 10,
                spark_loops: 10,
                scatter_sparks: 15,
                scatter_speed: 4.0,
                bubbles: 20,
                bubble_spread: 10.0,
            },
        );
    }

    pub fn explode_small(&mut self, rng: &mut LuaRng, pos: V2) {
        self.small_explosion.add(pos, V2::ZERO);
        self.explode_shape(
            rng,
            pos,
            Blast {
                blast_loops: 5,
                spark_loops: 5,
                scatter_sparks: 10,
                scatter_speed: 3.0,
                bubbles: 10,
                bubble_spread: 5.0,
            },
        );
    }

    /// Note the small emitter: `explode_tiny` in the Lua adds its flash to
    /// `small_explosion_emitter`, not to a tiny one. Copied as written.
    pub fn explode_tiny(&mut self, rng: &mut LuaRng, pos: V2) {
        self.small_explosion.add(pos, V2::ZERO);
        self.explode_shape(
            rng,
            pos,
            Blast {
                blast_loops: 2,
                spark_loops: 5,
                scatter_sparks: 5,
                scatter_speed: 2.0,
                bubbles: 5,
                bubble_spread: 5.0,
            },
        );
    }

    /// Ten sparks flung off a laser strike. 20 draws.
    pub fn laser_hit(&mut self, rng: &mut LuaRng, pos: V2, velocity: V2) {
        for _ in 0..10 {
            let scattered = velocity + V2::random(rng);
            self.spark.add(pos, scattered);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// How many draws a closure takes off the stream.
    fn draws(f: impl FnOnce(&mut Particles, &mut LuaRng)) -> u64 {
        let mut particles = Particles::new();
        let mut rng = LuaRng::new(1, 0);
        let mut counter = LuaRng::new(1, 0);

        f(&mut particles, &mut rng);

        // replay the counter until its state matches, which recovers the count
        for n in 0..100_000 {
            if counter == rng {
                return n;
            }
            counter.next_u64();
        }
        panic!("more than 100000 draws");
    }

    #[test]
    fn bubble_takes_two_draws() {
        assert_eq!(draws(|p, r| p.add_bubble(r, V2::ZERO)), 2);
    }

    #[test]
    fn explosion_draw_counts_match_the_lua() {
        // These are measured, not derived: `lua5.4 test/particle_draws.lua`
        // runs the real components/particles.lua against a counting RNG and
        // prints them. Regenerate there if this ever fails.
        assert_eq!(draws(|p, r| p.explode_big(r, V2::ZERO)), 1280);
        assert_eq!(draws(|p, r| p.explode_mid(r, V2::ZERO)), 380);
        assert_eq!(draws(|p, r| p.explode_small(r, V2::ZERO)), 150);
        assert_eq!(draws(|p, r| p.explode_tiny(r, V2::ZERO)), 68);
    }

    #[test]
    fn laser_hit_takes_twenty_draws() {
        assert_eq!(draws(|p, r| p.laser_hit(r, V2::ZERO, V2::ZERO)), 20);
    }

    #[test]
    fn emitter_ring_wraps_without_growing() {
        let mut emitter = Emitter::new(1.0, 1.0, 5, 1.0, 0.0);
        for _ in 0..PARTICLE_COUNT + 10 {
            emitter.add(V2::ZERO, V2::ZERO);
        }
        assert_eq!(emitter.particles.len(), PARTICLE_COUNT);
    }

    #[test]
    fn particles_expire() {
        let mut emitter = Emitter::new(1.0, 1.0, 3, 1.0, 0.0);
        emitter.add(V2::ZERO, v2(1.0, 0.0));
        assert_eq!(emitter.live_count(), 1);
        for _ in 0..3 {
            emitter.update();
        }
        assert_eq!(emitter.live_count(), 0);
    }
}
