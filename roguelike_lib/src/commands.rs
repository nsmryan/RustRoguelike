use std::str::FromStr;
use std::time::Instant;

use roguelike_core::types::*;
use roguelike_core::movement::*;
use roguelike_core::map::*;
use roguelike_core::messaging::*;
use roguelike_core::constants::*;

use roguelike_engine::input::*;
use roguelike_engine::game::*;
use roguelike_engine::generation::*;


// TODO
// components: get and set
// map: create from MapConfigs
//      redo map with current config
//
// possibly:
// inject messages?
// change settings
// change config
// line positions?
// blocked pos, blocked path, etc
// fov
// convenience, like visible entity ids

// MousePos(i32, i32),
// MouseButton(MouseClick, Pos, Option<Pos>, KeyDir),
// Esc,
// Tab,
// Quit,

pub enum GameCmd {
    PlayerId,
    Pos(u64),
    SetPos(u64, i32, i32),
    Hp(u64),
    SetHp(u64, i32),
    Facing(u64),
    SetFacing(u64, Direction),
    MapSize,
    TileWalls(i32, i32),
    SetTileWalls(i32, i32, TileType, Wall, Wall), // type, left, bottom
    Surface(i32, i32),
    SetSurface(i32, i32, Surface),
    EntityName(u64),
    EntityType(u64),
    Spawn(EntityName, i32, i32),
    Remove(u64),
    Kill(u64),
    Give(Item),
    ListEntities,
    ListEntitiesPos(i32, i32),
    Key(char, KeyDir),
    Ctrl(KeyDir),
    Alt(KeyDir),
    Shift(KeyDir),
    Exit,
}

impl FromStr for GameCmd {
    type Err = String;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let s: &mut str = &mut string.to_string();
        s.make_ascii_lowercase();

        // TODO probably next() for cmd and have only arguments in args
        let mut args = s.split(" "); // .collect::<Vec<&str>>();
        let cmd = args.next().unwrap();

