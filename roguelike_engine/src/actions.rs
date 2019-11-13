use rand::prelude::*;

use tcod::line::*;

use roguelike_core::map::*;
use roguelike_core::types::*;
use roguelike_core::constants::*;
use roguelike_core::movement::*;
use roguelike_core::config::*;
use roguelike_core::generation::*;

use crate::input::*;


pub fn handle_input(input_action: InputAction,
                    mouse_state: &MouseState,
                    game_data: &mut GameData, 
                    god_mode: &mut bool,
                    display_overlays: &mut bool,
                    config: &Config) -> PlayerAction {
    use PlayerAction::*;

    let player_handle = game_data.find_player().unwrap();

    let player_action: PlayerAction;

    let player_alive = game_data.objects.get(player_handle).unwrap().alive;

    if mouse_state.left_pressed {
        // TODO add back in with new inventory
        /*
        let mut found_stone = false;
        for index in 0..inventory.len() {
            let (mx, my) = (game.mouse_state.pos.0, game.mouse_state.pos.1);
            if inventory[index].item == Some(Item::Stone) {
                let item = inventory.swap_remove(index);
                throw_stone((mx as i32, my as i32), item, game, map, objects);
                found_stone = true;
                break;
            }
        }

        if found_stone {
            player_action = TookTurn;
        } else {
            player_action = DidntTakeTurn;
        }
        */
        player_action = DidntTakeTurn;
    } else {
        match (input_action, player_alive) {
            (InputAction::Move(move_action), true) => {
                player_action = player_move_or_attack(move_action,
                                                      game_data)
            }

            (InputAction::FullScreen, _) => {
                // TODO don't know how to do this in ggez...
                player_action = DidntTakeTurn;
            },

            (InputAction::Pickup, true) => {
                let player = &game_data.objects[player_handle];
                let item_id = game_data.objects.keys().filter(|key| {
                    return (game_data.objects[*key].pos() == player.pos()) && game_data.objects[*key].item.is_some();
                }).next();
                if let Some(key) = item_id {
                    pick_item_up(player_handle, key, &mut game_data.objects);
                }
                player_action = DidntTakeTurn;
            }

            (InputAction::Inventory, true) => {
                player_action = DidntTakeTurn;
            }

            (InputAction::Exit, _) => {
                player_action = Exit;
            }

            (InputAction::ExploreAll, _) => {
                for x in 0..game_data.map.width() {
                    for y in 0..game_data.map.height() {
                        game_data.map.tiles[x as usize][y as usize].explored = true;
                    }
                }
                player_action = DidntTakeTurn;
            }

            (InputAction::RegenerateMap, _) => {
                let mut rng: SmallRng = SeedableRng::seed_from_u64(2);
                let (data, _position) = make_map(&mut game_data.objects, config, &mut rng);
                game_data.map = data.map;
                player_action = DidntTakeTurn;
            }

            (InputAction::ToggleOverlays, _) => {
                *display_overlays = !(*display_overlays);

                player_action = DidntTakeTurn;
            }

            (InputAction::GodMode, true) => {
                let god_mode_hp = 1000000;
                let handle = game_data.find_player().unwrap();
                if let Some(ref mut fighter) = game_data.objects[handle].fighter {
                    fighter.hp = god_mode_hp;
                    fighter.max_hp = god_mode_hp;
                }

                // set god mode flag
                *god_mode = true;

                // set all tiles to be transparent and walkable. walkable is not current used
                // anywhere
                for x in 0..game_data.map.tiles.len() {
                    for y in 0..game_data.map.tiles[0].len() {
                        game_data.map.set_cell(x as i32, y as i32, true, true);
                    }
                }
                game_data.map.update_map();

                player_action = TookTurn;
            }

            (_, _) => {
                player_action = DidntTakeTurn;
            }
        }
    }

    return player_action;
}

fn use_item(object_id: ObjectId,
            item_id: ObjectId,
            objects: &mut [Object]) {
    //if let Some(item) = inventory[inventory_id].item {
    //    let _on_use = match item {
    //        Stone => unimplemented!(),
    //        Goal => unimplemented!(), // gather_goal,
    //    };
    //    /*
    //    match on_use(inventory_id, objects, config) {
    //        UseResult::UsedUp => {
    //            inventory.remove(inventory_id);
    //        }
    //        UseResult::Cancelled => {
    //            // messages.message("Cancelled", WHITE);
    //        }

    //        UseResult::Keep => {
    //        }
    //    }
    //    */
    //} else {
    //    // messages.message(format!("The {} cannot be used.", inventory[inventory_id].name), WHITE);
    //}
}

//fn gather_goal(_inventory_id: usize, _objects: &mut [Object], _config: &Config) -> UseResult {
    // messages.message("You've got the goal object! Nice work.", config.color_orange.color());
 //   UseResult::Keep
//}

fn pick_item_up(object_id: ObjectId,
                item_id: ObjectId,
                objects: &mut ObjMap) {
    objects[object_id].inventory.push(item_id);
    objects[object_id].set_pos(-1, -1);
}

pub fn throw_stone(pos: (i32, i32),
                   mut stone: Object,
                   map: &mut Map,
                   objects: &mut Vec<Object>) {
    let (mx, my) = pos;

    let start_x = objects.get(PLAYER).unwrap().x;
    let start_y = objects.get(PLAYER).unwrap().y;
    let end_x = mx / FONT_WIDTH;
    let end_y = my / FONT_HEIGHT;
    let throw_line = Line::new((start_x, start_y), (end_x, end_y));

    // get target position in direction of player click
    let (target_x, target_y) =
        throw_line.into_iter().take(PLAYER_THROW_DIST).last().unwrap();

    stone.x = start_x;
    stone.y = start_y;
    objects.push(stone);

    // add animation to animation list
    // TODO add back in with animations
    //let animation =
    //    Animation::Thrown(obj_id,
    //                      Line::new((start_x, start_y),
    //                      (target_x, target_y)));
    //game.animations.push(animation);

    // add sound to map
    for pos in map.pos_in_radius((target_x, target_y), STONE_SOUND_RADIUS) {
        map[pos].sound = Some((target_x, target_y));
    }
}
