
use slotmap::DefaultKey;

use crate::types::Pos;


pub type SpriteKey = DefaultKey;

pub type SpriteIndex = f32;


#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Effect {
    Sound(Pos, usize, usize), // center, current radius, max radius
}

#[derive(Clone, Debug, PartialEq)]
pub struct Sprite {
    pub name: String,
    pub sprite_key: SpriteKey,
    pub index: SpriteIndex,
    pub max_index: SpriteIndex,
    pub speed: f32,
}

impl Sprite {
    pub fn new(name: String,
               sprite_key: SpriteKey,
               index: SpriteIndex,
               max_index: SpriteIndex,
               speed: f32) -> Sprite {
        return Sprite { name,
                        sprite_key,
                        index,
                        max_index,
                        speed,
        };
    }

    pub fn make_sprite(name: String, sprite_key: SpriteKey, max_index: SpriteIndex) -> Sprite {
        return Sprite {
            name,
            sprite_key,
            max_index,
            index: 0.0,
            speed: 1.0,
        };
    }

    /// step a sprite forward once, returning whether it reached the end of its
    /// animation or not.
    pub fn step(&mut self) -> bool {
        self.index = self.index + self.speed;
        if self.index as usize >= self.max_index as usize {
            self.index = 0.0;
            return true;
        }

        return false;
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Animation {
    Loop(Sprite),               // play sprite sheet in loop
    Between(Sprite, Pos, Pos),  // start, end
    //Then(Box<Animation>, Box<Animation>), // play sprite, then transition to next animation
}
