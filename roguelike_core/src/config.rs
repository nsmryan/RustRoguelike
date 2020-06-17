use std::fs::File;
use std::io::Read;

use crate::types::*;
use crate::map::*;

use serde_derive::*;
use serde_yaml;


#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Config {
    pub color_dark_brown: Color,
    pub color_medium_brown: Color,
    pub color_light_green: Color,
    pub color_tile_blue_light: Color,
    pub color_tile_blue_dark: Color,
    pub color_light_brown: Color,
    pub color_ice_blue: Color,
    pub color_dark_blue: Color,
    pub color_very_dark_blue: Color,    
    pub color_orange: Color,
    pub color_red: Color,
    pub color_medium_grey: Color,
    pub color_mint_green: Color,
    pub color_blueish_grey: Color,
    pub color_pink: Color,
    pub color_rose_red: Color,
    pub color_light_orange: Color,
    pub color_bone_white: Color,
    pub color_warm_grey: Color,
    pub color_soft_green: Color,
    pub color_light_grey: Color,
    pub color_console: Color,
    pub fov_distance: i32,
    pub load_map_file_every_frame: bool,
    pub tile_noise_scaler: f64,
    pub highlight_player_move: u8,
    pub highlight_attack: u8,
    pub sound_alpha: u8,
    pub grid_alpha: u8,
    pub grid_alpha_visible: u8,
    pub grid_alpha_overlay: u8,
    pub map_load: MapLoadConfig,
    pub map_file: String,
    pub idle_speed: f32,
    pub draw_mouse_line: bool,
    pub draw_star_path: bool,
    pub rate: usize,
    pub item_throw_speed: f32,
    pub key_speed: f32,
    pub player_attack_speed: f32,
    pub player_vault_sprite_speed: f32,
    pub player_vault_move_speed: f32,
    pub sound_timeout: f32,
    pub yell_radius: usize,
    pub fog_of_war: bool,
    pub player_health: i32,
    pub explored_alpha: u8,
    pub sound_rubble_radius: usize,
    pub sound_grass_radius: usize,
    pub sound_radius_crushed: usize,
    pub sound_radius_attack: usize,
    pub sound_radius_trap: usize,
    pub overlay_directions: bool,
    pub overlay_player_fov: bool,
    pub fov_radius_monster: i32,
    pub fov_radius_player: i32,
    pub console_max_height: u32,
    pub console_speed: f32,
}

impl Config {
    pub fn from_file(file_name: &str) -> Config {
        let mut file =
            File::open(file_name).expect(&format!("Could not open/parse config file {}", file_name));
        let mut config_string = String::new();
        file.read_to_string(&mut config_string)
            .expect(&format!("Could not read contents of {}", file_name));
        let config = serde_yaml::from_str(&config_string).expect(&format!("Could not parse {} file!", file_name));

        return config
    }
}
