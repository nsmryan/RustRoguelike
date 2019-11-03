use std::convert::Into;

use num::clamp;

use ggez::graphics::Color;

use tcod::line::*;

use roguelike_core::map::*;
use roguelike_core::types::*;

use crate::constants::*;


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

#[derive(Clone, Debug, PartialEq)]
pub struct Object {
    pub x: i32,
    pub y: i32,
    pub char: char,
    pub color: Color,
    pub name: String,
    pub blocks: bool,
    pub alive: bool,
    pub fighter: Option<Fighter>,
    pub ai: Option<Ai>,
    pub behavior: Option<Behavior>,
    pub item: Option<Item>,
    pub momentum: Option<Momentum>,
    pub movement: Option<Reach>,
    pub attack: Option<Reach>,
    pub animation: Option<Animation>,
}

impl Object {
    pub fn new(x: i32, y: i32, chr: char, color: Color, name: &str, blocks: bool) -> Self {
        Object {
            x,
            y,
            char: chr,
            color,
            name: name.into(),
            blocks,
            alive: false,
            fighter: None,
            ai: None,
            behavior: None,
            item: None,        
            momentum: None,
            movement: None,
            attack: None,
            animation: None,
        }
    }

    pub fn pos(&self) -> (i32, i32) {
        (self.x, self.y)
    }

    pub fn set_pos(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
    }

    pub fn distance_to(&self, other: &Object) -> f32 {
        return self.distance(&Position::new(other.x, other.y));
    }

    pub fn distance(&self, other: &Position) -> f32 {
        let dx = other.0 - self.x;
        let dy = other.1 - self.y;
        return ((dx.pow(2) + dy.pow(2)) as f32).sqrt();
    }

    pub fn take_damage(&mut self, damage: i32) {
        if let Some(fighter) = self.fighter.as_mut() {
            if damage > 0 {
                fighter.hp -= damage;
            }
        }

        if let Some(fighter) = self.fighter {
            if fighter.hp <= 0 {
                self.alive = false;
            }
        }
    }

    pub fn attack(&mut self, target: &mut Object) {
        let damage = self.fighter.map_or(0, |f| f.power) - target.fighter.map_or(0, |f| f.defense);

        if damage > 0 {
            //messages.message(format!("{} attacks {} for {} hit points.", self.name, target.name, damage), WHITE);
            target.take_damage(damage);
        } else {
            //messages.message(format!("{} attacks {} but it has no effect!", self.name, target.name), WHITE);
        }
    }

    pub fn heal(&mut self, amount: i32) {
        if let Some(ref mut fighter) = self.fighter {
            fighter.hp += amount;
            if fighter.hp > fighter.max_hp {
                fighter.hp = fighter.max_hp;
            }
        }
    }

    pub fn make_stone(config: &Config, x: i32, y: i32) -> Object {
        Object::new(x, y, 'o', config.color_light_grey.color(), "stone", false)
    }
}

// TODO should put in some kind of utilities module
pub fn is_blocked(map: &Map, x: i32, y: i32, objects: &[Object]) -> bool {
    if map[(x, y)].blocked {
        return true;
    }

    let mut is_blocked = false;
    for object in objects.iter() {
        if object.blocks && object.pos() == (x, y) {
            is_blocked = true;
            break;
        }
    }

    return is_blocked;
}

pub fn clear_path(map: &Map, start: (i32, i32), end: (i32, i32), objects: &[Object]) -> bool {
    let line = Line::new((start.0, start.1), (end.0, end.1));

    let path_blocked =
        line.into_iter().any(|point| is_blocked(map, point.0, point.1, objects));

    return !path_blocked;
}

pub struct GameData {
    pub map: Map,
    pub objects: Vec<Object>,
}

impl GameData {
    pub fn new(map: Map, objects: Vec<Object>) -> GameData {
        GameData {
            map,
            objects,
        }
    }
}

