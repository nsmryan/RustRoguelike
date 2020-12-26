#![allow(dead_code)]

use std::boxed::Box;
use std::mem;
use std::ffi::CStr;

use serde_json;

use log::{LevelFilter, trace};
use simple_logging;

use roguelike_core::types::*;
use roguelike_core::config::Config;
use roguelike_core::map::*;

use roguelike_engine::game::*;
use roguelike_engine::make_map::read_map_xp;
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

    unsafe {
        let map_cstr = CStr::from_ptr(map_name);
        trace!("map string");
        let map_str = map_cstr.to_str().unwrap();
        trace!("loading map {}", map_str);

        let pos = read_map_xp(&game.config, &mut game.data, &mut game.msg_log, map_str);

        let player_id = game.data.find_by_name(EntityName::Player).unwrap();
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
pub extern "C" fn alloc_buffer(buf_len: i32) -> *mut u8 {
    trace!("allocating {} bytes", buf_len);

    let layout = std::alloc::Layout::from_size_align(buf_len as usize, 1).unwrap();

    let ptr;
    unsafe {
        ptr = std::alloc::alloc(layout);
    }

    trace!("ptr = {:?}", ptr);

    return ptr;
    //let mut vec = Vec::with_capacity(buf_len as usize);

    //let ptr = vec.as_mut_ptr();
    //trace!("allocated {:?}", ptr);
    //mem::forget(vec);

    //unsafe {
    //    *ptr = 199;
    //}

    //return ptr;
}

#[no_mangle]
pub extern "C" fn free_buffer(ptr: *mut u8, buf_len: i32) {
    unsafe {
        let layout = std::alloc::Layout::from_size_align(buf_len as usize, 1).unwrap();
        std::alloc::dealloc(ptr, layout);
        //Vec::from_raw_parts(ptr, 0, buf_len as usize);
    }
}

#[no_mangle]
pub extern "C" fn read_message(game_ptr: *mut Game, msg_ptr: *mut u8, msg_len: *mut i32) {
    trace!("read_message {:?}, to {:?}", game_ptr, msg_ptr);

    let mut game: Box<Game>;

    unsafe {
        *msg_len = 0;

        game = Box::from_raw(game_ptr);

        trace!("creating message");
        if let Some(msg) = game.msg_log.turn_messages.pop_front() {
            let msg_str = serde_json::to_string(&msg).unwrap();

            let msg_vec: Vec<u8> = msg_str.into_bytes();

            for index in 0..msg_vec.len() {
                *msg_ptr.offset(index as isize) = msg_vec[index];
            }
            trace!("copied");
            *msg_ptr.offset(msg_vec.len() as isize) = 0;

            *msg_len = msg_vec.len() as i32 + 1;
            trace!("msg_len = {}", *msg_len);
        }
    }
    trace!("read message done");

    mem::forget(game);
}

#[no_mangle]
pub extern "C" fn read_map(game_ptr: *mut Game, width: *mut i32, height: *mut i32) -> *mut Tile {
    trace!("reading map");

    let game: Box<Game>;
    let mut tile_buf = std::ptr::null_mut();
    unsafe {
        game = Box::from_raw(game_ptr);

        if *width != game.data.map.width() || *height != game.data.map.height() {
            trace!("width = {}, height = {}", game.data.map.width(), game.data.map.height());
            *width = game.data.map.width();
            *height = game.data.map.height();
            trace!("assigned");
        } else {
            trace!("getting tiles");

            let count = *height * *width;
            let temp_buf = alloc_buffer(count * std::mem::size_of::<Tile>() as i32);
            tile_buf = temp_buf.cast::<Tile>();

            for x in 0..game.data.map.width() {
                for y in 0..game.data.map.height() {
                    let offset = x + y * game.data.map.width();
                    let tile = game.data.map[(x, y)];
                    *tile_buf.offset(offset as isize) = tile;
                }
            }

            //for x in 0..game.data.map.width() {
            //    for y in 0..game.data.map.height() {
            //        let offset = x + y * game.data.map.width();
            //        let tile = game.data.map[(x, y)];
            //        trace!("tile({}, {}) = {:?}", x, y, tile);
            //        *map.offset(offset as isize) = tile;
            //        let tile = *map.offset(offset as isize);
            //        trace!("*tile({}, {}) = {:?}", x, y, tile);
            //    }
            //}
        }
    }

    mem::forget(game);

    trace!("map read done");

    return tile_buf;
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

    //trace!("calling step_game");
    //let game_result = game.step_game(0.1);
    trace!("finished step_game");

    /* Reload Configuration */
    //game.config = Config::from_file("config.yaml");

    mem::forget(game);

    // TODO should probably return a game result
}

