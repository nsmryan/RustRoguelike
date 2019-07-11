#[allow(unused_imports)]use tcod::input::{self, Event, Mouse};
#[allow(unused_imports)]use tcod::input::Key;
#[allow(unused_imports)]use tcod::input::KeyCode::*;
#[allow(unused_imports)]use tcod::console::*;
#[allow(unused_imports)]use tcod::colors::*;
use tcod::line::*;

use crate::types::*;
use crate::constants::*;
use crate::map::*;
use crate::display::*;
use crate::ai::*;


pub fn handle_input(game: &mut Game,
                    key: Key,
                    map: &mut Map,
                    objects: &mut Vec<Object>,
                    inventory: &mut Vec<Object>,
                    config: &Config,
                    messages: &mut Messages) -> PlayerAction {
    use PlayerAction::*;

    let player_action: PlayerAction;

    let player_alive = objects[PLAYER].alive;

    if game.mouse.lbutton_pressed {
        let mut found_stone = false;
        for index in 0..inventory.len() {
            let (mx, my) = (game.mouse.x, game.mouse.y);
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
    } else {
        match (key, player_alive) {
            (Key { code: Up,      .. }, true)  |
            (Key { code: Number8, .. }, true)  |
            (Key { code: NumPad8, .. }, true) => {
                player_move_or_attack(0, -1, map, objects, messages);
                player_action = TookTurn;
            }

            (Key { code: Down,    .. }, true) |
            (Key { code: Number2, .. }, true) |
            (Key { code: NumPad2, .. }, true) => {
                player_move_or_attack(0, 1, map, objects, messages);
                player_action = TookTurn;
            }
            (Key { code: Left,    .. }, true) |
            (Key { code: Number4, .. }, true) |
            (Key { code: NumPad4, .. }, true) => {
                player_move_or_attack(-1, 0, map, objects, messages);
                player_action = TookTurn;
            }

            (Key { code: Right,   .. }, true) |
            (Key { code: Number6, .. }, true) |
            (Key { code: NumPad6, .. }, true) => {
                player_move_or_attack(1, 0, map, objects, messages);
                player_action = TookTurn;
            }

            (Key { code: Number9, .. }, true)  |
            (Key { code: NumPad9, .. }, true) => {
                player_move_or_attack(1, -1, map, objects, messages);
                player_action = TookTurn;
            }

            (Key { code: Number3, .. }, true) |
            (Key { code: NumPad3, .. }, true) => {
                player_move_or_attack(1, 1, map, objects, messages);
                player_action = TookTurn;
            }

            (Key { code: Number1, .. }, true) |
            (Key { code: NumPad1, .. }, true) => {
                player_move_or_attack(-1, 1, map, objects, messages);
                player_action = TookTurn;
            }

            (Key { code: Number7, .. }, true) |
            (Key { code: NumPad7, .. }, true) => {
                player_move_or_attack(-1, -1, map, objects, messages);
                player_action = TookTurn;
            }

            (Key { code: Number5, .. }, true) |
            (Key { code: NumPad5, .. }, true) => {
                objects[PLAYER].momentum = Some((0, 0));
                player_action = TookTurn;
            }

            (Key { code: Enter, alt: true, .. }, _) => {
                let fullscreen = game.root.is_fullscreen();
                game.root.set_default_foreground(WHITE);
                game.root.set_fullscreen(!fullscreen);
                player_action = DidntTakeTurn;
            },

            (Key {printable: 'g', .. }, true) => {
                let item_id = objects.iter().position(|object| {
                    object.pos() == objects[PLAYER].pos() && object.item.is_some()
                });
                if let Some(item_id) = item_id {
                    pick_item_up(item_id, objects, inventory, messages);
                }
                player_action = DidntTakeTurn;
            }

            (Key {printable: 'i', .. }, true) => {
                let inventory_index =
                    inventory_menu(inventory,
                                   "Press the key next to an item to use it, or any other to cancel.\n",
                                   &mut game.root);
                if let Some(inventory_index) = inventory_index {
                    use_item(inventory_index, inventory, objects, messages);
                }
                player_action = DidntTakeTurn;
            }

            (Key { code: Escape, .. }, _) => {
                player_action = Exit;
            }

            (Key {printable: 'v', .. }, true) => {
                for x in 0..MAP_WIDTH {
                    for y in 0..MAP_HEIGHT {
                        map.0[x as usize][y as usize].explored = true;
                    }
                }
                player_action = DidntTakeTurn;
            }

            (Key {code: NumPadAdd, .. }, true) => {
                game.display_overlays = !game.display_overlays;

                player_action = DidntTakeTurn;
            }

            (Key {printable: 't', .. }, true) => {
                let fighter = objects[PLAYER].fighter.unwrap();
                let god_mode_hp = 1000000;
                objects[PLAYER].fighter =
                    Some(Fighter { hp: god_mode_hp, max_hp: god_mode_hp, ..fighter });

                player_action = DidntTakeTurn;
            }

            (_, _) => {
                player_action = DidntTakeTurn;
            }
        }
    }

    return player_action;
}

pub fn throw_stone(pos: (i32, i32),
                   mut stone: Object,
                   game: &mut Game,
                   map: &mut Map,
                   objects: &mut Vec<Object>) {
    let (mx, my) = pos;
    let obj_id = objects.len();

    let start_x = objects[PLAYER].x;
    let start_y = objects[PLAYER].y;
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
    let animation =
        Animation::Thrown(obj_id,
                          Line::new((start_x, start_y),
                                    (target_x, target_y)));
    game.animations.push(animation);

    // add sound to map
    for pos in map.pos_in_radius((target_x, target_y), STONE_SOUND_RADIUS) {
        map[pos].sound = Some((target_x, target_y));
    }
}

fn use_item(inventory_id: usize,
            inventory: &mut Vec<Object>,
            objects: &mut [Object],
            messages: &mut Messages) {
    use Item::*;

    if let Some(item) = inventory[inventory_id].item {
        let on_use = match item {
            Stone => unimplemented!(),
            Goal => gather_goal,
        };
        match on_use(inventory_id, objects, messages) {
            UseResult::UsedUp => {
                inventory.remove(inventory_id);
            }
            UseResult::Cancelled => {
                messages.message("Cancelled", WHITE);
            }

            UseResult::Keep => {
            }
        }
    } else {
        messages.message(format!("The {} cannot be used.", inventory[inventory_id].name), WHITE);
    }
}

fn inventory_menu(inventory: &[Object], header: &str, root: &mut Root) -> Option<usize> {
    let options = if inventory.len() == 0 {
        vec!("Inventory is empty.".into())
    } else {
        inventory.iter().map(|item| { item.name.clone() }).collect()
    };

    let inventory_index = menu(header, &options, INVENTORY_WIDTH, root);

    if inventory.len() > 0 {
        inventory_index
    } else {
        None
    }
}

fn pick_item_up(object_id: usize,
                objects: &mut Vec<Object>,
                inventory: &mut Vec<Object>,
                messages: &mut Messages) {
    if inventory.len() >= 26 {
        messages.message(format!("Your inventory is full, cannot pick up {}", objects[object_id].name), RED);
    } else {
        let item = objects.swap_remove(object_id);
        messages.message(format!("You picked up a {}!", item.name), GREEN);
        inventory.push(item);
    }
}

fn player_move_or_attack(dx: i32, dy: i32, map: &Map, objects: &mut [Object], messages: &mut Messages) {
    let x = objects[PLAYER].x + dx;
    let y = objects[PLAYER].y + dy;
    let target_id = objects.iter().position(|object| {
        object.fighter.is_some() && object.pos() == (x, y)
    });

    match target_id {
        Some(target_id) => {
            let (player, target) = mut_two(PLAYER, target_id, objects);
             player.attack(target);
        }

        None => {
            move_player_by(objects, map, dx, dy);
        }
    }
}

fn gather_goal(_inventory_id: usize, _objects: &mut [Object], messages: &mut Messages) -> UseResult {
    messages.message("You've got the goal object! Nice work.", LIGHT_VIOLET);
    UseResult::Keep
}

