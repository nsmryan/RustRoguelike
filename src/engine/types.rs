use std::convert::Into;

use num::clamp;

use ggez::graphics::Color;

use crate::constants::*;
use crate::engine::map::*;


// TODO rename this file to config

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Default)]
pub struct ColorConfig {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl ColorConfig {
    pub fn color(&self) -> Color {
        Color::new(self.r as f32 / 256.0,
                   self.g as f32 / 256.0,
                   self.b as f32 / 256.0,
                   1.0)
    }

    pub fn from_color(color: Color) -> ColorConfig {
        ColorConfig { r: (color.r * 256.0) as u8,
                      g: (color.g * 256.0) as u8,
                      b: (color.b * 256.0) as u8,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Default)]
pub struct Config {
    pub color_dark_brown: ColorConfig,
    pub color_medium_brown: ColorConfig,
    pub color_light_green: ColorConfig,
    pub color_tile_blue_light: ColorConfig,
    pub color_tile_blue_dark: ColorConfig,
    pub color_light_brown: ColorConfig,
    pub color_ice_blue: ColorConfig,
    pub color_dark_blue: ColorConfig,
    pub color_very_dark_blue: ColorConfig,    
    pub color_orange: ColorConfig,
    pub color_red: ColorConfig,
    pub color_medium_grey: ColorConfig,
    pub color_mint_green: ColorConfig,
    pub color_blueish_grey: ColorConfig,
    pub color_pink: ColorConfig,
    pub color_rose_red: ColorConfig,
    pub color_light_orange: ColorConfig,
    pub color_bone_white: ColorConfig,
    pub color_warm_grey: ColorConfig,
    pub color_soft_green: ColorConfig,
    pub color_light_grey: ColorConfig,
    pub fov_distance: i32,
    pub load_map_file: bool,
    pub load_map_file_every_frame: bool,
    pub tile_noise_scaler: f64,
    pub highlight_alpha: f32,
}

