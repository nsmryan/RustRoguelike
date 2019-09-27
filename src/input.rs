use rand::prelude::*;

#[allow(unused_imports)]use tcod::input::{self, Event, Mouse};
#[allow(unused_imports)]use tcod::input::Key;
#[allow(unused_imports)]use tcod::input::KeyCode::*;
#[allow(unused_imports)]use tcod::console::*;
use tcod::line::*;

use ggez::graphics::WHITE;
use ggez::event::{KeyCode, KeyMods};

use crate::engine::types::*;
use crate::engine::map::*;
use crate::engine::ai::*;
use crate::constants::*;
use crate::game::*;


pub fn handle_input(game: &mut Game,
                    input_action: InputAction,
                    map: &mut Map,
                    objects: &mut Vec<Object>,
                    inventory: &mut Vec<Object>,
                    messages: &mut Messages,
                    config: &Config) -> PlayerAction {
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
        match (input_action, player_alive) {
            (InputAction::Move(move_action), true) => {
                player_action = player_move_or_attack(move_action,
                                                      map,
                                                      objects,
                                                      messages,
                                                      config);
            }

            //(Key { code: Enter, alt: true, .. }, _) => {
            (InputAction::FullScreen, _) => {
                // TODO removed for ggez
                // let fullscreen = game.root.is_fullscreen();
                // game.root.set_default_foreground(WHITE);
                // game.root.set_fullscreen(!fullscreen);
                player_action = DidntTakeTurn;
            },

            // (Key {printable: 'g', .. }, true) => {
            (InputAction::Pickup, true) => {
                let item_id = objects.iter().position(|object| {
                    object.pos() == objects[PLAYER].pos() && object.item.is_some()
                });
                if let Some(item_id) = item_id {
                    pick_item_up(item_id, objects, inventory, config, messages);
                }
                player_action = DidntTakeTurn;
            }

            // (Key {printable: 'i', .. }, true) => {
            (InputAction::Inventory, true) => {
                player_action = DidntTakeTurn;
            }

            // (Key { code: Escape, .. }, _) => {
            (InputAction::Exit, _) => {
                player_action = Exit;
            }

            // (Key {printable: 'v', .. }, true) => {
            (InputAction::ExploreAll, _) => {
                for x in 0..MAP_WIDTH {
                    for y in 0..MAP_HEIGHT {
                        map.tiles[x as usize][y as usize].explored = true;
                    }
                }
                player_action = DidntTakeTurn;
            }

            // (Key {printable: 'r', .. }, true) => {
            (InputAction::RegenerateMap, _) => {
                let mut rng: SmallRng = SeedableRng::seed_from_u64(2);
                let (map_regen, _position) = make_map(objects, config, &mut rng);
                setup_fov(&mut game.fov, &map_regen);
                map.tiles = map_regen.tiles;
                player_action = DidntTakeTurn;
            }

            // (Key {code: NumPadAdd, .. }, true) => {
            (InputAction::ToggleOverlays, _) => {
                game.display_overlays = !game.display_overlays;

                player_action = DidntTakeTurn;
            }

            //(Key {printable: 't', .. }, true) => {
            (InputAction::GodMode, true) => {
                let fighter = objects[PLAYER].fighter.unwrap();
                let god_mode_hp = 1000000;
                objects[PLAYER].fighter =
                    Some(Fighter { hp: god_mode_hp, max_hp: god_mode_hp, ..fighter });

                // set god mode flag
                game.god_mode = true;

                // set all tiles to be transparent and walkable. walkable is not current used
                // anywhere
                for x in 0..map.tiles.len() {
                    for y in 0..map.tiles[0].len() {
                        game.fov.set(x as i32, y as i32, true, true);
                    }
                }

                player_action = DidntTakeTurn;
            }

            //(_, _) => {
            (_, _) => {
                player_action = DidntTakeTurn;
            }
        }
    }

    return player_action;
}

/// Check whether a move, given as an offset from an object's current position,
/// hits a wall or object.
pub fn move_valid(object_id: ObjectId, objects: &[Object], dx: i32, dy: i32, map: &Map) -> bool {
    return check_collision(object_id, objects, dx, dy, map).is_none();
}


pub fn move_just_before(object_id: ObjectId, objects: &[Object], dx: i32, dy: i32, map: &Map) -> Option<(i32, i32)> {
    let x = objects[object_id].x;
    let y = objects[object_id].y;
    let move_line = Line::new((x, y), (x + dx, y + dy));

    let mut pos = None;
    let mut collided = false;

    for (x_pos, y_pos) in move_line.into_iter() {
        if map.is_blocked(x_pos, y_pos, objects) ||
           map.is_blocked_by_wall(x_pos, y_pos, dx, dy) {
            collided = true;
            break;
       }
       pos = Some((x_pos, y_pos));
    }

    if !collided {
        pos = None;
    }

    return pos;
}

