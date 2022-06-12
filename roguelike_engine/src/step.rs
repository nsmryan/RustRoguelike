use logging_timer::timer;

use roguelike_utils::comp::*;

#[cfg(test)]
use roguelike_utils::math::Pos;

#[cfg(test)]
use roguelike_map::*;

use roguelike_core::types::*;
use roguelike_core::ai::*;
use roguelike_core::messaging::{Msg, MsgLog};

#[cfg(test)]
use roguelike_core::movement::MoveMode;
#[cfg(test)]
use roguelike_core::utils::*;
#[cfg(test)]
use roguelike_core::config::*;
#[cfg(test)]
use roguelike_core::movement::*;
use roguelike_core::entities::*;
use roguelike_core::level::*;

use roguelike_core::constants::*;

#[cfg(test)]
use crate::actions::InputAction;
#[cfg(test)]
use crate::generation::*;
#[cfg(test)]
use crate::map_construct::*;

use crate::resolve::resolve_messages;
use crate::game::*;


pub fn step_logic(game: &mut Game) -> bool {
    // clean up removable entities
    clean_entities(&mut game.level.entities, &mut game.msg_log);

    let player_id = game.level.find_by_name(EntityName::Player).unwrap();

    for id in game.level.entities.ids.iter() {
        game.level.entities.took_turn[id] = false;
        game.level.entities.status[id].blinked = false;

        // Slowly thaw any frozen entities.
        if let Some(status) = game.level.entities.status.get_mut(id) {
            if status.frozen > 0 {
                game.msg_log.log(Msg::Thaw(*id, 1));
            }
        }
    }

    resolve_messages(game);

    let won_level = level_exit_condition_met(&game.level);

    // resolve enemy action
    let monster = timer!("MONSTER");
    if game.level.entities.took_turn[&player_id] &&
       game.level.entities.status[&player_id].alive &&
       !won_level {
        step_ai(game);
        run_thumpers(game);
    }
    drop(monster);

    resolve_messages(game);

    // check status effects
    for entity_id in game.level.entities.ids.iter() {
        if let Some(mut status) = game.level.entities.status.get_mut(entity_id) {
            // frozen status is handled separately.

            if status.stone > 0 {
                status.stone -= 1;
            }

            if status.soft_steps > 0 {
                status.soft_steps -= 1;
            }
        }
    }

    if game.level.entities.took_turn[&player_id] {
        game.settings.turn_count += 1;

        // check on whether the player has their hammer raised
        if let Some((item_id, dir, turns)) = game.level.entities.status[&player_id].hammer_raised {
            if turns == 0 {
                game.level.entities.status[&player_id].hammer_raised = None;
            } else {
                game.level.entities.status[&player_id].hammer_raised = Some((item_id, dir, turns - 1));
            }
        }

        if game.level.entities.stamina[&player_id].cooldown == 0 {
            game.level.entities.stamina[&player_id].cooldown = game.config.player_stamina_cooldown;

            if game.level.entities.stamina[&player_id].amount < game.config.player_stamina {
                game.msg_log.log(Msg::GainStamina(player_id, 1));
            }
        } else {
            game.level.entities.stamina[&player_id].cooldown -= 1;
        }
    }

    if game.msg_log.messages.len() > 0 {
        resolve_messages(game);
    }

    // perform count down of entities waiting to be removed
    game.level.entities.count_down();

    for id in game.level.entities.ids.iter() {
        if game.level.entities.needs_removal[id] {
            game.msg_log.log(Msg::MarkedForRemoval(*id));
        }
    }

    return level_exit_condition_met(&game.level);
}

/// Check whether the exit condition for the game is met.
fn level_exit_condition_met(level: &Level) -> bool {
    // loop over objects in inventory, and check whether any
    // are the key object.
    let player_id = level.find_by_name(EntityName::Player).unwrap();
    let player_pos = level.entities.pos[&player_id];

    let mut exit_condition = false;

    if (player_pos.x + 1) == level.map.width() {
        exit_condition = true;
    }
    // Have Key, On exit tile
    /*
    if let Some(exit_id) = level.find_by_name(EntityName::Exit) {
        let exit_pos = level.entities.pos[&exit_id];

        let has_key = level.is_in_inventory(player_id, Item::Key).is_some();

        let on_exit_tile = exit_pos == player_pos;

        exit_condition = has_key && on_exit_tile;
    }
    */

    return exit_condition;
}

