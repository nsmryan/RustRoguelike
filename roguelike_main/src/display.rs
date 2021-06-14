use bmp::Image;

use sdl2::render::{Texture, WindowCanvas, TextureCreator, BlendMode};
use sdl2::video::WindowContext;
use sdl2::rect::{Rect};
use sdl2::pixels::{PixelFormatEnum, Color as Sdl2Color};

use indexmap::map::IndexMap;

use roguelike_core::types::*;
use roguelike_core::constants::*;
use roguelike_core::config::*;
use roguelike_core::messaging::*;
use roguelike_core::map::*;
use roguelike_core::animation::{Sprite, AnimKey, Effect, SpriteKey, Animation, SpriteAnim, SpriteIndex};
use roguelike_core::utils::aoe_fill;


pub struct Display {
    pub state: DisplayState,
    pub targets: DisplayTargets,
    pub mouse_state: MouseState,
}

impl Display {
    pub fn new(canvas: WindowCanvas) -> Display {
        return Display { state: DisplayState::new(),
                         targets: DisplayTargets::new(canvas),
                         mouse_state: Default::default(),
        };
    }

    pub fn update_display(&mut self) {
        self.targets.canvas_panel.target.present();
    }

    pub fn save_screenshot(&mut self, name: &str) {
        let format = PixelFormatEnum::RGB24;
        let (width, height) = self.targets.canvas_panel.target.output_size().unwrap();

        let pixels = self.targets.canvas_panel.target.read_pixels(None, format).unwrap();

        let mut shot = Image::new(width, height);

        for x in 0..width {
            for y in 0..height {
                let index = (x + y * width) as usize * 3;
                let pixel = bmp::Pixel::new(pixels[index + 0],
                                            pixels[index + 1],
                                            pixels[index + 2]);
                shot.set_pixel(x, y, pixel);
            }
        }

        shot.save(format!("{}.bmp", name)).unwrap();
    }

    pub fn add_spritesheet(&mut self, name: String, texture: Texture, rows: usize) {
        let sprite_sheet = SpriteSheet::new(name, texture, rows);
        let sprite_key = self.state.next_sprite_key;
        self.state.next_sprite_key += 1;
        self.state.sprites.insert(sprite_key, sprite_sheet);
    }

    pub fn sprite_exists(&self, name: &str) -> bool {
        for (_key, sprite_sheet) in self.state.sprites.iter() {
            if sprite_sheet.name == *name {
                return true;
            }
        }

        return false;
    }

    /// Create a sprite by looking up a texture and constructing the
    /// SpriteAnim structure.
    pub fn new_sprite(&self, name: String, speed: f32) -> SpriteAnim {
        let sprite_key = self.state.lookup_spritekey(&name);
        let max_index = self.state.sprites[&sprite_key].num_sprites;
        return SpriteAnim::make_anim(name, sprite_key, max_index as f32, speed);
    }

    pub fn font_sprite(&self, chr: char) -> SpriteAnim {
        let sprite_key = self.state.lookup_spritekey(&"font".to_string());
        return SpriteAnim::new(format!("{}", chr),
                               sprite_key,
                               chr as i32 as SpriteIndex,
                               chr as i32 as SpriteIndex,
                               0.0);
    }

    /// Create and play a looping sprite
    pub fn loop_sprite(&mut self, sprite_name: &str, speed: f32) -> AnimKey {
        let sprite_anim = self.new_sprite(sprite_name.to_string(), speed);
        
        let anim = Animation::Loop(sprite_anim);

        let key = self.play_animation(anim);

        return key;
    }

    pub fn get_idle_animation(&mut self, entity_id: EntityId, data: &mut GameData, config: &Config) -> Option<AnimKey> {
        let player_id = data.find_by_name(EntityName::Player).unwrap();

        if entity_id == player_id {
            let stance = data.entities.stance[&entity_id];

            let key;
            if data.using(entity_id, Item::Dagger) && stance == Stance::Crouching {
                key = self.loop_sprite("player_crouch_dagger", config.idle_speed);
            } else if data.using(entity_id, Item::Dagger) {
                key = self.loop_sprite("player_idle_dagger", config.idle_speed);
            } else if data.using(entity_id, Item::Hammer) {
                key = self.loop_sprite("player_idle_hammer", config.idle_speed);
            } else if data.using(entity_id, Item::Shield) {
                key = self.loop_sprite("player_idle_shield", config.idle_speed);
            } else if stance == Stance::Crouching {
                key = self.loop_sprite("player_crouching", config.idle_speed);
            } else {
                key = self.loop_sprite("player_idle", config.idle_speed);
            }
            return Some(key);
        }

        return None;
    }

