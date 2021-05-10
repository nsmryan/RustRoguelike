mod commands;

use std::io::BufRead;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

use roguelike_core::config::*;
use roguelike_engine::game::*;
use roguelike_engine::make_map::*;

use crate::commands::*;


const CONFIG_NAME: &str = "config.yaml";

pub fn main() {
    let config = Config::from_file(CONFIG_NAME);
    let seed = 1;
    let mut game = Game::new(seed, config.clone()).unwrap();
    game.load_vaults("resources/vaults/");

    make_map(&config.map_load, &mut game);

    let io_recv = spawn_input_reader();

    while game.settings.running {

        if let Ok(msg) = io_recv.recv_timeout(Duration::from_millis(100)) {
            if let Ok(cmd) = msg.parse::<GameCmd>() {
                let result = execute_game_command(&cmd, &mut game);
                game.log_output(&result);
            }
        }

        // TODO is this actually necessary? it seems like it should be
        // using turn_messages instead, or done jnternally
        while let Some(msg) = game.msg_log.pop() {
            game.log_msg(&format!("{}", msg));
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
