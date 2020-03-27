use crate::types::*;
use crate::map::*;

use serde_derive::*;


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
    pub goal_speed: f32,
    pub player_attack_speed: f32,
    pub sound_timeout: f32,
    pub player_yell_radius: usize,
    pub fog_of_war: bool,
    pub player_health: i32,
    pub explored_alpha: u8,
}

