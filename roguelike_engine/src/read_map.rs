use std::fs::File;
use std::io::BufReader;

use slotmap::dense::*;

use rexpaint::*;

use roguelike_core::constants::*;
use roguelike_core::messaging::*;
use roguelike_core::map::*;
use roguelike_core::types::*;
use roguelike_core::config::*;

use crate::display::*;
use crate::generation::*;


pub fn read_map_xp(config: &Config,
                   msg_log: &mut MsgLog,
                   file_name: &str) -> (ObjMap, Map, (i32, i32)) {
    let file = File::open(file_name).unwrap();
    let mut buf_reader = BufReader::new(file);
    let xp = XpFile::read(&mut buf_reader).unwrap();

    let mut map = Map::from_dims(xp.layers[0].width, xp.layers[0].height);
    let mut objects = DenseSlotMap::new();
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
                                map[pos].chr = Some(chr);
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
                                let col = make_column(config, pos);
                                objects.insert(col);
                            }

                            MAP_THIN_WALL_TOP => {
                                map[pos].chr = Some(chr);
                                map[(x, y - 1)].bottom_wall = Wall::ShortWall;
                            }

                            MAP_THIN_WALL_BOTTOM => {
                                map[pos].chr = Some(chr);
                                map[pos].bottom_wall = Wall::ShortWall;
                            }

                            MAP_THIN_WALL_LEFT => {
                                map[pos].chr = Some(chr);
                                map[pos].left_wall = Wall::ShortWall;
                            }

                            MAP_THIN_WALL_RIGHT => {
                                map[pos].chr = Some(chr);
                                map[(x + 1, y)].left_wall = Wall::ShortWall;
                            }

                            MAP_THIN_WALL_TOP_LEFT => {
                                map[pos].chr = Some(chr);
                                map[pos].left_wall = Wall::ShortWall;
                                map[(x, y - 1)].bottom_wall = Wall::ShortWall;
                            }

                            MAP_THIN_WALL_BOTTOM_LEFT => {
                                map[pos].chr = Some(chr);
                                map[pos].left_wall = Wall::ShortWall;
                                map[pos].bottom_wall = Wall::ShortWall;
                            }

                            MAP_THIN_WALL_TOP_RIGHT => {
                                map[pos].chr = Some(chr);
                                map[(x, y - 1)].bottom_wall = Wall::ShortWall;
                                map[(x - 1, y)].left_wall = Wall::ShortWall;
                            }

                            MAP_THIN_WALL_BOTTOM_RIGHT => {
                                map[pos].chr = Some(chr);
                                map[pos].bottom_wall = Wall::ShortWall;
                                map[(x + 1, y)].left_wall = Wall::ShortWall;
                            }

                            MAP_THICK_WALL_TOP => {
                                map[pos].chr = Some(chr);
                                map[(x, y - 1)].bottom_wall = Wall::ShortWall;
                            }

                            MAP_THICK_WALL_LEFT => {
                                map[pos].chr = Some(chr);
                                map[pos].left_wall = Wall::TallWall;
                            }

                            MAP_THICK_WALL_RIGHT => {
                                map[pos].chr = Some(chr);
                                map[(x + 1, y)].left_wall = Wall::ShortWall;
                            }

                            MAP_THICK_WALL_BOTTOM => {
                                map[pos].chr = Some(chr);
                                map[pos].bottom_wall = Wall::TallWall;
                            }

                            MAP_THICK_WALL_TOP_LEFT => {
                                map[pos].chr = Some(chr);
                                map[pos].left_wall = Wall::TallWall;
                                map[(x, y - 1)].bottom_wall = Wall::TallWall;
                            }

                            MAP_THICK_WALL_BOTTOM_LEFT => {
                                map[pos].chr = Some(chr);
                                map[pos].bottom_wall = Wall::TallWall;
                                map[pos].left_wall = Wall::TallWall;
                            }

                            MAP_THICK_WALL_TOP_RIGHT => {
                                map[pos].chr = Some(chr);
                                map[(x, y - 1)].bottom_wall = Wall::TallWall;
                                map[(x + 1, y)].left_wall = Wall::TallWall;
                            }

                            MAP_THICK_WALL_BOTTOM_RIGHT => {
                                map[pos].chr = Some(chr);
                                map[pos].bottom_wall = Wall::TallWall;
                                map[(x + 1, y)].left_wall = Wall::TallWall;
                            }

                            MAP_DOT_TOP_LEFT => {
                                map[pos].chr = Some(chr);
                                map[pos].blocked = true;
                            }

                            MAP_DOT_TOP_RIGHT => {
                                map[pos].chr = Some(chr);
                                map[pos].blocked = true;
                            }

                            MAP_DOT_BOTTOM_LEFT => {
                                map[pos].chr = Some(chr);
                                map[pos].blocked = true;
                            }

                            MAP_DOT_BOTTOM_RIGHT => {
                                map[pos].chr = Some(chr);
                                map[pos].blocked = true;
                            }

                            MAP_ROOK => {
                                map[pos].chr = Some(chr);
                                map[pos].blocked = true;
                            }

                            MAP_DOT_MIDDLE | MAP_ORB => {
                                map[pos].chr = Some(chr);
                                map[pos].blocked = true;
                            }

                            MAP_EMPTY => {
                                map[pos].chr = Some(MAP_EMPTY_CHAR as char);
                            }

                            MAP_STATUE_1 | MAP_STATUE_2 | MAP_STATUE_3 |
                                MAP_STATUE_4 | MAP_STATUE_5 | MAP_STATUE_6 => {
                                    map[pos].chr = Some(chr);
                                    map[pos].blocked = true;
                                }

                            MAP_WIDE_SPIKES| MAP_TALL_SPIKES => {
                                map[pos].chr = Some(chr);
                                map[pos].blocked = true;
                            }

                            MAP_WALL => {
                                map[pos].chr = Some(chr);
                                map[pos].blocked = true;
                            }

                            ENTITY_CLOAK_GUY => {
                                map[pos].chr = Some(chr);
                                map[pos].blocked = true;
                            }

                            _ => {
                                map[pos].chr = Some(chr);
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
                                objects.insert(make_gol(config, pos, msg_log));
                            }

                            ENTITY_EXIT => {
                                objects.insert(make_exit(config, pos));
                            }

                            ENTITY_ELF => {
                                objects.insert(make_elf(config, pos, msg_log));
                            }

                            MAP_EMPTY => {
                                // Nothing to do here...
                            }

                            ENTITY_DAGGER => {
                                let dagger = make_dagger(config, pos);
                                objects.insert(dagger);
                            }

                            ENTITY_KEY => {
                                let key = make_key(config, pos, msg_log);
                                objects.insert(key);
                            }

                            ENTITY_STONE => {
                                let stone = make_stone(config, pos);
                                objects.insert(stone);
                            }

                            ENTITY_SHIELD => {
                                let shield = make_shield(config, Pos::new(x, y));
                                objects.insert(shield);
                            }

                            ENTITY_SPIKE_TRAP => {
                                objects.insert(make_spikes(config, pos, msg_log));
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

