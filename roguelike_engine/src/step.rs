use logging_timer::timer;

use roguelike_core::types::*;
use roguelike_core::ai::*;
use roguelike_core::messaging::{Msg, MsgLog};
use roguelike_core::map::{FovResult};

#[cfg(test)]
use roguelike_core::movement::MoveMode;
#[cfg(test)]
use roguelike_core::utils::*;
#[cfg(test)]
use roguelike_core::config::*;
#[cfg(test)]
use roguelike_core::movement::*;
#[cfg(test)]
use roguelike_core::map::*;
#[cfg(test)]
use roguelike_core::constants::*;

#[cfg(test)]
use crate::actions::InputAction;
#[cfg(test)]
use crate::generation::*;
#[cfg(test)]
use crate::make_map::*;

use crate::resolve::resolve_messages;
use crate::game::*;


pub fn step_logic(game: &mut Game) -> bool {
    // clean up removable entities
    clean_entities(&mut game.data.entities, &mut game.msg_log);

    let player_id = game.data.find_by_name(EntityName::Player).unwrap();

    for id in game.data.entities.ids.iter() {
        game.data.entities.took_turn[id] = false;
        game.data.entities.status[id].blinked = false;
    }

    game.msg_log.log_front(Msg::StartTurn);

    resolve_messages(game);

    let won_level = level_exit_condition_met(&game.data);

    // resolve enemy action
    let monster = timer!("MONSTER");
    if game.data.entities.took_turn[&player_id] &&
       game.data.entities.status[&player_id].alive &&
       !won_level {
        step_ai(game);
    }
    drop(monster);

    // send PlayerTurn action in case there is cleanup to perform, or another system
    // needs to know that the turn is finished.
    game.msg_log.log(Msg::PlayerTurn);
    resolve_messages(game);

    // check status effects
    for entity_id in game.data.entities.ids.iter() {
        if let Some(mut status) = game.data.entities.status.get_mut(entity_id) {
            if status.frozen > 0 {
                status.frozen -= 1;
            }

            if status.stone > 0 {
                status.stone -= 1;
            }

            if status.soft_steps > 0 {
                status.soft_steps -= 1;
            }
        }
    }

    if game.data.entities.took_turn[&player_id] {
        game.settings.turn_count += 1;

        // check on whether the player has their hammer raised
        if let Some((item_id, dir, turns)) = game.data.entities.status[&player_id].hammer_raised {
            if turns == 0 {
                game.data.entities.status[&player_id].hammer_raised = None;
            } else {
                game.data.entities.status[&player_id].hammer_raised = Some((item_id, dir, turns - 1));
            }
        }
    }

    // perform count down of entities waiting to be removed
    game.data.entities.count_down();

    post_step(game);

    return level_exit_condition_met(&game.data);
}

pub fn post_step(game: &mut Game) {
    let player_id = game.data.find_by_name(EntityName::Player).unwrap();
    let (map_width, map_height) = game.data.map.size();

    for y in 0..map_height {
        for x in 0..map_width {
            let pos = Pos::new(x, y);
            let fov_result;
            if game.settings.god_mode {
                fov_result = FovResult::Inside;
            } else {
                fov_result = game.data.pos_in_fov_edge(player_id, pos, &game.config);
            }
            game.msg_log.log(Msg::TileFov(pos, fov_result));
        }
    }
}

/// Check whether the exit condition for the game is met.
fn level_exit_condition_met(data: &Level) -> bool {
    // loop over objects in inventory, and check whether any
    // are the key object.
    let player_id = data.find_by_name(EntityName::Player).unwrap();
    let player_pos = data.entities.pos[&player_id];

    let mut exit_condition = false;
    if let Some(exit_id) = data.find_by_name(EntityName::Exit) {
        let exit_pos = data.entities.pos[&exit_id];

        let has_key = data.is_in_inventory(player_id, Item::Key).is_some();

        let on_exit_tile = exit_pos == player_pos;

        exit_condition = has_key && on_exit_tile;
    }

    return exit_condition;
}

