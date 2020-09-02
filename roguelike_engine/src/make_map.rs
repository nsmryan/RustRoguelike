use std::fs::File;
use std::io::BufReader;
use std::collections::HashSet;

use rand::prelude::*;

use pathfinding::directed::astar::astar;

use rexpaint::*;

use wfc_image::*;
use image;
use image::GenericImageView;

use log::trace;

use roguelike_core::constants::*;
use roguelike_core::messaging::*;
use roguelike_core::map::*;
use roguelike_core::types::*;
use roguelike_core::config::*;
use roguelike_core::utils::*;

use crate::generation::*;
use crate::game::*;


pub fn parse_maps_file(file_name: &str) -> Vec<String> {
    let file_contents =
        std::fs::read_to_string(file_name).expect(&format!("Could not read {}", file_name));
    return file_contents.lines().map(|s| s.to_string()).collect::<Vec<String>>();
}


fn remove_commas(s: String) -> String {
    let s = s.chars().collect::<Vec<char>>();
    let mut new_s = String::new();
    let mut index = 0;
    if s[0] == ',' {
        new_s.push(' ');
    }

    while index < s.len() {
        if s[index] == ',' {
           if index + 1 < s.len() && s[index + 1] == ',' {
                new_s.push(' ');
           }
        } else {
            new_s.push(s[index]);
        }
        index += 1;
    }

    if s[s.len() - 1] == ',' {
        new_s.push(' ');
    }

    return new_s;
}

#[test]
fn test_remove_commas() {
    assert_eq!("   ".to_string(), remove_commas(",,".to_string()));
    assert_eq!("   ".to_string(), remove_commas(", ,".to_string()));
    assert_eq!("   ".to_string(), remove_commas(" , , ".to_string()));
    assert_eq!("  9".to_string(), remove_commas(" , ,9".to_string()));
    assert_eq!("0 9".to_string(), remove_commas("0, ,9".to_string()));
    assert_eq!("% %".to_string(), remove_commas("%,,%".to_string()));
}

pub fn parse_ascii_map(file_name: &str, game: &mut Game) -> Vec<Vec<Tile>> {
    let file_contents =
        std::fs::read_to_string(file_name).expect(&format!("Could not read {}", file_name));

    let lines = file_contents.lines()
                             .map(|l| remove_commas(l.to_string()))
                             .map(|l| l.chars().collect::<Vec<char>>())
                             .collect::<Vec<Vec<char>>>();

    return parse_ascii_chars(lines, game);
}

fn parse_ascii_chars(lines: Vec<Vec<char>>, game: &mut Game) -> Vec<Vec<Tile>> {
    assert!(lines.len() % 2 == 0);
    assert!((lines[0].len() - 1)% 2 == 0);

    let height = lines.len() / 2;
    let width = (lines[0].len() / 2) - 1;

    let mut tile_map = vec![vec![Tile::empty(); height]; width];
    for y in 0..height {
        for x in 0..width {
            let tile_chr = lines[y * 2][x * 2 + 1];
            let left_wall = lines[y * 2][x * 2];
            let bottom_wall = lines[y * 2 + 1][x * 2 + 1];
            let tile = tile_from_ascii(tile_chr, left_wall, bottom_wall, Pos::new(x as i32, y as i32), game);
            tile_map[x][y] = tile;
        }
    }

    return tile_map;
}

fn tile_from_ascii(tile_chr: char, left_wall: char, bottom_wall: char, pos: Pos, game: &mut Game) -> Tile {
    let mut tile;
    match tile_chr {
        ' ' | '\t' | '.' => {
            tile = Tile::empty();
        }

        ';' => {
            tile = Tile::rubble();
        }

        '%' => {
            tile = Tile::water();
        }
        
        '#' => {
            tile = Tile::wall_with(MAP_WALL as char);
        }

        '"' => {
            tile = Tile::grass();
        }

        'I' => {
            tile = Tile::empty();
            make_column(&mut game.data.entities, &game.config, pos, &mut game.msg_log);
        }

        _ => {
            panic!(format!("Unexpected char '{}'", tile_chr));
        }
    }

    if left_wall == '|' {
        tile.left_wall = Wall::ShortWall;
    }

    if bottom_wall == '_' || bottom_wall == '\u{2014}' {
        tile.bottom_wall = Wall::ShortWall;
    }

    return tile;
}

