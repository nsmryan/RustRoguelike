use logging_timer::timer;

use roguelike_core::types::*;
use roguelike_core::config::*;
use roguelike_core::ai::*;
use roguelike_core::map::*;
use roguelike_core::messaging::Msg;
use roguelike_core::movement::{Direction, MoveMode};
#[cfg(test)]
use roguelike_core::utils::*;


use crate::game::*;
use crate::actions::InputAction;
use crate::generation::*;
use crate::resolve::resolve_messages;
#[cfg(test)]
use crate::make_map::*;


pub fn step_logic(game: &mut Game) -> bool {
    let player_id = game.data.find_by_name(EntityName::Player).unwrap();

    for id in game.data.entities.ids.iter() {
        game.data.entities.took_turn[id] = false;
    }

    eprintln!();
    eprintln!("Turn {}:", game.settings.turn_count);

    game.msg_log.log_front(Msg::StartTurn);

    resolve_messages(&mut game.data, &mut game.msg_log, &mut game.rng, &game.config);

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
    resolve_messages(&mut game.data, &mut game.msg_log, &mut game.rng, &game.config);

    // check status effects
    for entity_id in game.data.entities.ids.iter() {
        if let Some(mut status) = game.data.entities.status.get_mut(entity_id) {
            if status.frozen > 0 {
                status.frozen -= 1;
            }
        }
    }

    if game.data.entities.took_turn[&player_id] {
        game.settings.turn_count += 1;
    }

    // perform count down, removing entities that ask to be removed
    game.data.count_down();

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
    let mut game = Game::new(0, config.clone()).unwrap();
    let mut input_action;

    let player_id = game.data.find_by_name(EntityName::Player).unwrap();
    make_map(&MapLoadConfig::Empty, &mut game);
    game.data.entities.pos[&player_id] = Pos::new(0, 0);

    input_action = InputAction::Move(Direction::Right, MoveMode::Walk);
    game.step_game(input_action, 0.1);
    let player_pos = game.data.entities.pos[&player_id];
    assert_eq!(Pos::new(1, 0), player_pos);

    input_action = InputAction::Move(Direction::Down, MoveMode::Walk);
    game.step_game(input_action, 0.1);
    let player_pos = game.data.entities.pos[&player_id];
    assert_eq!(Pos::new(1, 1), player_pos);

    input_action = InputAction::Move(Direction::Left, MoveMode::Walk);
    game.step_game(input_action, 0.1);
    let player_pos = game.data.entities.pos[&player_id];
    assert_eq!(Pos::new(0, 1), player_pos);

    input_action = InputAction::Move(Direction::Up, MoveMode::Walk);
    game.step_game(input_action, 0.1);
    let player_pos = game.data.entities.pos[&player_id];
    assert_eq!(Pos::new(0, 0), player_pos);
}

pub fn test_running() {
    let config = Config::from_file("../config.yaml");
    let mut game = Game::new(0, config.clone()).unwrap();
    let mut input_action;

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
    input_action = InputAction::Move(Direction::Down, MoveMode::Walk);
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

    input_action = InputAction::Move(Direction::Right, MoveMode::Walk);
    game.step_game(input_action, 0.1);
    assert_eq!(Pos::new(5, 5), game.data.entities.pos[&player_id]);
    assert_eq!(Pos::new(6, 5), game.data.entities.pos[&pawn]);
}

