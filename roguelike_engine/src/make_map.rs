use std::fs::File;
use std::io::BufReader;
use std::io::Write;

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
            unimplemented!()
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

/// Read file into a vector of lines
pub fn parse_map_file(file_name: &str) -> Vec<String> {
    let file_contents =
        std::fs::read_to_string(file_name).expect(&format!("Could not read {}", file_name));
    return file_contents.lines().map(|s| s.to_string()).collect::<Vec<String>>();
}

