use std::str::FromStr;

use roguelike_utils::comp::*;

use roguelike_map::map::*;

use roguelike_core::types::*;
use roguelike_core::messaging::*;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd)]
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
    StoneThrower(bool),
    WhetStone(bool),
    SoftShoes(bool),
    LightTouch(bool),
    SureFooted(bool),
    QuickReflexes(bool),
    Visible(EntityId, i32, i32),
    Blink,
    Exit,
}

impl FromStr for GameCmd {
    type Err = String;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let s: &mut str = &mut string.to_string();
        s.make_ascii_lowercase();

        let mut args = s.split(" ");
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
            let chr_name = args.next().ok_or("no arg")?;
            let chr;
            if chr_name == "space" {
                chr = ' ';
            } else if chr_name == "backspace" {
                chr = '\x08';
            } else if chr_name == "return" {
                chr = '\x0d';
            } else if chr_name == "esc" {
                chr = '\x1b';
            } else {
                chr = chr_name.parse::<char>().map_err(|err| format!("{}", err))?;
            }
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
        } else if cmd == "stone_thrower" {
            let onoff = args.next().ok_or("no arg")?.parse::<bool>().map_err(|err| format!("{}", err))?;
            return Ok(GameCmd::StoneThrower(onoff));
        } else if cmd == "whet_stone" {
            let onoff = args.next().ok_or("no arg")?.parse::<bool>().map_err(|err| format!("{}", err))?;
            return Ok(GameCmd::WhetStone(onoff));
        } else if cmd == "soft_shoes" {
            let onoff = args.next().ok_or("no arg")?.parse::<bool>().map_err(|err| format!("{}", err))?;
            return Ok(GameCmd::SoftShoes(onoff));
        } else if cmd == "light_touch" {
            let onoff = args.next().ok_or("no arg")?.parse::<bool>().map_err(|err| format!("{}", err))?;
            return Ok(GameCmd::LightTouch(onoff));
        } else if cmd == "sure_footed" {
            let onoff = args.next().ok_or("no arg")?.parse::<bool>().map_err(|err| format!("{}", err))?;
            return Ok(GameCmd::SureFooted(onoff));
        } else if cmd == "quick_reflexes" {
            let onoff = args.next().ok_or("no arg")?.parse::<bool>().map_err(|err| format!("{}", err))?;
            return Ok(GameCmd::QuickReflexes(onoff));
        } else if cmd == "visible" {
            let id = args.next().ok_or("no arg")?.parse::<u64>().map_err(|err| format!("{}", err))?;
            let x  = args.next().ok_or("no arg")?.parse::<i32>().map_err(|err| format!("{}", err))?;
            let y  = args.next().ok_or("no arg")?.parse::<i32>().map_err(|err| format!("{}", err))?;
            return Ok(GameCmd::Visible(id, x, y));
        } else if cmd == "blink" {
            return Ok(GameCmd::Blink);
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
        } else if matches!(self, GameCmd::StoneThrower(_)) {
            return "stone_thrower";
        } else if matches!(self, GameCmd::WhetStone(_)) {
            return "whet_stone";
        } else if matches!(self, GameCmd::SoftShoes(_)) {
            return "soft_shoes";
        } else if matches!(self, GameCmd::LightTouch(_)) {
            return "light_touch";
        } else if matches!(self, GameCmd::SureFooted(_)) {
            return "sure_footed";
        } else if matches!(self, GameCmd::QuickReflexes(_)) {
            return "quick_reflexes";
        } else if matches!(self, GameCmd::Visible(_, _, _)) {
            return "visible";
        } else if matches!(self, GameCmd::Blink) {
            return "blink";
        } else if matches!(self, GameCmd::Exit) {
            return "exit";
        } else {
            return "UNKNOWN";
        }
    }
}