#[test]
pub fn test_game_step() {
    let mut config = Config::from_file("../config.yaml");
    config.map_load = MapLoadConfig::Empty;
    let mut game = Game::new(0, config.clone());
    let mut input_action;

    let player_id = game.data.find_by_name(EntityName::Player).unwrap();
    make_map(&MapLoadConfig::Empty, &mut game);
    game.data.entities.pos[&player_id] = Pos::new(0, 0);

    input_action = InputAction::Move(Direction::Right);
    game.step_game(input_action);
    let player_pos = game.data.entities.pos[&player_id];
    assert_eq!(Pos::new(1, 0), player_pos);

    input_action = InputAction::Move(Direction::Down);
    game.step_game(input_action);
    let player_pos = game.data.entities.pos[&player_id];
    assert_eq!(Pos::new(1, 1), player_pos);

    input_action = InputAction::Move(Direction::Left);
    game.step_game(input_action);
    let player_pos = game.data.entities.pos[&player_id];
    assert_eq!(Pos::new(0, 1), player_pos);

    input_action = InputAction::Move(Direction::Up);
    game.step_game(input_action);
    let player_pos = game.data.entities.pos[&player_id];
    assert_eq!(Pos::new(0, 0), player_pos);
}


fn step_ai(game: &mut Game) {
    let ai_ids: Vec<EntityId> = game.data.entities.active_ais();

    for key in ai_ids.iter() {
       ai_take_turn(*key, &mut game.data, &game.config, &mut game.msg_log);

       resolve_messages(game);

       // if there are remaining messages for an entity, clear them
       game.data.entities.messages[key].clear();
    }
}

#[test]
fn test_ai_idle_player_in_fov() {
    let config = Config::from_file("../config.yaml");
    let mut game = Game::new(0, config);
    make_map(&MapLoadConfig::Empty, &mut game);

    let start_pos = Pos::new(1, 1);
    let gol = make_gol(&mut game.data.entities, &game.config, start_pos, &mut game.msg_log);

    let player_id = game.data.find_by_name(EntityName::Player).unwrap();
    game.data.entities.pos[&player_id] = add_pos(start_pos, Pos::new(1, 1));

    game.msg_log.clear();
    ai_idle(gol, &mut game.data, &mut game.msg_log, &game.config);

    let player_pos = game.data.entities.pos[&player_id];

    assert_eq!(2, game.msg_log.messages.len());
    assert_eq!(game.msg_log.messages[0], Msg::FaceTowards(gol, player_pos));
    assert_eq!(game.msg_log.messages[1], Msg::StateChange(gol, Behavior::Attacking(player_id)));
}

#[test]
fn test_ai_idle_was_attacked() {
    let config = Config::from_file("../config.yaml");
    let mut game = Game::new(0, config);
    make_map(&MapLoadConfig::Empty, &mut game);

    let start_pos = Pos::new(0, 0);
    let gol = make_gol(&mut game.data.entities, &game.config, start_pos, &mut game.msg_log);

    let player_id = game.data.find_by_name(EntityName::Player).unwrap();
    game.data.entities.pos[&player_id] = add_pos(start_pos, Pos::new(1, 1));

    game.msg_log.clear();
    // move the player a tile away

    game.data.entities.pos[&player_id] = add_pos(start_pos, Pos::new(3, 0));

    // place a wall between the player and the gol
    game.data.map[(2, 0)] = Tile::wall();

    // check that no messages are created as the monster can't see the player
    ai_idle(gol, &mut game.data, &mut game.msg_log, &game.config);
    dbg!(&game.msg_log.messages);
    assert_eq!(0, game.msg_log.messages.len());

    // if the player attacks, the monster turns and state changes to attacking
    game.data.entities.messages[&gol].push(Message::Attack(player_id));
    ai_idle(gol, &mut game.data, &mut game.msg_log, &game.config);

    let player_pos = game.data.entities.pos[&player_id];
    assert_eq!(2, game.msg_log.messages.len());
    assert_eq!(game.msg_log.messages[0], Msg::FaceTowards(gol, player_pos));
    assert_eq!(game.msg_log.messages[1], Msg::StateChange(gol, Behavior::Attacking(player_id)));
}