    /// Add an animation to the current animation system, returning
    /// a key used to reference this animation
    pub fn play_animation(&mut self, animation: Animation) -> AnimKey {
        let anim_key = self.state.next_anim_key;
        self.state.next_anim_key += 1;
        self.state.animations.insert(anim_key, animation);
        return anim_key;
    }

    pub fn clear_level_state(&mut self) {
        self.state.impressions.clear();
        self.state.prev_turn_fov.clear();
        self.state.current_turn_fov.clear();
        self.state.sound_tiles.clear();
    }

    pub fn process_message(&mut self, msg: Msg, data: &mut GameData, config: &Config) {
        match msg {
            Msg::StartTurn => {
                self.state.sound_tiles.clear();
            }

            Msg::Sound(cause_id, source_pos, radius, should_animate) => {
                if should_animate {
                    // NOTE this is a duplicate computation, also done in logic message processing
                    let sound_aoe =
                        aoe_fill(&data.map, AoeEffect::Sound, source_pos, radius, config);

                    // Add to this turn's sound tiles list
                    self.state.sound_tiles.extend(sound_aoe.positions().iter());

                    let player_id = data.find_by_name(EntityName::Player).unwrap();
                    let player_pos = data.entities.pos[&player_id];

                    // only play the sound effect if the player position is included
                    let sound_hits_player = sound_aoe.positions().iter().any(|pos| *pos == player_pos);
                    let sound_from_monster = data.entities.typ.get(&cause_id) == Some(&EntityType::Enemy);
                    let player_can_see_source = data.is_in_fov(player_id, cause_id, config);
                    let visible_monster_sound = sound_from_monster && player_can_see_source;
                    if !visible_monster_sound && sound_hits_player {
                        let sound_effect = Effect::Sound(sound_aoe, 0.0);
                        self.state.play_effect(sound_effect);
                    }
                }
            }

            Msg::ItemThrow(_thrower, item_id, start, _end) => {
                // NOTE we use the entities position instead of 'end' because we
                // want where it hit, not where it was thrown to.
                let end = data.entities.pos[&item_id];

                let sound_aoe = aoe_fill(&data.map, AoeEffect::Sound, end, config.sound_radius_stone, config);

                let chr = data.entities.chr[&item_id];
                let item_sprite = self.font_sprite(chr);

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

            Msg::PickedUp(entity_id, _item_id) => {
                if let Some(anim_key) = self.get_idle_animation(entity_id, data, config) {
                    data.entities.set_animation(entity_id, anim_key);
                }
            }

            Msg::Moved(entity_id, _move_type, _pos) => {
                if let Some(anim_key) = self.get_idle_animation(entity_id, data, config) {
                    data.entities.set_animation(entity_id, anim_key);
                }
            }

            Msg::Killed(_attacker, attacked, _damage) => {
                if data.entities.typ[&attacked] != EntityType::Player {
                    data.entities.animation[&attacked].clear();

                    let sprite_name = format!("{:?}_die", data.entities.name[&attacked]);
                    if self.sprite_exists(&sprite_name) {
                        let sprite = self.new_sprite(sprite_name, 1.0);
                        let anim = self.play_animation(Animation::Once(sprite));
                        data.entities.animation[&attacked].clear();
                        data.entities.animation[&attacked].push_front(anim);
                    }
                }
            }

            Msg::HammerSwing(entity_id, _pos) => {
                if data.entities.typ[&entity_id] == EntityType::Player {
                    let attack_sprite =
                        self.new_sprite("player_attack_hammer".to_string(), config.player_attack_hammer_speed);
                    let attack_anim = Animation::Once(attack_sprite);
                    let attack_key = self.play_animation(attack_anim);

                    data.entities.animation[&entity_id].clear();
                    data.entities.animation[&entity_id].push_back(attack_key);

                    if let Some(idle_key) = self.get_idle_animation(entity_id, data, config) {
                        data.entities.animation[&entity_id].push_back(idle_key);
                    }
                }
            }

            Msg::Stabbed(entity_id, _hit_entity) => {
                if data.entities.typ[&entity_id] == EntityType::Player {
                    let attack_sprite =
                        self.new_sprite("player_attack_dagger".to_string(), config.player_attack_speed);
                    let attack_anim = Animation::Once(attack_sprite);
                    let attack_key = self.play_animation(attack_anim);

                    data.entities.animation[&entity_id].clear();
                    data.entities.animation[&entity_id].push_back(attack_key);

                    if let Some(idle_key) = self.get_idle_animation(entity_id, data, config) {
                        data.entities.animation[&entity_id].push_back(idle_key);
                    }
                }
            }

            Msg::SwordSwing(entity_id, _pos) => {
                if data.entities.typ[&entity_id] == EntityType::Player {
                    if let Some(idle_key) = self.get_idle_animation(entity_id, data, config) {
                        data.entities.animation[&entity_id].clear();
                        data.entities.animation[&entity_id].push_back(idle_key);
                    }
                }
            }

            Msg::Attack(attacker, _attacked, _damage) => {
                if data.entities.typ[&attacker] == EntityType::Player {
                    let attack_sprite =
                        self.new_sprite("player_attack".to_string(), config.player_attack_speed);
                    let attack_anim = Animation::Once(attack_sprite);
                    let attack_key = self.play_animation(attack_anim);


                    data.entities.animation[&attacker].clear();
                    data.entities.animation[&attacker].push_back(attack_key);

                    if let Some(idle_key) = self.get_idle_animation(attacker, data, config) {
                        data.entities.animation[&attacker].push_back(idle_key);
                    }
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

            Msg::SpawnedObject(entity_id, _typ, _pos, _name, _facing) => {
                if data.entities.ids.contains(&entity_id) {
                    let mut anim_desc = None;
                    if data.entities.typ[&entity_id] == EntityType::Player {
                        anim_desc = Some(("player_idle", config.idle_speed));
                    } else if data.entities.name[&entity_id] == EntityName::Key {
                        anim_desc = Some(("key", config.key_speed));
                    } else if data.entities.name[&entity_id] == EntityName::SpikeTrap {
                        anim_desc = Some(("spikes", config.idle_speed));
                    } else if data.entities.name[&entity_id] == EntityName::Pawn {
                        //anim_desc = Some(("elf_idle", config.idle_speed));
                    } else if data.entities.name[&entity_id] == EntityName::Gol {
                        //anim_desc = Some(("gol_idle", config.idle_speed));
                    }

                    if let Some((name, speed)) = anim_desc {
                        let anim_key = self.loop_sprite(name, speed);
                        data.entities.animation[&entity_id].push_front(anim_key);
                    }
                }
            }

            Msg::PlayerTurn => {
                let player_id = data.find_by_name(EntityName::Player).unwrap();

                self.state.prev_turn_fov.clear();
                self.state.prev_turn_fov.extend(self.state.current_turn_fov.iter());
                self.state.current_turn_fov.clear();

                for entity_id in data.entities.ids.clone() {
                    if entity_id != player_id && data.is_in_fov(player_id, entity_id, config) {
                        self.state.current_turn_fov.push(entity_id);
                    }
                }

                for entity_id in self.state.prev_turn_fov.iter() {
                    if data.entities.typ.get(entity_id) != Some(&EntityType::Enemy) {
                        continue;
                    }

                    if !data.is_in_fov(player_id, *entity_id, config) {
                        if let Some(sprite) = self.state.drawn_sprites.get(entity_id) {
                            let pos = data.entities.pos[entity_id];
                            self.state.impressions.push(Impression::new(*sprite, pos));
                        }
                    }
                }

                /* Remove impressions that are currently visible */
                let mut impressions_visible = Vec::new();
                for (index, impression) in self.state.impressions.iter().enumerate() {
                    if data.pos_in_fov(player_id, impression.pos, config) {
                        impressions_visible.push(index);
                    }
                }
                impressions_visible.sort();
                impressions_visible.reverse();
                for index in impressions_visible.iter() {
                    self.state.impressions.swap_remove(*index);
                }
            }

            _ => {
            }
        }
    }
}

type TextureKey = u64;

#[derive(Copy, Clone, PartialEq, Debug, Default)]
pub struct MouseState {
    pub x: i32,
    pub y: i32,
    pub left_pressed: bool,
    pub middle_pressed: bool,
    pub right_pressed: bool,
    pub wheel: f32,
}

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug)]
pub struct Area {
    pub x_offset: usize,
    pub y_offset: usize,
    pub width: usize,
    pub height: usize,
}

impl Area {
    pub fn new(width: usize, height: usize) -> Area {
        return Area { x_offset: 0, y_offset: 0, width, height };
    }

