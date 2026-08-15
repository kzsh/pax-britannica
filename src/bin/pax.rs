//! The game, in a window.
//!
//! `dokidoki/kernel.lua`'s main loop: a fixed 60Hz update with a frameskip cap,
//! and one render per displayed frame. The game's physics constants assume a
//! 1/60 tick, so the tick rate is fixed and only the number of ticks per
//! rendered frame varies.
//!
//! Everything gameplay-related happens in [`the_game::step`]; this file is
//! window, clock, input and sound.
//!
//! Input is the four keyboard buttons `A`, `F`, `H` and `L`, one per player, as
//! in `components/the_one_button.lua`. That component also polls four joysticks;
//! macroquad has no gamepad API, so pads are not wired up.

use std::time::{SystemTime, UNIX_EPOCH};

use macroquad::audio::{PlaySoundParams, load_sound, play_sound};
use macroquad::prelude::*;

use pax_britannica::game::Game;
use pax_britannica::render::{self, Assets};
use pax_britannica::rng::LuaRng;
use pax_britannica::the_game;

/// `kernel.fps`.
const TICK: f64 = 1.0 / 60.0;

/// `kernel.max_frameskip`: the most updates one rendered frame may catch up by
/// before the clock is simply moved forward and the lost time dropped.
const MAX_FRAMESKIP: usize = 6;

/// `components/the_one_button.lua`'s `player_keys`.
const PLAYER_KEYS: [KeyCode; 4] = [KeyCode::A, KeyCode::F, KeyCode::H, KeyCode::L];

fn window_conf() -> Conf {
    Conf {
        window_title: "Pax Britannica".to_owned(),
        window_width: 1024,
        window_height: 768,
        ..Default::default()
    }
}

/// `math.randomseed()` with no argument: a fresh stream every run.
fn seed() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_nanos() as i64)
        .unwrap_or(1)
}

async fn start_music() {
    match load_sound("audio/music.ogg").await {
        Ok(music) => play_sound(
            &music,
            PlaySoundParams {
                looped: true,
                volume: 1.0,
            },
        ),
        Err(error) => macroquad::logging::warn!("could not load audio/music.ogg: {error}"),
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let assets = match Assets::load().await {
        Ok(assets) => assets,
        Err(error) => {
            macroquad::logging::error!("could not load sprites: {error}");
            return;
        }
    };

    if !std::env::args().any(|arg| arg == "--no-music") {
        start_music().await;
    }

    let mut game = the_game::make(LuaRng::new(seed(), 0));
    let mut behind = 0.0;

    loop {
        if is_key_pressed(KeyCode::Escape) {
            game.log.print_stats();
            return;
        }

        read_input(&mut game);

        behind += get_frame_time() as f64;
        let mut ticks = 0;
        while behind >= TICK && ticks < MAX_FRAMESKIP {
            the_game::step(&mut game);
            behind -= TICK;
            ticks += 1;
        }
        if behind >= TICK {
            // the underrun branch: drop the backlog rather than spiral
            behind = 0.0;
        }

        render::frame(&game, &assets);
        next_frame().await;
    }
}

/// Polls the four buttons. `the_one_button`'s own `update_setup` phase latches
/// these into held/pressed/released at the top of each update.
fn read_input(game: &mut Game) {
    for (player, key) in PLAYER_KEYS.iter().enumerate() {
        game.the_one_button.keys[player] = is_key_down(*key);
    }
}
