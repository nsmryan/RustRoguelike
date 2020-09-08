
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sprite {
    Char(char),
    Sprite(u32, SpriteKey), // index, sprite key
}

impl Sprite {
    pub fn sprite(index: u32, sheet: SpriteKey) -> Sprite {
        return Sprite::Sprite(index, sheet);
    }

    pub fn char(chr: char) -> Sprite {
        return Sprite::Char(chr);
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpriteAnim {
    pub name: Name,
    pub sprite_key: SpriteKey,
    pub index: SpriteIndex,
    pub start_index: SpriteIndex,
    pub max_index: SpriteIndex,
    pub speed: f32,
}

impl SpriteAnim {
    pub fn new(name: String,
               sprite_key: SpriteKey,
               index: SpriteIndex,
               max_index: SpriteIndex,
               speed: f32) -> SpriteAnim {
        return SpriteAnim { name: name.into(),
                        sprite_key,
                        index,
                        start_index: index,
                        max_index,
                        speed,
        };
    }

    pub fn make_anim(name: String,
                     sprite_key: SpriteKey,
                     max_index: SpriteIndex,
                     speed: f32) -> SpriteAnim {
        return SpriteAnim {
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

    pub fn sprite(&self) -> Sprite {
        return Sprite::sprite(self.index as u32, self.sprite_key);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Animation {
    Loop(SpriteAnim),                         // play sprite sheet in loop
    Between(SpriteAnim, Pos, Pos, f32, f32),  // start, end, dist, blocks_per_sec
    Once(SpriteAnim),                         // play sprite once and end
    PlayEffect(Effect),
}

