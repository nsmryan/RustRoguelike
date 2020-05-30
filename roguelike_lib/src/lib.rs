#![allow(dead_code)]

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::boxed::Box;
use std::default::Default;
use std::mem;
use std::ffi::CStr;

use serde_json;

use log::{LevelFilter, trace};
use simple_logging;

use roguelike_core::types::*;
use roguelike_core::config::Config;
use roguelike_core::messaging::Msg;
use roguelike_core::ai::*;
use roguelike_core::movement::*;
use roguelike_core::map::*;

use roguelike_engine::game::*;
use roguelike_engine::make_map::read_map_xp;
use roguelike_engine::resolve::resolve_messages;


// TODO pass map back and forth


/*
#[repr(packed(1))]
#[derive(PartialEq, Copy, Clone, Default)]
pub struct FfiMsg {
    msg_id: u16,
    entity_id: EntityId,
    other_id: EntityId,
    first_pos: Pos,
    second_pos: Pos,
    obj_type: ObjType,
    behavior: Behavior,
    move_mode: MoveMode,
    game_state: GameState,
    movement: Movement,
    hp: Hp,
    radius: usize,
    animate: bool,
}
*/

#[no_mangle]
pub extern "C" fn create_game(seed: u64, config_name: *mut i8, map_name: *mut i8) -> *mut Game {
    simple_logging::log_to_file("game.log", LevelFilter::Trace);

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

        let (map, pos) = read_map_xp(&game.config, &mut game.data.entities, &mut game.msg_log, map_str);

        // TODO merge this into creating the map
        game.data.map = map;
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
            *msg_len = msg_vec.len() as i32;
            msg_ptr = msg_vec.as_mut_ptr();

            mem::forget(msg_vec);
        }
    }
    trace!("read message done");

    mem::forget(game);

    return msg_ptr;
}

#[no_mangle]
pub extern "C" fn step_game(game_ptr: *mut Game) {
    trace!("stepping game");
    let mut game: Box<Game>;
    unsafe {
        game = Box::from_raw(game_ptr);
    }

    trace!("calling step_game");
    let game_result = game.step_game(0.1);
    trace!("finished step_game");

    /* Reload Configuration */
    //game.config = Config::from_file("config.yaml");

    mem::forget(game);

    // TODO should probably return a game result
}

