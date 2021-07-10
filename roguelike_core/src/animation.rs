use crate::types::{Name, Pos};
use crate::map::Aoe;


pub type SpriteKey = i64;

pub type AnimKey = i64;

pub type SpriteIndex = f32;


#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    Sound(Aoe, f32), // area of effect, time since start
    Beam(usize, Pos, Pos), // start, end
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
    pub looped: bool,
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
                        looped: false,
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
            looped: false,
        };
    }

    /// step a sprite forward once, returning whether it reached the end of its
    /// animation or not.
    pub fn step(&mut self, dt: f32) {
        let index_range = self.max_index - self.start_index;
        let new_index = self.index + (dt * self.speed);

        self.looped = new_index > self.max_index;
        if self.looped {
            self.index = self.start_index + (new_index % index_range);
        } else {
            self.index = new_index;
        }
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

impl Animation {
    pub fn sprite_anim_mut(&mut self) -> Option<&mut SpriteAnim> {
        match self {
            Animation::Loop(sprite_anim) => return Some(sprite_anim),
            Animation::Between(sprite_anim, _, _, _, _) => return Some(sprite_anim),
            Animation::Once(sprite_anim) => return Some(sprite_anim),
            Animation::PlayEffect(_) => return None,
        }
    }
}