#[test]
fn test_ai_idle_heard_sound() {
    let config = Config::from_file("../config.yaml");
    let mut game = Game::new(0, config);
    make_map(&MapLoadConfig::Empty, &mut game);

    let start_pos = Pos::new(0, 0);
    let gol = make_gol(&mut game.data.entities, &game.config, start_pos, &mut game.msg_log);

    let player_id = game.data.find_by_name(EntityName::Player).unwrap();
    game.data.entities.pos[&player_id] = add_pos(start_pos, Pos::new(1, 1));

    game.msg_log.clear();

    // move the player a tile away
    game.data.entities.pos[&player_id] = add_pos(start_pos, Pos::new(3, 0));

    // place a wall between the player and the gol
    game.data.map[(2, 0)] = Tile::wall();

    // check that no messages are created as the monster can't see the player
    ai_idle(gol, &mut game.data, &mut game.msg_log, &game.config);
    dbg!(&game.msg_log.messages);
    assert_eq!(0, game.msg_log.messages.len());

    // if the monster hears a sound, they investigate
    let sound_pos = Pos::new(0, 1);
    game.data.entities.messages[&gol].push(Message::Sound(player_id, sound_pos));
    ai_idle(gol, &mut game.data, &mut game.msg_log, &game.config);

    assert_eq!(2, game.msg_log.messages.len());
    assert_eq!(game.msg_log.messages[0], Msg::FaceTowards(gol, sound_pos));
    assert_eq!(game.msg_log.messages[1], Msg::StateChange(gol, Behavior::Investigating(sound_pos)));
}

#[test]
fn test_ai_investigate_player_in_fov() {
    let config = Config::from_file("../config.yaml");
    let mut game = Game::new(0, config);
    make_map(&MapLoadConfig::Empty, &mut game);


    let start_pos = Pos::new(0, 0);
    let gol = make_gol(&mut game.data.entities, &game.config, start_pos, &mut game.msg_log);
    game.data.entities.direction[&gol] = Direction::Right;

    let player_id = game.data.find_by_name(EntityName::Player).unwrap();
    game.data.entities.pos[&player_id] = add_pos(start_pos, Pos::new(1, 1));

    let player_pos = game.data.entities.pos[&player_id];
    game.data.entities.behavior[&gol] = Behavior::Investigating(player_pos);

    game.msg_log.clear();
    ai_investigate(player_pos, gol, &mut game.data, &mut game.msg_log, &game.config);

    assert_eq!(2, game.msg_log.messages.len());
    assert_eq!(game.msg_log.messages[0], Msg::FaceTowards(gol, player_pos));
    assert_eq!(game.msg_log.messages[1], Msg::StateChange(gol, Behavior::Attacking(player_id)));
}

#[test]
fn test_ai_investigate_not_in_fov_heard_sound() {
    let config = Config::from_file("../config.yaml");
    let mut game = Game::new(0, config);
    make_map(&MapLoadConfig::Empty, &mut game);


    let start_pos = Pos::new(0, 0);
    let gol = make_gol(&mut game.data.entities, &game.config, start_pos, &mut game.msg_log);
    game.msg_log.clear();
    game.data.entities.direction[&gol] = Direction::Right;

    let player_id = game.data.find_by_name(EntityName::Player).unwrap();
    game.data.entities.pos[&player_id] = add_pos(start_pos, Pos::new(1, 1));

    let player_pos = game.data.entities.pos[&player_id];
    game.data.entities.behavior[&gol] = Behavior::Investigating(player_pos);

    // move the player a tile away
    game.data.entities.pos[&player_id] = add_pos(start_pos, Pos::new(3, 0));

    // place a wall between the player and the gol
    game.data.map[(2, 0)] = Tile::wall();

    // if the monster hears a sound, they investigate
    let sound_pos = Pos::new(0, 1);
    game.data.entities.messages[&gol].push(Message::Sound(player_id, sound_pos));
    ai_investigate(player_pos, gol, &mut game.data, &mut game.msg_log, &game.config);

    assert_eq!(1, game.msg_log.messages.len());
    assert_eq!(game.msg_log.messages[0], Msg::StateChange(gol, Behavior::Investigating(sound_pos)));
}

