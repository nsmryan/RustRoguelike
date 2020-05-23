#![allow(dead_code)]

use std::env;
use std::time::{Duration, Instant};
use std::path::Path;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use rand::prelude::*;

use walkdir::WalkDir;

use roguelike_core::types::*;
use roguelike_core::config::Config;
use roguelike_core::messaging::Msg;
use roguelike_core::constants::*;
use roguelike_core::animation::SpriteKey;
use roguelike_core::movement::Direction;

use roguelike_engine::game::*;
use roguelike_engine::actions::*;
use roguelike_engine::read_map::read_map_xp;
use roguelike_engine::resolve::resolve_messages;

fn main() {
    let args = env::args().collect::<Vec<String>>();

    let seed: u64;
    if args.len() > 1 {
        let mut hasher = DefaultHasher::new();
        args[1].hash(&mut hasher);
        seed = hasher.finish();
    } else {
        seed = rand::thread_rng().gen();
    }
    println!("Seed: {} (0x{:X})", seed, seed);

    run(seed).unwrap();
}

pub fn run(seed: u64) -> Result<(), String> {
    let config = Config::from_file("config.yaml");

    let mut game = Game::new(seed, config.clone())?;

    let start_time = Instant::now();
    let mut frame_time = Instant::now();

    let game_result = game.step_game(0.1);

    /* Reload Configuration */
    game.config = Config::from_file("config.yaml");

    return Ok(());
}