/// Moves the given object with a given offset, returning the square that it collides with, or None
/// indicating no collision.
pub fn check_collision(object_id: ObjectId, objects: &[Object], dx: i32, dy: i32, map: &Map) -> Option<(i32, i32)> {
    let x = objects[object_id].x;
    let y = objects[object_id].y;
    let move_line = Line::new((x, y), (x + dx, y + dy));

    let mut pos = None;

    for (x_pos, y_pos) in move_line.into_iter() {
        if map.is_blocked(x_pos, y_pos, objects) &&
           map.is_blocked_by_wall(x_pos, y_pos, dx, dy) {
            pos = Some((x_pos, y_pos));
            break;
       }
    }

    return pos;
}

/*
pub fn move_action(input_action: InputAction, object_id: ObjectId, objects: &[Object], map: &Map) -> Option<(i32, i32)> {
    let pos: Option<(i32, i32)>;
    match objects[object_id].momentum {
        None => {
            match input_action {
                Left => pos = Some((-1, 0));
                Right => pos = Some((1, 0));
                Up => pos = Some((0, 1));
                Down => pos = Some((0, -1));
                DownLeft => pos = Some((-1, -1));
                DownRight => pos = Some((-1, 1));
                UpLeft => pos = Some((-1, 1));
                UpRight => pos = Some((1, 1));
                Center => pos = Some((0, 0));
                _ => panic!(format!("Unexpected input {:?}!", input_action));
            }
        }

        Some(momentum) => {
            let momentum = objects[PLAYER].momentum.unwrap();
            let mut mx = momentum.mx;
            let mut my = momentum.my;
            let mut took_half_turn = false;

            let has_momentum = mx.abs() > 1 || my.abs() > 1;
            let momentum_diagonal = mx.abs() != 0 && my.abs() != 0;
            let side_move = dx.abs() != 0 && dy.abs() != 0;
            let same_direction = mx.signum() == dx.signum() && my.signum() == dy.signum();
            let momentum_magnitude = cmp::max(mx, my);

            let momentum_change: MomentumChange;

            let player_action: PlayerAction;

            // if the space is not blocked, move
            if move_valid(PLAYER, objects, dx, dy, map) {
                objects[PLAYER].set_pos(x + dx, y + dy);
                momentum_change = MomentumChange::CurrentDirection;

                // if the player has enough momentum, they get another action, taking only a half turn
                // if the player previous took a half turn, they cannot take another
                if momentum_magnitude > 1 && !momentum.took_half_turn {
                    player_action = PlayerAction::TookHalfTurn;
                    took_half_turn = true;
                } else {
                    player_action = PlayerAction::TookTurn;
                }
            } else if has_momentum &&
                      side_move &&
                      !momentum_diagonal &&
                      move_valid(PLAYER, objects, mx.signum(), my.signum(), map) &&
                      move_valid(PLAYER, objects, dx, dy, map) &&
                      !map.is_blocked(x + mx.signum(), y + my.signum(), objects) && // free next to wall
                      !map.is_blocked(x + 2*mx.signum(), y + 2*my.signum(), objects) && // free space to move to
                      !map.is_blocked_by_wall(x, y, dx, dy) &&
                      map[(x + dx, y + dy)].tile_type == TileType::Wall {
                // jump off wall
                objects[PLAYER].set_pos(x + 2*mx.signum(), y + 2*my.signum());
                momentum_change = MomentumChange::PreviousDirection;
                player_action = PlayerAction::TookTurn;
            } else if has_momentum &&
                      same_direction &&
                      map[(x + dx, y + dy)].tile_type == TileType::ShortWall &&
                      !map.is_blocked(x + 2*dx, y + 2*dy, objects) &&
                      !map.is_blocked_by_wall(x, y, dx, dy) {
                // if the location is blocked by a short wall, and the next location in the
                // line is not, and we have momentum, then jump over obstacle
                objects[PLAYER].set_pos(x + 2*dx, y + 2*dy);
                momentum_change = MomentumChange::CurrentDirection;
                player_action = PlayerAction::TookTurn;
            } else {
                // otherwise we hit a wall and lose our momentum
                momentum_change = MomentumChange::Lost;
                player_action = PlayerAction::TookTurn;
            }

            match momentum_change {
                MomentumChange::Lost => {
                    mx = 0;
                    my = 0;
                }

                MomentumChange::PreviousDirection => {
                    mx = clamp(mx + mx.signum(), -MAX_MOMENTUM, MAX_MOMENTUM);
                    my = clamp(my + my.signum(), -MAX_MOMENTUM, MAX_MOMENTUM);
                }

                MomentumChange::CurrentDirection => {
                    if same_direction {
                        mx = clamp(mx + dx, -MAX_MOMENTUM, MAX_MOMENTUM);
                        my = clamp(my + dy, -MAX_MOMENTUM, MAX_MOMENTUM);
                    } else {
                        mx = dx;
                        my = dy;
                    }
                }
            }

            objects[PLAYER].momentum = 
                Some(Momentum {
                    mx: mx,
                    my: my,
                    took_half_turn: took_half_turn,
                });
        }
    }

    return pos;
}

pub fn valid_moves(object_id: ObjectId, objects: &[Object], map: &Map) -> Vec<(i32, i32)> {
    let x = objects[object_id].x;
    let y = objects[object_id].y;
    let mut moves: Vec<(i32, i32)> = Vec::new();

    let possible_moves =
        vec![InputAction::Left,
             InputAction::Right,
             InputAction::Up,
             InputAction::Down,
             InputAction::DownLeft,
             InputAction::DownRight,
             InputAction::UpLeft,
             InputAction::UpRight,
             InputAction::Center];

    for input_action in possible_moves {
        if let Some(pos) = move_action(input_action, object_id, objects, map) {
            moves.push(pos);
        }
    }

    return moves;
}
*/

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
            config: &Config,
            messages: &mut Messages) {
    use Item::*;

    if let Some(item) = inventory[inventory_id].item {
        let on_use = match item {
            Stone => unimplemented!(),
            Goal => gather_goal,
        };
        match on_use(inventory_id, objects, config, messages) {
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

fn gather_goal(_inventory_id: usize, _objects: &mut [Object], config: &Config, messages: &mut Messages) -> UseResult {
    messages.message("You've got the goal object! Nice work.", config.color_orange.color());
    UseResult::Keep
}

fn pick_item_up(object_id: usize,
                objects: &mut Vec<Object>,
                inventory: &mut Vec<Object>,
                config: &Config,
                messages: &mut Messages) {
    if inventory.len() >= 26 {
        messages.message(format!("Your inventory is full, cannot pick up {}", objects[object_id].name), config.color_red.color());
    } else {
        let item = objects.swap_remove(object_id);
        messages.message(format!("You picked up a {}!", item.name), config.color_light_green.color());
        inventory.push(item);
    }
}

fn player_move_or_attack(move_action: MoveAction,
                         map: &Map,
                         objects: &mut [Object],
                         _messages: &mut Messages,
                         config: &Config) -> PlayerAction {
    let player_action: PlayerAction;

    let movement = calculate_move(move_action, PLAYER, objects, map);

    match movement {
        Some(Movement::Attack(_dx, _dy, target_id)) => {
            let (player, target) = mut_two(PLAYER, target_id, objects);
            player.attack(target, config);
            player_action = PlayerAction::TookTurn;
        }

        // TODO may want to check if any movement occurred, and not take turn
        // if the player is against a wall.
        Some(Movement::Collide(x, y)) => {
            objects[PLAYER].set_pos(x, y);
            objects[PLAYER].momentum.unwrap().clear();
            player_action = PlayerAction::TookTurn;
        }

        Some(Movement::Move(x, y)) | Some(Movement::JumpWall(x, y)) => {
            dbg!(x, y);
            objects[PLAYER].set_pos(x, y);
            let mut momentum = objects[PLAYER].momentum.unwrap();
            momentum.moved(x, y);
            if momentum.magnitude() > 1 && !momentum.took_half_turn {
                player_action = PlayerAction::TookHalfTurn;
            } else {
                player_action = PlayerAction::TookTurn;
            }

            momentum.took_half_turn = player_action == PlayerAction::TookHalfTurn;
            objects[PLAYER].momentum = Some(momentum);
        }

        Some(Movement::WallKick(x, y, dir_x, dir_y)) => {
            let mut momentum = objects[PLAYER].momentum.unwrap();
            objects[PLAYER].set_pos(x, y);
            momentum.set_momentum(dir_x, dir_y);

            // TODO could check for enemy and attack
            player_action = PlayerAction::TookTurn;
        }

        None => {
             player_action = PlayerAction::DidntTakeTurn;
        }
    }

    return player_action;
}

pub fn calculate_move(action: MoveAction,
                      object_id: ObjectId,
                      objects: &mut [Object],
                      map: &Map) -> Option<Movement> {
    let movement: Option<Movement>;

    let (x, y) = objects[object_id].pos();
    let (dx, dy) = action.into_move();

    if let Some(collision_pos) = check_collision(object_id, objects, dx, dy, map) {
        if map[collision_pos].blocked {
            match objects[object_id].momentum {
                Some(momentum) => {
            dbg!();
                    let _side_move = dx.abs() != 0 && dy.abs() != 0;
                    //let same_direction = momentum.mx.signum() == dx.signum() && momentum.my.signum() == dy.signum();

                    // TODO check if hit wall
                    //        if wall, check for wall kick
                    //        else collide with wall
                    //      check if hit enemy
                    //        if so, move to them and return collide

                    // if max momentum, and will hit short wall, and there is space beyond the
                    // wall, than jump over the wall.
                    if momentum.magnitude() == MAX_MOMENTUM &&
                       map[collision_pos].tile_type == TileType::ShortWall &&
                       !map.is_blocked(collision_pos.0 + 2*dx.signum(), collision_pos.1 + 2 * dy.signum(), objects) {

                           movement = Some(Movement::JumpWall(collision_pos.0 + 2 * dx.signum(), collision_pos.1 + 2 * dy.signum()));
                    // check for diagonal wall kick
                    /*
                    } else if map[collision_pos].tile_type == TileType::Wall &&
                              momentum.running() &&
                              side_move &&
                              move_valid(PLAYER, objects, mx.signum(), my.signum(), map) &&
                              move_valid(PLAYER, objects, dx, dy, map) &&
                              !map.is_blocked(x + mx.signum(), y + my.signum(), objects) && // free next to wall
                              !map.is_blocked(x + 2*mx.signum(), y + 2*my.signum(), objects) {
                              (dx.signum() == 1 && dy.signum() == -1 && !map.is_blocked(x + 1, y)) ||

                        objects[PLAYER].set_pos(x + 2*mx.signum(), y + 2*my.signum());
                    */
                    } else { // otherwise move normally
                        movement = Some(Movement::Move(x + dx, y + dy));
                    }
                },

                None => {
            dbg!();
                    movement = Some(Movement::Move(x + dx, y + dy));
                },
            }
        } else {
            // otherwise, we hit an object
            let new_x;
            let new_y;
            if let Some((just_before_x, just_before_y)) = move_just_before(PLAYER, objects, dx, dy, map) {
                new_x = just_before_x;
                new_y = just_before_y;
            } else {
                new_x = x;
                new_y = y;
            }

            // TODO consider some kind of check on whether we attack or not
            //      perhaps instead of Attack, say CollideObject
            let object_id = objects.iter().position(|o| o.x == x && o.y == y).unwrap();
            if object_id != PLAYER {
                movement = Some(Movement::Attack(new_x, new_y, object_id));
            } else {
                dbg!(x, y, new_x, new_y);
                movement = Some(Movement::Move(new_x, new_y));
            }
        }
    } else {
        dbg!((x, y, x+dx, y + dy));
        movement = Some(Movement::Move(x + dx, y + dy));
    }

    return movement;
}

pub fn map_keycode_to_action(keycode: KeyCode, keymods: KeyMods) -> InputAction {
    let input_action: InputAction;

    match keycode {
        KeyCode::Key8 | KeyCode::Numpad8 | KeyCode::Up => {
            input_action = InputAction::Move(MoveAction::Up);
        }

        KeyCode::Key6 | KeyCode::Numpad6 | KeyCode::Right => {
            input_action = InputAction::Move(MoveAction::Right);
        }

        KeyCode::Key2 | KeyCode::Numpad2 | KeyCode::Down => {
            input_action = InputAction::Move(MoveAction::Down);
        }

        KeyCode::Key4 | KeyCode::Numpad4 | KeyCode::Left => {
            input_action = InputAction::Move(MoveAction::Left);
        }

        KeyCode::Key7 | KeyCode::Numpad7 => {
            input_action = InputAction::Move(MoveAction::UpLeft);
        }

        KeyCode::Key9 | KeyCode::Numpad9 => {
            input_action = InputAction::Move(MoveAction::UpRight);
        }

        KeyCode::Key3 | KeyCode::Numpad3 => {
            input_action = InputAction::Move(MoveAction::DownRight);
        }

        KeyCode::Key1 | KeyCode::Numpad1 => {
            input_action = InputAction::Move(MoveAction::DownLeft);
        }

        KeyCode::Key5 | KeyCode::Numpad5 => {
            input_action = InputAction::Move(MoveAction::Center);
        }

        KeyCode::Return => {
            if keymods.contains(KeyMods::ALT) {
                input_action = InputAction::FullScreen;
            } else {
                input_action = InputAction::None;
            }
        }

        KeyCode::G => {
            input_action = InputAction::Pickup;
        }

        KeyCode::I => {
            input_action = InputAction::Inventory;
        }

        KeyCode::V => {
            input_action = InputAction::ExploreAll;
        }

        KeyCode::Escape => {
            input_action = InputAction::Exit;
        }

        KeyCode::R => {
            input_action = InputAction::RegenerateMap;
        }

        KeyCode::Add => {
            input_action = InputAction::ToggleOverlays;
        }

        KeyCode::T => {
            input_action = InputAction::GodMode;
        }

        _ => {
            input_action = InputAction::None;
        }
    }

    return input_action;
}
