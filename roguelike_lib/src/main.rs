mod commands;

use std::io::BufRead;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

use roguelike_core::config::*;
use roguelike_engine::game::*;
use roguelike_engine::make_map::*;
use roguelike_engine::log::*;

use crate::commands::*;


const CONFIG_NAME: &str = "config.yaml";

pub fn main() {
    let config = Config::from_file(CONFIG_NAME);
    let seed = 1;
    let mut game = Game::new(seed, config.clone());
    game.load_vaults("resources/vaults/");

    make_map(&config.map_load, &mut game);

    let io_recv = spawn_input_reader();

    let mut log = Log::new();

    while game.settings.running {

        if let Ok(msg) = io_recv.recv_timeout(Duration::from_millis(100)) {
            if let Ok(cmd) = msg.parse::<GameCmd>() {
                let result = execute_game_command(&cmd, &mut game);
                log.log_output(&result);

                for msg in game.msg_log.turn_messages.iter() {
                    log.log_msg(&format!("{}", msg));
                }

                let map_str = game.level.map.compact_chrs();
                log.log_map(&map_str);
            }
        }
    }
}

fn spawn_input_reader() -> Receiver<String> {
    let (io_send, io_recv) = mpsc::channel();

    thread::spawn(move || {
        let stdin = std::io::stdin();
        let stdin = stdin.lock().lines();

        for line in stdin {
            let text = line.unwrap();
            if !text.is_empty() {
                io_send.send(text).unwrap();
            }
        }
    });

    return io_recv;
}
