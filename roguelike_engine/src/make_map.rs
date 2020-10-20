use std::fs::File;
use std::io::BufReader;
use std::str::FromStr;

use rexpaint::*;

use log::trace;

use roguelike_core::constants::*;
use roguelike_core::messaging::*;
use roguelike_core::map::*;
use roguelike_core::types::*;
use roguelike_core::config::*;

use crate::generation::*;
use crate::game::*;
use crate::procgen::*;


#[derive(Copy, Clone, PartialOrd, PartialEq, Debug)]
pub enum VaultTag {
    Medium,
    Rare,
    NoRot,
    NoMirror,
    NoReplace,
}


impl FromStr for VaultTag {
    type Err = String;

    fn from_str(original_str: &str) -> Result<Self, Self::Err> {

        let s: &mut str = &mut original_str.to_string();
        s.make_ascii_lowercase();

        if s == "medium" {
            return Ok(VaultTag::Medium);
        } else if s == "rare" {
            return Ok(VaultTag::Rare);
        } else if s == "norot" {
            return Ok(VaultTag::NoRot);
        } else if s == "nomirror" {
            return Ok(VaultTag::NoMirror);
        } else if s == "noreplace" {
            return Ok(VaultTag::NoReplace);
        }

        return Err(format!("Could not decode vault tag '{}'", original_str));
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Vault {
    pub data: GameData,
    pub tags: Vec<VaultTag>,
}

impl Vault {
    pub fn new(tiles: Vec<Vec<Tile>>, tags: Vec<VaultTag>) -> Vault {
        let map = Map::with_vec(tiles);
        return Vault { data: GameData::new(map, Entities::new()), tags };
    }

    pub fn empty() -> Vault {
        return Vault { data: GameData::empty(0, 0), tags: Vec::new() };
    }
}

/// Read file into a vector of lines
fn parse_map_file(file_name: &str) -> Vec<String> {
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

/// Read Vault file into Vault structure
pub fn parse_vault(file_name: &str, config: &Config) -> Vault {
    let file_contents =
        std::fs::read_to_string(file_name).expect(&format!("Could not read {}", file_name));

    let mut lines = Vec::new();
    let mut tags: Vec<VaultTag> = Vec::new();

    for line in file_contents.lines() {
        let cleaned_line = remove_commas(line.to_string());

        if cleaned_line.starts_with("::") {
            for tag_str in cleaned_line.split_at(2).1.trim().split(" ") {
                if tag_str.starts_with("::") {
                    break;
                }
                tags.push(VaultTag::from_str(tag_str).unwrap());
            }
            break;
        }

        let char_line = cleaned_line.chars().collect::<Vec<char>>();
        lines.push(char_line);
    }

    let vault = parse_ascii_chars(lines, config);

    return vault;
}

fn parse_ascii_chars(lines: Vec<Vec<char>>, config: &Config) -> Vault {
    let height = lines.len() / 2;
    let width = (lines[0].len() - 1) / 2;

    let tile_map = vec![vec![Tile::empty(); height]; width];
    let mut vault = Vault::new(tile_map, Vec::new());

    println!("{}, {}", width, height);
    for y in 0..height {
        for x in 0..width {
            let tile_chr = lines[y * 2][x * 2 + 1];
            let left_wall = lines[y * 2][x * 2];
            let bottom_wall = lines[y * 2 + 1][x * 2 + 1];
            let tile = tile_from_ascii(tile_chr, left_wall, bottom_wall, Pos::new(x as i32, y as i32), &mut vault, config);
            vault.data.map[(x as i32, y as i32)] = tile;
        }
    }

    return vault;
}

fn tile_from_ascii(tile_chr: char, left_wall: char, bottom_wall: char, pos: Pos, vault: &mut Vault, config: &Config) -> Tile {
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
            let mut msg_log = MsgLog::new();
            make_column(&mut vault.data.entities, config, pos, &mut msg_log);
        }

        'p' => {
            tile = Tile::empty();
            let mut msg_log = MsgLog::new();
            make_elf(&mut vault.data.entities, config, pos, &mut msg_log);
        }

        'g' => {
            tile = Tile::empty();
            let mut msg_log = MsgLog::new();
            make_gol(&mut vault.data.entities, config, pos, &mut msg_log);
        }

        'o' => {
            tile = Tile::empty();
            let mut msg_log = MsgLog::new();
            make_stone(&mut vault.data.entities, config, pos, &mut msg_log);
        }

        '*' => {
            tile = Tile::empty();
            // TODO trap
        }

        'S' => {
            tile = Tile::empty();
            // TODO Statue - choose from list of statues
        }

        'v' => {
            tile = Tile::empty();
            let mut msg_log = MsgLog::new();
            make_dagger(&mut vault.data.entities, config, pos, &mut msg_log);
        }

        _ => {
            tile = Tile::empty();
            dbg!(format!("Unexpected char '{}'", tile_chr));
        }
    }

    if left_wall == '|' || left_wall == '\u{c780}' || left_wall as u16 == 8212 {
        tile.left_wall = Wall::ShortWall;
    }

    if bottom_wall == '_' || bottom_wall == '\u{2014}' || bottom_wall as u16 == 124 {
        tile.bottom_wall = Wall::ShortWall;
    }

    return tile;
}

#[test]
fn test_parse_vault() {
    let lines = vec!(vec!('|', '"', ' ', '#'),
                     vec!(' ', ' ', ' ', ' '),
                     vec!(' ', '#', ' ', ' '),
                     vec!(' ', '_', ' ', '_'),
                     vec!(' ', ' ', ' ', 'w'),
                     vec!(' ', ' ', ' ', ' '),
                     );
    let tiles = parse_ascii_chars(lines, &Config::default());

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

pub fn make_map(map_load_config: &MapLoadConfig, game: &mut Game) {
    let player_position: Pos;

    let player_id = game.data.find_player().unwrap();
    game.data.clear_except(vec!(player_id));

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
            let template_file = "resources/wfc_seed_2.png";
            game.data.map = generate_bare_map(20, 20, &template_file, &mut game.rng);
            player_position = saturate_map(game);
        }

        MapLoadConfig::TestVaults => {
            player_position = Pos::new(0, 0);

            let mut max_width = 0;
            let mut max_height = 0;
            for vault in game.vaults.iter() {
                let (width, height) = vault.data.map.size();
                max_width = std::cmp::max(max_width, width);
                max_height = std::cmp::max(max_height, height);
            }
            let square = (game.vaults.len() as f32).sqrt().ceil() as u32;
            let max_dim = std::cmp::max(max_width, max_height);

            game.data.map = Map::from_dims(std::cmp::min(MAP_WIDTH as u32, max_dim as u32 * square as u32),
                                           std::cmp::min(MAP_HEIGHT as u32, max_dim as u32 * square as u32));

            let vaults = game.vaults.clone();
            for (index, vault) in vaults.iter().enumerate() {
                let x_pos = index % square as usize;
                let y_pos = index / square as usize;
                let offset = Pos::new(max_width * x_pos as i32 + 2 * x_pos as i32,
                                      max_height * y_pos as i32 + 2 * y_pos as i32);

                let (width, height) = vault.data.map.size();
                if offset.x + width < MAP_WIDTH && offset.y + height < MAP_HEIGHT {
                    place_vault(&mut game.data, vault, offset);
                }
            }
        }

        MapLoadConfig::VaultFile(file_name) => {
            let vault: Vault = parse_vault(&format!("resources/{}", file_name), &game.config);

            game.data.map = Map::with_vec(vault.data.map.tiles);

            player_position = Pos::new(4, 4);
        }

        MapLoadConfig::FromFile(file_name) => {
            let maps: Vec<String> = parse_map_file(&format!("resources/{}", file_name));

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

    if let None = game.data.find_mouse() {
        make_mouse(&mut game.data.entities, &game.config, &mut game.msg_log);
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

                            MAP_WATER | MAP_WATER_XP => {
                                data.map[pos] = Tile::water();
                                data.map[pos].chr = MAP_WATER;
                            }

                            MAP_RUBBLE => {
                                data.map[pos].surface = Surface::Rubble;
                            }

                            MAP_GRASS => {
                                data.map[pos].surface = Surface::Grass;
                            }

                            _ => {
                                dbg!(format!("Unexpected character {} in ground layer!", chr as u8));
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

    trace!("map updated");

    return player_position;
}