#[test]
fn test_ai_investigate_moves() {
    let config = Config::from_file("../config.yaml");
    let mut game = Game::new(0, config);
    make_map(&MapLoadConfig::Empty, &mut game);


    let start_pos = Pos::new(0, 0);
    let gol = make_gol(&mut game.data.entities, &game.config, start_pos, &mut game.msg_log);
    game.data.entities.direction[&gol] = Direction::Right;

    let player_id = game.data.find_by_name(EntityName::Player).unwrap();
    game.data.entities.pos[&player_id] = add_pos(start_pos, Pos::new(5, 1));

    // place walls between the player and the gol
    game.data.map[(2, 0)] = Tile::wall();
    game.data.map[(2, 1)] = Tile::wall();

    // if the monster hears a sound, they investigate
    let sound_pos = Pos::new(0, 1);
    game.msg_log.clear();
    game.data.entities.messages[&gol].push(Message::Sound(player_id, sound_pos));
    ai_investigate(sound_pos, gol, &mut game.data, &mut game.msg_log, &game.config);

    assert_eq!(1, game.msg_log.messages.len());
    assert_eq!(game.msg_log.messages[0], Msg::StateChange(gol, Behavior::Investigating(sound_pos)));

    // if they investigate again, they try to move to the sound
    game.msg_log.clear();
    ai_investigate(sound_pos, gol, &mut game.data, &mut game.msg_log, &game.config);
    assert_eq!(1, game.msg_log.messages.len());
    let direction = Direction::from_positions(start_pos, sound_pos).unwrap();
    assert_eq!(Msg::TryMove(gol, direction, 1, MoveMode::Walk), game.msg_log.messages[0]);
}

#[test]
fn test_pick_up_primary() {
    let mut game = Game::new(0, Config::from_file("../config.yaml"));
    make_map(&MapLoadConfig::Empty, &mut game);

    let player_id = game.data.find_by_name(EntityName::Player).unwrap();
    let start_pos = game.data.entities.pos[&player_id];

    let sword = make_sword(&mut game.data.entities, &game.config, start_pos, &mut game.msg_log);
    let next_pos = move_x(start_pos, 1);
    let hammer = make_hammer(&mut game.data.entities, &game.config, next_pos, &mut game.msg_log);

    assert_eq!(0, game.data.entities.inventory[&player_id].len());
    game.step_game(InputAction::Pickup);
    assert_eq!(1, game.data.entities.inventory[&player_id].len());
    let item_id = game.data.entities.inventory[&player_id][0];
    assert_eq!(sword, item_id);

    game.step_game(InputAction::Move(Direction::Right));
    assert_eq!(game.data.entities.pos[&hammer], game.data.entities.pos[&player_id]);

    game.step_game(InputAction::Pickup);
    assert_eq!(1, game.data.entities.inventory[&player_id].len());
    let item_id = game.data.entities.inventory[&player_id][0];
    assert_eq!(hammer, item_id);
}

#[test]
fn test_pick_up_consumables() {
    let mut game = Game::new(0, Config::from_file("../config.yaml"));
    make_map(&MapLoadConfig::Empty, &mut game);

    let player_id = game.data.find_by_name(EntityName::Player).unwrap();
    let start_pos = game.data.entities.pos[&player_id];

    let lantern0 = make_lantern(&mut game.data.entities, &game.config, start_pos, &mut game.msg_log);
    let next_pos = move_x(start_pos, 1);
    let lantern1 = make_lantern(&mut game.data.entities, &game.config, next_pos, &mut game.msg_log);

    assert_eq!(0, game.data.entities.inventory[&player_id].len());
    game.step_game(InputAction::Pickup);
    assert_eq!(1, game.data.entities.inventory[&player_id].len());
    let item_id = game.data.entities.inventory[&player_id][0];
    assert_eq!(lantern0, item_id);

    game.step_game(InputAction::Move(Direction::Right));
    assert_eq!(game.data.entities.pos[&lantern1], game.data.entities.pos[&player_id]);

    game.step_game(InputAction::Pickup);
    assert_eq!(1, game.data.entities.inventory[&player_id].len());
    let item_id = game.data.entities.inventory[&player_id][0];
    assert_eq!(lantern1, item_id);
}

