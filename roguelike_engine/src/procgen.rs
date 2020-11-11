use std::fs::File;
use std::io::{Read, BufReader};
use std::collections::HashSet;

use rand::prelude::*;

use serde::{Serialize, Deserialize};

use pathfinding::directed::astar::astar;

use wfc_image::*;
use image;
use image::GenericImageView;

use roguelike_core::constants::*;
use roguelike_core::map::*;
use roguelike_core::types::*;
use roguelike_core::utils::*;

use crate::generation::*;
use crate::game::*;
use crate::make_map::*;


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

pub fn generate_bare_map(width: u32, height: u32, template_file: &str, rng: &mut SmallRng) -> Map {
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
    let map_image = 
        wfc_image::generate_image_with_rng(&seed_image,
                                           core::num::NonZeroU32::new(3).unwrap(),
                                           wfc_image::Size::new(width, height),
                                           &orientations, 
                                           wfc_image::wrap::WrapNone,
                                           ForbidNothing,
                                           wfc_image::retry::NumTimes(3),
                                           rng).unwrap();
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

pub fn saturate_map(game: &mut Game, cmds: &Vec<ProcCmd>) -> Pos {
    // find structures-
    // find blocks that are next to exactly one block (search through all tiles, and
    // don't accept tiles that are already accepted).
    //
    // place grass in open areas and perhaps in very enclosed areas
    // place rubble near blocks
    //
    // place goal and exit, and pathing between them, knocking out tiles that
    // block the player from completing the level.

    handle_diagonal_full_tile_walls(game);

    /* clear out an island */
    let island_radius =
        cmds.iter().filter_map(|cmd| {
            if let ProcCmd::Island(radius) = cmd {
                return Some(radius) 
            };
            return None;
    }).map(|r| *r).next().unwrap_or(0);

    place_vaults(game);

    clear_island(game, island_radius);

    /* detect structures left */
    let mut structures = find_structures(&game.data.map);
    println!();
    println!("{} singles", structures.iter().filter(|s| s.typ == StructureType::Single).count());
    println!("{} lines", structures.iter().filter(|s| s.typ == StructureType::Line).count());
    println!("{} Ls", structures.iter().filter(|s| s.typ == StructureType::Path).count());
    println!("{} complex", structures.iter().filter(|s| s.typ == StructureType::Complex).count());

    /* modify structures with rubble, columns, etc */
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
        if structure.typ == StructureType::Single {
            if game.rng.gen_range(0.0, 1.0) > 0.1 {
                make_column(&mut game.data.entities, &game.config, structure.blocks[0], &mut game.msg_log);
                to_remove.push(index);
            }
        } else if structure.typ == StructureType::Line { 
            if structure.blocks.len() > 5 && num_rubbles < max_rubbles {
                let index = game.rng.gen_range(0, structure.blocks.len());
                let block = structure.blocks[index];
                game.data.map[block] = Tile::empty();
                game.data.map[block].surface = Surface::Rubble;
                num_rubbles += 1;
            }
        }

        if (structure.typ == StructureType::Line || structure.typ == StructureType::Complex) &&
           game.rng.gen_range(0.0, 1.0) < 0.5 {
           let wall_type;
           if game.rng.gen_range(0.0, 1.0) < 1.0 {
               wall_type = Wall::ShortWall;
           } else {
               wall_type = Wall::TallWall;
           }

           for pos in structure.blocks.iter() {
               game.data.map[*pos] = Tile::empty();

               for neighbor in game.data.map.cardinal_neighbors(*pos) {
                   if game.data.map.is_within_bounds(neighbor) &&
                      game.data.map[neighbor].block_move {
                       if pos.x != neighbor.x {
                           game.data.map[*pos].bottom_wall = wall_type;
                       }
                       if pos.y != neighbor.y {
                           game.data.map[*pos].left_wall = wall_type;
                       }
                   }
               } 
           }

           if game.rng.gen_range(0.0, 1.0) < 0.25 {
               break;
           }
        }
    }

    // remove structures that have been turned into other things
    to_remove.sort();
    to_remove.reverse();
    for index in to_remove.iter() {
        for block in structures[*index].blocks.iter() {
            game.data.map[*block] = Tile::empty();
        }
        structures.swap_remove(*index);
    }

    let range_disperse =
        cmds.iter().filter_map(|cmd| {
            if let ProcCmd::Grass(range, disperse) = cmd {
                return Some((range, disperse)) 
            };
            return None;
    }).next().unwrap_or((&(0, 0), &0));
    let num_grass_to_place = game.rng.gen_range((range_disperse.0).0, (range_disperse.0).1);
    place_grass(game, num_grass_to_place, *range_disperse.1);

    clear_island(game, island_radius);

    let player_id = game.data.find_player().unwrap();
    let player_pos = find_available_tile(game).unwrap();
    game.data.entities.pos[&player_id] = player_pos;

    clear_island(game, island_radius);

    place_key_and_goal(game, player_pos);

    place_items(game, cmds);

    place_monsters(game, player_pos, cmds);

    place_traps(game, cmds);

    place_triggers(game, cmds);

    clear_island(game, island_radius);

    ensure_iter_and_full_walls(game);

    return player_pos;
}

