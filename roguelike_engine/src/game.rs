use rand::prelude::*;

use serde::{Serialize, Deserialize};

//use sdl2::keyboard::Keycode;

use roguelike_core::types::*;
use roguelike_core::config::*;
use roguelike_core::ai::*;
use roguelike_core::map::*;
use roguelike_core::messaging::{Msg, MsgLog};
use roguelike_core::movement::{Direction, Action, Reach};
use roguelike_core::utils::{move_towards, distance, sub_pos, next_pos};
#[cfg(test)]
use roguelike_core::movement::*;


use crate::actions;
use crate::actions::InputAction; //, KeyDirection};
use crate::generation::*;
use crate::make_map::read_map_xp;
use crate::resolve::resolve_messages;
#[cfg(test)]
use crate::make_map::*;


#[derive(Copy, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum GameResult {
    Continue,
    Stop,
}

#[derive(Copy, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum SelectionAction {
    Throw,
    Hammer,
    Interact,
    PlaceTrap,
    GrassThrow,
    PassWall,
    Rubble,
    Reform,
    Swap,
}

impl SelectionAction {
    pub fn action_from_pos(&self, pos: Pos, data: &GameData) -> Action {
        let mut action: Action;

        match self {
            SelectionAction::Throw => {
                let player_id = data.find_player().unwrap();
                let item_id =
                    data.entities.selected_item.get(&player_id).expect("Throwing an item, but nothing selected!");
                action = Action::ThrowItem(pos, *item_id);
            }

            SelectionAction::Hammer => {
                action = Action::UseItem(pos);
            }

            SelectionAction::Interact => {
                // TODO implement
                // if there is a trap, toggled armed
                // perhaps other interactions are possible
                action = Action::NoAction;
                if let Some(entity_id) = data.has_entity(pos) {
                    if let Some(_trap) = data.entities.trap.get(&entity_id) {
                        action = Action::ArmDisarmTrap(entity_id);
                    }
                }
            }

            SelectionAction::PlaceTrap => {
                let player_id = data.find_player().unwrap();
                let trap_id =
                    data.entities.selected_item.get(&player_id).expect("Placing a trap, but nothing selected!");
                action = Action::PlaceTrap(pos, *trap_id);
            }

            SelectionAction::GrassThrow => {
                let player_id = data.find_player().unwrap();
                let player_pos = data.entities.pos[&player_id];
                let dxy = sub_pos(pos, player_pos);
                let direction = Direction::from_dxy(dxy.x, dxy.y).unwrap();
                action = Action::GrassThrow(player_id, direction);
            }

            SelectionAction::PassWall => {
                action = Action::NoAction;

                let player_id = data.find_player().unwrap();
                let player_pos = data.entities.pos[&player_id];
                let dxy = sub_pos(pos, player_pos);
                let blocked = data.map.is_blocked_by_wall(player_pos, dxy.x, dxy.y);
                
                let mut pass_pos = pos;
                if let Some(blocked) = blocked {
                    if data.map[blocked.end_pos].blocked {
                        let next = next_pos(player_pos, dxy);
                        if  !data.map[next].blocked {
                            action = Action::PassWall(player_id, next);
                        }
                    } else {
                        action = Action::PassWall(player_id, pos);
                    }
                }
            }

            SelectionAction::Rubble => {
                action = Action::NoAction;

                let player_id = data.find_player().unwrap();
                let player_pos = data.entities.pos[&player_id];
                let dxy = sub_pos(pos, player_pos);
                let blocked = data.map.is_blocked_by_wall(player_pos, dxy.x, dxy.y);

                if let Some(blocked) = blocked {
                    if data.has_blocking_entity(pos).is_none() {
                        action = Action::Rubble(player_id, blocked);
                    }
                }
            }

            SelectionAction::Reform => {
                action = Action::NoAction;

                let player_id = data.find_player().unwrap();
                if data.map[pos].surface == Surface::Rubble &&
                   data.has_blocking_entity(pos).is_none() {
                    action = Action::Reform(player_id, pos);
                }
            }

            SelectionAction::Swap => {
                action = Action::NoAction;

                let player_id = data.find_player().unwrap();
                if let Some(entity_id) = data.has_blocking_entity(pos) {
                    action = Action::Swap(player_id, entity_id);
                }
            }
        }

        return action;
    }
}


#[derive(Copy, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum SelectionType {
    WithinReach(Reach),
    WithinRadius(usize),
}