#[test]
pub fn test_hammer_small_wall() {
    let config = Config::from_file("../config.yaml");
    let mut game = Game::new(0, config.clone()).unwrap();
    let mut input_action;

    let player_id = game.data.find_by_name(EntityName::Player).unwrap();
    game.data.map = Map::from_dims(10, 10);
    let player_pos = Pos::new(4, 4);
    game.data.entities.pos[&player_id] = player_pos;


    game.data.map[player_pos].bottom_wall = Wall::ShortWall;

    let gol_pos = Pos::new(4, 5);
    let gol = make_gol(&mut game.data.entities, &game.config, gol_pos, &mut game.msg_log);

    let hammer = make_hammer(&mut game.data.entities, &game.config, Pos::new(4, 7), &mut game.msg_log);

    game.data.entities.inventory[&player_id].push_front(hammer);

    input_action = InputAction::UseItem(Direction::Down, 0);
    game.step_game(input_action, 0.1);

    for msg in game.msg_log.turn_messages.iter() {
        println!("{:?}", msg);
    }

    // gol is no longer in entities list after being crushed
    assert!(game.data.entities.is_dead(gol));

    assert!(game.msg_log.turn_messages.iter().any(|msg| {
        matches!(msg, Msg::HammerHitWall(_, _))
    }));

    assert_eq!(Surface::Rubble, game.data.map[gol_pos].surface);

    let pawn_pos = Pos::new(3, 4);
    let pawn = make_pawn(&mut game.data.entities, &game.config, pawn_pos, &mut game.msg_log);
    assert_eq!(true, game.data.entities.status[&pawn].alive);

    // add the hammer back and hit the pawn with it to test hitting entities
    let hammer = make_hammer(&mut game.data.entities, &game.config, Pos::new(4, 7), &mut game.msg_log);
    game.data.entities.inventory[&player_id].push_front(hammer);

    input_action = InputAction::UseItem(Direction::Left, 0);
    game.step_game(input_action, 0.1);

    input_action = InputAction::MapClick(pawn_pos, pawn_pos);
    game.step_game(input_action, 0.1);

    assert!(game.data.entities.is_dead(pawn));

    assert!(game.msg_log.turn_messages.iter().any(|msg| {
        *msg == Msg::HammerHitEntity(player_id, pawn)
    }));

    assert_ne!(Surface::Rubble, game.data.map[pawn_pos].surface);
}

fn step_ai(game: &mut Game) {
    let ai_ids: Vec<EntityId> = game.data.entities.active_ais();

    for key in ai_ids.iter() {
       ai_take_turn(*key, &mut game.data, &game.config, &mut game.msg_log);

       resolve_messages(&mut game.data, &mut game.msg_log, &mut game.rng, &game.config);

       // if there are remaining messages for an entity, clear them
       game.data.entities.messages[key].clear();
    }
}

#[test]
fn test_ai_idle_player_in_fov() {
    let config = Config::from_file("../config.yaml");
    let mut game = Game::new(0, config).unwrap();
    make_map(&MapLoadConfig::Empty, &mut game);

    let start_pos = Pos::new(1, 1);
    let gol = make_gol(&mut game.data.entities, &game.config, start_pos, &mut game.msg_log);

    let player_id = game.data.find_by_name(EntityName::Player).unwrap();
    game.data.entities.pos[&player_id] = add_pos(start_pos, Pos::new(1, 1));

    game.msg_log.clear();
    ai_idle(gol, &mut game.data, &mut game.msg_log, &game.config);

    let player_pos = game.data.entities.pos[&player_id];

    assert_eq!(game.msg_log.messages[0], Msg::FaceTowards(gol, player_pos));
    assert_eq!(game.msg_log.messages[1], Msg::StateChange(gol, Behavior::Attacking(player_id)));
}

#[test]
fn test_ai_idle_was_attacked() {
    let config = Config::from_file("../config.yaml");
    let mut game = Game::new(0, config).unwrap();
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
    assert_eq!(game.msg_log.messages[0], Msg::FaceTowards(gol, player_pos));
    assert_eq!(game.msg_log.messages[1], Msg::StateChange(gol, Behavior::Attacking(player_id)));
}

#[test]
fn test_ai_idle_heard_sound() {
    let config = Config::from_file("../config.yaml");
    let mut game = Game::new(0, config).unwrap();
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
    let mut game = Game::new(0, config).unwrap();
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
    let mut game = Game::new(0, config).unwrap();
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
    let mut game = Game::new(0, config).unwrap();
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
