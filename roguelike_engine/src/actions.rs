use rand::prelude::*;

use tcod::line::*;

use roguelike_core::types::*;
use roguelike_core::constants::*;
use roguelike_core::movement::*;
use roguelike_core::config::*;

use crate::game::*;
use crate::input::*;
use crate::display::*;
use crate::generation::*;


pub fn handle_input(input_action: InputAction,
                    _mouse_state: &MouseState,
                    game_data: &mut GameData, 
                    settings: &mut GameSettings,
                    display_state: &mut DisplayState,
                    config: &Config) -> Action {
    use Action::*;
    use Movement::*;

    let player_handle = game_data.find_player().unwrap();
    let player_x = game_data.objects[player_handle].x;
    let player_y = game_data.objects[player_handle].y;
    let player_pos = Pos::new(player_x, player_y);

    let player_turn: Action;

    let player_alive = game_data.objects[player_handle].alive;

    match (input_action, player_alive) {
        // NOTE needs to move to control handling
        // If the player pressed a movement button, attempt to move
        (InputAction::Move(move_action), true) => {
            player_turn = player_move_or_attack(move_action, game_data);

            // if the player moved, requiring an animation to play, then select the animation
            match player_turn {
                Action::Move(WallKick(pos, _dx, _dy)) => {
                    let sprite_key = display_state.lookup_sprite("player_wall_kick".to_string()).unwrap();
                    let sprite_index = 0.0;
                    game_data.objects[player_handle].animation = 
                        Some(Animation::WallKick(sprite_key, sprite_index, player_pos, pos));
                }

                _ => {
                }
            }
        }

        // NOTE needs to move to control handling
        (InputAction::Pickup, true) => {
            let player = &game_data.objects[player_handle];
            let item_id = game_data.objects.keys().filter(|key| {
                return (game_data.objects[*key].pos() == player.pos()) && game_data.objects[*key].item.is_some();
            }).next();
            if let Some(key) = item_id {
                pick_item_up(player_handle, key, &mut game_data.objects);
            }
            player_turn = Action::none();
        }

        (InputAction::Inventory, true) => {
            player_turn = Action::none();
        }

        (InputAction::Exit, _) => {
            settings.exiting = true;
            player_turn = NoAction;
        }

        (InputAction::ExploreAll, _) => {
            for x in 0..game_data.map.width() {
                for y in 0..game_data.map.height() {
                    game_data.map.tiles[x as usize][y as usize].explored = true;
                }
            }
            player_turn = Action::none();
        }

        (InputAction::RegenerateMap, _) => {
            let mut rng: SmallRng = SeedableRng::seed_from_u64(2);
            let (data, _position) =
                make_map(&settings.map_type, &mut game_data.objects, config, display_state, &mut rng);
            game_data.map = data.map;
            player_turn = Action::none();
        }

        (InputAction::ToggleOverlays, _) => {
            display_state.display_overlays = !(display_state.display_overlays);

            player_turn = Action::none();
        }

        (InputAction::GodMode, true) => {
            let god_mode_hp = 1000000;
            let handle = game_data.find_player().unwrap();
            if let Some(ref mut fighter) = game_data.objects[handle].fighter {
                fighter.hp = god_mode_hp;
                fighter.max_hp = god_mode_hp;
            }

            // set god mode flag
            settings.god_mode = true;

            // set all tiles to be transparent and walkable. walkable is not current used
            // anywhere
            for x in 0..game_data.map.tiles.len() {
                for y in 0..game_data.map.tiles[0].len() {
                    game_data.map.set_cell(x as i32, y as i32, true, true);
                }
            }
            game_data.map.update_map();

            player_turn = Action::none();
        }

        // NOTE needs to move to control handling
        (InputAction::MapClick(_map_loc, map_cell), _) => {
            let mut stone = None;
            let mut stone_index = None;
            for (index, obj_id) in game_data.objects[player_handle].inventory.iter().enumerate() {
                if let Some(Item::Stone) = game_data.objects[*obj_id].item {
                    stone = Some(obj_id);
                    stone_index = Some(index);
                    break;
                }
            }

            if let (Some(stone_handle), Some(index)) = (stone, stone_index) {
                throw_stone(player_pos, map_cell, *stone_handle, game_data);

                game_data.objects[player_handle].inventory.remove(index);

                player_turn = Action::ThrowStone;
            } else {
                player_turn = Action::none();
            }
        }

        (_, _) => {
            player_turn = Action::none();
        }
    }

    return player_turn;
}

fn pick_item_up(object_id: ObjectId,
                item_id: ObjectId,
                objects: &mut ObjMap) {
    objects[object_id].inventory.push(item_id);
    objects[item_id].set_xy(-1, -1);
}

pub fn throw_stone(start_pos: Pos,
                   end_pos: Pos,
                   stone_handle: ObjectId,
                   data: &mut GameData) {
    let throw_line = Line::new(start_pos.to_tuple(), end_pos.to_tuple());

    // TODO draw line to end, and move until radius or hit wall

    // get target position in direction of player click
    let (target_x, target_y) =
        throw_line.into_iter().take(PLAYER_THROW_DIST).last().unwrap();

    data.objects[stone_handle].set_xy(target_x, target_y);

    // Create the stone throw animation
    data.objects[stone_handle].animation =
        Some(Animation::StoneThrow(start_pos, Pos::new(target_x, target_y)));

    // alert monsters within sound range
    for obj in data.objects.values_mut() {
        if distance(obj.pos(), end_pos) <  SOUND_RADIUS as i32 {
            if obj.behavior == Some(Behavior::Idle) {
                obj.behavior = Some(Behavior::Investigating(Pos::from(end_pos)));
            }
        }
    }
}