#[derive(Copy, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Selection {
    pub typ: SelectionType,
    pub action: SelectionAction,
    pub only_visible: bool,
    // TODO consider adding:
    // SelectionFilter enum with Entity/Wall/Empty/Any
    // position to selection will have to check available positions and find one that matches
    // the filter
}

impl Default for Selection {
    fn default() -> Selection {
        return Selection::new(SelectionType::WithinRadius(0), SelectionAction::Throw);
    }
}

impl Selection {
    pub fn new(typ: SelectionType, action: SelectionAction) -> Self {
        return Selection {
            typ,
            action,
            only_visible: false,
        };
    }

    pub fn selected_pos(&self, pos: Pos, selected: Pos, fov_radius: i32, data: &mut GameData) -> Option<Pos> {
        let mut maybe_selected_pos: Option<Pos>;

        match self.typ {
            SelectionType::WithinReach(reach) => {
                let selected_pos = reach.closest_to(pos, selected);

                maybe_selected_pos = Some(selected_pos);
            }

            SelectionType::WithinRadius(radius) => {
                let selected_pos: Pos;
                if distance(selected, pos) as usize <= radius {
                    selected_pos = selected;
                } else {
                    selected_pos = move_towards(pos, selected, radius);
                }

                maybe_selected_pos = Some(selected_pos);
            }
        }

        if self.only_visible {
            if let Some(selected_pos) = maybe_selected_pos {
                if !data.map.is_in_fov(pos, selected_pos, fov_radius) {
                    maybe_selected_pos = None;
                }
            }
        }

        return maybe_selected_pos;
    }

