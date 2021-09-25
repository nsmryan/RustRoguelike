use roguelike_core::types::{Name, Pos};
use roguelike_core::utils::{distance, move_towards};
use roguelike_core::map::Aoe;
use roguelike_core::config::Config;


pub type SpriteKey = i64;

pub type SpriteIndex = f32;


#[derive(Clone, Debug, PartialEq)]
pub struct Particle {
    pub duration: f32,
    pub pos: Pos,
}

impl Particle {
    pub fn new(duration: f32, pos: Pos) -> Particle {
        return Particle { duration, pos };
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    Sound(Aoe, f32), // area of effect, time since start
    Beam(usize, Pos, Pos), // start, end
    Particles(f32, Vec<Particle>), // spawn rate, current particles
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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sprite {
    pub index: u32, 
    pub key: SpriteKey,
    pub flip_horiz: bool,
    pub flip_vert: bool,
}

impl Sprite {
    pub fn new(index: u32, key: SpriteKey) -> Sprite {
        let flip_vert = false;
        let flip_horiz = false;
        return Sprite { index, key, flip_horiz, flip_vert };
    }

    pub fn with_flip(index: u32, key: SpriteKey, flip_horiz: bool, flip_vert: bool) -> Sprite {
        return Sprite { index, key, flip_horiz, flip_vert };
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
    pub flip_horiz: bool,
    pub flip_vert: bool,
}

impl SpriteAnim {
    pub fn new(name: String,
               sprite_key: SpriteKey,
               index: SpriteIndex,
               max_index: SpriteIndex,
               speed: f32) -> SpriteAnim {
        let flip_vert = false;
        let flip_horiz = false;
        return SpriteAnim { name: name.into(),
                        sprite_key,
                        index,
                        start_index: index,
                        max_index,
                        speed,
                        looped: false,
                        flip_horiz,
                        flip_vert,
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
        return Sprite::with_flip(self.index as u32, self.sprite_key, self.flip_horiz, self.flip_vert);
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
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

    pub fn step(&mut self, pos: Pos, dt: f32, config: &Config) -> AnimationResult {
        let mut animation_result = AnimationResult::new();
        animation_result.pos = pos;

        match self {
            Animation::Between(ref mut sprite_anim, start, end, ref mut dist, blocks_per_sec) => {
               *dist = *dist + (*blocks_per_sec / config.frame_rate as f32); 
               let num_blocks = *dist as usize;

               let draw_pos = move_towards(*start, *end, num_blocks);

               let sprite = sprite_anim.sprite();
               animation_result.sprite = Some(sprite);

               animation_result.pos = draw_pos;

               animation_result.done = *dist >= distance(*start, *end) as f32;
            }

            Animation::Loop(ref mut sprite_anim) => {
                let sprite = sprite_anim.sprite();
                animation_result.sprite = Some(sprite);

                sprite_anim.step(dt);

                // a looping animation never finishes
                animation_result.done = false;
            }

            Animation::PlayEffect(_effect) => {
                animation_result.done = true;
            }

            Animation::Once(ref mut sprite_anim) => {
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

