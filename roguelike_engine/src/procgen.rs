use std::fs::File;
use std::io::{Read, BufReader};
use std::collections::HashSet;

use serde::{Serialize, Deserialize};

use pathfinding::directed::astar::astar;

use rand::rngs::SmallRng;
use rand::{SeedableRng};

use wfc_image::*;
use image;
use image::GenericImageView;

use euclid::*;

use roguelike_utils::line::*;
use roguelike_utils::rng::*;
use roguelike_utils::comp::*;

use roguelike_core::constants::*;
use roguelike_core::map::*;
use roguelike_core::types::*;
use roguelike_core::utils::*;

use crate::generation::*;
use crate::game::*;
use crate::vault::*;


#[derive(Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Debug)]
pub enum StructureType {
    Single,
    Line,
    Path,
    Complex,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Structure {
    pub blocks: Vec<Pos>,
    pub typ: StructureType,
}

impl Structure {
    pub fn new() -> Structure {
        return Structure { blocks: Vec::new(), typ: StructureType::Single };
    }

    pub fn add_block(&mut self, block: Pos) {
        self.blocks.push(block);
    }
}

#[derive(Clone, PartialEq, Eq, Ord, PartialOrd, Debug, Serialize, Deserialize)]
pub enum ProcCmd {
    Island(i32), // radius
    Entities(EntityName, usize, usize),
    Items(Item, usize, usize),
    MaxItems(usize),
    MaxGates(usize),
    Traps(Trap, usize, usize),
    MaxTraps(usize),
    Grass((usize, usize), i32), // (min, max), disperse distance
    Rubble(usize),
    Columns(usize),
    SeedFile(String),
    Vaults(usize), // maximum number of vaults
}

impl ProcCmd {
    pub fn from_file(file_name: &str) -> Vec<ProcCmd> {
        let mut file =
            File::open(file_name).expect(&format!("Could not open/parse procgen file {}", file_name));
        let mut procgen_string = String::new();
        file.read_to_string(&mut procgen_string)
            .expect(&format!("Could not read contents of {}", file_name));

        let cmds = serde_yaml::from_str(&procgen_string).expect(&format!("Could not parse {} file!", file_name));
        
        return cmds;
    }
}

pub fn generate_bare_map(width: u32, height: u32, template_file: &str, rng: &mut Rand32) -> Map {
    let mut new_map = Map::from_dims(width, height);

    let file = File::open(template_file).unwrap();
    let reader = BufReader::new(file);
    let seed_image = image::load(reader, image::ImageFormat::Png).unwrap();
    let orientations = [Orientation::Original,
                        Orientation::Clockwise90,
                        Orientation::Clockwise180,
                        Orientation::Clockwise270,
                        Orientation::DiagonallyFlipped,
                        Orientation::DiagonallyFlippedClockwise90,
                        Orientation::DiagonallyFlippedClockwise180,
                        Orientation::DiagonallyFlippedClockwise270];

    // Rand32 cannot be used here because of the RngCore trait restriction.
    let seed: [u8; 32] = [rng.rand_u32() as u8; 32];
    let mut small_rng = SmallRng::from_seed(seed);
    let map_image = 
        wfc_image::generate_image_with_rng(&seed_image,
                                           core::num::NonZeroU32::new(3).unwrap(),
                                           wfc_image::Size::new(width, height),
                                           &orientations, 
                                           wfc_image::wrap::WrapNone,
                                           ForbidNothing,
                                           wfc_image::retry::NumTimes(3),
                                           &mut small_rng).unwrap();
    map_image.save("wfc_map.png").unwrap();

    for x in 0..width {
        for y in 0..height {
            let pixel = map_image.get_pixel(x, y);
            if pixel.0[0] == 0 {
                let pos = Pos::new(x as i32, y as i32);
                new_map[pos] = Tile::wall_with(MAP_WALL as char);
            }
         }
    }

    return new_map;
}

fn check_map(game: &Game) {
    for wall_pos in game.level.map.get_wall_pos() {
        for id in game.level.entities.ids.iter() {
            if wall_pos == game.level.entities.pos[id] {
                panic!("A wall overlapped with an entity!");
            }
        }
    }
}

pub fn saturate_map(game: &mut Game, cmds: &Vec<ProcCmd>) -> Pos {
    // this is problematic for movement, so ensure they don't occur
    handle_diagonal_full_tile_walls(game);

    /* clear out an island */
    let island_radius =
        cmds.iter().filter_map(|cmd| {
            if let ProcCmd::Island(radius) = cmd {
                return Some(radius) 
            };
            return None;
    }).map(|r| *r).next().unwrap_or(0);

    place_vaults(game, cmds);

    clear_island(game, island_radius);

    /* detect structures left */
    let mut structures = find_structures(&game.level.map);
    eprintln!();
    eprintln!("{} singles", structures.iter().filter(|s| s.typ == StructureType::Single).count());
    eprintln!("{} lines", structures.iter().filter(|s| s.typ == StructureType::Line).count());
    eprintln!("{} paths", structures.iter().filter(|s| s.typ == StructureType::Path).count());
    eprintln!("{} complex", structures.iter().filter(|s| s.typ == StructureType::Complex).count());

    /* modify structures with rubble, columns, etc */
    modify_structures(game, cmds, &mut structures);

    // lay down grass with a given dispersion and range from the found tile
    let range_disperse =
        cmds.iter().filter_map(|cmd| {
            if let ProcCmd::Grass(range, disperse) = cmd {
                return Some((range, disperse)) 
            };
            return None;
    }).next().unwrap_or((&(0, 0), &0));
    let high = (range_disperse.0).1 as u32;
    let low = (range_disperse.0).0 as u32;
    let num_grass_to_place = rng_range_u32(&mut game.rng, low, high) as usize;
    place_grass(game, num_grass_to_place, *range_disperse.1);

    // clear about the island again to ensure tiles haven't been placed outside
    clear_island(game, island_radius);

    // find a place to put the player
    let player_id = game.level.find_by_name(EntityName::Player).unwrap();
    // TODO ideally we would loop instead of unwrap, generating new levels
    // until a player position was found.
    let player_pos = find_available_on_side(game, true).unwrap();
    game.level.entities.pos[&player_id] = player_pos;

    clear_island(game, island_radius);

    place_items(game, cmds);

    place_monsters(game, player_id, cmds);

    place_traps(game, cmds);

    place_triggers(game, cmds);

    // find a place to put the key and goal, ensuring that they are reachable
    place_key_and_goal(game, player_pos);

    // clear the island once more just in case
    clear_island(game, island_radius);

    // ensure that the map looks okay in 3D
    ensure_iter_and_full_walls(game);

    check_map(game);

    return player_pos;
}

fn modify_structures(game: &mut Game, cmds: &Vec<ProcCmd>, structures: &mut Vec<Structure>) {
    let max_rubbles =
        cmds.iter().filter_map(|cmd| {
            if let ProcCmd::Rubble(num_rubble) = cmd {
                return Some(num_rubble) 
            };
            return None;
    }).map(|r| *r).next().unwrap_or(0);
    let mut num_rubbles = 0;

    let mut to_remove: Vec<usize> = Vec::new();
    for (index, structure) in structures.iter().enumerate() {
        // turn some lone single-tile walls into columns
        if structure.typ == StructureType::Single {
            if rng_range(&mut game.rng, 0.0, 1.0) < 0.3 {
                make_column(&mut game.level.entities, &game.config, structure.blocks[0], &mut game.msg_log);
                to_remove.push(index);
            }
        } else if rng_range(&mut game.rng, 0.0, 1.0) < 0.3 {
            if num_rubbles < max_rubbles {
                let index = rng_range_u32(&mut game.rng, 0, structure.blocks.len() as u32) as usize;
                let block = structure.blocks[index];
                game.level.map[block] = Tile::empty();
                game.level.map[block].surface = Surface::Rubble;
                num_rubbles += 1;
            }
        }

        // turn some structures into short or tall walls
        if structure.typ != StructureType::Single && rng_range(&mut game.rng, 0.0, 1.0) < 0.7 {
           let wall_type;
           if rng_range(&mut game.rng, 0.0, 1.0) < 1.0 {
               wall_type = Wall::ShortWall;
           } else {
               wall_type = Wall::TallWall;
           }

           for pos in structure.blocks.iter() {
               game.level.map[*pos] = Tile::empty();

               for neighbor in game.level.map.cardinal_neighbors(*pos) {
                   if game.level.map.is_within_bounds(neighbor) &&
                      game.level.map[neighbor].block_move {
                       if pos.x != neighbor.x {
                           game.level.map[*pos].bottom_wall = wall_type;
                       }
                       if pos.y != neighbor.y {
                           game.level.map[*pos].left_wall = wall_type;
                       }
                   }
               } 
           }
        }
    }

    // remove structures that have been turned into other things
    to_remove.sort();
    to_remove.reverse();
    for index in to_remove.iter() {
        for block in structures[*index].blocks.iter() {
            game.level.map[*block] = Tile::empty();
        }
        structures.swap_remove(*index);
    }
}

/// Look for intertile walls that are adjacent to full tile walls.
/// These don't make sense in 3D, so remove them.
fn ensure_iter_and_full_walls(game: &mut Game) {
    let (width, height) = game.level.map.size();

    for y in 0..(height - 1) {
        for x in 0..(width - 1) {
            if game.level.map[(x, y)].block_move {
                game.level.map[(x, y)].left_wall = Wall::Empty;
                game.level.map[(x, y)].bottom_wall = Wall::Empty;
            }

            if game.level.map[(x + 1, y)].block_move {
                game.level.map[(x, y)].left_wall = Wall::Empty;
            }

            if game.level.map[(x, y + 1)].block_move {
                game.level.map[(x, y)].bottom_wall = Wall::Empty;
            }
        }
    }
}

/// Ensure that diagonal full tile walls do not occur.
fn handle_diagonal_full_tile_walls(game: &mut Game) {
    let (width, height) = game.level.map.size();

    for y in 0..(height - 1) {
        for x in 0..(width - 1) {
            if game.level.map[(x, y)].block_move         && 
               game.level.map[(x + 1, y + 1)].block_move &&
               !game.level.map[(x + 1, y)].block_move    && 
               !game.level.map[(x, y + 1)].block_move {
                   game.level.map[(x + 1, y)] = Tile::wall();
            } else if game.level.map[(x + 1, y)].block_move  && 
                      game.level.map[(x, y + 1)].block_move  &&
                      !game.level.map[(x, y)].block_move &&
                      !game.level.map[(x + 1, y + 1)].block_move {
                   game.level.map[(x, y)] = Tile::wall();
            }
        }
    }
}

fn place_items(game: &mut Game, cmds: &Vec<ProcCmd>) {
    let potential_pos = game.level.get_clear_pos();

    let mut num_items = 0;
    let max_items = cmds.iter().filter_map(|cmd| {
        if let ProcCmd::MaxItems(n) = cmd {
            return Some(n);
        }
        return None;
    }).map(|n| *n).next().unwrap_or(10000);

    for cmd in cmds.iter() {
        if let ProcCmd::Items(typ, min, max) = cmd {
            let num_gen = rng_range_u32(&mut game.rng, *min as u32, (*max + 1) as u32) as usize;
            for _ in 0..num_gen {
                num_items += 1;
                if num_items >= max_items {
                    return;
                }

                let len = potential_pos.len();

                if len == 0 {
                    break;
                }

                let index = rng_range_u32(&mut game.rng, 0, len as u32) as usize;
                let pos = potential_pos[index];

                match typ {
                    Item::Dagger => { make_dagger(&mut game.level.entities, &game.config, pos, &mut game.msg_log); },
                    Item::Sword => { make_sword(&mut game.level.entities, &game.config, pos, &mut game.msg_log); },
                    Item::Shield => { make_shield(&mut game.level.entities, &game.config, pos, &mut game.msg_log); },
                    Item::Hammer => { make_hammer(&mut game.level.entities, &game.config, pos, &mut game.msg_log); },
                    Item::Spear => { make_spear(&mut game.level.entities, &game.config, pos, &mut game.msg_log); },
                    Item::Stone => { make_stone(&mut game.level.entities, &game.config, pos, &mut game.msg_log); },
                    Item::Lantern => { make_lantern(&mut game.level.entities, &game.config, pos, &mut game.msg_log); },
                    _ => {},
                }
            }
        }
    }
}

fn place_triggers(game: &mut Game, cmds: &Vec<ProcCmd>) {
    let potential_pos = game.level.get_clear_pos();

    let mut near_walls = HashSet::new();

    for pos in potential_pos {
        for neighbor in game.level.map.cardinal_neighbors(pos) {
            if game.level.map[neighbor].tile_type == TileType::Wall {
                near_walls.insert(pos);
            }
        }
    }

    let max_gates = cmds.iter().filter_map(|cmd| {
        if let ProcCmd::MaxGates(n) = cmd {
            return Some(n);
        }
        return None;
    }).map(|n| *n).next().unwrap_or(0);

    let mut gate_positions = near_walls.iter().map(|p| *p).collect::<Vec<Pos>>();

    // if there are no possible positions, exit early
    if gate_positions.len() == 0 {
        return;
    }

    for _ in 0..max_gates {
        let gate_pos_index = rng_range_u32(&mut game.rng, 0, gate_positions.len() as u32) as usize;
        let gate_pos = gate_positions[gate_pos_index];
        gate_positions.swap_remove(gate_pos_index);

        let trigger = make_gate_trigger(&mut game.level.entities, &game.config, gate_pos, &mut game.msg_log);

        // Find a nearby position to use as the trigger's gate. this should always succeed because
        // the position was chosen to be near a wall.
        for neighbor in game.level.map.cardinal_neighbors(gate_pos) {
            if game.level.map[neighbor].tile_type == TileType::Wall {
                game.level.entities.gate_pos[&trigger] = neighbor;
                break;
            }
        }

        // clear the surface of the tile.
        game.level.map[gate_pos] = Tile::empty();
    }
}

fn place_traps(game: &mut Game, cmds: &Vec<ProcCmd>) {
    let potential_pos = game.level.get_clear_pos();

    let mut num_traps = 0;
    let max_traps = cmds.iter().filter_map(|cmd| {
        if let ProcCmd::MaxTraps(n) = cmd {
            return Some(n);
        }
        return None;
    }).map(|n| *n).next().unwrap_or(10000);

    for cmd in cmds.iter() {
        if let ProcCmd::Traps(typ, min, max) = cmd {
            let num_gen = rng_range_u32(&mut game.rng, *min as u32, (*max + 1) as u32);
            for _ in 0..num_gen {
                num_traps += 1;
                if num_traps >= max_traps {
                    return;
                }

                let len = potential_pos.len();

                if len == 0 {
                    return;
                }

                let index = rng_range_u32(&mut game.rng, 0, len as u32) as usize;
                let pos = potential_pos[index];

                match typ {
                    Trap::Spikes => { make_spike_trap(&mut game.level.entities, &game.config, pos, &mut game.msg_log); },
                    Trap::Sound => { make_sound_trap(&mut game.level.entities, &game.config, pos, &mut game.msg_log); },
                    Trap::Blink => { make_blink_trap(&mut game.level.entities, &game.config, pos, &mut game.msg_log); },
                    Trap::Freeze => { make_freeze_trap(&mut game.level.entities, &game.config, pos, &mut game.msg_log); },
                }

                // clear tile surface
                game.level.map[pos] = Tile::empty();
            }
        }
    }
}

fn place_monsters(game: &mut Game, player_id: EntityId, cmds: &Vec<ProcCmd>) {
    let player_pos = game.level.entities.pos[&player_id];

    // get empty positions, but make sure they are not close to the player
    let mut potential_pos = 
        game.level.get_clear_pos()
                 .iter()
                 .filter(|p| distance(player_pos, **p) > 4)
                 .map(|p| *p)
                 .collect::<Vec<Pos>>();

    for cmd in cmds.iter() {
        if let ProcCmd::Entities(typ, min, max) = cmd {
            let num_gen = rng_range_u32(&mut game.rng, *min as u32, *max as u32) as usize;

            for _ in 0..num_gen {
                let len = potential_pos.len();

                if len == 0 {
                    break;
                }

                let index = rng_range_u32(&mut game.rng, 0, len as u32) as usize;
                let pos = potential_pos[index];

                let id;
                match typ {
                    EntityName::Gol => { id = Some(make_gol(&mut game.level.entities, &game.config, pos, &mut game.msg_log)); },
                    EntityName::Pawn => { id = Some(make_pawn(&mut game.level.entities, &game.config, pos, &mut game.msg_log)); },
                    EntityName::Spire => { id = Some(make_spire(&mut game.level.entities, &game.config, pos, &mut game.msg_log)); },
                    EntityName::Armil => { id = Some(make_armil(&mut game.level.entities, &game.config, pos, &mut game.msg_log)); },
                    EntityName::Rook => { id = Some(make_rook(&mut game.level.entities, &game.config, pos, &mut game.msg_log)); },
                    _ => { id = None; },
                }
                if let Some(id) = id {
                    // if the entity can see the player, turn it around so it is facing away.
                    if game.level.is_in_fov(id, player_id, &game.config) == FovResult::Inside {
                        game.level.entities.direction[&id] = 
                            game.level.entities.direction[&id].reverse();
                    }
                }

                potential_pos.remove(index);
            }
        }
    }
}

// TODO choose based on frequency given by tags
fn place_vaults(game: &mut Game, cmds: &Vec<ProcCmd>) {
    for cmd in cmds.iter() {
        if let ProcCmd::Vaults(max) = cmd {
            for _ in 0..*max {
                let (width, height) = game.level.map.size();
                let x = rng_range_i32(&mut game.rng, 0, width);
                let y = rng_range_i32(&mut game.rng, 0, height);
                let offset = Pos::new(x, y);

                let vault_index = rng_range_u32(&mut game.rng, 0, game.vaults.len() as u32) as usize;
                let vault = &game.vaults[vault_index];
                eprintln!("Placing vault {} at {}", vault_index, offset);
                place_vault(&mut game.level, vault, offset, &mut game.rng);
            }
        }
    }
}

// TODO rotate and mirror according to tags
pub fn place_vault(level: &mut Level, vault: &Vault, offset: Pos, rng: &mut Rand32) {
                        
    let mirror = !vault.tags.contains(&VaultTag::NoMirror) && rng_range(rng, 0.0, 1.0) < 0.5;

    let mut rotation = Rotation::Degrees0;
    if !vault.tags.contains(&VaultTag::NoRotate) && rng_range(rng, 0.0, 1.0) < 0.5 {
        let rand = rng_range(rng, 0.0, 3.0).round();
        let index = rand as usize;
        let rotations = &[Rotation::Degrees0, Rotation::Degrees90, Rotation::Degrees180, Rotation::Degrees270];
        rotation = rotations[index];
    }

    place_vault_with(level, vault, offset, rotation, mirror);
}

pub fn place_vault_with(level: &mut Level, vault: &Vault, offset: Pos, rotation: Rotation, mirror: bool) {
    let mut actual_vault = vault.clone();
    actual_vault.level.map = reorient_map(&actual_vault.level.map, rotation, mirror);

    let (width, height) = actual_vault.level.map.size();

    // update map with vault tiles
    for vault_pos in actual_vault.level.map.get_all_pos() {
        let map_pos = add_pos(offset, vault_pos);
        if level.map.is_within_bounds(map_pos) {
            level.map[map_pos] = actual_vault.level.map[vault_pos];

            for entity_id in level.get_entities_at_pos(map_pos) {
                if level.entities.typ[&entity_id] == EntityType::Player {
                    level.map[vault_pos] = Tile::empty();
                } else {
                    level.entities.remove_entity(entity_id);
                }
            }
        }
    }

    let mut entities_to_remove: Vec<EntityId> = Vec::new();
    let mut vault_entities_to_remove: Vec<EntityId> = Vec::new();

    // move entities to their new place in the map
    let mut entities = actual_vault.level.entities.clone();
    for id in actual_vault.level.entities.ids.iter() {
        let mut entity_pos = entities.pos[id];
        if mirror {
            entity_pos = mirror_in_x(entity_pos, width);
        }
        entity_pos = rotation.rotate(entity_pos, width, height);
        entity_pos = add_pos(offset, entity_pos);
        if level.map.is_within_bounds(entity_pos) && !level.map[entity_pos].block_move {
            entities.pos[id] = entity_pos;
        } else {
            vault_entities_to_remove.push(*id);
            continue;
        }

        // look for entities already at this position
        for entity_id in level.get_entities_at_pos(entity_pos) {
            if level.entities.typ[&entity_id] == EntityType::Player {
                // remove vault entity to avoid removing player
                vault_entities_to_remove.push(*id);
            } else {
                entities_to_remove.push(entity_id);
            }
        }
    }

    for remove_id in vault_entities_to_remove {
        entities.remove_entity(remove_id);
    }

    for remove_id in entities_to_remove {
        level.entities.remove_entity(remove_id);
    }

    level.entities.merge(&entities);
}

fn place_grass(game: &mut Game, num_grass_to_place: usize, disperse: i32) {
    let (width, height) = game.level.map.size();

    let mut potential_grass_pos = Vec::new();
    for x in 0..width {
        for y in 0..height {
            let pos = Pos::new(x, y);

            if !game.level.map[pos].block_move {
                let count = floodfill(&game.level.map, pos, 3).len();
                if count > 28 && count < 35 {
                    potential_grass_pos.push(pos);
                }
            }
        }
    }

    shuffle(&mut game.rng, &mut potential_grass_pos);
    let num_grass_to_place = std::cmp::min(num_grass_to_place, potential_grass_pos.len());
    for pos_index in 0..num_grass_to_place {
        let pos = potential_grass_pos[pos_index];
        game.level.map[pos].surface = Surface::Grass;
        ensure_grass(&mut game.level.entities, pos, &mut game.msg_log);

        for _ in 0..4 {
            let offset_pos = Pos::new(pos.x + rng_range_i32(&mut game.rng, 0, disperse),
                                      pos.y + rng_range_i32(&mut game.rng, 0, disperse));
            if game.level.map.is_within_bounds(offset_pos) &&
               !game.level.map[offset_pos].block_move {
                game.level.map[offset_pos].surface = Surface::Grass;
                ensure_grass(&mut game.level.entities, offset_pos, &mut game.msg_log);
            }
        }
    }
}

fn find_available_on_side(game: &mut Game, left: bool) -> Option<Pos> {
    let mut avail_pos: Option<Pos> = None;
    let mut x_most = if left { i32::MAX } else { 0 };

    let potential_pos = game.level.get_clear_pos();
    let mut index = 1.0;
    for pos in potential_pos {
        if !game.level.map[pos].block_move && game.level.has_blocking_entity(pos).is_none() {
            let x_more;
            let x_more_strict;
            if left {
                x_more = pos.x <= x_most;
                x_more_strict = pos.x < x_most;
                x_most = std::cmp::min(x_most, pos.x);
            } else {
                x_more = pos.x >= x_most;
                x_more_strict = pos.x > x_most;
                x_most = std::cmp::max(x_most, pos.x);
            }

            let rng_choice = rng_range(&mut game.rng, 0.0, 1.0) < (1.0 / index);

            if x_more {
                if rng_choice || x_more_strict {
                    avail_pos = Some(pos);
                }
            }

            index += 1.0;
        }
    }

    return avail_pos;
}

fn find_available_tile(game: &mut Game) -> Option<Pos> {
    let mut avail_pos = None;

    let potential_pos = game.level.get_clear_pos();
    let mut index = 1.0;
    for pos in potential_pos {
        if game.level.has_blocking_entity(pos).is_none() {
            if rng_range(&mut game.rng, 0.0, 1.0) < (1.0 / index) {
                avail_pos = Some(pos);
            }

            index += 1.0;
        }
    }

    return avail_pos;
}

fn clear_path_to(game: &mut Game, start_pos: Pos, target_pos: Pos) {
    fn blocked_tile_cost(pos: Pos, map: &Map) -> i32 {
        if map[pos].block_move {
            return 15;
        } 

        return 0;
    }

    fn move_tile_cost(pos: Pos, next_pos: Pos, map: &Map) -> i32 {
        if map.path_blocked_move(pos, next_pos).is_some() {
            return 15;
        } 

        return 1;
    }

    // clear a path to the target position
    let path = 
        astar(&start_pos,
              |&pos| {
                  game.level.map.cardinal_neighbors(pos)
                               .iter()
                               .map(|p| (*p, move_tile_cost(pos, *p, &game.level.map)))
                               .collect::<Vec<(Pos, i32)>>()
              },
              |&pos| blocked_tile_cost(pos, &game.level.map) + distance(start_pos, pos) as i32,
              |&pos| pos == target_pos);

    if let Some((results, _cost)) = path {
        for pos in results {
            game.level.map[pos].clear_walls();
        }
    }
}

fn place_key_and_goal(game: &mut Game, player_pos: Pos) {
    // place goal and key
    let key_pos = find_available_tile(game).unwrap();
    game.level.map[key_pos] = Tile::empty();
    for entity_id in game.level.get_entities_at_pos(key_pos) {
        game.level.entities.remove_entity(entity_id);
    }
    make_key(&mut game.level.entities, &game.config, key_pos, &mut game.msg_log);
    clear_path_to(game, player_pos, key_pos);

    // Find the goal position, ensuring it is not too close to the key
    let mut goal_pos = find_available_on_side(game, false).unwrap();
    while distance(key_pos, goal_pos) < 4 {
        goal_pos = find_available_tile(game).unwrap();
    }

    game.level.map[goal_pos] = Tile::empty();
    for entity_id in game.level.get_entities_at_pos(goal_pos) {
        game.level.entities.remove_entity(entity_id);
    }
    make_exit(&mut game.level.entities, &game.config, goal_pos, &mut game.msg_log);
    clear_path_to(game, player_pos, goal_pos);
}

fn clear_island(game: &mut Game, island_radius: i32) {
    fn dist(pos1: Pos, pos2: Pos) -> f32 {
        return (((pos1.x - pos2.x).pow(2) + (pos1.y - pos2.y).pow(2)) as f32).sqrt();
    }

    let (width, height) = game.level.map.size();
    let x_mid = width / 2;
    let y_mid = height / 2;
    let mid_pos = Pos::new(x_mid, y_mid);

    for y in 0..height {
        for x in 0..width {
            let pos = Pos::new(x, y);

            if dist(pos, mid_pos) >= island_radius as f32 {
                game.level.map[pos] = Tile::water();
                game.level.map[pos].chr = MAP_WATER;

                for entity_id in game.level.has_entities(pos).clone() {
                    game.level.entities.remove_entity(entity_id);
                }
            }
        }
    }
}


fn process_block(block: Pos, structure: &mut Structure, blocks: &Vec<Pos>, seen: &mut HashSet<Pos>) {
    let adjacent = adjacent_blocks(block, blocks, seen);

    let mut needs_processing = false;
    if adjacent.len() == 1 {
        needs_processing = true;
        if structure.typ == StructureType::Line && structure.blocks.len() > 1 {
            let len = structure.blocks.len();
            if sub_pos(structure.blocks[len - 2], structure.blocks[len - 1]) !=
               sub_pos(structure.blocks[len - 1], adjacent[0]) {
               structure.typ = StructureType::Path;
            }
        }

    } else if adjacent.len() > 1 {
        needs_processing = true;

        // this structure must be complex- if there are multiple adj, they are new
        // meaning we split in at least two directions
        structure.typ = StructureType::Complex;
    }

    if needs_processing {
        for adj in adjacent.iter() {
            structure.add_block(*adj);
            seen.insert(*adj);
        }

        for adj in adjacent.iter() {
            process_block(*adj, structure, blocks, seen);
        }
    }
}

fn adjacent_blocks(block: Pos, blocks: &Vec<Pos>, seen: &HashSet<Pos>) -> Vec<Pos> {
    let mut result = Vec::new();

    let adjacents = [move_x(block, 1), move_y(block, 1), move_x(block, -1), move_y(block, -1)];
    for adj in adjacents.iter() {
        if blocks.contains(adj) && !seen.contains(&adj) {
            result.push(*adj);
        }
    }

    return result;
}

#[test]
fn test_adjacent_blocks() {
    let mut map = Map::from_dims(5, 5);
    map[(2, 2)] = Tile::wall();

    map[(1, 2)] = Tile::wall();
    map[(2, 1)] = Tile::wall();
    map[(3, 2)] = Tile::wall();
    map[(2, 3)] = Tile::wall();

    let mut seen = HashSet::new();

    let blocks = find_structural_blocks(&map);
    assert_eq!(4, adjacent_blocks(Pos::new(2, 2), &blocks, &seen).len());
    assert_eq!(2, adjacent_blocks(Pos::new(1, 1), &blocks, &seen).len());
    assert_eq!(1, adjacent_blocks(Pos::new(2, 1), &blocks, &seen).len());
    seen.insert(Pos::new(1, 2));
    assert_eq!(3, adjacent_blocks(Pos::new(2, 2), &blocks, &seen).len());
}

fn find_structural_blocks(map: &Map) -> Vec<Pos> {
    let (width, height) = map.size();
    let mut blocks = Vec::new();
    for y in 0..height {
        for x in 0..width {
            if map[(x, y)].tile_type.is_wall() {
                blocks.push(Pos::new(x, y));
            }
        }
    }

    return blocks;
}

fn find_structures(map: &Map) -> Vec<Structure> {
    let blocks = find_structural_blocks(map);

    eprintln!("Blocks in structures: {}", blocks.len());

    let mut structures = Vec::new();
    let mut seen: HashSet<Pos> = HashSet::new();
    for block in blocks.iter() {
        if !seen.contains(&block) {
            let mut structure = Structure::new();

            let adjacent = adjacent_blocks(*block, &blocks, &seen);

            if adjacent.len() != 2 {
                structure.add_block(*block);
                seen.insert(*block);

                if adjacent.len() == 1 {
                    // found start of a structure (line, L, or complex)- process structure
                    structure.typ = StructureType::Line;
                    process_block(*block, &mut structure, &blocks, &mut seen);
                } else if adjacent.len() > 2 {
                    // found part of a complex structure- process all pieces
                    structure.typ = StructureType::Complex;

                    for adj in adjacent.iter() {
                        seen.insert(*adj);
                    }

                    for adj in adjacent {
                        process_block(adj, &mut structure, &blocks, &mut seen);
                    }
                }

                structures.push(structure);
            }
            // else we are in the middle of a line, so we will pick it up later
            // or we have a single block, which uses the default structure type of Single
        }
    }

    eprintln!("Number of structures: {}", structures.len());

    return structures;
}

#[test]
fn test_find_simple_structures() {
    let mut map = Map::from_dims(5, 5);

    // find a single line
    map[(0, 2)] = Tile::wall();
    map[(1, 2)] = Tile::wall();
    map[(2, 2)] = Tile::wall();
    let structures = find_structures(&map);
    assert_eq!(1, structures.len());
    assert_eq!(StructureType::Line, structures[0].typ);
    assert_eq!(3, structures[0].blocks.len());

    // add a lone block and check that it is found along with the line
    map[(0, 0)] = Tile::wall();
    let structures = find_structures(&map);
    assert_eq!(2, structures.len());
    assert!(structures.iter().find(|s| s.typ == StructureType::Single).is_some());
    assert!(structures.iter().find(|s| s.typ == StructureType::Line).is_some());

    // add a vertical line and check that all structures are found
    map[(4, 0)] = Tile::wall();
    map[(4, 1)] = Tile::wall();
    map[(4, 2)] = Tile::wall();
    map[(4, 3)] = Tile::wall();
    let structures = find_structures(&map);
    assert_eq!(3, structures.len());
    assert!(structures.iter().find(|s| s.typ == StructureType::Single).is_some());
    assert!(structures.iter().filter(|s| s.typ == StructureType::Line).count() == 2);
}

#[test]
fn test_find_complex_structures() {
    let mut map = Map::from_dims(5, 5);

    // lay down an L
    map[(0, 2)] = Tile::wall();
    map[(1, 2)] = Tile::wall();
    map[(2, 2)] = Tile::wall();
    map[(2, 3)] = Tile::wall();
    let structures = find_structures(&map);
    assert_eq!(1, structures.len());
    assert_eq!(StructureType::Path, structures[0].typ);
    assert_eq!(4, structures[0].blocks.len());

    // turn it into a 'complex' structure and check that it is discovered
    map[(2, 1)] = Tile::wall();
    let structures = find_structures(&map);
    assert_eq!(1, structures.len());
    assert_eq!(StructureType::Complex, structures[0].typ);
    assert_eq!(5, structures[0].blocks.len());
}


pub fn place_block(map: &mut Map, start: Pos, width: i32, tile: Tile) -> Vec<Pos> {
    let mut positions = Vec::new();

    for x in 0..width {
        for y in 0..width {
            let pos = start + Vector2D::new(x, y);
            map[pos] = tile;
            positions.push(pos);
        }
    }

    return positions;
}

pub fn place_line(map: &mut Map, start: Pos, end: Pos, tile: Tile) -> Vec<Pos> {
    let mut positions = Vec::new();
    let line = line(start, end);

    for pos in line {
        if map.is_within_bounds(pos) {
            map[pos] = tile;
            positions.push(pos);
        }
    }

    positions
}

pub fn add_obstacle(map: &mut Map, pos: Pos, obstacle: Obstacle, rng: &mut Rand32) {
    match obstacle {
        Obstacle::Block => {
            map.tiles[pos.x as usize][pos.y as usize] = Tile::wall();
        }

        Obstacle::Wall => {
            let end_pos = if rng_trial(rng, 0.5) {
                move_x(pos, 3)
            } else {
                move_y(pos, 3)
            };
            place_line(map, pos, end_pos, Tile::wall());
        }

        Obstacle::ShortWall => {
            let end_pos = if rng_trial(rng, 0.5) {
                move_x(pos, 3)
            } else {
                move_y(pos, 3)
            };
            place_line(map, pos, end_pos, Tile::short_wall());
        }

        Obstacle::Square => {
            place_block(map, pos, 2, Tile::wall());
        }

        Obstacle::LShape => {
            let mut dir = 1;
            if rng_trial(rng, 0.5) {
                dir = -1;
            }

            if rng_trial(rng, 0.5) {
                for x in 0..3 {
                    map.tiles[pos.x as usize + x][pos.y as usize] = Tile::wall();
                }
                map.tiles[pos.x as usize][(pos.y + dir) as usize] = Tile::wall();
            } else {
                for y in 0..3 {
                    map.tiles[pos.x as usize][pos.y as usize + y] = Tile::wall();
                }
                map.tiles[(pos.x + dir) as usize][pos.y as usize] = Tile::wall();
            }
        }

        Obstacle::Building => {
            let size = 2;

            let mut positions = vec!();
            positions.append(&mut place_line(map, move_by(pos, Pos::new(-size, size)),  move_by(pos, Pos::new(size,  size)), Tile::wall()));
            positions.append(&mut place_line(map, move_by(pos, Pos::new(-size, size)),  move_by(pos, Pos::new(-size, -size)), Tile::wall()));
            positions.append(&mut place_line(map, move_by(pos, Pos::new(-size, -size)), move_by(pos, Pos::new(size, -size)), Tile::wall()));
            positions.append(&mut place_line(map, move_by(pos, Pos::new(size, -size)),  move_by(pos, Pos::new(size,  size)), Tile::wall()));

            for _ in 0..rng_range_u32(rng, 0, 10) {
                positions.swap_remove(rng_range_u32(rng, 0, positions.len() as u32) as usize);
            }
        }
    }
}
