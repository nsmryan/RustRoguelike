mod commands;

use std::io;
use std::io::{BufRead};

use roguelike_core::config::*;
use roguelike_engine::game::*;
use roguelike_engine::make_map::*;

use crate::commands::*;


const CONFIG_NAME: &str = "config.yaml";

pub fn main() {
    let stdin = io::stdin();
    let mut iter = stdin.lock().lines();

    let config = Config::from_file(CONFIG_NAME);
    let seed = 1;
    let mut game = Game::new(seed, config.clone()).unwrap();

    make_map(&config.map_load, &mut game);

    while game.settings.running {
        let msg = iter.next().unwrap().unwrap();

        let cmd = msg.parse::<GameCmd>()
                     .expect(&format!("Unexpected command {}", msg));

        let result = execute_game_command(&cmd, &mut game);
        println!("{}", result);
    }
}