/// Look for intertile walls that are adjacent to full tile walls.
/// These don't make sense in 3D, so remove them.
fn ensure_iter_and_full_walls(game: &mut Game) {
    let (width, height) = game.data.map.size();

    for y in 0..(height - 1) {
        for x in 0..(width - 1) {
            if game.data.map[(x, y)].block_move {
                game.data.map[(x, y)].left_wall = Wall::Empty;
                game.data.map[(x, y)].bottom_wall = Wall::Empty;
            }

            if game.data.map[(x + 1, y)].block_move {
                game.data.map[(x, y)].left_wall = Wall::Empty;
            }

            if game.data.map[(x, y + 1)].block_move {
                game.data.map[(x, y)].bottom_wall = Wall::Empty;
            }
        }
    }
}

/// Ensure that diagonal full tile walls do not occur.
fn handle_diagonal_full_tile_walls(game: &mut Game) {
    let (width, height) = game.data.map.size();

    for y in 0..(height - 1) {
        for x in 0..(width - 1) {
            if game.data.map[(x, y)].block_move         && 
               game.data.map[(x + 1, y + 1)].block_move &&
               !game.data.map[(x + 1, y)].block_move    && 
               !game.data.map[(x, y + 1)].block_move {
                   game.data.map[(x + 1, y)] = Tile::wall();
            } else if game.data.map[(x + 1, y)].block_move  && 
                      game.data.map[(x, y + 1)].block_move  &&
                      !game.data.map[(x, y)].block_move &&
                      !game.data.map[(x + 1, y + 1)].block_move {
                   game.data.map[(x, y)] = Tile::wall();
            }
        }
    }
}

fn place_items(game: &mut Game, cmds: &Vec<ProcCmd>) {
    let potential_pos = game.data.get_clear_pos();

    let mut num_items = 0;
    let max_items = cmds.iter().filter_map(|cmd| {
        if let ProcCmd::MaxItems(n) = cmd {
            return Some(n);
        }
        return None;
    }).map(|n| *n).next().unwrap_or(10000);

    for cmd in cmds.iter() {
        if let ProcCmd::Items(typ, min, max) = cmd {
            let num_gen = game.rng.gen_range(min, max + 1);
            for _ in 0..num_gen {
                num_items += 1;
                if num_items >= max_items {
                    return;
                }

                let len = potential_pos.len();

                if len == 0 {
                    break;
                }

                let index = game.rng.gen_range(0, len);
                let pos = potential_pos[index];

                match typ {
                    Item::Dagger => { make_dagger(&mut game.data.entities, &game.config, pos, &mut game.msg_log); },
                    Item::Sword => { make_sword(&mut game.data.entities, &game.config, pos, &mut game.msg_log); },
                    Item::Shield => { make_shield(&mut game.data.entities, &game.config, pos, &mut game.msg_log); },
                    Item::Hammer => { make_hammer(&mut game.data.entities, &game.config, pos, &mut game.msg_log); },
                    _ => {},
                }
            }
        }
    }
}