#[test]
fn test_parse_ascii_map() {
    let lines = vec!(vec!('|', '"', ' ', '#'),
                     vec!(' ', ' ', ' ', ' '),
                     vec!(' ', '#', ' ', ' '),
                     vec!(' ', '_', ' ', '_'),
                     vec!(' ', ' ', ' ', 'w'),
                     vec!(' ', ' ', ' ', ' '),
                     );
    let mut game = Game::new(0, Config::default()).unwrap();
    let tiles = parse_ascii_chars(lines, &mut game);

    let mut expected_tiles = vec![vec![Tile::empty(); 2]; 3];
    expected_tiles[0][0].left_wall = Wall::ShortWall;
    expected_tiles[0][0].surface = Surface::Grass;

    expected_tiles[0][1].blocked = true;
    expected_tiles[0][1].block_sight = true;
    expected_tiles[0][1].tile_type = TileType::Wall;

    expected_tiles[1][0].blocked = true;
    expected_tiles[1][0].block_sight = true;
    expected_tiles[1][0].tile_type = TileType::Wall;

    expected_tiles[0][1].chr = MAP_WALL;
    expected_tiles[1][0].chr = MAP_WALL;

    expected_tiles[1][0].bottom_wall = Wall::ShortWall;
    expected_tiles[1][1].bottom_wall = Wall::ShortWall;

    expected_tiles[2][1] = Tile::water();

    for (actual, expected) in tiles.iter().flatten().zip(expected_tiles.iter().flatten()) {
        assert_eq!(expected, actual);
    }
}

pub fn generate_map(width: u32, height: u32, rng: &mut SmallRng) -> Map {
    let mut new_map = Map::from_dims(width, height);

    let mut file = File::open("resources/wfc_seed_2.png").unwrap();
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
    map_image.save("wfc_map.png");

    for x in 0..width {
        for y in 0..height {
            let pixel = map_image.get_pixel(x, y);
            if pixel.0[0] == 0 {
                let pos = Pos::new(x as i32, y as i32);
                new_map[pos].chr = MAP_WALL as u8;
                new_map[pos].blocked = true;
                new_map[pos].block_sight = true;
            }
         }
    }

    return new_map;
}

pub fn handle_diagonal_full_tile_walls(game: &mut Game) {
    let (width, height) = game.data.map.size();

    // ensure that diagonal full tile walls do not occur.
    for y in 0..(height - 1) {
        for x in 0..(width - 1) {
            if game.data.map[(x, y)].blocked         && 
               game.data.map[(x + 1, y + 1)].blocked &&
               !game.data.map[(x + 1, y)].blocked    && 
               !game.data.map[(x, y + 1)].blocked {
                   game.data.map[(x + 1, y)] = Tile::wall();
            } else if game.data.map[(x + 1, y)].blocked  && 
                      game.data.map[(x, y + 1)].blocked  &&
                      !game.data.map[(x, y)].blocked &&
                      !game.data.map[(x + 1, y + 1)].blocked {
                   game.data.map[(x, y)] = Tile::wall();
            }
        }
    }
}

