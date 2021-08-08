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
    pub color_shadow: Color,
    pub load_map_file_every_frame: bool,
    pub tile_noise_scaler: f64,
    pub highlight_player_move: u8,
    pub highlight_alpha_attack: u8,
    pub sound_alpha: u8,
    pub grid_alpha: u8,
    pub grid_alpha_visible: u8,
    pub grid_alpha_overlay: u8,
    pub map_load: MapLoadConfig,
    pub map_file: String,
    pub idle_speed: f32,
    pub draw_mouse_line: bool,
    pub draw_star_path: bool,
    pub frame_rate: usize,
    pub item_throw_speed: f32,
    pub key_speed: f32,
    pub player_attack_speed: f32,
    pub player_attack_hammer_speed: f32,
    pub player_vault_sprite_speed: f32,
    pub player_vault_move_speed: f32,
    pub sound_timeout: f32,
    pub yell_radius: usize,
    pub swap_radius: usize,
    pub fog_of_war: bool,
    pub player_health: i32,
    pub explored_alpha: u8,
    pub fov_edge_alpha: u8,
    pub sound_rubble_radius: usize,
    pub sound_golem_idle_radius: usize,
    pub sound_grass_radius: usize,
    pub sound_radius_crushed: usize,
    pub sound_radius_attack: usize,
    pub sound_radius_trap: usize,
    pub sound_radius_monster: usize,
    pub sound_radius_stone: usize,
    pub sound_radius_player: usize,
    pub sound_radius_hammer: usize,
    pub freeze_trap_radius: usize,
    pub push_stun_turns: usize,
    pub sword_stun_turns: usize,
    pub overlay_directions: bool,
    pub overlay_player_fov: bool,
    pub overlay_fov_alg: bool,
    pub overlay_floodfill: bool,
    pub fov_radius_monster: i32,
    pub fov_radius_player: i32,
    pub sound_radius_sneak: usize,
    pub sound_radius_walk: usize,
    pub sound_radius_run: usize,
    pub dampen_blocked_tile: i32,
    pub dampen_short_wall: i32,
    pub dampen_tall_wall: i32,
    pub take_screenshot: bool,
    pub show_info: bool,
    pub use_cursor: bool,
    pub cursor_long: i32,
    pub repeat_delay: f32,
    pub write_map_distribution: bool,
    pub print_key_log: bool,
    pub idle_animations: bool,
    pub recording: bool,
    pub fire_speed: f32,
    pub beam_duration: usize,
    pub draw_directional_arrow: bool,
    pub sword_step_num_turns: usize,
    pub dagger_stab_num_turns: usize,
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