#[test]
pub fn test_game_step() {
    let mut config = Config::from_file("../config.yaml");
    config.map_load = MapLoadConfig::Empty;
    let mut game = Game::new(0, config.clone());
    let mut input_action;

    let player_id = game.level.find_by_name(EntityName::Player).unwrap();
    map_construct(&MapLoadConfig::Empty, &mut game);
    game.level.entities.pos[&player_id] = Pos::new(0, 0);

    input_action = InputAction::Move(Direction::Right);
    game.step_game(input_action);
    let player_pos = game.level.entities.pos[&player_id];
    assert_eq!(Pos::new(1, 0), player_pos);

    input_action = InputAction::Move(Direction::Down);
    game.step_game(input_action);
    let player_pos = game.level.entities.pos[&player_id];
    assert_eq!(Pos::new(1, 1), player_pos);

    input_action = InputAction::Move(Direction::Left);
    game.step_game(input_action);
    let player_pos = game.level.entities.pos[&player_id];
    assert_eq!(Pos::new(0, 1), player_pos);

    input_action = InputAction::Move(Direction::Up);
    game.step_game(input_action);
    let player_pos = game.level.entities.pos[&player_id];
    assert_eq!(Pos::new(0, 0), player_pos);
}


fn step_ai(game: &mut Game) {
    let ai_ids: Vec<EntityId> = game.level.entities.active_ais();

    for entity_id in ai_ids.iter() {
        while !game.level.entities.took_turn[entity_id] {
            ai_take_turn(*entity_id, &mut game.level, &game.config, &mut game.msg_log);

            // If the AI has nothing to do, end its turn
            if game.msg_log.messages.len() == 0 {
                game.level.entities.took_turn[entity_id] = true;
            } else {
                resolve_messages(game);
            }
        }

        // If there are remaining messages for an entity, clear them
        game.level.entities.messages[entity_id].clear();
    }
}

#[test]
fn test_ai_idle_player_in_fov() {
    let config = Config::from_file("../config.yaml");
    let mut game = Game::new(0, config);
    map_construct(&MapLoadConfig::Empty, &mut game);

    let start_pos = Pos::new(1, 1);
    let gol = make_gol(&mut game.level.entities, &game.config, start_pos, &mut game.msg_log);

    let player_id = game.level.find_by_name(EntityName::Player).unwrap();
    game.level.entities.pos[&player_id] = add_pos(start_pos, Pos::new(1, 1));

    game.msg_log.clear();
    ai_idle(gol, &mut game.level, &mut game.msg_log, &game.config);

    let player_pos = game.level.entities.pos[&player_id];

    assert_eq!(2, game.msg_log.messages.len());
    assert_eq!(game.msg_log.messages[0], Msg::FaceTowards(gol, player_pos));
    assert_eq!(game.msg_log.messages[1], Msg::StateChange(gol, Behavior::Alert(player_pos)));
}

#[test]
fn test_ai_idle_was_attacked() {
    let config = Config::from_file("../config.yaml");
    let mut game = Game::new(0, config);
    map_construct(&MapLoadConfig::Empty, &mut game);

    let start_pos = Pos::new(0, 0);
    let gol = make_gol(&mut game.level.entities, &game.config, start_pos, &mut game.msg_log);

    let player_id = game.level.find_by_name(EntityName::Player).unwrap();
    game.level.entities.pos[&player_id] = add_pos(start_pos, Pos::new(1, 1));

    game.msg_log.clear();
    // move the player a tile away

    game.level.entities.pos[&player_id] = add_pos(start_pos, Pos::new(3, 0));

    // place a wall between the player and the gol
    game.level.map[(2, 0)] = Tile::wall();

    // check that no messages are created as the monster can't see the player
    ai_idle(gol, &mut game.level, &mut game.msg_log, &game.config);
    dbg!(&game.msg_log.messages);
    assert_eq!(0, game.msg_log.messages.len());

    // if the player attacks, the monster turns and state changes to attacking
    game.level.entities.messages[&gol].push(Message::Attack(player_id));
    ai_idle(gol, &mut game.level, &mut game.msg_log, &game.config);

    let player_pos = game.level.entities.pos[&player_id];
    assert_eq!(2, game.msg_log.messages.len());
    assert_eq!(game.msg_log.messages[0], Msg::FaceTowards(gol, player_pos));
    assert_eq!(game.msg_log.messages[1], Msg::StateChange(gol, Behavior::Attacking(player_id)));
}