fn place_triggers(game: &mut Game, cmds: &Vec<ProcCmd>) {
    let potential_pos = game.data.get_clear_pos();

    let mut num_triggers = 0;
    let max_triggers = 2;

    // TODO Should collect locations near walls in a set, and chose a random number of random pos,
    // without replacement
    let mut near_walls = HashSet::new();

    for pos in potential_pos {
        for neighbor in game.data.map.cardinal_neighbors(pos) {
            if game.data.map[neighbor].tile_type == TileType::Wall {
                near_walls.insert(pos);
            }
        }
    }

    let mut num_gates = 0;
    let max_gates = cmds.iter().filter_map(|cmd| {
        if let ProcCmd::MaxGates(n) = cmd {
            return Some(n);
        }
        return None;
    }).map(|n| *n).next().unwrap_or(0);

    let gate_positions = near_walls.iter().map(|p| *p).collect::<Vec<Pos>>();

    // if there are no possible positions, exit early
    if gate_positions.len() == 0 {
        return;
    }

    for _ in 0..max_gates {
        let gate_pos_index = game.rng.gen_range(0, gate_positions.len());
        let gate_pos = gate_positions[gate_pos_index];

        let gate = make_gate_trigger(&mut game.data.entities, &game.config, gate_pos, &mut game.msg_log);
        game.data.entities.gate_pos.insert(gate, Some(gate_pos));

        num_gates += 1;
        if num_gates >= max_gates {
            return;
        }
    }
}

fn place_traps(game: &mut Game, cmds: &Vec<ProcCmd>) {
    let potential_pos = game.data.get_clear_pos();

    let mut num_traps = 0;
    let max_traps = cmds.iter().filter_map(|cmd| {
        if let ProcCmd::MaxTraps(n) = cmd {
            return Some(n);
        }
        return None;
    }).map(|n| *n).next().unwrap_or(10000);

    for cmd in cmds.iter() {
        if let ProcCmd::Traps(typ, min, max) = cmd {
            let num_gen = game.rng.gen_range(min, max + 1);
            for _ in 0..num_gen {
                num_traps += 1;
                if num_traps >= max_traps {
                    return;
                }

                let len = potential_pos.len();

                if len == 0 {
                    return;
                }

                let index = game.rng.gen_range(0, len);
                let pos = potential_pos[index];

                match typ {
                    Trap::Spikes => { make_spike_trap(&mut game.data.entities, &game.config, pos, &mut game.msg_log); },
                    Trap::Sound => { make_sound_trap(&mut game.data.entities, &game.config, pos, &mut game.msg_log); },
                    Trap::Blink => { make_blink_trap(&mut game.data.entities, &game.config, pos, &mut game.msg_log); },
                    Trap::Freeze => { make_freeze_trap(&mut game.data.entities, &game.config, pos, &mut game.msg_log); },
                }
            }
        }
    }
}

fn place_monsters(game: &mut Game, player_pos: Pos, cmds: &Vec<ProcCmd>) {
    // get empty positions, but make sure they are not close to the player
    let mut potential_pos = 
        game.data.get_clear_pos()
                 .iter()
                 .filter(|p| distance(player_pos, **p) > 4)
                 .map(|p| *p)
                 .collect::<Vec<Pos>>();

    for cmd in cmds.iter() {
        if let ProcCmd::Entities(typ, min, max) = cmd {
            let num_gen = game.rng.gen_range(min, max);

            for _ in 0..num_gen {
                let len = potential_pos.len();

                if len == 0 {
                    break;
                }

                let index = game.rng.gen_range(0, len);
                let pos = potential_pos[index];

                let id;
                match typ {
                    EntityName::Gol => { id = Some(make_gol(&mut game.data.entities, &game.config, pos, &mut game.msg_log)); },
                    EntityName::Pawn => { id = Some(make_pawn(&mut game.data.entities, &game.config, pos, &mut game.msg_log)); },
                    EntityName::Spire => { id = Some(make_spire(&mut game.data.entities, &game.config, pos, &mut game.msg_log)); },
                    _ => { id = None; },
                }
                if let Some(id) = id {
                    if game.data.is_in_fov(id, player_pos, &game.config) {
                        game.data.entities.direction[&id] = 
                            game.data.entities.direction[&id].reverse();
                    }
                }

                potential_pos.remove(index);
            }
        }
    }
}

