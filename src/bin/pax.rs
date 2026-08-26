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

use macroquad::audio::{PlaySoundParams, Sound, load_sound, play_sound};
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
///
/// miniquad's clock rather than `SystemTime`, which has no implementation on
/// `wasm32-unknown-unknown` and panics there. `date::now` is seconds since the
/// epoch as a float; nanoseconds give the low bits something to vary.
fn seed() -> i64 {
    (macroquad::miniquad::date::now() * 1e9) as i64
}

async fn load_music() -> Option<Sound> {
    match load_sound("audio/music.ogg").await {
        Ok(music) => Some(music),
        Err(error) => {
            macroquad::logging::warn!("could not load audio/music.ogg: {error}");
            None
        }
    }
}

fn start_music(music: &Sound) {
    play_sound(
        music,
        PlaySoundParams {
            looped: true,
            volume: 1.0,
        },
    );
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

    // std::env::args is empty on wasm, so the browser build always has music.
    // The Sound is held for the whole run: dropping it deletes it from the
    // mixer, which would cut the loop off mid-playback.
    let music = if std::env::args().any(|arg| arg == "--no-music") {
        None
    } else {
        load_music().await
    };

    // A browser refuses to start audio before the page has been interacted
    // with, so there the soundtrack waits for the first button press. On a
    // desktop it plays over the title screen, as the original does.
    let mut playing = false;
    if !cfg!(target_arch = "wasm32")
        && let Some(music) = &music
    {
        start_music(music);
        playing = true;
    }

    let mut game = the_game::make(LuaRng::new(seed(), 0));
    let mut behind = 0.0;

    loop {
        if is_key_pressed(KeyCode::Escape) {
            game.log.print_stats();
            return;
        }

        read_input(&mut game);

        if !playing
            && game.the_one_button.keys.iter().any(|held| *held)
            && let Some(music) = &music
        {
            start_music(music);
            playing = true;
        }

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
