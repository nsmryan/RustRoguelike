use std::fs::File;
use std::io::BufReader;

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

use crate::generation::*;
use crate::game::*;


pub fn parse_maps_file(file_name: &str) -> Vec<String> {
    let file_contents =
        std::fs::read_to_string(file_name).expect(&format!("Could not read {}", file_name));
    return file_contents.lines().map(|s| s.to_string()).collect::<Vec<String>>();
}

pub fn parse_ascii_map(file_name: &str) -> Vec<Vec<Tile>> {
    let file_contents =
        std::fs::read_to_string(file_name).expect(&format!("Could not read {}", file_name));

    let lines = file_contents.lines()
                             .map(|s| s.chars().collect::<Vec<char>>())
                             .collect::<Vec<Vec<char>>>();

    return parse_ascii_chars(lines);
}

fn parse_ascii_chars(lines: Vec<Vec<char>>) -> Vec<Vec<Tile>> {
    assert!(lines.len() % 2 == 0);
    assert!(lines[0].len() % 2 == 0);

    let height = lines.len() / 2;
    let width = lines[0].len() / 2;

    let mut tile_map = Vec::new();
    for y in 0..height {
        let mut tiles = Vec::new();
        for x in 0..width {
            let tile = tile_from_ascii(lines[y * 2][x * 2 + 1], lines[y * 2][x * 2], lines[y * 2 + 1][x * 2 + 1]);
            tiles.push(tile);
        }
        tile_map.push(tiles);
    }

    return tile_map;
}

fn tile_from_ascii(tile_chr: char, left_wall: char, bottom_wall: char) -> Tile {
    let mut tile;
    match tile_chr {
        ' ' => {
            tile = Tile::empty();
        }

        ';' => {
            tile = Tile::rubble();
        }

        'w' => {
            tile = Tile::water();
        }
        
        '#' => {
            tile = Tile::wall_with(MAP_WALL as char);
        }

        '"' => {
            tile = Tile::grass();
        }

        _ => {
            panic!(format!("Unexpected char '{}'", tile_chr));
        }
    }

    if left_wall == '|' {
        tile.left_wall = Wall::ShortWall;
    }

    if bottom_wall == '_' {
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
    let tiles = parse_ascii_chars(lines);

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
            let width: u32 = 30;
            let height: u32 = 30;
            let new_map = Map::from_dims(width, height);
            game.data.map = new_map;
            player_position = Pos::new(0, 0);

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
            trace!("wfc start");
            let map_image = 
                wfc_image::generate_image_with_rng(&seed_image,
                                                   core::num::NonZeroU32::new(3).unwrap(),
                                                   wfc_image::Size::new(width, height),
                                                   &orientations, 
                                                   wfc_image::wrap::WrapNone,
                                                   ForbidNothing,
                                                   wfc_image::retry::NumTimes(3),
                                                   &mut game.rng).unwrap();
            trace!("wfc end");
            map_image.save("wfc_map.png");

            for x in 0..width {
                for y in 0..height {
                    let pixel = map_image.get_pixel(x, y);
                    if pixel.0[0] == 0 {
                        let pos = Pos::new(x as i32, y as i32);
                        game.data.map[pos].chr = MAP_WALL as u8;
                        game.data.map[pos].blocked = true;
                        game.data.map[pos].block_sight = true;
                   }
                }
            }
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

