use std::fs::File;
use std::io::BufReader;

use rexpaint::*;

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

        MapLoadConfig::FromFile => {
            let maps: Vec<String> = parse_maps_file("resources/maps.txt");

            let map_name = format!("resources/{}", maps[0]);
            let mut position =
                read_map_xp(&game.config, &mut game.data, &mut game.msg_log, &map_name);
            if position == (0, 0) {
                position = (game.data.map.width() / 2, game.data.map.height() / 2);
            }
            player_position = Pos::from(position);
        }

        MapLoadConfig::Random => {
            game.data.map = Map::from_dims(MAP_WIDTH as usize, MAP_HEIGHT as usize);
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

    data.map = Map::from_dims(xp.layers[0].width, xp.layers[0].height);
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
                                data.map[pos].chr = chr as u8;
                                data.map[(x, y - 1)].bottom_wall = Wall::ShortWall;
                            }

                            MAP_THIN_WALL_BOTTOM => {
                                data.map[pos].chr = chr as u8;
                                data.map[pos].bottom_wall = Wall::ShortWall;
                            }

                            MAP_THIN_WALL_LEFT => {
                                data.map[pos].chr = chr as u8;
                                data.map[pos].left_wall = Wall::ShortWall;
                            }

                            MAP_THIN_WALL_RIGHT => {
                                data.map[pos].chr = chr as u8;
                                data.map[(x + 1, y)].left_wall = Wall::ShortWall;
                            }

                            MAP_THIN_WALL_TOP_LEFT => {
                                data.map[pos].chr = chr as u8;
                                data.map[pos].left_wall = Wall::ShortWall;
                                data.map[(x, y - 1)].bottom_wall = Wall::ShortWall;
                            }

                            MAP_THIN_WALL_BOTTOM_LEFT => {
                                data.map[pos].chr = chr as u8;
                                data.map[pos].left_wall = Wall::ShortWall;
                                data.map[pos].bottom_wall = Wall::ShortWall;
                            }

                            MAP_THIN_WALL_TOP_RIGHT => {
                                data.map[pos].chr = chr as u8;
                                data.map[(x, y - 1)].bottom_wall = Wall::ShortWall;
                                data.map[(x - 1, y)].left_wall = Wall::ShortWall;
                            }

                            MAP_THIN_WALL_BOTTOM_RIGHT => {
                                data.map[pos].chr = chr as u8;
                                data.map[pos].bottom_wall = Wall::ShortWall;
                                data.map[(x + 1, y)].left_wall = Wall::ShortWall;
                            }

                            MAP_THICK_WALL_TOP => {
                                data.map[pos].chr = chr as u8;
                                data.map[(x, y - 1)].bottom_wall = Wall::ShortWall;
                            }

                            MAP_THICK_WALL_LEFT => {
                                data.map[pos].chr = chr as u8;
                                data.map[pos].left_wall = Wall::TallWall;
                            }

                            MAP_THICK_WALL_RIGHT => {
                                data.map[pos].chr = chr as u8;
                                data.map[(x + 1, y)].left_wall = Wall::ShortWall;
                            }

                            MAP_THICK_WALL_BOTTOM => {
                                data.map[pos].chr = chr as u8;
                                data.map[pos].bottom_wall = Wall::TallWall;
                            }

                            MAP_THICK_WALL_TOP_LEFT => {
                                data.map[pos].chr = chr as u8;
                                data.map[pos].left_wall = Wall::TallWall;
                                data.map[(x, y - 1)].bottom_wall = Wall::TallWall;
                            }

                            MAP_THICK_WALL_BOTTOM_LEFT => {
                                data.map[pos].chr = chr as u8;
                                data.map[pos].bottom_wall = Wall::TallWall;
                                data.map[pos].left_wall = Wall::TallWall;
                            }

                            MAP_THICK_WALL_TOP_RIGHT => {
                                data.map[pos].chr = chr as u8;
                                data.map[(x, y - 1)].bottom_wall = Wall::TallWall;
                                data.map[(x + 1, y)].left_wall = Wall::TallWall;
                            }

                            MAP_THICK_WALL_BOTTOM_RIGHT => {
                                data.map[pos].chr = chr as u8;
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

                            MAP_DOT_MIDDLE | MAP_ORB => {
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
                                }

                            MAP_WIDE_SPIKES | MAP_TALL_SPIKES => {
                                data.map[pos].chr = chr as u8;
                                data.map[pos].blocked = true;
                            }

                            MAP_WALL => {
                                data.map[pos].chr = chr as u8;
                                data.map[pos].blocked = true;
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