        if cmd == "player_id" {
            return Ok(GameCmd::PlayerId);
        } else if cmd == "pos" {
            let id = args.next().ok_or("no arg")?.parse::<u64>().map_err(|err| format!("{}", err))?;
            return Ok(GameCmd::Pos(id));
        } else if cmd == "set_pos" {
            let id = args.next().ok_or("no arg")?.parse::<u64>().map_err(|err| format!("{}", err))?;
            let x  = args.next().ok_or("no arg")?.parse::<i32>().map_err(|err| format!("{}", err))?;
            let y  = args.next().ok_or("no arg")?.parse::<i32>().map_err(|err| format!("{}", err))?;
            return Ok(GameCmd::SetPos(id, x, y));
        } else if cmd == "hp" {
            let id = args.next().ok_or("no arg")?.parse::<u64>().map_err(|err| format!("{}", err))?;
            if let Some(hp) = args.next() {
                let hp  = hp.parse::<i32>().map_err(|err| format!("{}", err))?;
                return Ok(GameCmd::SetHp(id, hp));
            } else {
                return Ok(GameCmd::Hp(id));
            }
        } else if cmd == "facing" {
            let id = args.next().ok_or("no arg")?.parse::<u64>().map_err(|err| format!("{}", err))?;
            if let Some(dir) = args.next() {
                let dir  = dir.parse::<Direction>().map_err(|err| format!("{}", err))?;
                return Ok(GameCmd::SetFacing(id, dir));
            } else {
                return Ok(GameCmd::Facing(id));
            }
        } else if cmd == "map_size" {
            return Ok(GameCmd::MapSize);
        } else if cmd == "tile_walls" {
            let x  = args.next().ok_or("no arg")?.parse::<i32>().map_err(|err| format!("{}", err))?;
            let y  = args.next().ok_or("no arg")?.parse::<i32>().map_err(|err| format!("{}", err))?;
            return Ok(GameCmd::TileWalls(x, y));
        } else if cmd == "set_tile_walls" {
            let x       = args.next().ok_or("no arg")?.parse::<i32>().map_err(|err| format!("{}", err))?;
            let y       = args.next().ok_or("no arg")?.parse::<i32>().map_err(|err| format!("{}", err))?;
            let typ     = args.next().ok_or("no arg")?.parse::<TileType>().map_err(|err| format!("{}", err))?;
            let left    = args.next().ok_or("no arg")?.parse::<Wall>().map_err(|err| format!("{}", err))?;
            let bottom  = args.next().ok_or("no arg")?.parse::<Wall>().map_err(|err| format!("{}", err))?;
            return Ok(GameCmd::SetTileWalls(x, y, typ, left, bottom));
        } else if cmd == "surface" {
            let x  = args.next().ok_or("no arg")?.parse::<i32>().map_err(|err| format!("{}", err))?;
            let y  = args.next().ok_or("no arg")?.parse::<i32>().map_err(|err| format!("{}", err))?;
            return Ok(GameCmd::Surface(x, y));
        } else if cmd == "set_surface" {
            let x  = args.next().ok_or("no arg")?.parse::<i32>().map_err(|err| format!("{}", err))?;
            let y  = args.next().ok_or("no arg")?.parse::<i32>().map_err(|err| format!("{}", err))?;
            let surface  = args.next().ok_or("no arg")?.parse::<Surface>().map_err(|err| format!("{}", err))?;
            return Ok(GameCmd::SetSurface(x, y, surface));
        } else if cmd == "entity_name" {
            let id = args.next().ok_or("no arg")?.parse::<u64>().map_err(|err| format!("{}", err))?;
            return Ok(GameCmd::EntityName(id));
        } else if cmd == "entity_type" {
            let id = args.next().ok_or("no arg")?.parse::<u64>().map_err(|err| format!("{}", err))?;
            return Ok(GameCmd::EntityType(id));
        } else if cmd == "spawn" {
            let entity_name = args.next().ok_or("no arg")?.parse::<EntityName>().map_err(|err| format!("{}", err))?;
            let x  = args.next().ok_or("no arg")?.parse::<i32>().map_err(|err| format!("{}", err))?;
            let y  = args.next().ok_or("no arg")?.parse::<i32>().map_err(|err| format!("{}", err))?;

            return Ok(GameCmd::Spawn(entity_name, x, y));
        } else if cmd == "remove" {
            let id = args.next().ok_or("no arg")?.parse::<u64>().map_err(|err| format!("{}", err))?;
            return Ok(GameCmd::Remove(id));
        } else if cmd == "kill" {
            let id = args.next().ok_or("no arg")?.parse::<u64>().map_err(|err| format!("{}", err))?;
            return Ok(GameCmd::Kill(id));
        } else if cmd == "give" {
            let item = args.next().ok_or("no arg")?.parse::<Item>().map_err(|err| format!("{}", err))?;
            return Ok(GameCmd::Give(item));
        } else if cmd == "ids" {
            return Ok(GameCmd::ListEntities);
        } else if cmd == "ids_pos" {
            let x  = args.next().ok_or("no arg")?.parse::<i32>().map_err(|err| format!("{}", err))?;
            let y  = args.next().ok_or("no arg")?.parse::<i32>().map_err(|err| format!("{}", err))?;
            return Ok(GameCmd::ListEntitiesPos(x, y));
        } else if cmd == "key" {
            let chr = args.next().ok_or("no arg")?.parse::<char>().map_err(|err| format!("{}", err))?;
            let dir = args.next().ok_or("no arg")?.parse::<KeyDir>().map_err(|err| format!("{}", err))?;
            return Ok(GameCmd::Key(chr, dir));
        } else if cmd == "ctrl" {
            let dir = args.next().ok_or("no arg")?.parse::<KeyDir>().map_err(|err| format!("{}", err))?;
            return Ok(GameCmd::Ctrl(dir));
        } else if cmd == "alt" {
            let dir = args.next().ok_or("no arg")?.parse::<KeyDir>().map_err(|err| format!("{}", err))?;
            return Ok(GameCmd::Alt(dir));
        } else if cmd == "shift" {
            let dir = args.next().ok_or("no arg")?.parse::<KeyDir>().map_err(|err| format!("{}", err))?;
            return Ok(GameCmd::Shift(dir));
        } else if cmd == "exit" {
            return Ok(GameCmd::Exit);
        }

        return Err("OUTPUT: error '{}' not expected!".to_string());
    }
}

