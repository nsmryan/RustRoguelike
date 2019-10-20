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


#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Collision {
    NoCollision(i32, i32),
    BlockedTile((i32, i32), (i32, i32)),
    Wall((i32, i32), (i32, i32)),
    Entity(ObjectId, (i32, i32)),
}

impl Collision {
    pub fn no_collsion(&self) -> bool {
        match self {
            Collision::NoCollision(_, _) => true,
            _ => false,
        }
    }

    pub fn move_location(&self) -> (i32, i32) {
        let last_pos;

        match *self {
            Collision::NoCollision(x, y) => {
                last_pos = (x, y);
            }

            Collision::BlockedTile(_, (x, y)) => {
                last_pos = (x, y);
            }

            Collision::Wall(_, (x, y)) => {
                last_pos = (x, y);
            }

            Collision::Entity(_, (x, y)) => {
                last_pos = (x, y);
            }
        }

        return last_pos;
    }
}


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

    if game.mouse_state.pressed.0 {
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
    } else {
        match (input_action, player_alive) {
            (InputAction::Move(move_action), true) => {
                player_action = player_move_or_attack(move_action,
                                                      map,
                                                      objects,
                                                      messages,
                                                      config);
            }

            (InputAction::FullScreen, _) => {
                // TODO don't know how to do this in ggez...
                player_action = DidntTakeTurn;
            },

            (InputAction::Pickup, true) => {
                let item_id = objects.iter().position(|object| {
                    object.pos() == objects[PLAYER].pos() && object.item.is_some()
                });
                if let Some(item_id) = item_id {
                    pick_item_up(item_id, objects, inventory, config, messages);
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
                for x in 0..MAP_WIDTH {
                    for y in 0..MAP_HEIGHT {
                        map.tiles[x as usize][y as usize].explored = true;
                    }
                }
                player_action = DidntTakeTurn;
            }

            (InputAction::RegenerateMap, _) => {
                let mut rng: SmallRng = SeedableRng::seed_from_u64(2);
                let (map_regen, _position) = make_map(objects, config, &mut rng);
                setup_fov(&mut game.fov, &map_regen);
                map.tiles = map_regen.tiles;
                player_action = DidntTakeTurn;
            }

            (InputAction::ToggleOverlays, _) => {
                game.display_overlays = !game.display_overlays;

                player_action = DidntTakeTurn;
            }

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

                player_action = TookTurn;
            }

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
    return check_collision(object_id, objects, dx, dy, map).no_collsion();
}

pub fn line_inclusive(x: i32, y: i32, dx: i32, dy: i32) -> impl Iterator<Item=(i32, i32)> {
    let start_loc = (x + (dx.signum() * dx.abs() * -1),
                     y + (dy.signum() * dy.abs() * -1));
    let end_loc = (x + dx, y + dy);

    return Line::new(start_loc, end_loc).into_iter();
}

pub fn move_just_before(object_id: ObjectId, objects: &[Object], dx: i32, dy: i32, map: &Map) -> Option<(i32, i32)> {
    let x = objects[object_id].x;
    let y = objects[object_id].y;
    let move_line = Line::new((x, y), (x + dx, y + dy));

    let mut pos = None;
    let mut collided = false;

    for (x_pos, y_pos) in move_line.into_iter() {
        if !map.is_within_bounds(x_pos, y_pos) {
            break;
        }

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
pub fn check_collision(object_id: ObjectId,
                       objects: &[Object],
                       dx: i32,
                       dy: i32,
                       map: &Map) -> Collision {
    let x = objects[object_id].x;
    let y = objects[object_id].y;
    let move_line = Line::new((x, y), (x + dx, y + dy));

    let mut last_pos = (x, y);
    let mut result: Collision = Collision::NoCollision(x + dx, y + dy);

    if !map.is_within_bounds(x + dx, y + dy) ||
       map.is_blocked_by_wall(x, y, dx, dy) {
        result = Collision::Wall((x, y), (x, y));
    } else {
        for (x_pos, y_pos) in move_line.into_iter() {
            if map.is_blocked(x_pos, y_pos, objects) {
                if map[(x_pos, y_pos)].blocked {
                    result = Collision::BlockedTile((x_pos, y_pos), last_pos);
                } else {
                    let entity_id = objects.iter()
                                           .enumerate()
                                           .find(|(_index, obj)| obj.pos() == (x_pos, y_pos))
                                           .unwrap()
                                           .0;

                    result = Collision::Entity(entity_id, last_pos);
                }
                break;
            }

            if map.is_blocked_by_wall(x_pos, y_pos, dx, dy) {
                result = Collision::Wall((x_pos, y_pos), (x_pos, y_pos));
                break;
            }

            last_pos = (x_pos, y_pos);
        }
    }

    return result;
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

    let movement = calculate_move(move_action, objects[PLAYER].movement.unwrap(), PLAYER, objects, map);

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
                      reach: Reach,
                      object_id: ObjectId,
                      objects: &[Object],
                      map: &Map) -> Option<Movement> {
    let movement: Option<Movement>;

    let (x, y) = objects[object_id].pos();
    if let Some(delta_pos) = reach.move_with_reach(&action) {
        let (dx, dy) = delta_pos.into_pair();
        // check if movement collides with a blocked location or an entity
        match check_collision(object_id, objects, dx, dy, map) {
            Collision::NoCollision(new_x, new_y) => {
                // no collision- just move to location
                movement = Some(Movement::Move(new_x, new_y));
            }

            Collision::BlockedTile((_tile_x, _tile_y), (new_x, new_y)) => {
                movement = Some(Movement::Move(new_x, new_y));
            }

            Collision::Wall((tile_x, tile_y), (new_x, new_y)) => {
                match objects[object_id].momentum {
                    Some(momentum) => {
                        // if max momentum, and will hit short wall, and there is space beyond the
                        // wall, than jump over the wall.
                        if momentum.magnitude() == MAX_MOMENTUM &&
                            map[(tile_x, tile_y)].tile_type == TileType::ShortWall &&
                            !map.is_blocked(tile_x + 2 * dx, tile_y + 2 * dy, objects) {
                                movement = Some(Movement::JumpWall(tile_x + 2 * dx, tile_y + 2 * dy));
                        } else { // otherwise move normally, stopping just before the blocking tile
                            movement = Some(Movement::Move(new_x, new_y));
                        }
                    },

                    None => {
                        // with no momentum, the movement will end just before the blocked location
                        movement = Some(Movement::Move(x + dx, y + dy));
                    },
                }
            }

            Collision::Entity(other_object_id, (new_x, new_y)) => {
                // record that an attack would occur. if this is not desired, the
                // calling code will handle this.
                movement = Some(Movement::Attack(new_x, new_y, other_object_id));
            }
        }
    } else {
        movement = None;

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
