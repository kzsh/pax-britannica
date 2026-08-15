//! The Rust counterpart of `test/headless.lua`: the whole game, no window.
//!
//! Same scripted input schedule as the Lua harness -- two players join in the
//! first frames, then all four hammer their buttons on different periods so
//! that every quadrant of the dial gets used. The point is not to match the Lua
//! frame for frame (see PORTING.md), but to confirm the mechanics work end to
//! end: factories appear, ships get built, shots land, matches end and restart.
//!
//! Usage: `cargo run --bin headless [frames]`

use pax_britannica::game::Game;
use pax_britannica::rng::LuaRng;
use pax_britannica::the_game;

const TRACKED: [&str; 7] = [
    "factory", "fighter", "bomber", "frigate", "laser", "bomb", "missile",
];

/// `test/harness.lua`'s schedule, as a pure function of the frame number.
fn input(game: &mut Game, frame: usize) {
    match frame {
        2 => {
            game.the_one_button.keys[0] = true;
            game.the_one_button.keys[1] = true;
        }
        3 => {
            game.the_one_button.keys[0] = false;
            game.the_one_button.keys[1] = false;
        }
        frame if frame > 3 => {
            for player in 0..4 {
                let period = 120 * (player + 1) * (player + 1);
                game.the_one_button.keys[player] = frame % period < period * 3 / 4;
            }
        }
        _ => {}
    }
}

fn main() {
    let frames: usize = std::env::args()
        .nth(1)
        .and_then(|arg| arg.parse().ok())
        .unwrap_or(1200);

    let mut game = the_game::make(LuaRng::new(1, 0));
    let mut peak = [0usize; TRACKED.len()];
    let mut restarts = 0;

    for frame in 1..=frames {
        input(&mut game, frame);

        if the_game::step(&mut game) {
            restarts += 1;
        }

        for (slot, tag) in peak.iter_mut().zip(TRACKED) {
            *slot = (*slot).max(game.world.tagged(tag).len());
        }
    }

    let count = |tag: &str| game.world.tagged(tag).len();
    println!("ran {frames} frames, {restarts} restarts");
    println!(
        "actors: {} factories, {} fighters, {} bombers, {} frigates, {} lasers",
        count("factory"),
        count("fighter"),
        count("bomber"),
        count("frigate"),
        count("laser"),
    );
    let peaks: Vec<String> = TRACKED
        .iter()
        .zip(peak)
        .map(|(tag, n)| format!("{tag}={n}"))
        .collect();
    println!("peak counts: {}", peaks.join(" "));
    println!(
        "background: {} debris, {} fish, {} live particles",
        count("debris"),
        count("fish"),
        game.particles.live_count(),
    );

    game.log.print_stats();
}
