use logging_timer::timer;

use roguelike_core::types::*;
use roguelike_core::config::*;
use roguelike_core::ai::*;
use roguelike_core::map::*;
use roguelike_core::messaging::Msg;
use roguelike_core::movement::{Direction, Action};
#[cfg(test)]
use roguelike_core::movement::*;


use crate::game::*;
use crate::actions::InputAction;
use crate::generation::*;
use crate::resolve::resolve_messages;
#[cfg(test)]
use crate::make_map::*;


pub fn step_logic(game: &mut Game, player_action: Action) -> bool {
    game.msg_log.clear();

    let player_id = game.data.find_by_name(EntityName::Player).unwrap();

    game.data.entities.action[&player_id] = player_action;

    /* Actions */
    game.msg_log.log(Msg::Action(player_id, player_action));

    eprintln!();
    eprintln!("Turn {}:", game.settings.turn_count);

    resolve_messages(&mut game.data, &mut game.msg_log, &mut game.settings, &mut game.rng, &game.config);

    let won_level = level_exit_condition_met(&game.data);

    // resolve enemy action
    let monster = timer!("MONSTER");
    if player_action.takes_turn() && game.data.entities.status[&player_id].alive && !won_level {
        let mut ai_id: Vec<EntityId> = Vec::new();

        // get entitiy ids for any active AI entity
        for key in game.data.entities.ids.iter() {
            if game.data.entities.ai.get(key).is_some()    &&
               game.data.entities.status[key].alive        &&
               game.data.entities.limbo.get(key).is_none() &&
               game.data.entities.fighter.get(key).is_some() {
               ai_id.push(*key);
           }
        }

        for key in ai_id.iter() {
           let action = ai_take_turn(*key, &mut game.data, &game.config, &mut game.msg_log);
           game.data.entities.action[key] = action;

           // if changing state, resolve now and allow another action
           if matches!(action, Action::StateChange(_)) {
                game.msg_log.log(Msg::Action(*key, action));
                resolve_messages(&mut game.data, &mut game.msg_log, &mut game.settings, &mut game.rng, &game.config);
                let backup_action = ai_take_turn(*key, &mut game.data, &game.config, &mut game.msg_log);
                game.data.entities.action[key] = backup_action;
            }
        }

        for key in ai_id.iter() {
            if let Some(action) = game.data.entities.action.get(key).map(|v| *v) {
                game.msg_log.log(Msg::Action(*key, action));
                resolve_messages(&mut game.data, &mut game.msg_log, &mut game.settings, &mut game.rng, &game.config);

                // check if fighter needs to be removed
                if let Some(fighter) = game.data.entities.fighter.get(key) {
                    if fighter.hp <= 0 {
                        game.data.entities.status[key].alive = false;
                        game.data.entities.blocks[key] = false;
                        game.data.entities.chr[key] = '%';
                        game.data.entities.fighter.remove(key);
                    }
                }
            }
        }

        for key in ai_id.iter() {
            // if there are remaining messages for an entity, clear them
            game.data.entities.messages[key].clear();

            let action = ai_take_turn(*key, &mut game.data, &game.config, &mut game.msg_log);
            if matches!(action, Action::StateChange(_)) {
                game.msg_log.log(Msg::Action(*key, action));
                game.data.entities.action[key] = action;
                resolve_messages(&mut game.data, &mut game.msg_log, &mut game.settings, &mut game.rng, &game.config);
            }
        }
    }
    drop(monster);

    // send player turn action in case there is cleanup to perform, or another system
    // needs to know that the turn is finished.
    game.msg_log.log(Msg::PlayerTurn());
    resolve_messages(&mut game.data, &mut game.msg_log, &mut game.settings, &mut game.rng, &game.config);

    let mut to_remove: Vec<EntityId> = Vec::new();

    // check status effects
    for entity_id in game.data.entities.ids.iter() {
        if let Some(mut status) = game.data.entities.status.get_mut(entity_id) {
            if status.frozen > 0 {
                status.frozen -= 1;
            }
        }
    }

    // perform count down
    for entity_id in game.data.entities.ids.iter() {
        if let Some(ref mut count) = game.data.entities.count_down.get_mut(entity_id) {
            if **count == 0 {
                to_remove.push(*entity_id);
            } else {
                **count -= 1;
            }
        }

        if game.data.entities.needs_removal[entity_id] &&
           game.data.entities.animation[entity_id].len() == 0 {
            to_remove.push(*entity_id);
        }
    }

    // remove objects waiting removal
    for key in to_remove {
        game.data.remove_entity(key);
    }

    if player_action.takes_turn() {
        game.settings.turn_count += 1;
    }

    return level_exit_condition_met(&game.data);
}

/// Check whether the exit condition for the game is met.
fn level_exit_condition_met(data: &GameData) -> bool {
    // loop over objects in inventory, and check whether any
    // are the key object.
    let player_id = data.find_by_name(EntityName::Player).unwrap();
    let player_pos = data.entities.pos[&player_id];

    let mut exit_condition = false;
    if let Some(exit_id) = data.find_by_name(EntityName::Exit) {
        let exit_pos = data.entities.pos[&exit_id];

        let has_key = data.is_in_inventory(player_id, Item::Goal).is_some();

        //let on_exit_tile = data.map[player_pos].tile_type == TileType::Exit;
        let on_exit_tile = exit_pos == player_pos;

        exit_condition = has_key && on_exit_tile;
    }

    return exit_condition;
}

