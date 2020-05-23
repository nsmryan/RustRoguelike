use std::fs::File;
use std::io::BufReader;

use rexpaint::*;

use roguelike_core::constants::*;
use roguelike_core::messaging::*;
use roguelike_core::map::*;
use roguelike_core::types::*;
use roguelike_core::config::*;

use crate::generation::*;


pub fn read_map_xp(config: &Config,
                   entities: &mut Entities,
                   msg_log: &mut MsgLog,
                   file_name: &str) -> (Map, (i32, i32)) {
    let file = File::open(file_name).unwrap();
    let mut buf_reader = BufReader::new(file);
    let xp = XpFile::read(&mut buf_reader).unwrap();

    let mut map = Map::from_dims(xp.layers[0].width, xp.layers[0].height);
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
                                map[pos] = Tile::water();
                                map[pos].chr = chr;
                            }

                            MAP_RUBBLE => {
                                map[pos].surface = Surface::Rubble;
                            }

                            MAP_GRASS => {
                                map[pos].surface = Surface::Grass;
                            }

                            _ => {
                                panic!(format!("Unexpected character {} in ground layer!", chr as u8));
                            }
                        }
                    }

                    MAP_LAYER_ENVIRONMENT => {
                        match chr as u8 {
                            MAP_COLUMN => {
                                make_column(entities, config, pos);
                            }

                            MAP_THIN_WALL_TOP => {
                                map[pos].chr = chr;
                                map[(x, y - 1)].bottom_wall = Wall::ShortWall;
                            }

                            MAP_THIN_WALL_BOTTOM => {
                                map[pos].chr = chr;
                                map[pos].bottom_wall = Wall::ShortWall;
                            }

                            MAP_THIN_WALL_LEFT => {
                                map[pos].chr = chr;
                                map[pos].left_wall = Wall::ShortWall;
                            }

                            MAP_THIN_WALL_RIGHT => {
                                map[pos].chr = chr;
                                map[(x + 1, y)].left_wall = Wall::ShortWall;
                            }

                            MAP_THIN_WALL_TOP_LEFT => {
                                map[pos].chr = chr;
                                map[pos].left_wall = Wall::ShortWall;
                                map[(x, y - 1)].bottom_wall = Wall::ShortWall;
                            }

                            MAP_THIN_WALL_BOTTOM_LEFT => {
                                map[pos].chr = chr;
                                map[pos].left_wall = Wall::ShortWall;
                                map[pos].bottom_wall = Wall::ShortWall;
                            }

                            MAP_THIN_WALL_TOP_RIGHT => {
                                map[pos].chr = chr;
                                map[(x, y - 1)].bottom_wall = Wall::ShortWall;
                                map[(x - 1, y)].left_wall = Wall::ShortWall;
                            }

                            MAP_THIN_WALL_BOTTOM_RIGHT => {
                                map[pos].chr = chr;
                                map[pos].bottom_wall = Wall::ShortWall;
                                map[(x + 1, y)].left_wall = Wall::ShortWall;
                            }

                            MAP_THICK_WALL_TOP => {
                                map[pos].chr = chr;
                                map[(x, y - 1)].bottom_wall = Wall::ShortWall;
                            }

                            MAP_THICK_WALL_LEFT => {
                                map[pos].chr = chr;
                                map[pos].left_wall = Wall::TallWall;
                            }

                            MAP_THICK_WALL_RIGHT => {
                                map[pos].chr = chr;
                                map[(x + 1, y)].left_wall = Wall::ShortWall;
                            }

                            MAP_THICK_WALL_BOTTOM => {
                                map[pos].chr = chr;
                                map[pos].bottom_wall = Wall::TallWall;
                            }

                            MAP_THICK_WALL_TOP_LEFT => {
                                map[pos].chr = chr;
                                map[pos].left_wall = Wall::TallWall;
                                map[(x, y - 1)].bottom_wall = Wall::TallWall;
                            }

                            MAP_THICK_WALL_BOTTOM_LEFT => {
                                map[pos].chr = chr;
                                map[pos].bottom_wall = Wall::TallWall;
                                map[pos].left_wall = Wall::TallWall;
                            }

                            MAP_THICK_WALL_TOP_RIGHT => {
                                map[pos].chr = chr;
                                map[(x, y - 1)].bottom_wall = Wall::TallWall;
                                map[(x + 1, y)].left_wall = Wall::TallWall;
                            }

                            MAP_THICK_WALL_BOTTOM_RIGHT => {
                                map[pos].chr = chr;
                                map[pos].bottom_wall = Wall::TallWall;
                                map[(x + 1, y)].left_wall = Wall::TallWall;
                            }

                            MAP_DOT_TOP_LEFT => {
                                map[pos].chr = chr;
                                map[pos].blocked = true;
                            }

                            MAP_DOT_TOP_RIGHT => {
                                map[pos].chr = chr;
                                map[pos].blocked = true;
                            }

                            MAP_DOT_BOTTOM_LEFT => {
                                map[pos].chr = chr;
                                map[pos].blocked = true;
                            }

                            MAP_DOT_BOTTOM_RIGHT => {
                                map[pos].chr = chr;
                                map[pos].blocked = true;
                            }

                            MAP_ROOK => {
                                map[pos].chr = chr;
                                map[pos].blocked = true;
                            }

                            MAP_DOT_MIDDLE | MAP_ORB => {
                                map[pos].chr = chr;
                                map[pos].blocked = true;
                            }

                            MAP_EMPTY => {
                                map[pos].chr = MAP_EMPTY_CHAR as char;
                            }

                            MAP_STATUE_1 | MAP_STATUE_2 | MAP_STATUE_3 |
                                MAP_STATUE_4 | MAP_STATUE_5 | MAP_STATUE_6 => {
                                    map[pos].chr = chr;
                                    map[pos].blocked = true;
                                }

                            MAP_WIDE_SPIKES| MAP_TALL_SPIKES => {
                                map[pos].chr = chr;
                                map[pos].blocked = true;
                            }

                            MAP_WALL => {
                                map[pos].chr = chr;
                                map[pos].blocked = true;
                            }

                            ENTITY_CLOAK_GUY => {
                                map[pos].chr = chr;
                                map[pos].blocked = true;
                            }

                            _ => {
                                map[pos].chr = chr;
                                map[pos].blocked = true;
                            }
                        }
                    }

                    MAP_LAYER_ENTITIES => {
                        match chr as u8 {
                            ENTITY_PLAYER => {
                                player_position = (x as i32, y as i32);
                            }

                            ENTITY_GOL => {
                                make_gol(entities, config, pos, msg_log);
                            }

                            ENTITY_EXIT => {
                                make_exit(entities, config, pos);
                            }

                            ENTITY_ELF => {
                                make_elf(entities, config, pos, msg_log);
                            }

                            MAP_EMPTY => {
                                // Nothing to do here...
                            }

                            ENTITY_DAGGER => {
                                make_dagger(entities, config, pos);
                            }

                            ENTITY_KEY => {
                                make_key(entities, config, pos, msg_log);
                            }

                            ENTITY_STONE => {
                                make_stone(entities, config, pos);
                            }

                            ENTITY_SHIELD => {
                                make_shield(entities, config, Pos::new(x, y));
                            }

                            ENTITY_SPIKE_TRAP => {
                                make_spikes(entities, config, pos, msg_log);
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

    map.update_map();

    return (map, player_position);
}