fn place_vaults(game: &mut Game) {
    if game.rng.gen_range(0.0, 1.0) < 0.99 {
        let vault_index = game.rng.gen_range(0, game.vaults.len());
        println!("Placing vault {}", vault_index);

        let (width, height) = game.data.map.size();
        let offset = Pos::new(game.rng.gen_range(0, width), game.rng.gen_range(0, height));

        let vault = &game.vaults[vault_index];
        if offset.x + vault.data.map.size().0  < width &&
           offset.y + vault.data.map.size().1 < height {
            place_vault(&mut game.data, vault, offset);
        }
    }
}

pub fn place_vault(data: &mut GameData, vault: &Vault, offset: Pos) {
    let (width, height) = vault.data.map.size();

    for x in 0..width {
        for y in 0..height {
            let pos = add_pos(offset, Pos::new(x, y));
            data.map[pos] = vault.data.map[(x, y)];
        }
    }

    let mut entities = vault.data.entities.clone();
    for id in vault.data.entities.ids.iter() {
        entities.pos[id] = 
            add_pos(offset, entities.pos[id]);
    }

    data.entities.merge(&entities);
}

fn place_grass(game: &mut Game, num_grass_to_place: usize, disperse: i32) {
    let (width, height) = game.data.map.size();

    let mut potential_grass_pos = Vec::new();
    for x in 0..width {
        for y in 0..height {
            let pos = Pos::new(x, y);

            if !game.data.map[pos].block_move {
                let count = game.data.map.floodfill(pos, 3).len();
                if count > 28 && count < 35 {
                    potential_grass_pos.push(pos);
                }
            }
        }
    }
    potential_grass_pos.shuffle(&mut game.rng);
    let num_grass_to_place = std::cmp::min(num_grass_to_place, potential_grass_pos.len());
    for pos_index in 0..num_grass_to_place {
        let pos = potential_grass_pos[pos_index];
        game.data.map[pos].surface = Surface::Grass;

        for _ in 0..4 {
            let offset_pos = Pos::new(pos.x + game.rng.gen_range(0, disperse),
                                      pos.y + game.rng.gen_range(0, disperse));
            if game.data.map.is_within_bounds(offset_pos) &&
               !game.data.map[offset_pos].block_move {
                game.data.map[offset_pos].surface = Surface::Grass;
            }
        }

    }
}

fn find_available_tile(game: &mut Game) -> Option<Pos> {
    let mut avail_pos = None;

    let (width, height) = game.data.map.size();
    let mut index = 1.0;
    for x in 0..width {
        for y in 0..height {
            let pos = Pos::new(x, y);

            if !game.data.map[pos].block_move && game.data.has_blocking_entity(pos).is_none() {
                if game.rng.gen_range(0.0, 1.0) < (1.0 / index) {
                    avail_pos = Some(pos);
                }

                index += 1.0;
            }
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
                  game.data.map.cardinal_neighbors(pos)
                               .iter()
                               .map(|p| (*p, move_tile_cost(pos, *p, &game.data.map)))
                               .collect::<Vec<(Pos, i32)>>()
              },
              |&pos| blocked_tile_cost(pos, &game.data.map) + distance(start_pos, pos) as i32,
              |&pos| pos == target_pos);

    if let Some((results, _cost)) = path {
        for pos in results {
            if game.data.map[pos].block_move {
                game.data.map[pos] = Tile::empty();
            }
        }
    }
}