#[test]
fn test_ai_idle_heard_sound() {
    let config = Config::from_file("../config.yaml");
    let mut game = Game::new(0, config);
    map_construct(&MapLoadConfig::Empty, &mut game);

    let start_pos = Pos::new(0, 0);
    let gol = make_gol(&mut game.level.entities, &game.config, start_pos, &mut game.msg_log);

    let player_id = game.level.find_by_name(EntityName::Player).unwrap();
    game.level.entities.pos[&player_id] = add_pos(start_pos, Pos::new(1, 1));

    game.msg_log.clear();

    // move the player a tile away
    game.level.entities.pos[&player_id] = add_pos(start_pos, Pos::new(3, 0));

    // place a wall between the player and the gol
    game.level.map[(2, 0)] = Tile::wall();

    // check that no messages are created as the monster can't see the player
    ai_idle(gol, &mut game.level, &mut game.msg_log, &game.config);
    dbg!(&game.msg_log.messages);
    assert_eq!(0, game.msg_log.messages.len());

    // if the monster hears a sound, they investigate
    let sound_pos = Pos::new(0, 1);
    game.level.entities.messages[&gol].push(Message::Sound(sound_pos));
    ai_idle(gol, &mut game.level, &mut game.msg_log, &game.config);

    assert_eq!(2, game.msg_log.messages.len());
    assert_eq!(game.msg_log.messages[0], Msg::FaceTowards(gol, sound_pos));
    assert_eq!(game.msg_log.messages[1], Msg::ReactToSound(gol, sound_pos));
}

#[test]
fn test_ai_investigate_player_in_fov() {
    let config = Config::from_file("../config.yaml");
    let mut game = Game::new(0, config);
    map_construct(&MapLoadConfig::Empty, &mut game);


    let start_pos = Pos::new(0, 0);
    let gol = make_gol(&mut game.level.entities, &game.config, start_pos, &mut game.msg_log);
    game.level.entities.direction[&gol] = Direction::Right;

    let player_id = game.level.find_by_name(EntityName::Player).unwrap();
    game.level.entities.pos[&player_id] = add_pos(start_pos, Pos::new(1, 1));

    let player_pos = game.level.entities.pos[&player_id];
    game.level.entities.behavior[&gol] = Behavior::Investigating(player_pos);

    game.msg_log.clear();
    ai_investigate(player_pos, gol, &mut game.level, &mut game.msg_log, &game.config);

    assert_eq!(2, game.msg_log.messages.len());
    assert_eq!(game.msg_log.messages[0], Msg::FaceTowards(gol, player_pos));
    assert_eq!(game.msg_log.messages[1], Msg::StateChange(gol, Behavior::Attacking(player_id)));
}

#[test]
fn test_ai_investigate_not_in_fov_heard_sound() {
    let config = Config::from_file("../config.yaml");
    let mut game = Game::new(0, config);
    map_construct(&MapLoadConfig::Empty, &mut game);


    let start_pos = Pos::new(0, 0);
    let gol = make_gol(&mut game.level.entities, &game.config, start_pos, &mut game.msg_log);
    game.msg_log.clear();
    game.level.entities.direction[&gol] = Direction::Right;

    let player_id = game.level.find_by_name(EntityName::Player).unwrap();
    game.level.entities.pos[&player_id] = add_pos(start_pos, Pos::new(1, 1));

    let player_pos = game.level.entities.pos[&player_id];
    game.level.entities.behavior[&gol] = Behavior::Investigating(player_pos);

    // move the player a tile away
    game.level.entities.pos[&player_id] = add_pos(start_pos, Pos::new(3, 0));

    // place a wall between the player and the gol
    game.level.map[(2, 0)] = Tile::wall();

    // if the monster hears a sound, they investigate
    let sound_pos = Pos::new(0, 1);
    game.level.entities.messages[&gol].push(Message::Sound(sound_pos));
    ai_investigate(player_pos, gol, &mut game.level, &mut game.msg_log, &game.config);

    assert_eq!(1, game.msg_log.messages.len());
    assert_eq!(game.msg_log.messages[0], Msg::StateChange(gol, Behavior::Investigating(sound_pos)));
}