#[test]
pub fn test_game_step() {
    let mut config = Config::from_file("../config.yaml");
    config.map_load = MapLoadConfig::Empty;
    let mut game = Game::new(0, config.clone()).unwrap();
    let mut input_action = InputAction::None;

    let player_id = game.data.find_by_name(EntityName::Player).unwrap();
    make_map(&MapLoadConfig::Empty, &mut game);
    let player_pos = game.data.entities.pos[&player_id];
    assert_eq!(Pos::new(0, 0), player_pos);

    input_action = InputAction::Move(Direction::Right);
    game.step_game(input_action, 0.1);
    let player_pos = game.data.entities.pos[&player_id];
    assert_eq!(Pos::new(1, 0), player_pos);

    input_action = InputAction::Move(Direction::Down);
    game.step_game(input_action, 0.1);
    let player_pos = game.data.entities.pos[&player_id];
    assert_eq!(Pos::new(1, 1), player_pos);

    input_action = InputAction::Move(Direction::Left);
    game.step_game(input_action, 0.1);
    let player_pos = game.data.entities.pos[&player_id];
    assert_eq!(Pos::new(0, 1), player_pos);

    input_action = InputAction::Move(Direction::Up);
    game.step_game(input_action, 0.1);
    let player_pos = game.data.entities.pos[&player_id];
    assert_eq!(Pos::new(0, 0), player_pos);
}

// TODO issue 151 removes walking and 150 removes pushing
//      so this test no longer makes any sense.
pub fn test_running() {
    let config = Config::from_file("../config.yaml");
    let mut game = Game::new(0, config.clone()).unwrap();
    let mut input_action = InputAction::None;

    let player_id = game.data.find_by_name(EntityName::Player).unwrap();
    game.data.map = Map::from_dims(10, 10);
    let player_pos = Pos::new(4, 4);
    game.data.entities.pos[&player_id] = player_pos;

    let gol_pos = Pos::new(4, 5);
    let gol = make_gol(&mut game.data.entities, &game.config, gol_pos, &mut game.msg_log);

    game.data.map[(4, 6)].block_move = true;

    // check that running into a monster crushes it against a wall when no empty tiles
    // between
    input_action = InputAction::IncreaseMoveMode;
    game.step_game(input_action, 0.1);

    assert!(game.data.entities.ids.contains(&gol));
    input_action = InputAction::Move(Direction::Down);
    game.step_game(input_action, 0.1);
    let player_pos = game.data.entities.pos[&player_id];
    assert_eq!(gol_pos, player_pos);

    // gol is no longer in entities list after being crushed
    assert!(!game.data.entities.ids.contains(&gol));

    // check that running into a monster, with water 2 tiles away, pushes monster
    // up to the water
    let pawn_pos = Pos::new(5, 5);
    let pawn = make_pawn(&mut game.data.entities, &game.config, pawn_pos, &mut game.msg_log);

    game.data.map[(7, 5)].tile_type = TileType::Water;

    input_action = InputAction::Move(Direction::Right);
    game.step_game(input_action, 0.1);
    assert_eq!(Pos::new(5, 5), game.data.entities.pos[&player_id]);
    assert_eq!(Pos::new(6, 5), game.data.entities.pos[&pawn]);
}

#[test]
pub fn test_hammer_small_wall() {
    let config = Config::from_file("../config.yaml");
    let mut game = Game::new(0, config.clone()).unwrap();
    let mut input_action = InputAction::None;

    let player_id = game.data.find_by_name(EntityName::Player).unwrap();
    game.data.map = Map::from_dims(10, 10);
    let player_pos = Pos::new(4, 4);
    game.data.entities.pos[&player_id] = player_pos;


    game.data.map[player_pos].bottom_wall = Wall::ShortWall;

    let gol_pos = Pos::new(4, 5);
    let gol = make_gol(&mut game.data.entities, &game.config, gol_pos, &mut game.msg_log);

    let hammer = make_hammer(&mut game.data.entities, &game.config, Pos::new(4, 7), &mut game.msg_log);

    game.data.entities.inventory[&player_id].push_front(hammer);

    input_action = InputAction::UseItem;
    game.step_game(input_action, 0.1);

    input_action = InputAction::MapClick(gol_pos, gol_pos);
    game.step_game(input_action, 0.1);

    for msg in game.msg_log.turn_messages.iter() {
        println!("{:?}", msg);
    }

    // gol is no longer in entities list after being crushed
    assert!(game.data.entities.is_dead(gol));

    assert!(game.msg_log.turn_messages.iter().any(|msg| {
        matches!(msg, Msg::HammerHitWall(player_id, _))
    }));

    assert_eq!(Surface::Rubble, game.data.map[gol_pos].surface);

    let pawn_pos = Pos::new(3, 4);
    let pawn = make_pawn(&mut game.data.entities, &game.config, pawn_pos, &mut game.msg_log);
    assert_eq!(true, game.data.entities.status[&pawn].alive);

    // add the hammer back and hit the pawn with it to test hitting entities
    let hammer = make_hammer(&mut game.data.entities, &game.config, Pos::new(4, 7), &mut game.msg_log);
    game.data.entities.inventory[&player_id].push_front(hammer);

    input_action = InputAction::UseItem;
    game.step_game(input_action, 0.1);

    input_action = InputAction::MapClick(pawn_pos, pawn_pos);
    game.step_game(input_action, 0.1);

    assert!(game.data.entities.is_dead(pawn));

    assert!(game.msg_log.turn_messages.iter().any(|msg| {
        matches!(msg, Msg::HammerHitEntity(player_id, pawn))
    }));

    assert_ne!(Surface::Rubble, game.data.map[pawn_pos].surface);
}