fn place_key_and_goal(game: &mut Game, player_pos: Pos) {
    // place goal and key
    let key_pos = find_available_tile(game).unwrap();
    game.data.map[key_pos] = Tile::empty();
    make_key(&mut game.data.entities, &game.config, key_pos, &mut game.msg_log);
    clear_path_to(game, player_pos, key_pos);

    // Find the goal position, ensuring it is not too close to the key
    let mut goal_pos = find_available_tile(game).unwrap();
    while distance(key_pos, goal_pos) < 4 {
        goal_pos = find_available_tile(game).unwrap();
    }

    game.data.map[goal_pos] = Tile::empty();
    make_exit(&mut game.data.entities, &game.config, goal_pos, &mut game.msg_log);
    clear_path_to(game, player_pos, goal_pos);
}

fn clear_island(game: &mut Game, island_radius: i32) {
    fn dist(pos1: Pos, pos2: Pos) -> f32 {
        return (((pos1.x - pos2.x).pow(2) + (pos1.y - pos2.y).pow(2)) as f32).sqrt();
    }

    let (width, height) = game.data.map.size();
    let x_mid = width / 2;
    let y_mid = height / 2;
    let mid_pos = Pos::new(x_mid, y_mid);

    for y in 0..height {
        for x in 0..width {
            let pos = Pos::new(x, y);

            if dist(pos, mid_pos) >= island_radius as f32 {
                game.data.map[pos] = Tile::water();
                game.data.map[pos].chr = MAP_WATER;

                for entity_id in game.data.has_entities(pos).clone() {
                    game.data.remove_entity(entity_id);
                }
            }
        }
    }
}


fn process_block(block: Pos, structure: &mut Structure, map: &Map, seen: &mut HashSet<Pos>) {
    let adjacent = adjacent_blocks(block, map, seen);

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
            process_block(*adj, structure, map, seen);
        }
    }
}

fn adjacent_blocks(block: Pos, map: &Map, seen: &HashSet<Pos>) -> Vec<Pos> {
    let mut result = Vec::new();

    let adjacents = [move_x(block, 1), move_y(block, 1), move_x(block, -1), move_y(block, -1)];
    for adj in adjacents.iter() {
        if map.is_within_bounds(*adj) && map[*adj].block_move && !seen.contains(&adj) {
            result.push(*adj);
        }
    }

    return result;
}

#[test]
fn test_adjacent_blocks() {
    let mut map = Map::from_dims(5, 5);
    let mid = Pos::new(2, 2);
    map[(2, 2)] = Tile::wall();

    map[(1, 2)] = Tile::wall();
    map[(2, 1)] = Tile::wall();
    map[(3, 2)] = Tile::wall();
    map[(2, 3)] = Tile::wall();

    let mut seen = HashSet::new();

    assert_eq!(4, adjacent_blocks(Pos::new(2, 2), &map, &seen).len());
    assert_eq!(2, adjacent_blocks(Pos::new(1, 1), &map, &seen).len());
    assert_eq!(1, adjacent_blocks(Pos::new(2, 1), &map, &seen).len());
    seen.insert(Pos::new(1, 2));
    assert_eq!(3, adjacent_blocks(Pos::new(2, 2), &map, &seen).len());
}

fn find_structures(map: &Map) -> Vec<Structure> {
    let (width, height) = map.size();
    let mut blocks = Vec::new();
    for y in 0..height {
        for x in 0..width {
            if map[(x, y)].block_move {
                blocks.push(Pos::new(x, y));
            }
        }
    }

    let mut structures = Vec::new();
    let mut seen: HashSet<Pos> = HashSet::new();
    for block in blocks {
        if !seen.contains(&block) {
            let mut structure = Structure::new();

            let adjacent = adjacent_blocks(block, &map, &seen);

            if adjacent.len() != 2 {
                structure.add_block(block);
                seen.insert(block);

                if adjacent.len() == 1 {
                    // found start of a structure (line, L, or complex)- process structure
                    structure.typ = StructureType::Line;
                    process_block(block, &mut structure, map, &mut seen);
                } else if adjacent.len() > 2 {
                    // found part of a complex structure- process all peices
                    structure.typ = StructureType::Complex;

                    for adj in adjacent.iter() {
                        seen.insert(*adj);
                    }

                    for adj in adjacent {
                        process_block(adj, &mut structure, map, &mut seen);
                    }
                }

                structures.push(structure);
            }
            // else we are in the middle of a line, so we will pick it up later
        }
    }

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

