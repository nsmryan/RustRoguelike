use std::collections::VecDeque;

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
use roguelike_core::animation::{Sprite, Effect, SpriteKey, Animation, SpriteAnim, SpriteIndex};
use roguelike_core::utils::aoe_fill;
use roguelike_core::movement::{Direction, MoveType};


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

    pub fn add_spritesheet(&mut self, name: String, texture: Texture) {
        let sprite_key = self.state.next_sprite_key;
        let sprite_sheet = SpriteSheet::new(name, texture);
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
        return SpriteAnim::new(name, sprite_key, 0.0, max_index as f32, speed);
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
    pub fn loop_sprite(&mut self, sprite_name: &str, speed: f32) -> Animation {
        let sprite_anim = self.new_sprite(sprite_name.to_string(), speed);
        
        let anim = Animation::Loop(sprite_anim);

        return anim;
    }

    pub fn play_idle_animation(&mut self, entity_id: EntityId, data: &mut GameData, config: &Config) {
        if let Some(anim) = self.get_idle_animation(entity_id, data, config) {
            self.state.play_animation(entity_id, anim);
        }
    }

    pub fn get_idle_animation(&mut self, entity_id: EntityId, data: &mut GameData, config: &Config) -> Option<Animation> {
        let player_id = data.find_by_name(EntityName::Player).unwrap();

        let name = data.entities.name[&entity_id];

        if name == EntityName::Player || name == EntityName::Gol || name == EntityName::Pawn || name == EntityName::Rook {
            let name = data.entities.name[&entity_id];
            let stance = data.entities.stance[&entity_id];
            let direction = data.entities.direction[&entity_id];

            let sheet_direction = sheet_direction(direction);
            let mut sheet_name = format!("{}_{}_{}", name, stance, sheet_direction);

            if !self.sprite_exists(&sheet_name) {
                sheet_name = format!("{}_{}_{}", name, Stance::Standing, sheet_direction);
            }
            let mut anim = self.loop_sprite(&sheet_name, config.idle_speed);
            anim.sprite_anim_mut().unwrap().flip_horiz = needs_flip_horiz(direction);

            return Some(anim);
        } else {
            if data.entities.name[&entity_id] == EntityName::Key {
                return Some(self.loop_sprite("key", config.idle_speed));
            } else if data.entities.name[&entity_id] == EntityName::SpikeTrap {
                return Some(self.loop_sprite("trap_damage", config.idle_speed));
            } else if data.entities.name[&entity_id] == EntityName::Armil {
                return Some(self.loop_sprite("armil_idle", config.idle_speed));
            } else if data.entities.name[&entity_id] == EntityName::Lantern {
                return Some(self.loop_sprite("lantern_idle", config.fire_speed));
            }
        }

        return None;
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

                        let pos = data.entities.pos[&cause_id];
                        // NOTE it is slightly odd to look up this sprite sheet here...
                        let tiles = self.state.lookup_spritekey("tiles");
                        let impression_sprite = Sprite::new(ENTITY_UNKNOWN as u32, tiles);
                        self.state.impressions.push(Impression::new(impression_sprite, pos));
                    }
                }
            }

            Msg::ItemThrow(_thrower, item_id, start, _end) => {
                // NOTE we use the entity's position instead of 'end' because we
                // want where it hit, not where it was thrown to.
                let end = data.entities.pos[&item_id];

                let sound_aoe = aoe_fill(&data.map, AoeEffect::Sound, end, config.sound_radius_stone, config);

                let chr = data.entities.chr[&item_id];
                let item_sprite = self.font_sprite(chr);

                let move_anim = Animation::Between(item_sprite, start, end, 0.0, config.item_throw_speed);
                let item_anim = Animation::PlayEffect(Effect::Sound(sound_aoe, 0.0));
                let loop_anim = Animation::Loop(item_sprite);

                self.state.play_animation(item_id, move_anim);
                self.state.append_animation(item_id, item_anim);
                self.state.append_animation(item_id, loop_anim);
            }

            Msg::PickedUp(entity_id, _item_id) => {
                self.play_idle_animation(entity_id, data, config);
            }

            Msg::Facing(entity_id, _pos) => {
                self.play_idle_animation(entity_id, data, config);
            }

            Msg::Killed(_attacker, attacked, _damage) => {
                if data.entities.typ[&attacked] != EntityType::Player {
                    self.state.clear_animations(attacked);

                    let sprite_name = format!("{:?}_death", data.entities.name[&attacked]);
                    if self.sprite_exists(&sprite_name) {
                        let sprite = self.new_sprite(sprite_name, 1.0);
                        self.state.play_animation(attacked, Animation::Once(sprite));
                    }
                }
            }

            Msg::HammerSwing(entity_id, _item_id, _pos) => {
                if data.entities.typ[&entity_id] == EntityType::Player {
                    let attack_sprite =
                        self.new_sprite("player_attack_hammer".to_string(), config.player_attack_hammer_speed);
                    let attack_anim = Animation::Once(attack_sprite);
                    self.state.play_animation(entity_id, attack_anim);

                    if let Some(idle_anim) = self.get_idle_animation(entity_id, data, config) {
                        self.state.append_animation(entity_id, idle_anim);
                    }
                }
            }

            Msg::Stabbed(entity_id, _hit_entity) => {
                if data.entities.typ[&entity_id] == EntityType::Player {
                    let attack_sprite =
                        self.new_sprite("player_attack_dagger".to_string(), config.player_attack_speed);
                    let attack_anim = Animation::Once(attack_sprite);
                    self.state.play_animation(entity_id, attack_anim);

                    if let Some(idle_anim) = self.get_idle_animation(entity_id, data, config) {
                        self.state.append_animation(entity_id, idle_anim);
                    }
                }
            }

            Msg::SwordSwing(entity_id, _item_id, _pos) => {
                if data.entities.typ[&entity_id] == EntityType::Player {
                    if let Some(idle_anim) = self.get_idle_animation(entity_id, data, config) {
                        self.state.play_animation(entity_id, idle_anim);
                    }
                }
            }

            Msg::Attack(attacker, attacked, _damage) => {
                if data.entities.typ[&attacker] == EntityType::Player {
                    let attack_sprite =
                        self.new_sprite("player_attack".to_string(), config.player_attack_speed);
                    let attack_anim = Animation::Once(attack_sprite);
                    self.state.play_animation(attacker, attack_anim);

                    if let Some(idle_anim) = self.get_idle_animation(attacker, data, config) {
                        self.state.play_animation(attacker, idle_anim);
                    }
                } else {
                    let attacker_pos = data.entities.pos[&attacker];
                    let attacked_pos = data.entities.pos[&attacked];
                    let beam_effect = Effect::Beam(config.beam_duration, attacker_pos, attacked_pos);
                    self.state.play_effect(beam_effect);
                }
            }

            Msg::JumpWall(_jumper, _start, _end) => {
                // This animation does not work
            }

            Msg::SpawnedObject(entity_id, _typ, _pos, _name, _facing) => {
                if data.entities.ids.contains(&entity_id) {
                    self.play_idle_animation(entity_id, data, config);
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
                    data.entities.status[&player_id].extra_fov += 1;
                    let is_in_fov_ext = 
                       data.pos_in_fov(player_id, impression.pos, &config);
                    data.entities.status[&player_id].extra_fov -= 1;

                    if is_in_fov_ext {
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
    pub animations: IndexMap<EntityId, VecDeque<Animation>>,
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
            animations: IndexMap::<EntityId, VecDeque<Animation>>::new(),
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

        panic!(format!("Could not find sprite '{}'", name));
    }

    pub fn update_animations(&mut self, dt: f32) {
        for anims in self.animations.values_mut() {
            if let Some(anim) = anims.get_mut(0) {
                if let Some(sprite_anim) = anim.sprite_anim_mut() {
                    sprite_anim.step(dt);
                }
            }
        }
    }

    pub fn draw_sprite(&mut self,
                       panel: &mut Panel<&mut WindowCanvas>,
                       sprite: Sprite,
                       pos: Pos,
                       color: Color) {
        let sprite_sheet = &mut self.sprites[&sprite.key];
        sprite_sheet.draw_sprite_at_cell(panel, sprite.index as usize, pos, color, 0.0, sprite.flip_horiz, sprite.flip_vert);
    }

    pub fn play_effect(&mut self, effect: Effect) {
        self.effects.push(effect);
    }

    pub fn play_animation(&mut self, entity_id: EntityId, animation: Animation) {
        // ensure that the entity has an animation vec. This is called on entity
        // spawn, so it is not necessary in the other animation functions.
        if self.animations.get(&entity_id).is_none() {
            self.animations.insert(entity_id, VecDeque::new());
        }

        self.clear_animations(entity_id);
        self.append_animation(entity_id, animation);
    }

    pub fn append_animation(&mut self, entity_id: EntityId, animation: Animation) {
        self.animations[&entity_id].push_back(animation);
    }

    pub fn clear_animations(&mut self, entity_id: EntityId) {
        self.animations[&entity_id].clear();
    }

    pub fn pop_animation(&mut self, entity_id: EntityId) {
        self.animations[&entity_id].pop_front();
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
    pub cols: usize,
    pub width: usize,
    pub height: usize,
}

impl SpriteSheet {
    pub fn new(name: String, texture: Texture) -> SpriteSheet {
        let tex_info = texture.query();
        let width = tex_info.width as usize;
        let height = tex_info.height as usize;

        let rows = height / FONT_HEIGHT as usize;
        let cols = width / FONT_WIDTH as usize;
        let num_sprites = cols * rows;

        return SpriteSheet {
            texture,
            name,
            num_sprites,
            rows,
            cols,
            width,
            height,
        };
    }

    pub fn num_cells(&self) -> (usize, usize) {
        return (self.cols, self.rows);
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
            self.draw_sprite_at_cell(panel, chr as usize, pos, color, 0.0, false, false);
            pos.x += 1;
        }
    }

    pub fn draw_char(&mut self,
                     panel: &mut Panel<&mut WindowCanvas>,
                     chr: char,
                     cell: Pos,
                     color: Color) {
        self.draw_sprite_at_cell(panel, chr as usize, cell, color, 0.0, false, false);
    }

    pub fn draw_sprite_at_cell(&mut self,
                               panel: &mut Panel<&mut WindowCanvas>,
                               index: usize,
                               cell: Pos,
                               color: Color,
                               rotation: f64,
                               flip_horiz: bool,
                               flip_vert: bool) {
        let (cell_width, cell_height) = panel.cell_dims();

        let pos = Pos::new(cell.x * cell_width as i32, cell.y * cell_height as i32);

        self.draw_sprite_full(panel, index, pos, color, rotation, flip_horiz, flip_vert);
    }

    pub fn draw_sprite_full(&mut self,
                            panel: &mut Panel<&mut WindowCanvas>,
                            index: usize,
                            pos: Pos,
                            color: Color,
                            rotation: f64,
                            flip_horizontal: bool,
                            flip_vertical: bool) {
        let cell_dims = panel.cell_dims();

        let src = self.sprite_src(index);

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
                             flip_horizontal,
                             flip_vertical).unwrap();
    }

    pub fn draw_sprite_direction(&mut self,
                                 panel: &mut Panel<&mut WindowCanvas>,
                                 index: usize,
                                 direction: Option<Direction>,
                                 pos: Pos,
                                 scale: f32,
                                 color: Color,
                                 rotation: f64) {
        let cell_dims = panel.cell_dims();

        let src = self.sprite_src(index);

        let (cell_width, cell_height) = cell_dims;
        let dst_width = (cell_width as f32 * scale) as u32;
        let dst_height = (cell_height as f32 * scale) as u32;

        let x_margin = ((cell_width - dst_width) / 2) as i32;
        let y_margin = ((cell_height - dst_height) / 2) as i32;

        let mut dst_x = pos.x * cell_width as i32;
        let mut dst_y = pos.y * cell_height as i32;
        match direction {
            None => {
                dst_x += x_margin;
                dst_y += y_margin;
            }
            
            Some(Direction::Left) => {
                dst_y += y_margin;
            }

            Some(Direction::Right) => {
                dst_x += cell_width as i32 - dst_width as i32;
                dst_y += y_margin;
            }

            Some(Direction::Up) => {
                dst_x += x_margin;
            }

            Some(Direction::Down) => {
                dst_x += x_margin;
                dst_y += cell_height as i32 - dst_height as i32;
            }

            Some(Direction::DownLeft) => {
                dst_y += cell_height as i32 - dst_height as i32;
            }

            Some(Direction::DownRight) => {
                dst_x += cell_width as i32 - dst_width as i32;
                dst_y += cell_height as i32 - dst_height as i32;
            }

            Some(Direction::UpLeft) => {
            }

            Some(Direction::UpRight) => {
                dst_x += cell_width as i32  - dst_width as i32;
            }
        }

        let dst = Rect::new(dst_x,
                            dst_y,
                            dst_width,
                            dst_height);

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

    fn sprite_src(&mut self, index: usize) -> Rect {
        let (num_cells_x, num_cells_y) = self.num_cells();
        let sprite_x = index % num_cells_x;
        let sprite_y = index / num_cells_x;

        assert!(sprite_y < num_cells_y);

        let (sprite_width, sprite_height) = self.sprite_dims();
        let src = Rect::new((sprite_x * sprite_width) as i32,
                            (sprite_y * sprite_height) as i32,
                            sprite_width as u32,
                            sprite_height as u32);

        return src;
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

fn sheet_direction(direction: Direction) -> Direction {
    match direction {
        Direction::Up => return Direction::Up,
        Direction::Down => return Direction::Down,
        Direction::Left => return Direction::Right,
        Direction::Right => return Direction::Right,
        Direction::UpRight => return Direction::UpRight,
        Direction::UpLeft => return Direction::UpRight,
        Direction::DownRight => return Direction::DownRight,
        Direction::DownLeft => return Direction::DownRight,
    }
}

fn needs_flip_horiz(direction: Direction) -> bool {
    match direction {
        Direction::Up => return false,
        Direction::Down => return false,
        Direction::Left => return true,
        Direction::Right => return false,
        Direction::UpRight => return false,
        Direction::UpLeft => return true,
        Direction::DownRight => return false,
        Direction::DownLeft => return true,
    }
}
