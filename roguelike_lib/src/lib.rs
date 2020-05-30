#![allow(dead_code)]

use std::boxed::Box;
use std::mem;
use std::ffi::CStr;

use serde_json;

use log::{LevelFilter, trace};
use simple_logging;

use roguelike_core::types::*;
use roguelike_core::config::Config;
use roguelike_core::messaging::Msg;
use roguelike_core::movement::*;
use roguelike_core::map::*;

use roguelike_engine::game::*;
use roguelike_engine::make_map::read_map_xp;
use roguelike_engine::resolve::resolve_messages;
use roguelike_engine::actions::*;


// TODO pass map back and forth


#[no_mangle]
pub extern "C" fn create_game(seed: u64, config_name: *mut i8, map_name: *mut i8) -> *mut Game {
    simple_logging::log_to_file("game.log", LevelFilter::Trace).unwrap();

    trace!("creating game");

    let config: Config;
    unsafe {
        let config_cstr = CStr::from_ptr(config_name);
        trace!("config string");
        let config_str = config_cstr.to_str().unwrap().to_owned();
        trace!("loading config {}", config_str);
        config = Config::from_file(&config_str);
    }

    trace!("game new");
    let mut game: Box<Game> = Box::new(Game::new(seed, config).unwrap());

    let  map: Map;
    unsafe {
        let map_cstr = CStr::from_ptr(map_name);
        trace!("map string");
        let map_str = map_cstr.to_str().unwrap();
        trace!("loading map {}", map_str);

        let pos = read_map_xp(&game.config, &mut game.data, &mut game.msg_log, map_str);

        let player_id = game.data.find_player().unwrap();
        game.data.entities.pos[&player_id] = Pos::from(pos);
    }

    let raw_ptr = Box::into_raw(game);

    trace!("returning game pointer {:?}", raw_ptr);

    return raw_ptr;
}

#[no_mangle]
pub extern "C" fn destroy_game(game_ptr: *mut Game) {
    trace!("destroying game {:?}", game_ptr);
    unsafe {
        Box::from_raw(game_ptr);
    }
    trace!("exiting destroy_game");
}

#[no_mangle]
pub extern "C" fn read_message(game_ptr: *mut Game, msg_len: *mut i32) -> *mut u8 {
    trace!("read_message {:?}", game_ptr);
    let mut game: Box<Game>;
    let mut msg_ptr: *mut u8 = std::ptr::null_mut();

    unsafe {
        *msg_len = 0;

        game = Box::from_raw(game_ptr);

        trace!("creating message");
        if let Some(msg) = game.msg_log.turn_messages.pop_front() {
            let mut msg_str = serde_json::to_string(&msg).unwrap();

            let mut msg_vec: Vec<u8> = msg_str.into_bytes();
            msg_vec.push(0);
            *msg_len = msg_vec.len() as i32;
            msg_ptr = msg_vec.as_mut_ptr();

            // TODO this is leaking memory- consider re-using the memory each time,
            // or copying it to a provided Unity byte[].
            mem::forget(msg_vec);
        }
    }
    trace!("read message done");

    mem::forget(game);

    return msg_ptr;
}

#[no_mangle]
pub extern "C" fn step_game(game_ptr: *mut Game, input: *mut i8) {
    trace!("stepping game");
    let mut game: Box<Game>;
    unsafe {
        game = Box::from_raw(game_ptr);
    }

    trace!("parsing input");
    let mut input_action: InputAction = InputAction::None;
    unsafe {
        let input_cstr = CStr::from_ptr(input);
        if let Ok(parsed_action) = serde_json::from_str(&input_cstr.to_str().unwrap()) {
            input_action = parsed_action;
            trace!("input parsed {:?}", parsed_action);
        } else {
            trace!("Received unparsable input {:?}", input_cstr);
        }
    }
    game.input_action = input_action;

    trace!("calling step_game");
    let game_result = game.step_game(0.1);
    trace!("finished step_game");

    /* Reload Configuration */
    //game.config = Config::from_file("config.yaml");

    mem::forget(game);

    // TODO should probably return a game result
}