pub fn execute_game_command(command: &GameCmd, game: &mut Game) -> String {
    let name = command.name();

    // TODO this isn't really correct- perhaps we could pass get_ticks() to execute_game_command
    let ticks = 0;

    let player_id = game.level.find_by_name(EntityName::Player).unwrap();

    match command {
        GameCmd::PlayerId => {
            return format!("{} {}", name, player_id);
        }

        GameCmd::Pos(id) => {
            if let Some(pos) = game.level.entities.pos.get(id) {
                return format!("{} {} {} {}", name, id, pos.x, pos.y);
            } else {
                return format!("{}", name);
            }
        }

        GameCmd::SetPos(id, x, y) => {
            // TODO this could return an error if id not found instead of failing silently
            if game.level.entities.pos.contains_key(id) {
                game.level.entities.pos[id] = Pos::new(*x, *y);
            }
            return format!("{}", name);
        }

        GameCmd::Hp(id) => {
            if let Some(hp) = game.level.entities.hp.get(id) {
                let hp = hp.hp;
                return format!("{} {}", name, hp);
            }
            return format!("{}", name);
        }

        GameCmd::SetHp(id, hp) => {
            if game.level.entities.hp.contains_key(id) {
                game.level.entities.hp[id].hp = *hp;
            }
            return format!("{}", name);
        }

        GameCmd::Facing(id) => {
            if let Some(dir) = game.level.entities.direction.get(id) {
                return format!("{} {}", name, dir);
            }
            return format!("{}", name);
        }

        GameCmd::SetFacing(id, dir) => {
            if game.level.entities.direction.contains_key(id) {
                game.level.entities.direction[id] = *dir;
            }
            return format!("{}", name);
        }

        GameCmd::MapSize => {
            return format!("{} {} {}", name, game.level.map.width(), game.level.map.height());
        }

        GameCmd::TileWalls(x, y) => {
            let tile = game.level.map[(*x, *y)];
            return format!("{} {} {} {} {} {}", name, x, y, tile.tile_type, tile.left_wall, tile.bottom_wall);
        }

        GameCmd::SetTileWalls(x, y, typ, left_wall, bottom_wall) => {
            game.level.map[(*x, *y)].tile_type = *typ;
            game.level.map[(*x, *y)].block_move = typ.is_wall();
            game.level.map[(*x, *y)].block_sight = typ.is_wall();
            game.level.map[(*x, *y)].left_wall = *left_wall;
            game.level.map[(*x, *y)].bottom_wall = *bottom_wall;
            return format!("{}", name);
        }

        GameCmd::Surface(x, y) => {
            return format!("{} {} {} {}", name, x, y, game.level.map[(*x, *y)].surface);
        }

        GameCmd::SetSurface(x, y, surface) => {
            game.level.map[(*x, *y)].surface = *surface;
            return format!("{}", name);
        }

        GameCmd::Blink => {
            game.msg_log.log(Msg::Blink(player_id));
            return format!("{}", name);
        }

        GameCmd::Exit => {
            game.settings.running = false;
            return format!("{}", name);
        }

        GameCmd::Spawn(entity_name, x, y) => {
            let pos = Pos::new(*x, *y);
            let id = make_entity(&mut game.level.entities,
                                 &game.config,
                                 *entity_name,
                                 pos,
                                 &mut game.msg_log);
            return format!("{} {}", name, id);
        }

        GameCmd::Remove(id) => {
            game.level.entities.remove_entity(*id);
            game.msg_log.log(Msg::RemovedEntity(*id));
            return format!("{}", name);
        }

        GameCmd::Kill(id) => {
            game.msg_log.log(Msg::Killed(player_id, *id, 1000));
            return format!("{}", name);
        }

        GameCmd::Give(item) => {
            let pos = game.level.entities.pos[&player_id];
            let item_id = make_item(&mut game.level.entities, &game.config, *item, pos, &mut game.msg_log);
            game.level.entities.pick_up_item(player_id, item_id);
            return format!("{}", name);
        }

        GameCmd::EntityName(id) => {
            return format!("{} {}", name, game.level.entities.name[id]);
        }

        GameCmd::EntityType(id) => {
            return format!("{} {}", name, game.level.entities.typ[id]);
        }

        GameCmd::ListEntities => {
            let ids = game.level.entities.ids.iter()
                          .map(|id| id.to_string())
                          .collect::<Vec<String>>()
                          .join(" ");
                 
            return format!("{} {}", name, ids);
        }

        GameCmd::ListEntitiesPos(x, y) => {
            let ids = game.level.get_entities_at_pos(Pos::new(*x, *y))
                               .iter()
                               .map(|id| id.to_string())
                               .collect::<Vec<String>>()
                               .join(" ");
            return format!("{} {}", name, ids);
        }

        GameCmd::Key(chr, dir) => {
            let input_event = InputEvent::Char(*chr, *dir);
            let input_action = game.input.handle_event(&mut game.settings, input_event, ticks, &game.config);
            game.step_game(input_action);
            return format!("{}", name);
        }

        GameCmd::Ctrl(dir) => {
            let input_event = InputEvent::Ctrl(*dir);
            let input_action = game.input.handle_event(&mut game.settings, input_event, ticks, &game.config);
            game.step_game(input_action);
            return format!("{}", name);
        }

        GameCmd::Alt(dir) => {
            let input_event = InputEvent::Alt(*dir);
            let input_action = game.input.handle_event(&mut game.settings, input_event, ticks, &game.config);
            game.step_game(input_action);
            return format!("{}", name);
        }

        GameCmd::Shift(dir) => {
            let input_event = InputEvent::Shift(*dir);
            let input_action = game.input.handle_event(&mut game.settings, input_event, ticks, &game.config);
            game.step_game(input_action);
            return format!("{}", name);
        }

        GameCmd::StoneThrower(onoff) => {
            game.level.entities.passive[&player_id].stone_thrower = *onoff;
            return format!("{}", name);
        }

        GameCmd::WhetStone(onoff) => {
            game.level.entities.passive[&player_id].whet_stone = *onoff;
            return format!("{}", name);
        }

        GameCmd::SoftShoes(onoff) => {
            game.level.entities.passive[&player_id].soft_shoes = *onoff;
            return format!("{}", name);
        }

        GameCmd::LightTouch(onoff) => {
            game.level.entities.passive[&player_id].light_touch = *onoff;
            return format!("{}", name);
        }

        GameCmd::SureFooted(onoff) => {
            game.level.entities.passive[&player_id].sure_footed = *onoff;
            return format!("{}", name);
        }

        GameCmd::QuickReflexes(onoff) => {
            game.level.entities.passive[&player_id].quick_reflexes = *onoff;
            return format!("{}", name);
        }

        GameCmd::Visible(entity_id, x, y) => {
            let pos = Pos::new(*x, *y);
            let visible = game.level.pos_in_fov(*entity_id, pos);
            return format!("{} {}", name, visible);
        }

        // let action = game.input.handle_event(&mut game.settings, event, frame_time, &game.config);
        // game.step_game(input_action);
    }
}