impl GameCmd {
    pub fn name(&self) -> &str {
        if matches!(self, GameCmd::PlayerId) {
            return "player_id";
        } else if matches!(self, GameCmd::Pos(_)) {
            return "pos";
        } else if matches!(self, GameCmd::SetPos(_, _, _)) {
            return "set_pos";
        } else if matches!(self, GameCmd::Hp(_)) {
            return "hp";
        } else if matches!(self, GameCmd::SetHp(_, _)) {
            return "set_hp";
        } else if matches!(self, GameCmd::Facing(_)) {
            return "facing";
        } else if matches!(self, GameCmd::SetFacing(_, _)) {
            return "facing";
        } else if matches!(self, GameCmd::MapSize) {
            return "map_size";
        } else if matches!(self, GameCmd::TileWalls(_, _)) {
            return "tile_walls";
        } else if matches!(self, GameCmd::SetTileWalls(_, _, _, _, _)) {
            return "set_tile_walls";
        } else if matches!(self, GameCmd::Surface(_, _)) {
            return "surface";
        } else if matches!(self, GameCmd::SetSurface(_, _, _)) {
            return "set_surface";
        } else if matches!(self, GameCmd::EntityName(_)) {
            return "entity_name";
        } else if matches!(self, GameCmd::EntityType(_)) {
            return "entity_type";
        } else if matches!(self, GameCmd::Spawn(_, _, _)) {
            return "spawn";
        } else if matches!(self, GameCmd::Remove(_)) {
            return "remove";
        } else if matches!(self, GameCmd::Kill(_)) {
            return "kill";
        } else if matches!(self, GameCmd::Give(_)) {
            return "give";
        } else if matches!(self, GameCmd::ListEntities) {
            return "ids";
        } else if matches!(self, GameCmd::ListEntitiesPos(_, _)) {
            return "ids_pos";
        } else if matches!(self, GameCmd::Key(_, _)) {
            return "key";
        } else if matches!(self, GameCmd::Ctrl(_)) {
            return "ctrl";
        } else if matches!(self, GameCmd::Alt(_)) {
            return "alt";
        } else if matches!(self, GameCmd::Shift(_)) {
            return "shift";
        } else if matches!(self, GameCmd::Exit) {
            return "exit";
        } else {
            return "UNKNOWN";
        }
    }
}