    pub fn new_at(x_offset: usize, y_offset: usize, width: usize, height: usize) -> Area {
        return Area { x_offset, y_offset, width, height };
    }

    pub fn split_left(&self, left_width: usize) -> (Area, Area) {
        assert!(left_width <= self.width);

        let right_width = self.width - left_width;
        let left = Area::new_at(self.x_offset, self.y_offset, left_width, self.height);
        let right = Area::new_at(self.x_offset + left_width, self.y_offset, right_width, self.height);

        return (left, right);
    }

    pub fn split_right(&self, right_width: usize) -> (Area, Area) {
        assert!(right_width <= self.width);

        let left_width = self.width - right_width;
        let left = Area::new_at(self.x_offset, self.y_offset, left_width, self.height);
        let right = Area::new_at(self.x_offset + left_width, self.y_offset, right_width, self.height);

        return (left, right);
    }

    pub fn split_top(&self, top_height: usize) -> (Area, Area) {
        assert!(top_height <= self.height);

        let top = Area::new_at(self.x_offset, self.y_offset, self.width, top_height);
        let bottom = Area::new_at(self.x_offset, self.y_offset + top_height, self.width, self.height - top_height);

        return (top, bottom);
    }

    pub fn split_bottom(&self, bottom_height: usize) -> (Area, Area) {
        assert!(bottom_height <= self.height);

        let top_height = self.height - bottom_height;
        let top = Area::new_at(self.x_offset, self.y_offset, self.width, top_height);
        let bottom = Area::new_at(self.x_offset, self.y_offset + top_height, self.width, bottom_height);

        return (top, bottom);
    }