#[test]
fn test_pick_up_misc() {
    let mut game = Game::new(0, Config::from_file("../config.yaml"));
    make_map(&MapLoadConfig::Empty, &mut game);

    let player_id = game.data.find_by_name(EntityName::Player).unwrap();
    let start_pos = game.data.entities.pos[&player_id];

    let stone0 = make_stone(&mut game.data.entities, &game.config, start_pos, &mut game.msg_log);
    let stone1 = make_stone(&mut game.data.entities, &game.config, start_pos, &mut game.msg_log);
    let key = make_key(&mut game.data.entities, &game.config, start_pos, &mut game.msg_log);

    assert_eq!(0, game.data.entities.inventory[&player_id].len());

    game.step_game(InputAction::Pickup);
    game.step_game(InputAction::Pickup);
    game.step_game(InputAction::Pickup);

    let inventory = game.data.entities.inventory[&player_id].clone();
    assert_eq!(3, inventory.len());
    assert!(inventory.iter().position(|id| *id == stone0).is_some());
    assert!(inventory.iter().position(|id| *id == stone1).is_some());
    assert!(inventory.iter().position(|id| *id == key).is_some());
}

#[test]
fn test_use_mode_stone() {
    let mut game = Game::new(0, Config::from_file("../config.yaml"));
    make_map(&MapLoadConfig::Empty, &mut game);

    let player_id = game.data.find_by_name(EntityName::Player).unwrap();
    let start_pos = game.data.entities.pos[&player_id];

    // make sure there is a key in the inventory, just to show that it is not
    // used when throwing a stone despite being a 'Misc' item class.
    let _key = make_key(&mut game.data.entities, &game.config, start_pos, &mut game.msg_log);
    game.step_game(InputAction::Pickup);

    let stone = make_stone(&mut game.data.entities, &game.config, start_pos, &mut game.msg_log);
    game.step_game(InputAction::Pickup);

    let gol_pos = move_x(start_pos, PLAYER_THROW_DIST as i32);
    let gol = make_gol(&mut game.data.entities, &game.config, gol_pos, &mut game.msg_log);

    game.step_game(InputAction::StartUseItem(ItemClass::Misc));
    game.step_game(InputAction::UseDir(Direction::Right));
    game.step_game(InputAction::FinalizeUse);

    assert_eq!(gol_pos, game.data.entities.pos[&stone]);

    // The gol remains in its starting position because it was stunned by the stone.
    assert_eq!(gol_pos, game.data.entities.pos[&gol]);
}

#[test]
fn test_use_mode_drop() {
    let mut game = Game::new(0, Config::from_file("../config.yaml"));
    make_map(&MapLoadConfig::Empty, &mut game);

    let player_id = game.data.find_by_name(EntityName::Player).unwrap();
    let start_pos = game.data.entities.pos[&player_id];

    let _stone = make_stone(&mut game.data.entities, &game.config, start_pos, &mut game.msg_log);
    game.step_game(InputAction::Pickup);

    let _lantern = make_lantern(&mut game.data.entities, &game.config, start_pos, &mut game.msg_log);
    game.step_game(InputAction::Pickup);

    let _sword = make_sword(&mut game.data.entities, &game.config, start_pos, &mut game.msg_log);
    game.step_game(InputAction::Pickup);

    assert_eq!(3, game.data.entities.inventory[&player_id].len());

    game.step_game(InputAction::StartUseItem(ItemClass::Misc));
    game.step_game(InputAction::DropItem);
    assert_eq!(2, game.data.entities.inventory[&player_id].len());

    game.step_game(InputAction::StartUseItem(ItemClass::Consumable));
    game.step_game(InputAction::DropItem);
    assert_eq!(1, game.data.entities.inventory[&player_id].len());

    game.step_game(InputAction::StartUseItem(ItemClass::Primary));
    game.step_game(InputAction::DropItem);
    assert_eq!(0, game.data.entities.inventory[&player_id].len());

    assert_eq!(GameState::Playing, game.settings.state);
}

fn clean_entities(entities: &mut Entities, msg_log: &mut MsgLog) {
    let mut remove_ids: Vec<EntityId> = Vec::new();
    for id in entities.ids.iter() {
        if entities.needs_removal[id] {
            remove_ids.push(*id);
        }
    }

    for id in remove_ids {
        msg_log.log(Msg::RemovedEntity(id));
        entities.remove_entity(id);
    }
}

