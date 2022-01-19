use std::fs::File;
use std::io::BufReader;
use std::io::Write;

use rexpaint::*;

use log::trace;

use roguelike_core::constants::*;
use roguelike_core::messaging::*;
use roguelike_core::map::*;
use roguelike_core::types::*;
use roguelike_core::config::*;
use roguelike_core::utils::tile_fill_metric;

use crate::generation::*;
use crate::game::*;
use crate::procgen::*;
use crate::vault::*;


pub fn make_map(map_load_config: &MapLoadConfig, game: &mut Game) {
    let player_position: Pos;

    game.clear_level_except_player();
    game.settings.map_load_config = map_load_config.clone();

    match map_load_config {
        MapLoadConfig::TestMap => {
            game.level.map = Map::from_dims(11, 12);
            make_test_map(game);
            player_position = Pos::new(0, 0);
        }

        MapLoadConfig::Empty => {
            let new_map = Map::from_dims(10, 10);
            game.level.map = new_map;
            player_position = Pos::new(0, 0);
        }

        MapLoadConfig::TestSmoke => {
            let new_map = Map::from_dims(10, 10);
            game.level.map = new_map;
            player_position = Pos::new(0, 0);

            make_smoke(&mut game.level.entities, &game.config, Pos::new(5, 1), 1, &mut game.msg_log);
            make_smoke(&mut game.level.entities, &game.config, Pos::new(5, 4), 2, &mut game.msg_log);
            make_smoke(&mut game.level.entities, &game.config, Pos::new(5, 7), 3, &mut game.msg_log);
            make_magnifier(&mut game.level.entities, &game.config, Pos::new(3, 2), 1, &mut game.msg_log);
            make_magnifier(&mut game.level.entities, &game.config, Pos::new(3, 6), 3, &mut game.msg_log);
        }

        MapLoadConfig::ProcGen(procgen_params) => {
            let file_name = format!("resources/procgen/{}", procgen_params);
            let cmds = ProcCmd::from_file(&file_name);

            let mut template_file = "resources/wfc_seed_2.png".to_string();
            for param in cmds.iter() {
                if let ProcCmd::SeedFile(file_name) = param {
                    template_file = format!("resources/{}", file_name);
                }
            }

            game.level.map = generate_bare_map(MAP_WIDTH as u32, MAP_HEIGHT as u32, &template_file, &mut game.rng);
            player_position = saturate_map(game, &cmds);
        }

        MapLoadConfig::TestVaults => {
            player_position = Pos::new(0, 0);

            let mut max_width = 0;
            let mut max_height = 0;
            for vault in game.vaults.iter() {
                let (width, height) = vault.level.map.size();
                max_width = std::cmp::max(max_width, width);
                max_height = std::cmp::max(max_height, height);
            }
            let square = (game.vaults.len() as f32).sqrt().ceil() as u32;
            let max_dim = std::cmp::max(max_width, max_height);

            let map_width = std::cmp::min(MAP_WIDTH as u32, max_dim as u32 * square as u32);
            let map_height = std::cmp::min(MAP_HEIGHT as u32, max_dim as u32 * square as u32);
            game.level.map = Map::from_dims(map_width, map_height);

            let vaults = game.vaults.clone();
            for (index, vault) in vaults.iter().enumerate() {
                let x_pos = index % square as usize;
                let y_pos = index / square as usize;
                let offset = Pos::new(max_width * x_pos as i32 + 2 * x_pos as i32,
                                      max_height * y_pos as i32 + 2 * y_pos as i32);

                place_vault(&mut game.level, vault, offset, &mut game.rng, &game.config, &mut game.msg_log);
            }
        }

        MapLoadConfig::VaultFile(file_name) => {
            let vault: Vault = parse_vault(&format!("resources/{}", file_name), &game.config);
            let (vault_width, vault_height) = vault.level.map.size();
            let map_width = 3 * vault_width;
            let map_height = 3 * vault_height;

            //game.level.map = Map::with_vec(vault.level.map.tiles);
            game.level.map = Map::from_dims(map_width as u32, map_height as u32);
            place_vault_with(&mut game.level, &vault, Pos::new(0, 0), Rotation::Degrees0, &game.config, &mut game.msg_log, false);
            place_vault_with(&mut game.level, &vault, Pos::new(2*vault_width, 0), Rotation::Degrees90, &game.config, &mut game.msg_log, false);
            place_vault_with(&mut game.level, &vault, Pos::new(0, 2*vault_height), Rotation::Degrees180, &game.config, &mut game.msg_log, false);
            place_vault_with(&mut game.level, &vault, Pos::new(2*vault_width, 2*vault_height), Rotation::Degrees270, &game.config, &mut game.msg_log, false);

            player_position = Pos::new(4, 4);
        }

        MapLoadConfig::FromFile(file_name) => {
            let maps: Vec<String> = parse_map_file(&format!("resources/{}", file_name));

            if game.settings.level_num >= maps.len() {
                dbg!(game.settings.level_num, maps.len());
                panic!("Map index too large");
            }

            let map_name = format!("resources/{}", maps[game.settings.level_num]);
            let mut position =
                read_map_xp(&game.config, &mut game.level, &mut game.msg_log, &map_name);
            if position == (0, 0) {
                position = (game.level.map.width() / 2, game.level.map.height() / 2);
            }
            player_position = Pos::from(position);
        }

        MapLoadConfig::Random => {
            game.level.map = Map::from_dims(MAP_WIDTH as u32, MAP_HEIGHT as u32);
            let starting_position = make_island(&mut game.level, &game.config, &mut game.msg_log, &mut game.rng);
            player_position = Pos::from(starting_position);
        }

        MapLoadConfig::TestWall => {
            let (new_map, position) = make_wall_test_map(&mut game.level.entities, &game.config, &mut game.msg_log);
            game.level.map = new_map;
            player_position = Pos::from(position);
        }

        MapLoadConfig::TestPlayer => {
            let (new_map, position) = make_player_test_map(&mut game.level.entities, &game.config, &mut game.msg_log);
            game.level.map = new_map;
            player_position = Pos::from(position);
        }

        MapLoadConfig::TestCorner => {
            let (new_map, position) = make_corner_test_map(&mut game.level.entities, &game.config, &mut game.msg_log);
            game.level.map = new_map;
            player_position = Pos::from(position);
        }

        MapLoadConfig::TestTraps => {
            let (new_map, position) = make_trap_test_map(&mut game.level.entities, &game.config, &mut game.msg_log);
            game.level.map = new_map;
            player_position = Pos::from(position);
        }
    }

    let player_id = game.level.find_by_name(EntityName::Player).unwrap();
    game.msg_log.log(Msg::SetPos(player_id, player_position));
    game.level.entities.set_pos(player_id, player_position);

    if game.config.write_map_distribution {
        write_map_distribution(game);
    }

    game.msg_log.log(Msg::NewLevel);
    game.settings.map_changed = true;
}

