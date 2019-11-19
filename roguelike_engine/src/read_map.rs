use std::fs::File;
use std::io::BufReader;

use slotmap::dense::*;

use rexpaint::*;

use roguelike_core::constants::*;
use roguelike_core::map::*;
use roguelike_core::types::*;
use roguelike_core::config::*;

use crate::display::*;
use crate::generation::*;


pub fn read_map_xp(config: &Config,
                   display_state: &DisplayState,
                   file_name: &str) -> (ObjMap, Map, (i32, i32)) {
    let file = File::open(file_name).unwrap();
    let mut buf_reader = BufReader::new(file);
    let xp = XpFile::read(&mut buf_reader).unwrap();

    let mut map = Map::from_dims(xp.layers[0].width, xp.layers[0].height);
    let mut objects = DenseSlotMap::new();
    let mut player_position = (0, 0);


    for (layer_index, layer) in xp.layers.iter().enumerate() {
        let width = layer.width;
        let height = layer.height;

        for x in 0..width {
            for y in 0..height {
                let index = y + height * x;
                let cell = layer.cells[index];

                let chr = std::char::from_u32(cell.ch).unwrap();

                match layer_index {
                    MAP_LAYER_GROUND => {
                        match chr as u8 {
                            0 => {
                            }

                            MAP_GROUND => {
                            }

                            MAP_WATER => {
                                map[(x, y)] = Tile::water();
                                map[(x, y)].chr = Some(chr);
                            }

                            _ => {
                                panic!(format!("Unexpected character {} in ground layer!", chr as u8));
                            }
                        }
                    }

                    MAP_LAYER_ENVIRONMENT => {
                        match chr as u8 {
                            MAP_THIN_WALL_TOP => {
                                map[(x, y)].chr = Some(chr);
                            }

                            MAP_THIN_WALL_BOTTOM => {
                                map[(x, y)].chr = Some(chr);
                                map[(x, y)].bottom_wall = Wall::ShortWall;
                            }

                            MAP_THIN_WALL_LEFT => {
                                map[(x, y)].chr = Some(chr);
                                map[(x, y)].left_wall = Wall::ShortWall;
                            }

                            MAP_THIN_WALL_RIGHT => {
                                map[(x, y)].chr = Some(chr);
                            }

                            MAP_THIN_WALL_TOP_LEFT => {
                                map[(x, y)].chr = Some(chr);
                                map[(x, y)].left_wall = Wall::ShortWall;
                            }

                            MAP_THIN_WALL_BOTTOM_LEFT => {
                                map[(x, y)].chr = Some(chr);
                                map[(x, y)].left_wall = Wall::ShortWall;
                                map[(x, y)].bottom_wall = Wall::ShortWall;
                            }

                            MAP_THIN_WALL_TOP_RIGHT => {
                                map[(x, y)].chr = Some(chr);
                            }

                            MAP_THIN_WALL_BOTTOM_RIGHT => {
                                map[(x, y)].chr = Some(chr);
                                map[(x, y)].bottom_wall = Wall::ShortWall;
                            }


                            MAP_THICK_WALL_TOP => {
                                map[(x, y)].chr = Some(chr);
                            }

                            MAP_THICK_WALL_LEFT => {
                                map[(x, y)].chr = Some(chr);
                                map[(x, y)].left_wall = Wall::TallWall;
                            }

                            MAP_THICK_WALL_RIGHT => {
                                map[(x, y)].chr = Some(chr);
                            }

                            MAP_THICK_WALL_BOTTOM => {
                                map[(x, y)].chr = Some(chr);
                                map[(x, y)].bottom_wall = Wall::TallWall;
                            }

                            MAP_THICK_WALL_TOP_LEFT => {
                                map[(x, y)].chr = Some(chr);
                                map[(x, y)].left_wall = Wall::TallWall;
                            }

                            MAP_THICK_WALL_BOTTOM_LEFT => {
                                map[(x, y)].chr = Some(chr);
                                map[(x, y)].bottom_wall = Wall::TallWall;
                                map[(x, y)].left_wall = Wall::TallWall;
                            }

                            MAP_THICK_WALL_TOP_RIGHT => {
                                map[(x, y)].chr = Some(chr);
                            }

                            MAP_THICK_WALL_BOTTOM_RIGHT => {
                                map[(x, y)].chr = Some(chr);
                                map[(x, y)].bottom_wall = Wall::TallWall;
                            }

                            MAP_DOT_TOP_LEFT => {
                                map[(x, y)].chr = Some(chr);
                                map[(x, y)].blocked = true;
                            }

                            MAP_DOT_TOP_RIGHT => {
                                map[(x, y)].chr = Some(chr);
                                map[(x, y)].blocked = true;
                            }

                            MAP_DOT_BOTTOM_LEFT => {
                                map[(x, y)].chr = Some(chr);
                                map[(x, y)].blocked = true;
                            }

                            MAP_DOT_BOTTOM_RIGHT => {
                                map[(x, y)].chr = Some(chr);
                                map[(x, y)].blocked = true;
                            }

                            MAP_ROOK => {
                                map[(x, y)].chr = Some(chr);
                                map[(x, y)].blocked = true;
                            }

                            MAP_DOT_MIDDLE | MAP_ORB => {
                                map[(x, y)].chr = Some(chr);
                                map[(x, y)].blocked = true;
                            }

                            MAP_EMPTY => {
                                map[(x, y)].chr = Some(MAP_EMPTY_CHAR as char);
                            }

                            MAP_STATUE_1 | MAP_STATUE_2 | MAP_STATUE_3 |
                                MAP_STATUE_4 | MAP_STATUE_5 | MAP_STATUE_6 => {
                                    map[(x, y)].chr = Some(chr);
                                    map[(x, y)].blocked = true;
                                }

                            MAP_WIDE_SPIKES| MAP_TALL_SPIKES => {
                                map[(x, y)].chr = Some(chr);
                                map[(x, y)].blocked = true;
                            }

                            MAP_WALL => {
                                map[(x, y)].chr = Some(chr);
                                map[(x, y)].blocked = true;
                            }

                            ENTITY_HERO => {
                                map[(x, y)].chr = Some(chr);
                                map[(x, y)].blocked = true;
                            }

                            ENTITY_CLOAK_GUY => {
                                map[(x, y)].chr = Some(chr);
                                map[(x, y)].blocked = true;
                            }

                            // TODO This should be in entity layer...
                            ENTITY_PLAYER => {
                                map[(x, y)].chr = Some(chr);
                                map[(x, y)].blocked = true;
                            }

                            _ => {
                                panic!(format!("Unexpected character {} in environment layer!", chr as u8));
                            }
                        }
                    }

                    MAP_LAYER_ENTITIES => {
                        match chr as u8 {
                            ENTITY_GOL => {
                                objects.insert(make_gol(config, x as i32, y as i32, display_state));
                            }

                            ENTITY_SWIRL_CIRCLE => {
                                // TODO should be different from elf
                                objects.insert(make_elf(config, x as i32, y as i32, display_state));
                            }

                            ENTITY_ORB => {
                                // TODO should be an objective
                            }

                            ENTITY_ELF => {
                                objects.insert(make_elf(config, x as i32, y as i32, display_state));
                            }

                            MAP_EMPTY => {
                                // Nothing to do here...
                            }

                            ENTITY_PLAYER => {
                                player_position = (x as i32, y as i32);
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

    return (objects, map, player_position);
}