pub fn saturate_map(game: &mut Game) -> Pos {
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

    let mut structures = find_structures(&game.data.map);
    println!("{} singles", structures.iter().filter(|s| s.typ == StructureType::Single).count());
    println!("{} lines", structures.iter().filter(|s| s.typ == StructureType::Line).count());
    println!("{} Ls", structures.iter().filter(|s| s.typ == StructureType::Path).count());
    println!("{} complex", structures.iter().filter(|s| s.typ == StructureType::Complex).count());

    let (width, height) = game.data.map.size();

    let mut to_remove: Vec<usize> = Vec::new();
    for (index, structure) in structures.iter().enumerate() {
        if structure.typ == StructureType::Single {
            if game.rng.gen_range(0.0, 1.0) > 0.1 {
                make_column(&mut game.data.entities, &game.config, structure.blocks[0], &mut game.msg_log);
                to_remove.push(index);
            }
        } else if structure.typ == StructureType::Line { 
            if structure.blocks.len() > 5 {
                let index = game.rng.gen_range(0, structure.blocks.len());
                let block = structure.blocks[index];
                game.data.map[block] = Tile::empty();
                game.data.map[block].surface = Surface::Rubble;
            }
        }
    }

    to_remove.sort();
    to_remove.reverse();
    for index in to_remove.iter() {
        for block in structures[*index].blocks.iter() {
            game.data.map[*block] = Tile::empty();
        }
        structures.swap_remove(*index);
    }

    // place goal and key
    let key_x = game.rng.gen_range(0, width);
    let key_y = game.rng.gen_range(0, height);
    let key_pos = Pos::new(key_x, key_y);
    game.data.map[key_pos] = Tile::empty();
    make_key(&mut game.data.entities, &game.config, key_pos, &mut game.msg_log);

    let goal_x = game.rng.gen_range(0, width);
    let goal_y = game.rng.gen_range(0, height);
    let goal_pos = Pos::new(goal_x, goal_y);
    game.data.map[goal_pos] = Tile::empty();
    make_exit(&mut game.data.entities, &game.config, goal_pos, &mut game.msg_log);

    fn blocked_tile_cost(pos: Pos, map: &Map) -> i32 {
        if map[pos].blocked {
            return 15;
        } 

        return 0;
    }

    let player_pos = Pos::new(0, 0);

    // clear a path to the key
    let key_path = 
        astar(&player_pos,
              |&pos| game.data.map.neighbors(pos).iter().map(|p| (*p, 1)).collect::<Vec<(Pos, i32)>>(),
              |&pos| blocked_tile_cost(pos, &game.data.map) + distance(player_pos, pos) as i32,
              |&pos| pos == key_pos);

    if let Some((results, _cost)) = key_path {
        for pos in results {
            if game.data.map[pos].blocked {
                game.data.map[pos] = Tile::empty();
            }
        }
    }

    // clear a path to the goal
    let goal_path = 
        astar(&player_pos,
              |&pos| game.data.map.neighbors(pos).iter().map(|p| (*p, 1)).collect::<Vec<(Pos, i32)>>(),
              |&pos| blocked_tile_cost(pos, &game.data.map) + distance(player_pos, pos) as i32,
              |&pos| pos == goal_pos);

    if let Some((results, _cost)) = goal_path {
        for pos in results {
            if game.data.map[pos].blocked {
                game.data.map[pos] = Tile::empty();
            }
        }
    }

    return player_pos;
}

#[derive(Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Debug)]
pub enum StructureType {
    Single,
    Line,
    Path,
    Complex,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Structure {
    blocks: Vec<Pos>,
    typ: StructureType,
}

impl Structure {
    pub fn new() -> Structure {
        return Structure { blocks: Vec::new(), typ: StructureType::Single };
    }

    pub fn add_block(&mut self, block: Pos) {
        self.blocks.push(block);
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
        if map.is_within_bounds(*adj) && map[*adj].blocked && !seen.contains(&adj) {
            result.push(*adj);
        }
    }