    pub fn centered(&self, width: usize, height: usize) -> Area {
        assert!(width <= self.width);
        assert!(height <= self.height);

        let x_offset = (self.width - width) / 2;
        let y_offset = (self.height - height) / 2;

        return Area::new_at(x_offset, y_offset, width, height);
    }

    pub fn cell_at_pixel(&self, pixel_pos: Pos) -> Option<(usize, usize)> {
        let cell_pos = Pos::new(pixel_pos.x / self.width as i32, pixel_pos.y / self.height as i32);

        return self.cell_at(cell_pos);
    }

    pub fn cell_at(&self, cell_pos: Pos) -> Option<(usize, usize)> {
        if cell_pos.x as usize >= self.x_offset && (cell_pos.x as usize) < self.x_offset + self.width &&
           cell_pos.y as usize >= self.y_offset && (cell_pos.y as usize) < self.y_offset + self.height {
               return Some((cell_pos.x as usize - self.x_offset, cell_pos.y as usize - self.y_offset));
        }

        return None;
    }
}

#[test]
pub fn test_area_splits_left() {
    let section = Area::new(100, 100);
    let (left, right) = section.split_left(20);

    assert_eq!(0, left.x_offset);
    assert_eq!(0, left.y_offset);
    assert_eq!(20, right.x_offset);
    assert_eq!(0, right.y_offset);

    assert_eq!(20, left.width);
    assert_eq!(80, right.width);
    assert_eq!(100, left.height);
    assert_eq!(100, right.height);
}

#[test]
pub fn test_area_splits_top() {
    let section = Area::new(100, 100);
    let (top, bottom) = section.split_top(20);

    assert_eq!(0, top.x_offset);
    assert_eq!(0, top.y_offset);
    assert_eq!(0, bottom.x_offset);
    assert_eq!(20, bottom.y_offset);

    assert_eq!(100, top.width);
    assert_eq!(100, bottom.width);
    assert_eq!(20, top.height);
    assert_eq!(80, bottom.height);
}

#[test]
pub fn test_area_splits_right() {
    let section = Area::new(100, 100);
    let (left, right) = section.split_right(20);

    assert_eq!(0, left.x_offset);
    assert_eq!(0, left.y_offset);
    assert_eq!(80, right.x_offset);
    assert_eq!(0, right.y_offset);

    assert_eq!(80, left.width);
    assert_eq!(20, right.width);
    assert_eq!(100, left.height);
    assert_eq!(100, right.height);
}

#[test]
pub fn test_area_splits_bottom() {
    let section = Area::new(100, 100);
    let (top, bottom) = section.split_bottom(20);
    assert_eq!(0, top.x_offset);
    assert_eq!(0, top.y_offset);
    assert_eq!(0, bottom.x_offset);
    assert_eq!(80, bottom.y_offset);

    assert_eq!(100, top.width);
    assert_eq!(100, bottom.width);
    assert_eq!(80, top.height);
    assert_eq!(20, bottom.height);
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Panel<T> {
    pub target: T,
    pub cells: (u32, u32),
    pub num_pixels: (u32, u32),
    pub dirty: bool,
}

impl Panel<Texture> {
    pub fn from_dims(texture_creator: &TextureCreator<WindowContext>, width: u32, height: u32, over_sample: u32) -> Panel<Texture> {
        let pixel_format = texture_creator.default_pixel_format();

        let tex =
            texture_creator.create_texture_target(pixel_format,
                                                  width as u32 * FONT_WIDTH as u32 * over_sample,
                                                  height as u32 * FONT_HEIGHT as u32 * over_sample).unwrap();
        let panel = Panel::with_texture((width as u32, height as u32), tex);

        return panel;
    }

    pub fn with_texture(cells: (u32, u32), texture: Texture) -> Panel<Texture> {
        let query = texture.query();
        let width = query.width;
        let height = query.height;
        return Panel { cells, target: texture, num_pixels: (width, height), dirty: true };
    }

    pub fn unit(&self) -> Panel<()> {
        return Panel { target: (), cells: self.cells, num_pixels: self.num_pixels, dirty: self.dirty };
    }
}

impl Panel<WindowCanvas> {
    pub fn with_canvas(cells: (u32, u32), canvas: WindowCanvas) -> Panel<WindowCanvas> {
        let (width, height) = canvas.output_size().unwrap();
        return Panel { cells, target: canvas, num_pixels: (width, height), dirty: true };
    }
}

impl<T> Panel<T> {
    pub fn cell_dims(&self) -> (u32, u32) {
        return (self.num_pixels.0 / self.cells.0, self.num_pixels.1 / self.cells.1);
    }

    pub fn with_target<S>(&self, target: S) -> Panel<S> {
        return Panel {
            target,
            cells: self.cells,
            num_pixels: self.num_pixels,
            dirty: true,
        };
    }

    pub fn area(&self) -> Area {
        return Area::new(self.cells.0 as usize, self.cells.1 as usize);
    }

    pub fn get_rect_up_left(&self, width: usize, height: usize) -> Rect {
        assert!(width as u32 <= self.cells.0);
        assert!(height as u32 <= self.cells.1);

        let (cell_width, cell_height) = self.cell_dims();

        let pixel_width = width as u32 * cell_width;
        let pixel_height = height as u32 * cell_height;

        return Rect::new(0, 0, pixel_width, pixel_height);
    }

    pub fn get_rect_from_area(&self, area: &Area) -> Rect {
        let cell_dims = self.cell_dims();

        let x_offset = area.x_offset as f32 * cell_dims.0 as f32;
        let y_offset = area.y_offset as f32 * cell_dims.1 as f32;

        let width = (area.width as f32 * cell_dims.0 as f32) as u32;
        let height = (area.height as f32 * cell_dims.1 as f32) as u32;

        // don't draw off the screen
        assert!(x_offset as u32 + width <= self.num_pixels.0);
        assert!(y_offset as u32 + height <= self.num_pixels.1);

        return Rect::new(x_offset as i32, y_offset as i32, width, height);
    }

    pub fn get_rect_within(&self, area: &Area, target_dims: (u32, u32)) -> Rect {
        let (target_width, target_height) = target_dims;
        let base_rect = self.get_rect_from_area(area);

        let scale_x = base_rect.w as f32 / target_width as f32;
        let scale_y = base_rect.h as f32 / target_height as f32;

        let scaler;
        if scale_x * target_height as f32 > base_rect.h as f32 {
            scaler = scale_y;
        } else {
            scaler = scale_x;
        }

        let final_target_width = target_width as f32 * scaler;
        let final_target_height = target_height as f32 * scaler;

        let x_inner_offset = (base_rect.w as f32 - final_target_width) / 2.0;
        let y_inner_offset = (base_rect.h as f32 - final_target_height) / 2.0;
        let x_offset = base_rect.x + x_inner_offset as i32;
        let y_offset = base_rect.y + y_inner_offset as i32;

        // check that we don't reach past the destination rect we should be drawing within
        assert!((x_offset as f32 + final_target_width  as f32) <= base_rect.x as f32 + base_rect.w as f32);
        assert!((y_offset as f32 + final_target_height as f32) <= base_rect.y as f32 + base_rect.h as f32);

        return Rect::new(x_offset as i32,
                         y_offset as i32,
                         final_target_width as u32,
                         final_target_height as u32);
    }
}


pub struct DisplayTargets {
    pub canvas_panel: Panel<WindowCanvas>,

    pub background_panel: Panel<Texture>,
    pub map_panel: Panel<Texture>,
    pub player_panel: Panel<Texture>,
    pub info_panel: Panel<Texture>,
    pub inventory_panel: Panel<Texture>,
    pub menu_panel: Panel<Texture>,

    pub texture_creator: TextureCreator<WindowContext>,

    pub screen_area: Area,
    pub map_area: Area,
    pub player_area: Area,
    pub remaining_area: Area,
    pub inventory_area: Area,
    pub info_area: Area,
    pub menu_area: Area,
}

impl DisplayTargets {
    pub fn new(canvas: WindowCanvas) -> DisplayTargets {

        let texture_creator = canvas.texture_creator();

        let over_sample = 5;
        let background_panel = Panel::from_dims(&texture_creator, MAP_WIDTH as u32, MAP_HEIGHT as u32, over_sample);

        let map_panel = Panel::from_dims(&texture_creator, MAP_WIDTH as u32, MAP_HEIGHT as u32, over_sample);

        let info_width = 14;

        let info_panel = Panel::from_dims(&texture_creator, info_width, 15, 1);
        let inventory_panel = Panel::from_dims(&texture_creator, info_width, 15, 1);
        let player_panel = Panel::from_dims(&texture_creator, info_width, 20, 1);
        let menu_panel = Panel::from_dims(&texture_creator, info_width + 5, 20, 1);
        let canvas_panel = Panel::with_canvas((SCREEN_WIDTH / FONT_WIDTH as u32, SCREEN_HEIGHT / FONT_HEIGHT as u32), canvas);

        let screen_area = canvas_panel.area();
        let (map_area, rest_area) = screen_area.split_right(info_panel.cells.0 as usize);
        let (player_area, remaining_area) = rest_area.split_top(20);
        let (inventory_area, info_area) = remaining_area.split_top(15);

        let menu_area = menu_panel.area();
        let menu_area = map_area.centered(menu_area.width, menu_area.height);

        return DisplayTargets {
            canvas_panel,
            texture_creator,
            background_panel,
            map_panel,
            player_panel,
            info_panel,
            menu_panel,
            inventory_panel,

            screen_area,
            map_area,
            player_area,
            remaining_area,
            inventory_area,
            info_area,
            menu_area,
        };
    }

    pub fn mouse_pos(&self, x: i32, y: i32, map_width: i32, map_height: i32) -> Option<(i32, i32)> {
        let map_rect = self.canvas_panel.get_rect_from_area(&self.map_area);
        return cell_within_rect(map_rect, (map_width, map_height), (x, y));
    }
}

pub struct DisplayState {
    // sprite state
    pub sprites: IndexMap<SpriteKey, SpriteSheet>,
    pub next_sprite_key: i64,

    // currently active effects
    pub effects: Vec<Effect>,

    // animation information
    pub animations: IndexMap<AnimKey, Animation>,
    pub next_anim_key: i64,


    // sprites drawn this frame
    pub drawn_sprites: IndexMap<EntityId, Sprite>,

    // impressions left on map
    pub impressions: Vec<Impression>,

    // FOV information used when drawing
    pub prev_turn_fov: Vec<EntityId>,
    pub current_turn_fov: Vec<EntityId>,

    // tiles that heard a sound
    pub sound_tiles: Vec<Pos>,
}

impl DisplayState {
    pub fn new() -> DisplayState {

        return DisplayState {
            sprites: IndexMap::new(),
            next_sprite_key: 0,
            effects: Vec::new(),
            animations: IndexMap::new(),
            next_anim_key: 0,
            drawn_sprites: IndexMap::new(),
            impressions: Vec::new(),
            prev_turn_fov: Vec::new(),
            current_turn_fov: Vec::new(),
            sound_tiles: Vec::new(),
        };
    }

    pub fn lookup_spritekey(&self, name: &str) -> SpriteKey {
        for (key, sprite_sheet) in self.sprites.iter() {
            if sprite_sheet.name == *name {
                return *key;
            }
        }

        //Msg::SpawnedObject(entity_id, _typ, _pos, _name) => {
        //    if config.idle_animations && data.entities.ids.contains(&entity_id) {
        //        let mut anim_desc = None;
        //        if data.entities.typ[&entity_id] == EntityType::Player {
        //            anim_desc = Some(("player_idle", config.idle_speed));
        //        } else if data.entities.name[&entity_id] == EntityName::Key {
        //            anim_desc = Some(("key", config.key_speed));
        //        } else if data.entities.name[&entity_id] == EntityName::SpikeTrap {
        //            anim_desc = Some(("spikes", config.idle_speed));
        //        } else if data.entities.name[&entity_id] == EntityName::Pawn {
        //            anim_desc = Some(("elf_idle", config.idle_speed));
        //        } else if data.entities.name[&entity_id] == EntityName::Gol {
        //            anim_desc = Some(("gol_idle", config.idle_speed));
        //        }
        panic!(format!("Could not find sprite '{}'", name));
    }

    pub fn update_animations(&mut self, dt: f32) {
        for anim in self.animations.values_mut() {
            if let Some(sprite_anim) = anim.sprite_anim_mut() {
                sprite_anim.step(dt);
            }
        }
    }

    pub fn draw_sprite(&mut self,
                       panel: &mut Panel<&mut WindowCanvas>,
                       sprite: Sprite,
                       pos: Pos,
                       color: Color) {

        let sprite_index;
        let sprite_key;
        match sprite {
            Sprite::Sprite(index, key) => {
                sprite_index = index as usize;
                sprite_key = key;
            },

            Sprite::Char(chr) => {
                sprite_index = chr as usize;
                sprite_key = self.lookup_spritekey("tiles");
            },
        }

        let sprite = &mut self.sprites[&sprite_key];
        sprite.draw_sprite_at_cell(panel, sprite_index, pos, color, 0.0);
    }

    pub fn play_effect(&mut self, effect: Effect) {
        self.effects.push(effect);
    }
}


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


pub struct SpriteSheet {
    pub texture: Texture,
    pub name: String,
    pub num_sprites: usize,
    pub rows: usize,
    pub width: usize,
    pub height: usize,
}

impl SpriteSheet {
    pub fn new(name: String, texture: Texture, rows: usize) -> SpriteSheet {
        let tex_info = texture.query();
        let width = tex_info.width as usize;
        let height = tex_info.height as usize;

        let num_sprites_per_row = width / FONT_WIDTH as usize;
        let num_sprites = num_sprites_per_row * rows;

        return SpriteSheet {
            texture,
            name,
            num_sprites,
            rows,
            width,
            height,
        };
    }

    pub fn sprites_per_row(&self) -> usize {
         return self.num_sprites / self.rows;
    }

    pub fn num_cells(&self) -> (usize, usize) {
        return (self.sprites_per_row(), self.rows);
    }

    pub fn num_pixels(&self) -> (usize, usize) {
        return (self.width, self.height);
    }

    pub fn sprite_dims(&self) -> (usize, usize) {
        let (num_width, num_height) = self.num_cells();
        return (self.width / num_width, self.height / num_height);
    }

    pub fn draw_text_list(&mut self,
                         panel: &mut Panel<&mut WindowCanvas>,
                         text_list: &Vec<String>,
                         cell: Pos,
                         color: Color) {
        for (index, text) in text_list.iter().enumerate() {
            let text_cell = Pos::new(cell.x, cell.y + index as i32);
            self.draw_text(panel, text, text_cell, color);
        }
    }

    pub fn draw_text(&mut self,
                     panel: &mut Panel<&mut WindowCanvas>,
                     text: &str,
                     cell: Pos,
                     color: Color) {
        let mut pos = cell;
        for chr in text.chars() {
            self.draw_sprite_at_cell(panel, chr as usize, pos, color, 0.0);
            pos.x += 1;
        }
    }

    // consider removing and just converting chars to usize
    pub fn draw_char(&mut self,
                     panel: &mut Panel<&mut WindowCanvas>,
                     chr: char,
                     cell: Pos,
                     color: Color) {
        self.draw_sprite_at_cell(panel, chr as usize, cell, color, 0.0);
    }

    pub fn draw_sprite_at_cell(&mut self,
                               panel: &mut Panel<&mut WindowCanvas>,
                               index: usize,
                               cell: Pos,
                               color: Color,
                               rotation: f64) {
        let (cell_width, cell_height) = panel.cell_dims();

        let pos = Pos::new(cell.x * cell_width as i32, cell.y * cell_height as i32);

        self.draw_sprite_full(panel, index, pos, color, rotation);
    }

    pub fn draw_sprite_full(&mut self,
                            panel: &mut Panel<&mut WindowCanvas>,
                            index: usize,
                            pos: Pos,
                            color: Color,
                            rotation: f64) {
        let cell_dims = panel.cell_dims();

        let (num_cells_x, _num_cells_y) = self.num_cells();
        let sprite_x = index % num_cells_x;
        let sprite_y = index / num_cells_x;

        let (sprite_width, sprite_height) = self.sprite_dims();
        let src = Rect::new((sprite_x * sprite_width) as i32,
                            (sprite_y * sprite_height) as i32,
                            sprite_width as u32,
                            sprite_height as u32);

        let (cell_width, cell_height) = cell_dims;

        let dst = Rect::new(pos.x as i32,
                            pos.y as i32,
                            cell_width as u32,
                            cell_height as u32);

        panel.target.set_blend_mode(BlendMode::Blend);
        self.texture.set_color_mod(color.r, color.g, color.b);
        self.texture.set_alpha_mod(color.a);

        panel.target.copy_ex(&self.texture,
                       Some(src),
                       Some(dst),
                       rotation,
                       None,
                       false,
                       false).unwrap();
    }
}

pub fn engine_color(color: &Color) -> Sdl2Color {
    return Sdl2Color::RGBA(color.r, color.g, color.b, color.a);
}

pub fn draw_outline_tile(panel: &mut Panel<&mut WindowCanvas>,
                         cell: Pos,
                         color: Color) {
    let cell_dims = panel.cell_dims();

    panel.target.set_blend_mode(BlendMode::Add);
    panel.target.set_draw_color(Sdl2Color::RGBA(color.r, color.g, color.b, color.a));

    let rect = Rect::new(cell.x * cell_dims.0 as i32 + 1,
                         cell.y * cell_dims.1 as i32 + 1,
                         cell_dims.0,
                         cell_dims.1);

    panel.target.draw_rect(rect).unwrap();
}

pub fn draw_tile_highlight(panel: &mut Panel<&mut WindowCanvas>,
                           cell: Pos,
                           color: Color) {
    let cell_dims = panel.cell_dims();

    panel.target.set_blend_mode(BlendMode::Blend);
    panel.target.set_draw_color(Sdl2Color::RGBA(color.r, color.g, color.b, color.a));

    let rect = Rect::new(cell.x * cell_dims.0 as i32,
                         cell.y * cell_dims.1 as i32,
                         cell_dims.0,
                         cell_dims.1);

    panel.target.fill_rect(rect).unwrap();
}

pub fn cell_within_rect(rect: Rect, area_cell_dims: (i32, i32), pixel_pos: (i32, i32)) -> Option<(i32, i32)> {
    if pixel_pos.0 >= rect.x && pixel_pos.0 < rect.x + rect.w &&
       pixel_pos.1 >= rect.y && pixel_pos.1 < rect.y + rect.h {

       let cell_dims = (rect.w / area_cell_dims.0, rect.h / area_cell_dims.1);
       let x_cell = (pixel_pos.0 - rect.x) / cell_dims.0;
       let y_cell = (pixel_pos.1 - rect.y) / cell_dims.1;

       //assert!(x_cell * area_cell_dims.0 < rect.x + rect.w);
       //assert!(y_cell * area_cell_dims.1 < rect.y + rect.h);

       // NOTE hacky way to prevent this situation
       if x_cell >= area_cell_dims.0 || y_cell >= area_cell_dims.1 {
           return None;
       }

       return Some((x_cell, y_cell));
    }

    return None;
}