pub fn execute_game_command(command: &GameCmd, game: &mut Game) -> String {
    let name = command.name();

    match command {
        GameCmd::PlayerId => {
            let player_id = game.data.find_by_name(EntityName::Player).unwrap();
            return format!("{} {}", name, player_id);
        }

        GameCmd::Pos(id) => {
            if let Some(pos) = game.data.entities.pos.get(id) {
                return format!("{} {} {}", name, pos.x, pos.y);
            } else {
                return format!("{}", name);
            }
        }

        GameCmd::SetPos(id, x, y) => {
            // TODO this could return an error if id not found instead of failing silently
            if let Some(pos) = game.data.entities.pos.get(id) {
                game.data.entities.pos[id] = Pos::new(*x, *y);
            }
            return format!("{}", name);
        }

        GameCmd::Hp(id) => {
            if let Some(fighter) = game.data.entities.fighter.get(id) {
                let hp = fighter.hp;
                return format!("{} {}", name, hp);
            }
            return format!("{}", name);
        }

        GameCmd::SetHp(id, hp) => {
            // ugly...
            if let Some(fighter) = game.data.entities.fighter.get(id) {
                game.data.entities.fighter[id].hp = *hp;
            }
            return format!("{}", name);
        }

        GameCmd::Facing(id) => {
            if let Some(dir) = game.data.entities.direction.get(id) {
                return format!("{} {}", name, dir);
            }
            return format!("{}", name);
        }

        GameCmd::SetFacing(id, dir) => {
            // also ugly- just check for id first
            if let Some(_dir) = game.data.entities.direction.get(id) {
                game.data.entities.direction[id] = *dir;
            }
            return format!("{}", name);
        }

        GameCmd::MapSize => {
            return format!("{} {} {}", name, game.data.map.width(), game.data.map.height());
        }

        GameCmd::TileWalls(x, y) => {
            let tile = game.data.map[(*x, *y)];
            return format!("{} {} {} {} {} {}", name, x, y, tile.tile_type, tile.left_wall, tile.bottom_wall);
        }

        GameCmd::SetTileWalls(x, y, typ, left_wall, bottom_wall) => {
            game.data.map[(*x, *y)].tile_type = *typ;
            game.data.map[(*x, *y)].block_move = typ.is_wall();
            game.data.map[(*x, *y)].block_sight = typ.is_wall();
            game.data.map[(*x, *y)].chr = MAP_WALL;
            game.data.map[(*x, *y)].left_wall = *left_wall;
            game.data.map[(*x, *y)].bottom_wall = *bottom_wall;
            return format!("{}", name);
        }

        GameCmd::Surface(x, y) => {
            return format!("{} {} {} {}", name, x, y, game.data.map[(*x, *y)].surface);
        }

        GameCmd::SetSurface(x, y, surface) => {
            game.data.map[(*x, *y)].surface = *surface;
            return format!("{}", name);
        }

        GameCmd::Exit => {
            game.settings.running = false;
            return format!("{}", name);
        }

        GameCmd::Spawn(entity_name, x, y) => {
            let pos = Pos::new(*x, *y);
            let id = make_entity(&mut game.data.entities,
                                 &game.config,
                                 *entity_name,
                                 pos,
                                 &mut game.msg_log);
            return format!("{} {}", name, id);
        }

        GameCmd::Remove(id) => {
            game.data.entities.remove_entity(*id);
            game.msg_log.log(Msg::RemovedEntity(*id));
            return format!("{}", name);
        }

        GameCmd::Kill(id) => {
            let player_id = game.data.find_by_name(EntityName::Player).unwrap();
            game.msg_log.log(Msg::Killed(player_id, *id, 1000));
            return format!("{}", name);
        }

        GameCmd::Give(item) => {
            let player_id = game.data.find_by_name(EntityName::Player).unwrap();
            let pos = game.data.entities.pos[&player_id];
            let item_id = make_item(&mut game.data.entities, &game.config, *item, pos, &mut game.msg_log);
            game.data.entities.pick_up_item(player_id, item_id);
            return format!("{}", name);
        }

        GameCmd::EntityName(id) => {
            return format!("{} {}", name, game.data.entities.name[id]);
        }

        GameCmd::EntityType(id) => {
            return format!("{} {}", name, game.data.entities.typ[id]);
        }

        GameCmd::ListEntities => {
            let ids = game.data.entities.ids.iter()
                          .map(|id| id.to_string())
                          .collect::<Vec<String>>()
                          .join(" ");
                 
            return format!("{} {}", name, ids);
        }

        GameCmd::ListEntitiesPos(x, y) => {
            let ids = game.data.get_entities_at_pos(Pos::new(*x, *y))
                               .iter()
                               .map(|id| id.to_string())
                               .collect::<Vec<String>>()
                               .join(" ");
            return format!("{} {}", name, ids);
        }

        GameCmd::Key(chr, dir) => {
            // TODO this isn't really correct...
            let time = Instant::now();
            let dt = Instant::now().duration_since(time).as_secs_f32();

            let input_event = InputEvent::Char(*chr, *dir);
            let input_action = game.input.handle_event(&mut game.settings, input_event, time, &game.config);
            game.step_game(input_action, dt);
            return format!("{}", name);
        }

        GameCmd::Ctrl(dir) => {
            let time = Instant::now();
            let dt = Instant::now().duration_since(time).as_secs_f32();

            let input_event = InputEvent::Ctrl(*dir);
            let input_action = game.input.handle_event(&mut game.settings, input_event, time, &game.config);
            game.step_game(input_action, dt);
            return format!("{}", name);
        }

        GameCmd::Alt(dir) => {
            let time = Instant::now();
            let dt = Instant::now().duration_since(time).as_secs_f32();

            let input_event = InputEvent::Alt(*dir);
            let input_action = game.input.handle_event(&mut game.settings, input_event, time, &game.config);
            game.step_game(input_action, dt);
            return format!("{}", name);
        }

        GameCmd::Shift(dir) => {
            let time = Instant::now();
            let dt = Instant::now().duration_since(time).as_secs_f32();

            let input_event = InputEvent::Shift(*dir);
            let input_action = game.input.handle_event(&mut game.settings, input_event, time, &game.config);
            game.step_game(input_action, dt);
            return format!("{}", name);
        }

        // let action = game.input.handle_event(&mut game.settings, event, frame_time, &game.config);
        // game.step_game(input_action, dt);
    }
}