    pub fn select(&self, pos: Pos, selected: Pos, fov_radius: i32, data: &mut GameData) -> Option<Action> {
        let maybe_selected_pos: Option<Pos> = self.selected_pos(pos, selected, fov_radius, data);

        if let Some(selected_pos) = maybe_selected_pos {
            return Some(self.action.action_from_pos(selected_pos, data));
        } else {
            return None;
        }
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct GameSettings {
    pub turn_count: usize,
    pub god_mode: bool,
    pub map_type: MapGenType,
    pub exiting: bool,
    pub state: GameState,
    pub draw_selection_overlay: bool,
    pub overlay: bool,
    pub console: bool,
    pub time: f32,
    pub render_map: bool,
    pub selection: Selection,
}

impl GameSettings {
    pub fn new(turn_count: usize,
               god_mode: bool) -> GameSettings {
        return GameSettings {
            turn_count,
            god_mode,
            map_type: MapGenType::Island,
            exiting: false,
            state: GameState::Playing,
            draw_selection_overlay: false,
            overlay: false,
            console: false,
            time: 0.0,
            render_map: true,
            selection: Selection::default(),
        };
    }
}

pub struct Game {
    pub config: Config,
    pub input_action: InputAction,
    //pub key_input: Vec<(KeyDirection, Keycode)>,
    pub mouse_state: MouseState,
    pub data: GameData,
    pub settings: GameSettings,
    pub msg_log: MsgLog,
    pub rng: SmallRng,
}

impl Game {
    pub fn new(seed: u64, config: Config) -> Result<Game, String> {
        let entities = Entities::new();
        let rng: SmallRng = SeedableRng::seed_from_u64(seed);

        let mut msg_log = MsgLog::new();

        let map = Map::empty();

        let mut data = GameData::new(map, entities);

        let player_id = make_player(&mut data.entities, &config, &mut msg_log);
        data.entities.pos[&player_id] = Pos::new(-1, -1);

        let stone_id = make_stone(&mut data.entities, &config, Pos::new(-1, -1), &mut msg_log);
        data.entities.inventory[&player_id].push_back(stone_id);

        let state = Game {
            config,
            input_action: InputAction::None,
            data,
            settings: GameSettings::new(0, false),
            mouse_state: Default::default(),
            msg_log,
            //key_input: Vec::new(),
            rng: rng,
        };

        return Ok(state);
    }

    pub fn step_game(&mut self, dt: f32) -> GameResult {
        self.settings.time += dt;

        match self.settings.state {
            GameState::Playing => {
                return self.step_playing();
            }

            GameState::Win => {
                return self.step_win();
            }

            GameState::Lose => {
                return self.step_lose();
            }

            GameState::Inventory => {
                return self.step_inventory();
            }

            GameState::Selection => {
                return self.step_selection();
            }

            GameState::SkillMenu => {
                return self.step_skill_menu();
            }
        }
    }

    fn step_win(&mut self) -> GameResult {

        if matches!(self.input_action, InputAction::Exit) {
            return GameResult::Stop;
        }

        self.msg_log.log(Msg::ChangeLevel());

        self.data.entities.clear();
        let _player_pos =
            read_map_xp(&self.config, &mut self.data, &mut self.msg_log, "resources/map.xp");

        self.settings.state = GameState::Playing;

        // NOTE Exit game on win for now
        return GameResult::Stop;
    }

    fn step_lose(&mut self) -> GameResult {
        if self.input_action == InputAction::Exit {
            return GameResult::Stop;
        }

        return GameResult::Continue;
    }

    fn step_inventory(&mut self) -> GameResult {
        let input = self.input_action;
        self.input_action = InputAction::None;

        actions::handle_input_inventory(input, &mut self.data, &mut self.settings, &mut self.msg_log);

        if self.settings.exiting {
            return GameResult::Stop;
        }

        return GameResult::Continue;
    }

    fn step_skill_menu(&mut self) -> GameResult {
        let input = self.input_action;
        self.input_action = InputAction::None;

        let player_action =
            actions::handle_input_skill_menu(input, &mut self.data, &mut self.settings, &mut self.msg_log);

        if player_action != Action::NoAction {
            let win = step_logic(player_action,
                                 &mut self.data,
                                 &mut self.settings,
                                 &self.config,
                                 &mut self.rng,
                                 &mut self.msg_log);

            if win {
                self.settings.state = GameState::Win;
            }
        }

        if self.settings.exiting {
            return GameResult::Stop;
        }

        return GameResult::Continue;
    }

    fn step_selection(&mut self) -> GameResult {
        let input = self.input_action;
        self.input_action = InputAction::None;

        self.settings.draw_selection_overlay = true;

        let player_action =
            actions::handle_input_selection(input,
                                           &mut self.data,
                                           &mut self.settings,
                                           &self.config,
                                           &mut self.msg_log);

        if player_action != Action::NoAction {
            let win = step_logic(player_action,
                                 &mut self.data,
                                 &mut self.settings,
                                 &self.config,
                                 &mut self.rng,
                                 &mut self.msg_log);
            if win {
                self.settings.state = GameState::Win;
            }
        }

        if self.settings.exiting {
            return GameResult::Stop;
        }

        return GameResult::Continue;
    }

//    fn step_console(&mut self) -> GameResult {
//        let input = self.input_action;
//        self.input_action = InputAction::None;
//
//        let time_since_open = self.settings.time - self.console.time_at_open;
//        let lerp_amount = clampf(time_since_open / self.config.console_speed, 0.0, 1.0);
//        self.console.height = lerp(self.console.height as f32,
//                                   self.config.console_max_height as f32,
//                                   lerp_amount) as u32;
//        if (self.console.height as i32 - self.config.console_max_height as i32).abs() < 2 {
//            self.console.height = self.config.console_max_height;
//        }
//
//        if self.key_input.len() > 0 {
//            // TODO add console back in
//            //actions::handle_input_console(input,
//            //                              &mut self.key_input,
//            //                              &mut self.console,
//            //                              &mut self.data,
//            //                              &mut self.display_state,
//            //                              &mut self.settings,
//            //                              &self.config,
//            //                              &mut self.msg_log);
//        }
//
//        return GameResult::Continue;
//    }

    fn step_playing(&mut self) -> GameResult {
        let player_action =
            actions::handle_input(self);

        if player_action != Action::NoAction {
            let win = step_logic(player_action,
                                 &mut self.data,
                                 &mut self.settings,
                                 &self.config,
                                 &mut self.rng,
                                 &mut self.msg_log);
            if win {
                self.settings.state = GameState::Win;
            }
        }

        if self.settings.exiting {
            return GameResult::Stop;
        }

        self.input_action = InputAction::None;

        return GameResult::Continue;
    }
}

/// Check whether the exit condition for the game is met.
fn win_condition_met(data: &GameData) -> bool {
    // loop over objects in inventory, and check whether any
    // are the key object.
    let player_id = data.find_player().unwrap();

    let has_key = 
        data.entities.inventory[&player_id].iter().any(|item_id| {
            data.entities.item.get(item_id) == Some(&Item::Goal)
        });

    let player_pos = data.entities.pos[&player_id];
    let on_exit_tile = data.map[player_pos].tile_type == TileType::Exit;

    let exit_condition = has_key && on_exit_tile;

    return exit_condition;
}

pub fn step_logic(player_action: Action,
                  data: &mut GameData, 
                  settings: &mut GameSettings,
                  config: &Config,
                  rng: &mut SmallRng,
                  msg_log: &mut MsgLog) -> bool {
    msg_log.clear();

    let player_id = data.find_player().unwrap();

    let previous_player_position =
        data.entities.pos[&player_id];

    data.entities.action[&player_id] = player_action;

    /* Actions */
    msg_log.log(Msg::Action(player_id, player_action));

    println!("Turn {}:", settings.turn_count);

    resolve_messages(data, msg_log, settings, rng, config);

    if data.entities.alive[&player_id] {
        let mut ai_id: Vec<EntityId> = Vec::new();

        for key in data.entities.ids.iter() {
            if data.entities.ai.get(key).is_some() &&
               data.entities.alive[key]            &&
               data.entities.fighter.get(key).is_some() {
               ai_id.push(*key);
           }
        }

        for key in ai_id.iter() {
            let action = ai_take_turn(*key, data, config, msg_log);
           data.entities.action[key] = action;

           // if changing state, resolve now and allow another action
           if matches!(action, Action::StateChange(_)) {
                msg_log.log(Msg::Action(*key, action));
                resolve_messages(data, msg_log, settings, rng, config);
                let backup_action = ai_take_turn(*key, data, config, msg_log);
                data.entities.action[key] = backup_action;
            }
        }

        for key in ai_id {
            if let Some(action) = data.entities.action.get(&key).map(|v| *v) {
                msg_log.log(Msg::Action(key, action));
                resolve_messages(data, msg_log, settings, rng, config);

                // check if fighter needs to be removed
                if let Some(fighter) = data.entities.fighter.get(&key) {
                    if fighter.hp <= 0 {
                        data.entities.alive[&key] = false;
                        data.entities.blocks[&key] = false;
                        data.entities.chr[&key] = '%';
                        data.entities.fighter.remove(&key);
                    }
                }
            }
        }
    }

    // TODO this shouldn't be necessary- it should be part of msg handling
    // check if player lost all hp
    if let Some(fighter) = data.entities.fighter.get(&player_id) {
        if fighter.hp <= 0 {
            // modify player
            {
                data.entities.alive[&player_id] = false;
                data.entities.color[&player_id] = config.color_red;
            }

            if settings.state == GameState::Playing {
                settings.state = GameState::Lose;
            }
        }
    }

    let mut to_remove: Vec<EntityId> = Vec::new();

    // perform count down
    for entity_id in data.entities.ids.iter() {
        if let Some(ref mut count) = data.entities.count_down.get_mut(entity_id) {
            if **count == 0 {
                to_remove.push(*entity_id);
            } else {
                **count -= 1;
            }
        }

        if data.entities.needs_removal[entity_id] &&
           data.entities.animation[entity_id].len() == 0 {
            to_remove.push(*entity_id);
        }
    }

    // remove objects waiting removal
    for key in to_remove {
        data.entities.remove(&key);
    }

    /* Recompute FOV */
    let player_pos = data.entities.pos[&player_id];
    if previous_player_position != player_pos {
        data.map.compute_fov(player_pos, config.fov_radius_player);
    }

    settings.turn_count += 1;

    return win_condition_met(data);
}

#[test]
pub fn test_game_step() {
    let mut config = Config::from_file("../config.yaml");
    config.map_load = MapLoadConfig::Empty;
    let mut game = Game::new(0, config.clone()).unwrap();

    let player_id = game.data.find_player().unwrap();
    make_map(&MapLoadConfig::Empty, &mut game);
    let player_pos = game.data.entities.pos[&player_id];
    assert_eq!(Pos::new(0, 0), player_pos);

    game.input_action = InputAction::Move(Direction::Right);
    game.step_game(0.1);
    let player_pos = game.data.entities.pos[&player_id];
    assert_eq!(Pos::new(1, 0), player_pos);

    game.input_action = InputAction::Move(Direction::Down);
    game.step_game(0.1);
    let player_pos = game.data.entities.pos[&player_id];
    assert_eq!(Pos::new(1, 1), player_pos);

    game.input_action = InputAction::Move(Direction::Left);
    game.step_game(0.1);
    let player_pos = game.data.entities.pos[&player_id];
    assert_eq!(Pos::new(0, 1), player_pos);

    game.input_action = InputAction::Move(Direction::Up);
    game.step_game(0.1);
    let player_pos = game.data.entities.pos[&player_id];
    assert_eq!(Pos::new(0, 0), player_pos);
}

#[test]
pub fn test_hammer_small_wall() {
    let mut config = Config::from_file("../config.yaml");
    let mut game = Game::new(0, config.clone()).unwrap();

    let player_id = game.data.find_player().unwrap();
    game.data.map = Map::from_dims(10, 10);
    let player_pos = Pos::new(4, 4);
    game.data.entities.pos[&player_id] = player_pos;


    game.data.map[player_pos].bottom_wall = Wall::ShortWall;

    let gol_pos = Pos::new(4, 5);
    let gol = make_gol(&mut game.data.entities, &game.config, gol_pos, &mut game.msg_log);

    let pawn_pos = Pos::new(3, 4);
    let pawn = make_elf(&mut game.data.entities, &game.config, pawn_pos, &mut game.msg_log);

    let hammer = make_hammer(&mut game.data.entities, &game.config, Pos::new(4, 7), &mut game.msg_log);

    game.data.entities.inventory[&player_id].push_front(hammer);

    game.input_action = InputAction::UseItem;
    game.step_game(0.1);

    game.input_action = InputAction::MapClick(gol_pos, gol_pos);
    game.step_game(0.1);

    for msg in game.msg_log.turn_messages.iter() {
        println!("{:?}", msg);
    }

    // gol is no longer in entities list after being crushed
    assert!(!game.data.entities.ids.contains(&gol));
    assert!(game.data.entities.ids.contains(&pawn));

    assert!(game.msg_log.turn_messages.iter().any(|msg| {
        matches!(msg, Msg::HammerHitWall(player_id, _))
    }));

    assert_eq!(Surface::Rubble, game.data.map[gol_pos].surface);

    // add the hammer back and hit the pawn with it to test hitting entities
    let hammer = make_hammer(&mut game.data.entities, &game.config, Pos::new(4, 7), &mut game.msg_log);
    game.data.entities.inventory[&player_id].push_front(hammer);

    game.input_action = InputAction::UseItem;
    game.step_game(0.1);

    game.input_action = InputAction::MapClick(pawn_pos, pawn_pos);
    game.step_game(0.1);

    assert!(!game.data.entities.ids.contains(&pawn));

    assert!(game.msg_log.turn_messages.iter().any(|msg| {
        matches!(msg, Msg::HammerHitEntity(player_id, pawn))
    }));

    assert_ne!(Surface::Rubble, game.data.map[pawn_pos].surface);
}


#[test]
pub fn test_game_map() {
    // Create Game and Map
    let mut config = Config::from_file("../config.yaml");
    config.map_load = MapLoadConfig::TestMap;
    let mut game = Game::new(0, config.clone()).unwrap();

    let player_id = game.data.find_player().unwrap();
    make_map(&MapLoadConfig::TestMap, &mut game);
    let player_pos = game.data.entities.pos[&player_id];
    assert_eq!(Pos::new(0, 0), player_pos);

    // Find entities
    let pawn = *game.data.entities.ids.iter().find(|id| {
        game.data.entities.name.get(*id) == Some(&EntityName::Pawn)
    }).unwrap();

    let spikes = *game.data.entities.ids.iter().find(|id| {
        game.data.entities.name.get(*id) == Some(&EntityName::Spike)
    }).unwrap();

    let gol = *game.data.entities.ids.iter().find(|id| {
        game.data.entities.name.get(*id) == Some(&EntityName::Gol)
    }).unwrap();

    let col1 = *game.data.entities.ids.iter().find(|id| {
        game.data.entities.name.get(*id) == Some(&EntityName::Column) &&
        game.data.entities.pos[*id] == Pos::new(7, 6)
    }).unwrap();
        
    let col2 = *game.data.entities.ids.iter().find(|id| {
        game.data.entities.name.get(*id) == Some(&EntityName::Column) &&
        game.data.entities.pos[*id] == Pos::new(7, 5)
    }).unwrap();

    // Utility functions
    let mut test_move = |game: &mut Game, dir, pos| {
        game.input_action = InputAction::Move(dir);
        game.step_game(0.1);
        let player_pos = game.data.entities.pos[&player_id];
        assert_eq!(Pos::from(pos), player_pos);
    };

    let mut input = |game: &mut Game, action| {
        game.input_action = action;
        game.step_game(0.1);
    };

    // make sure move modes work, to avoid errors further on
    assert_eq!(MoveMode::Walk, game.data.entities.move_mode[&player_id]);
    input(&mut game, InputAction::DecreaseMoveMode);
    assert_eq!(MoveMode::Sneak, game.data.entities.move_mode[&player_id]);
    input(&mut game, InputAction::IncreaseMoveMode);
    assert_eq!(MoveMode::Walk, game.data.entities.move_mode[&player_id]);
    input(&mut game, InputAction::IncreaseMoveMode);
    assert_eq!(MoveMode::Run, game.data.entities.move_mode[&player_id]);
    input(&mut game, InputAction::DecreaseMoveMode);
    assert_eq!(MoveMode::Walk, game.data.entities.move_mode[&player_id]);

    // walk around a bit
    test_move(&mut game, Direction::Right, (1, 0));
    test_move(&mut game, Direction::Down,  (1, 1));
    test_move(&mut game, Direction::Left,  (0, 1));

    // hit short wall below us
    test_move(&mut game, Direction::Down,  (0, 1));

    // test out short wall jumping
    input(&mut game, InputAction::IncreaseMoveMode);
    test_move(&mut game, Direction::Down,  (0, 2));
    test_move(&mut game, Direction::Up,    (0, 1));
    input(&mut game, InputAction::DecreaseMoveMode);
    test_move(&mut game, Direction::Up,    (0, 0));
    input(&mut game, InputAction::IncreaseMoveMode);
    test_move(&mut game, Direction::Down,  (0, 2));

    // run into wall
    test_move(&mut game, Direction::Right,  (2, 2));
    test_move(&mut game, Direction::Right,  (2, 2));
    input(&mut game, InputAction::DecreaseMoveMode);
    test_move(&mut game, Direction::Right,  (2, 2));
    test_move(&mut game, Direction::Left,   (1, 2));
    input(&mut game, InputAction::IncreaseMoveMode);
    test_move(&mut game, Direction::Right,  (2, 2));

    input(&mut game, InputAction::DecreaseMoveMode);
    test_move(&mut game, Direction::Down,  (2, 3));
    // hits left wall, doesn't move
    test_move(&mut game, Direction::Left,  (2, 3));
    input(&mut game, InputAction::IncreaseMoveMode);
    // jumps over wall
    test_move(&mut game, Direction::Left,  (1, 3));
    // jumps back over wall
    test_move(&mut game, Direction::Right, (2, 3));

    // trigger a trap by pushing a monster into it
    input(&mut game, InputAction::DecreaseMoveMode);
    input(&mut game, InputAction::DecreaseMoveMode);
    test_move(&mut game, Direction::Down, (2, 4));
    test_move(&mut game, Direction::Down, (2, 5));
    test_move(&mut game, Direction::Down, (2, 6));
    test_move(&mut game, Direction::Down, (2, 7));
    test_move(&mut game, Direction::Down, (2, 8));
    test_move(&mut game, Direction::Down, (2, 9));

    assert!(game.msg_log.turn_messages.iter().any(|msg| {
        matches!(msg, Msg::SpikeTrapTriggered(spike, pawn))
    }));

    assert!(game.msg_log.turn_messages.iter().any(|msg| {
        matches!(msg, Msg::Killed(spike, pawn, _))
    }));

    // pick up dagger and try to stab enemy through wall
    test_move(&mut game, Direction::DownRight, (3, 10));
    game.input_action = InputAction::Pickup;
    game.step_game(0.1);
    assert!(game.data.using(player_id, Item::Dagger));

    let gol_hp = game.data.entities.fighter[&gol].hp;

    test_move(&mut game, Direction::Right, (4, 10));
    test_move(&mut game, Direction::Right, (5, 10));
    test_move(&mut game, Direction::Right, (6, 10));
    test_move(&mut game, Direction::Right, (7, 10));
    test_move(&mut game, Direction::Right, (7, 10));

    assert!(game.data.entities.ids.contains(&gol));

    assert_eq!(gol_hp, game.data.entities.fighter[&gol].hp);
    assert!(game.data.entities.alive[&gol]);

    // walk into column and check that it knocks over the second column
    test_move(&mut game, Direction::Up, (7, 9));
    test_move(&mut game, Direction::Up, (7, 8));
    test_move(&mut game, Direction::Up, (7, 7));
    test_move(&mut game, Direction::Up, (7, 6));

    assert!(game.msg_log.turn_messages.iter().any(|msg| {
        matches!(msg, Msg::Crushed(player_id, col1))
    }));

    assert!(game.msg_log.turn_messages.iter().any(|msg| {
        matches!(msg, Msg::Crushed(col1, col2))
    }));
}

