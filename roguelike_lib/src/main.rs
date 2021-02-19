mod commands;

use std::io;
use std::io::{BufRead};

use roguelike_core::config::*;
use roguelike_engine::game::*;

use crate::commands::*;


const CONFIG_NAME: &str = "config.yaml";

pub fn main() {
    let stdin = io::stdin();
    let mut iter = stdin.lock().lines();

    let config = Config::from_file(CONFIG_NAME);
    let mut game = Game::new(0, config).unwrap();

    make_map(&config.map_load, &mut game);

    while game.settings.running {
        let msg = iter.next().unwrap().unwrap();

        let cmd = msg.parse::<Command>()
                     .expect(&format!("Unexpected command {}", msg));

        let result = execute(&cmd, &mut game);
        println!("{}", result);
    }
}

