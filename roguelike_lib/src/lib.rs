#![allow(dead_code)]

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::boxed::Box;
use std::default::Default;
use std::mem;

use serde_json;

use log::{LevelFilter, trace};
use simple_logging;

use roguelike_core::types::*;
use roguelike_core::config::Config;
use roguelike_core::messaging::Msg;
use roguelike_core::ai::*;
use roguelike_core::movement::*;

use roguelike_engine::game::*;
use roguelike_engine::read_map::read_map_xp;
use roguelike_engine::resolve::resolve_messages;


// TODO pass map back and forth


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

#[no_mangle]
pub extern "C" fn create_game(seed: u64) -> *mut Game {
    simple_logging::log_to_file("game.log", LevelFilter::Trace);

    trace!("creating game");

    trace!("loading config");
    // TODO add config loading back in
    //let config = Config::from_file("config.yaml");
    let config = Config::default();

    trace!("game new");
    let game: Box<Game> = Box::new(Game::new(seed, config).unwrap());

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

    //return game_result;
    mem::forget(game);
}

/*
        match msg {
            Msg::Pass() => {
                out_msg.msg_id = 1;
            }

            Msg::Crushed(entity_id, pos, obj_type) => {
                out_msg.msg_id = 2;
            }

            Msg::Sound(entity_id, pos, radius, should_animate) => {
                out_msg.msg_id = 3;
            }

            Msg::SoundTrapTriggered(entity_id, other_id) => {
                out_msg.msg_id = 4;
            }

            Msg::SpikeTrapTriggered(entity_id, other_id) => {
                out_msg.msg_id = 5;
            }

            Msg::PlayerDeath => {
                out_msg.msg_id = 6;
            }

            Msg::PickedUp(entity_id, other_id) => {
                out_msg.msg_id = 7;
            }

            Msg::ItemThrow(entity_id, other_id, start, end) => {
                out_msg.msg_id = 8;
            }

            Msg::Attack(entity_id, other_id, hp) => {
                out_msg.msg_id = 9;
            }

            Msg::Killed(entity_id, other_id, hp) => {
                out_msg.msg_id = 10;
            }

            Msg::Moved(entity_id, movement, pos) => {
                out_msg.msg_id = 11;
            }

            Msg::JumpWall(entity_id, start, end) => {
                out_msg.msg_id = 12;
            }

            Msg::WallKick(entity_id, pos) => {
                out_msg.msg_id = 13;
            }

            Msg::StateChange(entity_id, behavior) => {
                out_msg.msg_id = 13;
            }

            Msg::Collided(entity_id, pos) => {
                out_msg.msg_id = 14;
            }
            
            Msg::Yell(pos) => {
                out_msg.msg_id = 15;
            }

            Msg::GameState(game_state) => {
                out_msg.msg_id = 16;
            }

            Msg::MoveMode(move_mode) => {
                out_msg.msg_id = 17;
            }

            Msg::TriedRunWithShield => {
                out_msg.msg_id = 18;
            }

            Msg::SpawnedObject(entity_id) => {
                out_msg.msg_id = 19;
            }

            Msg::ChangeLevel() => {
                out_msg.msg_id = 20;
            }
        }
*/