#[test]
fn test_ai_investigate_moves() {
    let config = Config::from_file("../config.yaml");
    let mut game = Game::new(0, config);
    map_construct(&MapLoadConfig::Empty, &mut game);


    let start_pos = Pos::new(0, 0);
    let gol = make_gol(&mut game.level.entities, &game.config, start_pos, &mut game.msg_log);
    game.level.entities.direction[&gol] = Direction::Right;

    let player_id = game.level.find_by_name(EntityName::Player).unwrap();
    game.level.entities.pos[&player_id] = add_pos(start_pos, Pos::new(5, 1));

    // place walls between the player and the gol
    game.level.map[(2, 0)] = Tile::wall();
    game.level.map[(2, 1)] = Tile::wall();

    // if the monster hears a sound, they investigate
    let sound_pos = Pos::new(0, 1);
    game.msg_log.clear();
    game.level.entities.messages[&gol].push(Message::Sound(sound_pos));
    ai_investigate(sound_pos, gol, &mut game.level, &mut game.msg_log, &game.config);

    assert_eq!(1, game.msg_log.messages.len());
    assert_eq!(game.msg_log.messages[0], Msg::StateChange(gol, Behavior::Investigating(sound_pos)));

    // if they investigate again, they try to move to the sound
    game.msg_log.clear();
    ai_investigate(sound_pos, gol, &mut game.level, &mut game.msg_log, &game.config);
    assert_eq!(1, game.msg_log.messages.len());
    let direction = Direction::from_positions(start_pos, sound_pos).unwrap();
    assert_eq!(Msg::TryMove(gol, direction, 1, MoveMode::Walk), game.msg_log.messages[0]);
}

#[test]
fn test_pick_up_primary() {
    let mut game = Game::new(0, Config::from_file("../config.yaml"));
    map_construct(&MapLoadConfig::Empty, &mut game);

    let player_id = game.level.find_by_name(EntityName::Player).unwrap();
    let start_pos = game.level.entities.pos[&player_id];

    let sword = make_sword(&mut game.level.entities, &game.config, start_pos, &mut game.msg_log);
    let next_pos = move_x(start_pos, 1);
    let hammer = make_hammer(&mut game.level.entities, &game.config, next_pos, &mut game.msg_log);

    assert_eq!(0, game.level.entities.inventory[&player_id].len());
    game.step_game(InputAction::Pickup);
    assert_eq!(1, game.level.entities.inventory[&player_id].len());
    let item_id = game.level.entities.inventory[&player_id][0];
    assert_eq!(sword, item_id);

    game.step_game(InputAction::Move(Direction::Right));
    assert_eq!(game.level.entities.pos[&hammer], game.level.entities.pos[&player_id]);

    game.step_game(InputAction::Pickup);
    assert_eq!(1, game.level.entities.inventory[&player_id].len());
    let item_id = game.level.entities.inventory[&player_id][0];
    assert_eq!(hammer, item_id);
}

#[test]
fn test_pick_up_consumables() {
    let mut game = Game::new(0, Config::from_file("../config.yaml"));
    map_construct(&MapLoadConfig::Empty, &mut game);

    let player_id = game.level.find_by_name(EntityName::Player).unwrap();
    let start_pos = game.level.entities.pos[&player_id];

    let lantern0 = make_lantern(&mut game.level.entities, &game.config, start_pos, &mut game.msg_log);
    let next_pos = move_x(start_pos, 1);
    let lantern1 = make_lantern(&mut game.level.entities, &game.config, next_pos, &mut game.msg_log);

    assert_eq!(0, game.level.entities.inventory[&player_id].len());
    game.step_game(InputAction::Pickup);
    assert_eq!(1, game.level.entities.inventory[&player_id].len());
    let item_id = game.level.entities.inventory[&player_id][0];
    assert_eq!(lantern0, item_id);

    game.step_game(InputAction::Move(Direction::Right));
    assert_eq!(game.level.entities.pos[&lantern1], game.level.entities.pos[&player_id]);

    game.step_game(InputAction::Pickup);
    assert_eq!(1, game.level.entities.inventory[&player_id].len());
    let item_id = game.level.entities.inventory[&player_id][0];
    assert_eq!(lantern1, item_id);
}

#[test]
fn test_pick_up_misc() {
    let mut game = Game::new(0, Config::from_file("../config.yaml"));
    map_construct(&MapLoadConfig::Empty, &mut game);

    let player_id = game.level.find_by_name(EntityName::Player).unwrap();
    let start_pos = game.level.entities.pos[&player_id];

    let stone0 = make_stone(&mut game.level.entities, &game.config, start_pos, &mut game.msg_log);
    let stone1 = make_stone(&mut game.level.entities, &game.config, start_pos, &mut game.msg_log);
    let key = make_key(&mut game.level.entities, &game.config, start_pos, &mut game.msg_log);

    assert_eq!(0, game.level.entities.inventory[&player_id].len());

    game.step_game(InputAction::Pickup);
    game.step_game(InputAction::Pickup);
    game.step_game(InputAction::Pickup);

    let inventory = game.level.entities.inventory[&player_id].clone();
    assert_eq!(3, inventory.len());
    assert!(inventory.iter().position(|id| *id == stone0).is_some());
    assert!(inventory.iter().position(|id| *id == stone1).is_some());
    assert!(inventory.iter().position(|id| *id == key).is_some());
}