fn write_map_distribution(game: &mut Game) {
    let max = (2 * TILE_FILL_METRIC_DIST + 1).pow(2);
    let mut counts = vec![0; max + 1];

    for pos in game.level.map.get_all_pos() {
        let amount = tile_fill_metric(&game.level.map, pos);
        counts[amount] += 1;
    }

    let mut file = File::create("map_emptiness_distribution.txt").unwrap();
    for (index, count) in counts.iter().enumerate() {
        write!(file, "{} {}\n", index, count).unwrap();
    }
}

pub fn read_map_xp(config: &Config,
                   level: &mut Level,
                   msg_log: &mut MsgLog,
                   file_name: &str) -> (i32, i32) {
    trace!("opening map {}", file_name);
    let file = File::open(file_name).unwrap();

    let mut buf_reader = BufReader::new(file);

    trace!("reading in map level");
    let xp = XpFile::read(&mut buf_reader).unwrap();

    level.map = Map::from_dims(xp.layers[0].width as u32, xp.layers[0].height as u32);
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
                                level.map[pos] = Tile::water();
                            }

                            MAP_RUBBLE => {
                                level.map[pos].surface = Surface::Rubble;
                            }

                            MAP_GRASS => {
                                level.map[pos].surface = Surface::Grass;
                                ensure_grass(&mut level.entities, pos, msg_log);
                            }

                            _ => {
                                dbg!(format!("Unexpected character {} in ground layer!", chr as u8));
                            }
                        }
                    }

                    MAP_LAYER_ENVIRONMENT => {
                        match chr as u8 {
                            MAP_COLUMN => {
                                make_column(&mut level.entities, config, pos, msg_log);
                            }

                            MAP_THIN_WALL_TOP => {
                                level.map[(x, y - 1)].bottom_wall = Wall::ShortWall;
                            }

                            MAP_THIN_WALL_BOTTOM => {
                                level.map[pos].bottom_wall = Wall::ShortWall;
                            }

                            MAP_THIN_WALL_LEFT => {
                                level.map[pos].left_wall = Wall::ShortWall;
                            }

                            MAP_THIN_WALL_RIGHT => {
                                level.map[(x + 1, y)].left_wall = Wall::ShortWall;
                            }

                            MAP_THIN_WALL_TOP_LEFT => {
                                level.map[pos].left_wall = Wall::ShortWall;
                                level.map[(x, y - 1)].bottom_wall = Wall::ShortWall;
                            }

                            MAP_THIN_WALL_BOTTOM_LEFT => {
                                level.map[pos].left_wall = Wall::ShortWall;
                                level.map[pos].bottom_wall = Wall::ShortWall;
                            }

                            MAP_THIN_WALL_TOP_RIGHT => {
                                level.map[(x, y - 1)].bottom_wall = Wall::ShortWall;
                                level.map[(x - 1, y)].left_wall = Wall::ShortWall;
                            }

                            MAP_THIN_WALL_BOTTOM_RIGHT => {
                                level.map[pos].bottom_wall = Wall::ShortWall;
                                level.map[(x + 1, y)].left_wall = Wall::ShortWall;
                            }

                            MAP_THICK_WALL_TOP => {
                                level.map[(x, y - 1)].bottom_wall = Wall::ShortWall;
                            }

                            MAP_THICK_WALL_LEFT => {
                                level.map[pos].left_wall = Wall::TallWall;
                            }

                            MAP_THICK_WALL_RIGHT => {
                                level.map[(x + 1, y)].left_wall = Wall::ShortWall;
                            }

                            MAP_THICK_WALL_BOTTOM => {
                                level.map[pos].bottom_wall = Wall::TallWall;
                            }

                            MAP_THICK_WALL_TOP_LEFT => {
                                level.map[pos].left_wall = Wall::TallWall;
                                level.map[(x, y - 1)].bottom_wall = Wall::TallWall;
                            }

                            MAP_THICK_WALL_BOTTOM_LEFT => {
                                level.map[pos].bottom_wall = Wall::TallWall;
                                level.map[pos].left_wall = Wall::TallWall;
                            }

                            MAP_THICK_WALL_TOP_RIGHT => {
                                level.map[(x, y - 1)].bottom_wall = Wall::TallWall;
                                level.map[(x + 1, y)].left_wall = Wall::TallWall;
                            }

                            MAP_THICK_WALL_BOTTOM_RIGHT => {
                                level.map[pos].bottom_wall = Wall::TallWall;
                                level.map[(x + 1, y)].left_wall = Wall::TallWall;
                            }

                            MAP_DOT_TOP_LEFT => {
                                level.map[pos].block_move = true;
                            }

                            MAP_DOT_TOP_RIGHT => {
                                level.map[pos].block_move = true;
                            }

                            MAP_DOT_BOTTOM_LEFT => {
                                level.map[pos].block_move = true;
                            }

                            MAP_DOT_BOTTOM_RIGHT => {
                                level.map[pos].block_move = true;
                            }

                            MAP_ROOK => {
                                level.map[pos].block_move = true;
                            }

                            MAP_ORB => {
                                level.map[pos].block_move = true;
                            }

                            MAP_EMPTY => {
                            }

                            MAP_STATUE_1 | MAP_STATUE_2 | MAP_STATUE_3 |
                            MAP_STATUE_4 | MAP_STATUE_5 | MAP_STATUE_6 => {
                                // NOTE this should use make_statue instead
                                    level.map[pos].block_move = true;
                                    level.map[pos].block_sight = true;
                                }

                            MAP_WIDE_SPIKES | MAP_TALL_SPIKES => {
                                level.map[pos].block_move = true;
                                    level.map[pos].block_sight = true;
                            }

                            MAP_WALL => {
                                level.map[pos].block_move = true;
                                level.map[pos].block_sight = true;
                            }

                            ENTITY_CLOAK_GUY => {
                                level.map[pos].block_move = true;
                            }

                            _ => {
                                level.map[pos].block_move = true;
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
                                make_gol(&mut level.entities, config, pos, msg_log);
                            }

                            ENTITY_EXIT => {
                                make_exit(&mut level.entities, config, pos, msg_log);
                            }

                            ENTITY_ELF => {
                                make_pawn(&mut level.entities, config, pos, msg_log);
                            }

                            MAP_EMPTY => {
                                // Nothing to do here...
                            }

                            ENTITY_DAGGER => {
                                make_dagger(&mut level.entities, config, pos, msg_log);
                            }

                            ENTITY_KEY => {
                                make_key(&mut level.entities, config, pos, msg_log);
                            }

                            ENTITY_STONE => {
                                make_stone(&mut level.entities, config, pos, msg_log);
                            }

                            ENTITY_SHIELD => {
                                make_shield(&mut level.entities, config, Pos::new(x, y), msg_log);
                            }

                            ENTITY_HAMMER => {
                                make_hammer(&mut level.entities, config, Pos::new(x, y), msg_log);
                            }
 
                            ENTITY_SPIKE_TRAP => {
                                make_spike_trap(&mut level.entities, config, pos, msg_log);
                            }

                            _ => {
                                dbg!(chr);
                                panic!("Unexpected character in entities layer!");
                            }
                        }
                    }

                    _ => {
                        dbg!(layer_index);
                        panic!("Layer not expected in map file!");
                    }
                }
            }
        }
    }

    trace!("map read finished");

    trace!("map updated");

    return player_position;
}

/// Read file into a vector of lines
pub fn parse_map_file(file_name: &str) -> Vec<String> {
    let file_contents =
        std::fs::read_to_string(file_name).expect(&format!("Could not read {}", file_name));
    return file_contents.lines().map(|s| s.to_string()).collect::<Vec<String>>();
}