    return result;
}

#[test]
fn test_adjacent_blocks() {
    let mut map = Map::from_dims(5, 5);
    let mid = Pos::new(2, 2);
    map[(2, 2)].blocked = true;

    map[(1, 2)].blocked = true;
    map[(2, 1)].blocked = true;
    map[(3, 2)].blocked = true;
    map[(2, 3)].blocked = true;

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
            if map[(x, y)].blocked {
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
    map[(0, 2)].blocked = true;
    map[(1, 2)].blocked = true;
    map[(2, 2)].blocked = true;
    let structures = find_structures(&map);
    assert_eq!(1, structures.len());
    assert_eq!(StructureType::Line, structures[0].typ);
    assert_eq!(3, structures[0].blocks.len());

    // add a lone block and check that it is found along with the line
    map[(0, 0)].blocked = true;
    let structures = find_structures(&map);
    assert_eq!(2, structures.len());
    assert!(structures.iter().find(|s| s.typ == StructureType::Single).is_some());
    assert!(structures.iter().find(|s| s.typ == StructureType::Line).is_some());

    // add a vertical line and check that all structures are found
    map[(4, 0)].blocked = true;
    map[(4, 1)].blocked = true;
    map[(4, 2)].blocked = true;
    map[(4, 3)].blocked = true;
    let structures = find_structures(&map);
    assert_eq!(3, structures.len());
    assert!(structures.iter().find(|s| s.typ == StructureType::Single).is_some());
    assert!(structures.iter().filter(|s| s.typ == StructureType::Line).count() == 2);
}

#[test]
fn test_find_complex_structures() {
    let mut map = Map::from_dims(5, 5);

    // lay down an L
    map[(0, 2)].blocked = true;
    map[(1, 2)].blocked = true;
    map[(2, 2)].blocked = true;
    map[(2, 3)].blocked = true;
    let structures = find_structures(&map);
    assert_eq!(1, structures.len());
    assert_eq!(StructureType::Path, structures[0].typ);
    assert_eq!(4, structures[0].blocks.len());

    // turn it into a 'complex' structure and check that it is discovered
    map[(2, 1)].blocked = true;
    let structures = find_structures(&map);
    assert_eq!(1, structures.len());
    assert_eq!(StructureType::Complex, structures[0].typ);
    assert_eq!(5, structures[0].blocks.len());
}

pub fn make_map(map_load_config: &MapLoadConfig, game: &mut Game) {
    let player_position: Pos;

    match map_load_config {
        MapLoadConfig::TestMap => {
            game.data.map = Map::from_dims(11, 12);
            make_test_map(game);
            player_position = Pos::new(0, 0);
        }

        MapLoadConfig::Empty => {
            let new_map = Map::from_dims(10, 10);
            game.data.map = new_map;
            player_position = Pos::new(0, 0);
        }

        MapLoadConfig::TestRandom => {
            game.data.map = generate_map(20, 20, &mut game.rng);
            player_position = saturate_map(game);
        }

        MapLoadConfig::FromAsciiMap(file_name) => {
            let tiles: Vec<Vec<Tile>> = parse_ascii_map(&format!("resources/{}", file_name), game);

            game.data.map = Map::with_vec(tiles);

            player_position = Pos::new(4, 4);
        }

        MapLoadConfig::FromFile(file_name) => {
            let maps: Vec<String> = parse_maps_file(&format!("resources/{}", file_name));

            if game.settings.level_num >= maps.len() {
                panic!(format!("Map index {} too large ({} available", game.settings.level_num, maps.len()));
            }

            let map_name = format!("resources/{}", maps[game.settings.level_num]);
            let mut position =
                read_map_xp(&game.config, &mut game.data, &mut game.msg_log, &map_name);
            if position == (0, 0) {
                position = (game.data.map.width() / 2, game.data.map.height() / 2);
            }
            player_position = Pos::from(position);
        }

        MapLoadConfig::Random => {
            game.data.map = Map::from_dims(MAP_WIDTH as u32, MAP_HEIGHT as u32);
            let starting_position = make_island(&mut game.data, &game.config, &mut game.msg_log, &mut game.rng);
            player_position = Pos::from(starting_position);
        }

        MapLoadConfig::TestWall => {
            let (new_map, position) = make_wall_test_map(&mut game.data.entities, &game.config, &mut game.msg_log);
            game.data.map = new_map;
            player_position = Pos::from(position);
        }

        MapLoadConfig::TestPlayer => {
            let (new_map, position) = make_player_test_map(&mut game.data.entities, &game.config, &mut game.msg_log);
            game.data.map = new_map;
            player_position = Pos::from(position);
        }

        MapLoadConfig::TestCorner => {
            let (new_map, position) = make_corner_test_map(&mut game.data.entities, &game.config, &mut game.msg_log);
            game.data.map = new_map;
            player_position = Pos::from(position);
        }
    }

    let player_id = game.data.find_player().unwrap();
    game.data.entities.pos[&player_id] = player_position;
}

pub fn read_map_xp(config: &Config,
                   data: &mut GameData,
                   msg_log: &mut MsgLog,
                   file_name: &str) -> (i32, i32) {
    trace!("opening map {}", file_name);
    let file = File::open(file_name).unwrap();

    let mut buf_reader = BufReader::new(file);

    trace!("reading in map data");
    let xp = XpFile::read(&mut buf_reader).unwrap();

    data.map = Map::from_dims(xp.layers[0].width as u32, xp.layers[0].height as u32);
    let mut player_position = (0, 0);

    for (layer_index, layer) in xp.layers.iter().enumerate() {
        let width = layer.width as i32;
        let height = layer.height as i32;

        for x in 0..width {
            for y in 0..height {
                let index = y + height * x;
                let cell = layer.cells[index as usize];

                let pos = Pos::new(x, y);

                let chr = std::char::from_u32(cell.ch).unwrap();

                match layer_index {
                    MAP_LAYER_GROUND => {
                        match chr as u8 {
                            0 => {
                            }

                            MAP_GROUND => {
                            }

                            MAP_WATER => {
                                data.map[pos] = Tile::water();
                                data.map[pos].chr = chr as u8;
                            }

                            MAP_RUBBLE => {
                                data.map[pos].surface = Surface::Rubble;
                            }

                            MAP_GRASS => {
                                data.map[pos].surface = Surface::Grass;
                            }

                            _ => {
                                panic!(format!("Unexpected character {} in ground layer!", chr as u8));
                            }
                        }
                    }

                    MAP_LAYER_ENVIRONMENT => {
                        match chr as u8 {
                            MAP_COLUMN => {
                                make_column(&mut data.entities, config, pos, msg_log);
                            }

                            MAP_THIN_WALL_TOP => {
                                data.map[pos].chr = 0;
                                data.map[(x, y - 1)].bottom_wall = Wall::ShortWall;
                            }

                            MAP_THIN_WALL_BOTTOM => {
                                data.map[pos].chr = 0; 
                                data.map[pos].bottom_wall = Wall::ShortWall;
                            }

                            MAP_THIN_WALL_LEFT => {
                                data.map[pos].chr = 0; 
                                data.map[pos].left_wall = Wall::ShortWall;
                            }

                            MAP_THIN_WALL_RIGHT => {
                                data.map[pos].chr = 0; 
                                data.map[(x + 1, y)].left_wall = Wall::ShortWall;
                            }

                            MAP_THIN_WALL_TOP_LEFT => {
                                data.map[pos].chr = 0; 
                                data.map[pos].left_wall = Wall::ShortWall;
                                data.map[(x, y - 1)].bottom_wall = Wall::ShortWall;
                            }

                            MAP_THIN_WALL_BOTTOM_LEFT => {
                                data.map[pos].chr = 0; 
                                data.map[pos].left_wall = Wall::ShortWall;
                                data.map[pos].bottom_wall = Wall::ShortWall;
                            }

                            MAP_THIN_WALL_TOP_RIGHT => {
                                data.map[pos].chr = 0; 
                                data.map[(x, y - 1)].bottom_wall = Wall::ShortWall;
                                data.map[(x - 1, y)].left_wall = Wall::ShortWall;
                            }

                            MAP_THIN_WALL_BOTTOM_RIGHT => {
                                data.map[pos].chr = 0; 
                                data.map[pos].bottom_wall = Wall::ShortWall;
                                data.map[(x + 1, y)].left_wall = Wall::ShortWall;
                            }

                            MAP_THICK_WALL_TOP => {
                                data.map[pos].chr = 0; 
                                data.map[(x, y - 1)].bottom_wall = Wall::ShortWall;
                            }

                            MAP_THICK_WALL_LEFT => {
                                data.map[pos].chr = 0; 
                                data.map[pos].left_wall = Wall::TallWall;
                            }

                            MAP_THICK_WALL_RIGHT => {
                                data.map[pos].chr = 0; 
                                data.map[(x + 1, y)].left_wall = Wall::ShortWall;
                            }

                            MAP_THICK_WALL_BOTTOM => {
                                data.map[pos].chr = 0; 
                                data.map[pos].bottom_wall = Wall::TallWall;
                            }

                            MAP_THICK_WALL_TOP_LEFT => {
                                data.map[pos].chr = 0; 
                                data.map[pos].left_wall = Wall::TallWall;
                                data.map[(x, y - 1)].bottom_wall = Wall::TallWall;
                            }

                            MAP_THICK_WALL_BOTTOM_LEFT => {
                                data.map[pos].chr = 0; 
                                data.map[pos].bottom_wall = Wall::TallWall;
                                data.map[pos].left_wall = Wall::TallWall;
                            }

                            MAP_THICK_WALL_TOP_RIGHT => {
                                data.map[pos].chr = 0; 
                                data.map[(x, y - 1)].bottom_wall = Wall::TallWall;
                                data.map[(x + 1, y)].left_wall = Wall::TallWall;
                            }

                            MAP_THICK_WALL_BOTTOM_RIGHT => {
                                data.map[pos].chr = 0; 
                                data.map[pos].bottom_wall = Wall::TallWall;
                                data.map[(x + 1, y)].left_wall = Wall::TallWall;
                            }

                            MAP_DOT_TOP_LEFT => {
                                data.map[pos].chr = chr as u8;
                                data.map[pos].blocked = true;
                            }

                            MAP_DOT_TOP_RIGHT => {
                                data.map[pos].chr = chr as u8;
                                data.map[pos].blocked = true;
                            }

                            MAP_DOT_BOTTOM_LEFT => {
                                data.map[pos].chr = chr as u8;
                                data.map[pos].blocked = true;
                            }

                            MAP_DOT_BOTTOM_RIGHT => {
                                data.map[pos].chr = chr as u8;
                                data.map[pos].blocked = true;
                            }

                            MAP_ROOK => {
                                data.map[pos].chr = chr as u8;
                                data.map[pos].blocked = true;
                            }

                            MAP_ORB => {
                                data.map[pos].chr = chr as u8;
                                data.map[pos].blocked = true;
                            }

                            MAP_EMPTY => {
                                data.map[pos].chr = MAP_EMPTY_CHAR;
                            }

                            MAP_STATUE_1 | MAP_STATUE_2 | MAP_STATUE_3 |
                                MAP_STATUE_4 | MAP_STATUE_5 | MAP_STATUE_6 => {
                                    data.map[pos].chr = chr as u8;
                                    data.map[pos].blocked = true;
                                    data.map[pos].block_sight = true;
                                }

                            MAP_WIDE_SPIKES | MAP_TALL_SPIKES => {
                                data.map[pos].chr = chr as u8;
                                data.map[pos].blocked = true;
                                    data.map[pos].block_sight = true;
                            }

                            MAP_WALL => {
                                data.map[pos].chr = chr as u8;
                                data.map[pos].blocked = true;
                                data.map[pos].block_sight = true;
                            }

                            ENTITY_CLOAK_GUY => {
                                data.map[pos].chr = chr as u8;
                                data.map[pos].blocked = true;
                            }

                            _ => {
                                data.map[pos].chr = chr as u8;
                                data.map[pos].blocked = true;
                            }
                        }
                    }

                    MAP_LAYER_ENTITIES => {
                        match chr as u8 {
                            0 => {
                            }

                            ENTITY_PLAYER => {
                                player_position = (x as i32, y as i32);
                            }

                            ENTITY_GOL => {
                                make_gol(&mut data.entities, config, pos, msg_log);
                            }

                            ENTITY_EXIT => {
                                make_exit(&mut data.entities, config, pos, msg_log);
                            }

                            ENTITY_ELF => {
                                make_elf(&mut data.entities, config, pos, msg_log);
                            }

                            MAP_EMPTY => {
                                // Nothing to do here...
                            }

                            ENTITY_DAGGER => {
                                make_dagger(&mut data.entities, config, pos, msg_log);
                            }

                            ENTITY_KEY => {
                                make_key(&mut data.entities, config, pos, msg_log);
                            }

                            ENTITY_STONE => {
                                make_stone(&mut data.entities, config, pos, msg_log);
                            }

                            ENTITY_SHIELD => {
                                make_shield(&mut data.entities, config, Pos::new(x, y), msg_log);
                            }

                            ENTITY_HAMMER => {
                                make_hammer(&mut data.entities, config, Pos::new(x, y), msg_log);
                            }
 
                            ENTITY_SPIKE_TRAP => {
                                make_spikes(&mut data.entities, config, pos, msg_log);
                            }

                            _ => {
                                panic!(format!("Unexpected character {} in entities layer!", chr as u8));
                            }
                        }
                    }

                    _ => {
                        panic!(format!("Layer {} not expected in map file!", layer_index));
                    }
                }
            }
        }
    }

    trace!("map read finished");

    data.map.update_map();
 
    trace!("map updated");

    return player_position;
}