#[test]
fn test_use_mode_drop() {
    let mut game = Game::new(0, Config::from_file("../config.yaml"));
    map_construct(&MapLoadConfig::Empty, &mut game);

    let player_id = game.level.find_by_name(EntityName::Player).unwrap();
    let start_pos = game.level.entities.pos[&player_id];

    let _stone = make_stone(&mut game.level.entities, &game.config, start_pos, &mut game.msg_log);
    game.step_game(InputAction::Pickup);

    let _lantern = make_lantern(&mut game.level.entities, &game.config, start_pos, &mut game.msg_log);
    game.step_game(InputAction::Pickup);

    let _sword = make_sword(&mut game.level.entities, &game.config, start_pos, &mut game.msg_log);
    game.step_game(InputAction::Pickup);

    assert_eq!(3, game.level.entities.inventory[&player_id].len());

    game.step_game(InputAction::StartUseItem(ItemClass::Consumable));
    game.step_game(InputAction::DropItem);
    assert_eq!(2, game.level.entities.inventory[&player_id].len());

    game.step_game(InputAction::StartUseItem(ItemClass::Primary));
    game.step_game(InputAction::DropItem);
    assert_eq!(1, game.level.entities.inventory[&player_id].len());

    assert_eq!(GameState::Playing, game.settings.state);
}

#[test]
fn test_throw_stone() {
    let mut game = Game::new(0, Config::from_file("../config.yaml"));
    map_construct(&MapLoadConfig::Empty, &mut game);

    let player_id = game.level.find_by_name(EntityName::Player).unwrap();
    let start_pos = game.level.entities.pos[&player_id];

    /* test throwing at a column */
    let stone = make_stone(&mut game.level.entities, &game.config, start_pos, &mut game.msg_log);
    game.step_game(InputAction::Pickup);

    // throwing at an enemy is covered by test_use_mode_stone
    let col_pos = move_x(start_pos, PLAYER_THROW_DIST as i32);
    let col = make_column(&mut game.level.entities, &game.config, col_pos, &mut game.msg_log);

    game.step_game(InputAction::StartUseItem(ItemClass::Misc));
    assert_eq!(Some(start_pos), game.settings.cursor);
    for _ in 0..PLAYER_THROW_DIST {
        game.step_game(InputAction::CursorMove(Direction::Right, false, false));
    }
    game.step_game(InputAction::CursorToggle);

    // the stone lands before the column
    let land_pos = move_x(start_pos, PLAYER_THROW_DIST as i32 - 1);
    assert_eq!(land_pos, game.level.entities.pos[&stone]);


    /* test throwing at a wall */
    let stone = make_stone(&mut game.level.entities, &game.config, start_pos, &mut game.msg_log);
    game.step_game(InputAction::Pickup);

    let wall_pos = move_y(start_pos, 3 as i32);
    game.level.map[wall_pos] = Tile::wall();

    game.step_game(InputAction::StartUseItem(ItemClass::Misc));
    assert_eq!(Some(start_pos), game.settings.cursor);
    for _ in 0..3 {
        game.step_game(InputAction::CursorMove(Direction::Down, false, false));
    }
    game.step_game(InputAction::CursorToggle);

    // throwing the stone down at the wall lands it just before the wall.
    let land_pos = move_y(start_pos, 2);
    assert_eq!(land_pos, game.level.entities.pos[&stone]);
  
    /* test throwing at a empty floor */
    let stone = make_stone(&mut game.level.entities, &game.config, start_pos, &mut game.msg_log);
    game.step_game(InputAction::Pickup);

    let floor_pos = Direction::DownRight.offset_pos(start_pos, PLAYER_THROW_DIST as i32);

    game.step_game(InputAction::StartUseItem(ItemClass::Misc));
    assert_eq!(Some(start_pos), game.settings.cursor);
    for _ in 0..PLAYER_THROW_DIST {
        game.step_game(InputAction::CursorMove(Direction::DownRight, false, false));
    }
    game.step_game(InputAction::CursorToggle);

    // throwing the stone into an empty area lands it where it is thrown
    assert_eq!(floor_pos, game.level.entities.pos[&stone]);
}

fn run_thumpers(game: &mut Game) {
    for id in game.level.entities.ids.iter() {
        if game.level.entities.name[id] == EntityName::Thumper {
            let pos = game.level.entities.pos[id];
            game.msg_log.log(Msg::Sound(*id, pos, SOUND_RADIUS_THUMPER));
        }
    }
}

/// Remove entities that have previously been marked for removal.
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

