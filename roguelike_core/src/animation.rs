
use slotmap::DefaultKey;

use crate::types::{Name, Pos};


pub type SpriteKey = DefaultKey;

pub type AnimKey = DefaultKey;

pub type SpriteIndex = f32;


#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Effect {
    Sound(Pos, usize, usize), // center, current radius, max radius
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sprite {
    pub name: Name,
    pub sprite_key: SpriteKey,
    pub index: SpriteIndex,
    pub start_index: SpriteIndex,
    pub max_index: SpriteIndex,
    pub speed: f32,
}

impl Sprite {
    pub fn new(name: String,
               sprite_key: SpriteKey,
               index: SpriteIndex,
               max_index: SpriteIndex,
               speed: f32) -> Sprite {
        return Sprite { name: name.into(),
                        sprite_key,
                        index,
                        start_index: index,
                        max_index,
                        speed,
        };
    }

    pub fn make_sprite(name: String,
                       sprite_key: SpriteKey,
                       max_index: SpriteIndex,
                       speed: f32) -> Sprite {
        return Sprite {
            name: name.into(),
            sprite_key,
            max_index,
            index: 0.0,
            start_index: 0.0,
            speed,
        };
    }

    /// step a sprite forward once, returning whether it reached the end of its
    /// animation or not.
    pub fn step(&mut self) -> bool {
        self.index = self.index + self.speed;
        if self.index as usize >= self.max_index as usize {
            self.index = self.start_index;
            return true;
        }

        return false;
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Animation {
    Loop(Sprite),                         // play sprite sheet in loop
    Between(Sprite, Pos, Pos, f32, f32),  // start, end, dist, blocks_per_sec
    PlayEffect(Effect),
}
