use sdl2::render::{Texture, WindowCanvas, TextureCreator};
use sdl2::video::WindowContext;
use sdl2::rect::{Rect};
use sdl2::pixels::{Color as Sdl2Color};

use indexmap::map::IndexMap;

use roguelike_core::types::*;
use roguelike_core::constants::*;
use roguelike_core::config::*;
use roguelike_core::messaging::*;
use roguelike_core::map::*;
use roguelike_core::animation::{Sprite, AnimKey, Effect, SpriteKey, Animation, SpriteAnim, SpriteIndex};
use roguelike_core::movement::{Cardinal, MoveType};

use crate::plat::*;


#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnimationResult {
    pub done: bool,
    pub sprite: Option<Sprite>,
}

impl AnimationResult {
    pub fn new() -> AnimationResult {
        let sprite: Option<Sprite> = None;
        let done = false;
        return AnimationResult { done, sprite};
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Impression {
    pub sprite: Sprite,
    pub pos: Pos,
}

impl Impression {
    pub fn new(sprite: Sprite, pos: Pos) -> Impression {
        return Impression { sprite, pos };
    }
}

pub struct DisplayState {
    pub font_image: Texture,
    pub sprites: IndexMap<SpriteKey, SpriteSheet>,
    pub next_sprite_key: i64,
    pub screen_sections: Plan,
    pub zones: Vec<Plot>,
    pub canvas: WindowCanvas,
    pub effects: Vec<Effect>,
    pub animations: IndexMap<AnimKey, Animation>,
    pub next_anim_key: i64,
    pub background: Option<Texture>,
    pub texture_creator: TextureCreator<WindowContext>,
    pub drawn_sprites: IndexMap<EntityId, Sprite>,
    pub impressions: Vec<Impression>,
    pub prev_turn_fov: Vec<EntityId>,
    pub current_turn_fov: Vec<EntityId>,
}

impl DisplayState {
    pub fn new(screen_sections: Plan,
               font_image: Texture,
               canvas: WindowCanvas) -> DisplayState {

        let texture_creator = canvas.texture_creator();

        return DisplayState {
            font_image,
            sprites: IndexMap::new(),
            next_sprite_key: 0,
            screen_sections,
            canvas,
            zones: Vec::new(),
            effects: Vec::new(),
            animations: IndexMap::new(),
            next_anim_key: 0,
            background: None,
            texture_creator,
            drawn_sprites: IndexMap::new(),
            impressions: Vec::new(),
            prev_turn_fov: Vec::new(),
            current_turn_fov: Vec::new(),
        };
    }

    pub fn add_sprite(&mut self, sprite_sheet: SpriteSheet) {
        let sprite_key = self.next_sprite_key;
        self.next_sprite_key += 1;
        self.sprites.insert(sprite_key, sprite_sheet);
    }

    pub fn lookup_spritekey(&self, name: &String) -> Option<SpriteKey> {
        for (key, sprite_sheet) in self.sprites.iter() {
            if sprite_sheet.name == *name {
                return Some(*key);
            }
        }

        return None;
    }

    pub fn new_sprite(&self, name: String, speed: f32) -> Option<SpriteAnim> {
        if let Some(sprite_key) = self.lookup_spritekey(&name) {
            let max_index = self.sprites[&sprite_key].num_sprites;
            return Some(SpriteAnim::make_anim(name, sprite_key, max_index as f32, speed));
        }

        return None;
    }

    pub fn font_sprite(&self, chr: char) -> Option<SpriteAnim> {
        if let Some(sprite_key) = self.lookup_spritekey(&"font".to_string()) {
            return Some(SpriteAnim::new(format!("{}", chr),
                                        sprite_key,
                                        chr as i32 as SpriteIndex,
                                        chr as i32 as SpriteIndex,
                                        0.0));
        }

        return None;
    }

    pub fn draw_text(&mut self,
                     text: &String,
                     pos: Pos,
                     color: Color,
                     area: &Area) {
        for (index, chr) in text.chars().enumerate() {
            let chr_pos = Pos::new(pos.x + index as i32, pos.y);
            self.draw_char(chr, chr_pos, color, area);
        }
    }

    pub fn draw_text_list(&mut self,
                          text_list: &Vec<String>,
                          pos: Pos,
                          color: Color,
                          area: &Area) {
        let mut y_pos = 0;
        for text in text_list.iter() {
            self.draw_text(text, Pos::new(pos.x, pos.y + y_pos), color, area);
            y_pos += 1;
        }
    }

    pub fn draw_sprite(&mut self,
                       sprite: Sprite,
                       pos: Pos,
                       color: Color,
                       area: &Area) {
        match sprite {
            Sprite::Sprite(index, key) => {
                self.draw_tile(index as u32, key, pos, color, area);
            },

            Sprite::Char(chr) => {
                self.draw_char(chr, pos, color, area);
            },
        }
    }

    pub fn draw_tile(&mut self,
                     index: u32,
                     sprite_key: SpriteKey,
                     pos: Pos,
                     color: Color,
                     area: &Area) {
        let sprite_sheet = &mut self.sprites[&sprite_key];

        let sprites_per_row = sprite_sheet.sprites_per_row();
        let sprite_x = index as usize % sprites_per_row;
        let sprite_y = index as usize / sprites_per_row;

        let src = Rect::new(sprite_x as i32 * FONT_WIDTH,
                            sprite_y as i32 * FONT_HEIGHT,
                            FONT_WIDTH as u32,
                            FONT_HEIGHT as u32);

        let dst = area.char_rect(pos.x, pos.y);

        sprite_sheet.texture.set_color_mod(color.r, color.g, color.b);
        sprite_sheet.texture.set_alpha_mod(color.a);

        self.canvas.copy_ex(&sprite_sheet.texture,
                            Some(src),
                            Some(dst),
                            0.0,
                            None,
                            false,
                            false).unwrap();
    }

    pub fn draw_char_with_rotation(&mut self,
                                   chr: char,
                                   pos: Pos,
                                   color: Color,
                                   area: &Area,
                                   angle: f64) {
        let chr_x = (chr as i32) % FONT_WIDTH;
        let chr_y = (chr as i32) / FONT_HEIGHT;

        let src = Rect::new((chr_x * FONT_WIDTH) as i32,
                            (chr_y * FONT_HEIGHT) as i32,
                            FONT_WIDTH as u32,
                            FONT_HEIGHT as u32);

        let dst = area.char_rect(pos.x, pos.y);

        self.font_image.set_color_mod(color.r, color.g, color.b);
        self.font_image.set_alpha_mod(color.a);

        self.canvas.copy_ex(&self.font_image,
                            Some(src),
                            Some(dst),
                            angle,
                            None,
                            false,
                            false).unwrap();
    }

    pub fn draw_char(&mut self,
                     chr: char,
                     pos: Pos,
                     color: Color,
                     area: &Area) {
        self.draw_char_with_rotation(chr, pos, color, area, 0.0);
    }

    pub fn highlight_tile(&mut self,
                          pos: Pos,
                          color: Color,
                          area: &Area) {
        self.canvas.set_draw_color(Sdl2Color::RGBA(color.r, color.g, color.b, color.a));

        let dst_rect = area.char_rect(pos.x, pos.y);
        self.canvas.fill_rect(dst_rect).unwrap();
    }


    pub fn draw_tile_edge(&mut self, pos: Pos, area: &Area, color: Color, dir: Cardinal) {
        self.canvas.set_draw_color(Sdl2Color::RGBA(color.r, color.g, color.b, color.a));

        let tile_rect = area.char_rect(pos.x, pos.y);
        let width = 5;

        let side_rect;
        match dir {
            Cardinal::Up => {
                side_rect = Rect::new(tile_rect.x() + 1,
                                      tile_rect.y + 2,
                                      tile_rect.width() - 1,
                                      width);
            }

            Cardinal::Down => {
                side_rect = Rect::new(tile_rect.x() + 1,
                                      tile_rect.y + tile_rect.height() as i32 - 2,
                                      tile_rect.width() - 1,
                                      width);
            }

            Cardinal::Left => {
                side_rect = Rect::new(tile_rect.x() + 2,
                                      tile_rect.y + 1,
                                      width,
                                      tile_rect.height());
            }

            Cardinal::Right => {
                side_rect = Rect::new(tile_rect.x() + tile_rect.width() as i32 - 2,
                                      tile_rect.y + 1,
                                      10,
                                      tile_rect.height());
            }
        }

        self.canvas.fill_rect(side_rect).unwrap();
    }


    pub fn draw_tile_outline(&mut self, pos: Pos, area: &Area, color: Color) {
        self.canvas.set_draw_color(Sdl2Color::RGBA(color.r, color.g, color.b, color.a));

        let tile_rect = area.char_rect(pos.x, pos.y);

        let inner_rect = Rect::new(tile_rect.x() + 1,
                                   tile_rect.y + 1,
                                   tile_rect.width() - 1,
                                   tile_rect.height() - 1);
        self.canvas.draw_rect(inner_rect).unwrap();
    }

    pub fn play_animation(&mut self, animation: Animation) -> AnimKey {
        let anim_key = self.next_anim_key;
        self.next_anim_key += 1;
        self.animations.insert(anim_key, animation);
        return anim_key;
    }

    pub fn play_effect(&mut self, effect: Effect) {
        self.effects.push(effect);
    }

    pub fn process_message(&mut self, msg: Msg, data: &mut GameData, config: &Config) {
        match msg {
            Msg::Sound(_cause_id, source_pos, radius, should_animate) => {
                if should_animate {
                    // NOTE this is a duplicate computation, also done in logic message processing
                    let sound_aoe =
                        data.map.aoe_fill(AoeEffect::Sound, source_pos, radius);

                    let player_id = data.find_player().unwrap();
                    let player_pos = data.entities.pos[&player_id];

                    // only play the sound effect if the player position is included
                    let sound_hits_player = sound_aoe.positions().iter().any(|pos| *pos == player_pos);
                    let sound_from_player = source_pos == player_pos;
                    let player_cant_see_source = !data.is_in_fov(player_id, source_pos, config);
                    if sound_from_player || (player_cant_see_source && sound_hits_player) {
                        let sound_effect = Effect::Sound(sound_aoe, 0.0);
                        self.play_effect(sound_effect);
                    }
                }
            }

            Msg::ItemThrow(_thrower, item_id, start, end) => {
                let sound_aoe = data.map.aoe_fill(AoeEffect::Sound, end, config.sound_radius_stone);

                let chr = data.entities.chr[&item_id];
                let item_sprite =
                    self.font_sprite(chr)
                        .expect("Could not find item sprite!");

                let move_anim = Animation::Between(item_sprite, start, end, 0.0, config.item_throw_speed);
                let item_anim = Animation::PlayEffect(Effect::Sound(sound_aoe, 0.0));
                let loop_anim = Animation::Loop(item_sprite);

                let move_key = self.play_animation(move_anim);
                let item_key = self.play_animation(item_anim);
                let loop_key = self.play_animation(loop_anim);

                data.entities.animation[&item_id].clear();
                data.entities.animation[&item_id].push_back(move_key);
                data.entities.animation[&item_id].push_back(item_key);
                data.entities.animation[&item_id].push_back(loop_key);
            }

            Msg::Moved(object_id, movement, _pos) => {
                let player_handle = data.find_player().unwrap();

                if object_id == player_handle {
                    if !matches!(movement.typ, MoveType::Pass) {
                        let idle_sprite =
                            self.new_sprite("player_idle".to_string(), config.idle_speed)
                                              .unwrap();
                        let idle_anim = Animation::Loop(idle_sprite);
                        let idle_key = self.play_animation(idle_anim);

                        data.entities.animation[&player_handle].clear();
                        data.entities.animation[&player_handle].push_back(idle_key);
                    }
                }
            }

            Msg::Killed(_attacker, attacked, _damage) => {
                if data.entities.typ[&attacked] != EntityType::Player {
                    data.entities.animation[&attacked].clear();

                    let sprite_name = format!("{:?}_die", data.entities.name[&attacked]);
                    let maybe_sprite = self.new_sprite(sprite_name, 1.0);
                    if let Some(sprite) = maybe_sprite {
                        let anim = self.play_animation(Animation::Once(sprite));
                        data.entities.animation[&attacked].clear();
                        data.entities.animation[&attacked].push_front(anim);
                    }
                }
            }

            Msg::Attack(attacker, _attacked, _damage) => {
                if data.entities.typ[&attacker] == EntityType::Player {
                    let attack_sprite =
                        self.new_sprite("player_attack".to_string(), config.player_attack_speed)
                                          .unwrap();
                    let attack_anim = Animation::Once(attack_sprite);
                    let attack_key = self.play_animation(attack_anim);

                    let idle_sprite =
                        self.new_sprite("player_idle".to_string(), config.idle_speed)
                                          .unwrap();
                    let idle_anim = Animation::Loop(idle_sprite);
                    let idle_key = self.play_animation(idle_anim);

                    data.entities.animation[&attacker].clear();
                    data.entities.animation[&attacker].push_back(attack_key);
                    data.entities.animation[&attacker].push_back(idle_key);
                }
            }

            Msg::JumpWall(_jumper, _start, _end) => {
                /* This animation does not work
                if data.entities.typ[&jumper] == EntityType::Player {
                    let jump_sprite =
                        self.new_sprite("player_vault".to_string(), config.player_vault_sprite_speed)
                                          .unwrap();
                    let jump_anim = Animation::Between(jump_sprite, start, end, 0.0, config.player_vault_move_speed);
                    let jump_key = self.play_animation(jump_anim);

                    let idle_sprite =
                        self.new_sprite("player_idle".to_string(), config.idle_speed)
                                          .unwrap();
                    let idle_anim = Animation::Loop(idle_sprite);
                    let idle_key = self.play_animation(idle_anim);

                    data.entities.animation[&jumper].clear();
                    data.entities.animation[&jumper].push_back(jump_key);
                    data.entities.animation[&jumper].push_back(idle_key);
                }
                */
            }

            Msg::SpawnedObject(entity_id, _typ, _pos, _name) => {
                if data.entities.typ[&entity_id] == EntityType::Player {
                    let sprite = self.new_sprite("player_idle".to_string(), config.idle_speed)
                                                    .expect("Could not find sprite 'player_idle'");

                    let anim_key = self.play_animation(Animation::Loop(sprite));

                    data.entities.animation[&entity_id].push_front(anim_key);
                } else if data.entities.name[&entity_id] == EntityName::Key {
                    let sprite = self.new_sprite("key".to_string(), config.key_speed)
                                                     .expect("Could not find sprite 'key'");

                    let anim_key = self.play_animation(Animation::Loop(sprite));

                    data.entities.animation[&entity_id].push_front(anim_key);

                } else if data.entities.name[&entity_id] == EntityName::Spike {
                    let sprite = self.new_sprite("spikes".to_string(), config.idle_speed)
                                                     .expect("Could not find sprite 'spikes'");

                    let anim_key = self.play_animation(Animation::Loop(sprite));

                    data.entities.animation[&entity_id].push_front(anim_key);
                } else if data.entities.name[&entity_id] == EntityName::Pawn {
                    let sprite =  self.new_sprite("elf_idle".to_string(), config.idle_speed)
                                                     .expect("Could not find sprite 'elf_idle'");
                    let anim_key = self.play_animation(Animation::Loop(sprite));

                    data.entities.animation[&entity_id].push_front(anim_key);
                } else if data.entities.name[&entity_id] == EntityName::Gol {
                    let sprite = self.new_sprite("gol_idle".to_string(), config.idle_speed)
                                                     .expect("Could not find sprite 'gol_idle'");

                    let anim_key = self.play_animation(Animation::Loop(sprite));

                    data.entities.animation[&entity_id].push_front(anim_key);
                }
            }

            Msg::PlayerTurn() => {
                let player_id = data.find_player().unwrap();

                self.prev_turn_fov.clear();
                self.prev_turn_fov.extend(self.current_turn_fov.iter());
                self.current_turn_fov.clear();

                for entity_id in data.entities.ids.clone() {
                    let pos = data.entities.pos[&entity_id];
                    let name = data.entities.name[&entity_id];
                    let in_fov = data.is_in_fov(player_id, pos, config);
                    let player_pos = data.entities.pos[&player_id];
                    println!("{:?} in fov {} ({} to {})", name, in_fov, player_pos, pos);
                    if entity_id != player_id && data.is_in_fov(player_id, pos, config) {
                        self.current_turn_fov.push(entity_id);
                    }
                }

                println!("prev {}, cur {}", self.prev_turn_fov.len(), self.current_turn_fov.len());

                for entity_id in self.prev_turn_fov.iter() {
                    if data.entities.typ.get(entity_id) != Some(&EntityType::Enemy) {
                        continue;
                    }

                    let pos = data.entities.pos[entity_id];
                    if !data.is_in_fov(player_id, pos, config) {
                        if let Some(sprite) = self.drawn_sprites.get(entity_id) {
                            self.impressions.push(Impression::new(*sprite, pos));
                            println!("adding impression");
                        }
                    }
                }

                /* Remove impressions that are currently visible */
                let mut impressions_visible = Vec::new();
                for (index, impression) in self.impressions.iter().enumerate() {
                    if data.is_in_fov(player_id, impression.pos, config) {
                        impressions_visible.push(index);
                    }
                }
                println!("removing {} impressions", impressions_visible.len());
                impressions_visible.sort();
                impressions_visible.reverse();
                for index in impressions_visible.iter() {
                    self.impressions.swap_remove(*index);
                }
            }

            _ => {
            }
        }
    }
}


pub struct SpriteSheet {
    pub texture: Texture,
    pub name: String,
    pub num_sprites: usize,
    pub rows: usize,
}

impl SpriteSheet {
    pub fn new(name: String, texture: Texture, rows: usize) -> SpriteSheet {
        let num_sprites_per_row = texture.query().width as usize / FONT_WIDTH as usize;
        let num_sprites = num_sprites_per_row * rows;

        return SpriteSheet {
            texture,
            name,
            num_sprites,
            rows,
        };
    }

    pub fn sprites_per_row(&self) -> usize {
         return self.num_sprites / self.rows;
    }
}


#[derive(PartialEq, Debug, Copy, Clone)]
pub struct Area {
    pub x_offset: i32,
    pub y_offset: i32,
    pub width: usize,
    pub height: usize,
    pub font_width: usize,
    pub font_height: usize,
}

impl Area {
    pub fn new(x_offset: i32,
               y_offset: i32,
               width: usize,
               height: usize,
               font_width: usize,
               font_height: usize) -> Area {
        Area { x_offset,
               y_offset,
               width,
               height,
               font_width,
               font_height,
        }
    }

    pub fn char_rect(&self, x: i32, y: i32) -> Rect {
        return Rect::new(self.x_offset + x * self.font_width as i32,
                         self.y_offset + y * self.font_height as i32,
                         self.font_width as u32,
                         self.font_height as u32);
    }

    pub fn get_rect(&self) -> Rect {
        return Rect::new(self.x_offset,
                         self.y_offset,
                         self.width as u32,
                         self.height as u32);
    }
}


pub fn engine_color(color: &Color) -> Sdl2Color {
    return Sdl2Color::RGBA(color.r, color.g, color.b, color.a);
}

pub fn draw_char(canvas: &mut WindowCanvas,
                 font_image: &mut Texture,
                 chr: char,
                 pos: Pos,
                 color: Color,
                 area: &Area) {
    let chr_x = (chr as i32) % FONT_WIDTH;
    let chr_y = (chr as i32) / FONT_HEIGHT;

    let src = Rect::new((chr_x * FONT_WIDTH) as i32,
                        (chr_y * FONT_HEIGHT) as i32,
                        FONT_WIDTH as u32,
                        FONT_HEIGHT as u32);

    let dst = area.char_rect(pos.x, pos.y);

    font_image.set_color_mod(color.r, color.g, color.b);
    font_image.set_alpha_mod(color.a);

    canvas.copy_ex(font_image,
                   Some(src),
                   Some(dst),
                   0.0,
                   None,
                   false,
                   false).unwrap();
}
