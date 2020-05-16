
use crate::types::{Name, Pos};
use crate::map::Aoe;


pub type SpriteKey = i64;

pub type AnimKey = i64;

pub type SpriteIndex = f32;


#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    Sound(Aoe, f32), // area of effect, time since start
    HeardSomething(Pos, usize), // position of sound, turn that it occurred
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

#[derive(Clone, Debug, PartialEq)]
pub enum Animation {
    Loop(Sprite),                         // play sprite sheet in loop
    Between(Sprite, Pos, Pos, f32, f32),  // start, end, dist, blocks_per_sec
    Once(Sprite),                         // play sprite once and end
    PlayEffect(Effect),
}

