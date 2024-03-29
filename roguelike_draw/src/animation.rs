use serde::{Serialize, Deserialize};

use parse_display::{Display, FromStr};

use roguelike_utils::math::*;
use roguelike_utils::rng::*;

use roguelike_map::*;


pub type SpriteKey = usize;

pub type SpriteIndex = f32;

pub type Str = usize;


#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Particle {
    pub duration: f32,
    pub x: f32,
    pub y: f32,
}

impl Particle {
    pub fn new(duration: f32, x: f32, y: f32) -> Particle {
        return Particle { duration, x, y };
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Effect {
    Sound(Aoe, f32), // area of effect, time since start
    Beam(usize, Pos, Pos), // start, end
    Particles(f32, Vec<Particle>), // spawn rate, current particles
    Attack(Pos, Pos, SpriteAnim),
    Fade(Sprite, Color, u8, u8, Pos, f32, f32), // sprite, color, start alpha, end alpha, position, seconds, time taken
    NumberChange(i32, Pos, Color, usize), // Change amount, tile position, color, frame count
    Highlight(Color, Pos, bool, f32, f32), // color, tile, fade, duration, seconds
}

impl Effect {
    pub fn particles(rate: f32) -> Effect {
        return Effect::Particles(rate, Vec::new());
    }

    pub fn beam(duration: usize, start: Pos, end: Pos) -> Effect {
        return Effect::Beam(duration, start, end);
    }

    pub fn sound(aoe: Aoe) -> Effect {
        return Effect::Sound(aoe, 0.0);
    }

    pub fn attack(from: Pos, to: Pos, sprite_anim: SpriteAnim) -> Effect {
        return Effect::Attack(from, to, sprite_anim);
    }

    pub fn fade(sprite: Sprite, color: Color, start: u8, end: u8, pos: Pos, seconds: f32) -> Effect {
        return Effect::Fade(sprite, color, start, end, pos, seconds, 0.0);
    }

    pub fn highlight(color: Color, pos: Pos, fade: bool, seconds: f32) -> Effect {
        return Effect::Highlight(color, pos, fade, seconds, 0.0);
    }

    pub fn number_change(change: i32, pos: Pos, color: Color) -> Effect {
        return Effect::NumberChange(change, pos, color, 0);
    }
}

#[derive(Clone, Display, FromStr, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[display("sprite {index} {key} {flip_horiz} {flip_vert} {rotation}")]
pub struct Sprite {
    pub index: u32, 
    pub key: SpriteKey,
    pub flip_horiz: bool,
    pub flip_vert: bool,
    pub rotation: f64,
}

impl Sprite {
    pub fn new(index: u32, key: SpriteKey) -> Sprite {
        let flip_vert = false;
        let flip_horiz = false;
        let rotation = 0.0;
        return Sprite { index: index, key, flip_horiz, flip_vert, rotation };
    }

    pub fn from_key(key: SpriteKey) -> Sprite {
        let flip_vert = false;
        let flip_horiz = false;
        let rotation = 0.0;
        return Sprite { index: 0, key, flip_horiz, flip_vert, rotation };
    }

    pub fn with_flip(index: u32, key: SpriteKey, flip_horiz: bool, flip_vert: bool) -> Sprite {
        return Sprite { index, key, flip_horiz, flip_vert, rotation: 0.0 };
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpriteAnim {
    pub name: Str,
    pub sprite_key: SpriteKey,
    pub index: SpriteIndex,
    pub start_index: SpriteIndex,
    pub max_index: SpriteIndex,
    pub speed: f32,
    pub looped: bool,
    pub flip_horiz: bool,
    pub flip_vert: bool,
    pub rotation: f64,
}

impl SpriteAnim {
    pub fn new(name_str: Str,
               sprite_key: SpriteKey,
               index: SpriteIndex,
               max_index: SpriteIndex,
               speed: f32) -> SpriteAnim {

        let flip_vert = false;
        let flip_horiz = false;
        let rotation = 0.0;
        return SpriteAnim { name: name_str,
                        sprite_key,
                        index,
                        start_index: index,
                        max_index,
                        speed,
                        looped: false,
                        flip_horiz,
                        flip_vert,
                        rotation,
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
        let mut sprite = Sprite::with_flip(self.index as u32, self.sprite_key, self.flip_horiz, self.flip_vert);
        sprite.rotation = self.rotation;
        return sprite;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct AnimationResult {
    pub done: bool,
    pub sprite: Option<Sprite>,
    pub pos: Pos,
}

impl AnimationResult {
    pub fn new() -> AnimationResult {
        let sprite: Option<Sprite> = None;
        let done = false;
        let pos = Pos::new(-1, -1);
        return AnimationResult { done, sprite, pos };
    }
}


#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Animation {
    Loop(SpriteAnim),                         // play sprite sheet in loop
    RandomLoop(SpriteAnim),                   // play a random sprite sheet in loop
    Between(SpriteAnim, Pos, Pos, f32, f32),  // start, end, dist, blocks_per_sec
    Once(SpriteAnim),                         // play sprite once and end
    PlayEffect(Effect),
}

impl Animation {
    pub fn sprite_anim_mut(&mut self) -> Option<&mut SpriteAnim> {
        match self {
            Animation::Loop(sprite_anim) => return Some(sprite_anim),
            Animation::RandomLoop(sprite_anim) => return Some(sprite_anim),
            Animation::Between(sprite_anim, _, _, _, _) => return Some(sprite_anim),
            Animation::Once(sprite_anim) => return Some(sprite_anim),
            Animation::PlayEffect(_) => return None,
        }
    }

    pub fn step(&mut self, dt: f32, rng: &mut Rand32, frame_rate: f32) {
        match self {
            Animation::Between(_sprite_anim, _start, _end, ref mut dist, blocks_per_sec) => {
               *dist = *dist + (*blocks_per_sec / frame_rate); 
            }

            Animation::Loop(ref mut sprite_anim) => {
                sprite_anim.step(dt);
            }

            Animation::RandomLoop(ref mut sprite_anim) => {
                let last_index = sprite_anim.index;
                sprite_anim.index += dt * sprite_anim.speed;
                if last_index as u32 != sprite_anim.index as u32 {
                    sprite_anim.index = rng_range(rng, sprite_anim.start_index, sprite_anim.max_index);
                }
            }

            Animation::PlayEffect(_effect) => {
            }

            Animation::Once(_sprite_anim) => {
            }
        }
    }

    pub fn status(&self, pos: Pos) -> AnimationResult {
        let mut animation_result = AnimationResult::new();
        animation_result.pos = pos;

        match self {
            Animation::Between(ref sprite_anim, start, end, dist, _blocks_per_sec) => {
               let num_blocks = *dist as usize;

               animation_result.pos = move_towards(*start, *end, num_blocks);

               let sprite = sprite_anim.sprite();
               animation_result.sprite = Some(sprite);

               animation_result.done = *dist >= distance(*start, *end) as f32;
            }

            Animation::Loop(ref sprite_anim) => {
                let sprite = sprite_anim.sprite();
                animation_result.sprite = Some(sprite);

                // a looping animation never finishes
                animation_result.done = false;
            }

            Animation::RandomLoop(ref sprite_anim) => {
                let sprite = sprite_anim.sprite();
                animation_result.sprite = Some(sprite);

                // a random looping animation never finishes
                animation_result.done = false;
            }

            Animation::PlayEffect(_effect) => {
                animation_result.done = true;
            }

            Animation::Once(ref sprite_anim) => {
                if !sprite_anim.looped {
                    let sprite = sprite_anim.sprite();
                    animation_result.sprite = Some(sprite);
                }

                animation_result.done = sprite_anim.looped;
            }
        }

        return animation_result;
    }
}

