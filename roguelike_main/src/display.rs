use std::collections::HashMap;

use sdl2::render::{Texture, WindowCanvas, TextureCreator, BlendMode};
use sdl2::video::WindowContext;
use sdl2::rect::{Rect};
use sdl2::pixels::{Color as Sdl2Color};
use sdl2::ttf::{Sdl2TtfContext};

use indexmap::map::IndexMap;

use roguelike_core::types::*;
use roguelike_core::constants::*;
use roguelike_core::config::*;
use roguelike_core::messaging::*;
use roguelike_core::map::*;
use roguelike_core::animation::{Sprite, AnimKey, Effect, SpriteKey, Animation, SpriteAnim, SpriteIndex};
use roguelike_core::movement::{Cardinal, MoveType};


type TextureKey = u64;


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

    pub fn unit(cells: (u32, u32), dims: (u32, u32)) -> Panel<()> {
        return Panel { cells, target: (), num_pixels: dims, dirty: true };
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

#[test]
pub fn test_panel_with_area() {
    let panel = Panel::unit((10, 10), (100, 100));

    let area = panel.area();

    assert_eq!(0, area.x_offset);
    assert_eq!(0, area.y_offset);
    assert_eq!(panel.cells.0 as u32, area.width as u32);
    assert_eq!(panel.cells.1 as u32, area.height as u32);

    let (left, right) = area.split_right(2);

    let left_rect = panel.get_rect_within(&left);
    assert_eq!(0, left_rect.x);
    assert_eq!(0, left_rect.y);
    assert_eq!(80, left_rect.w);
    assert_eq!(100, left_rect.h);

    let right_rect = panel.get_rect_within(&right);
    dbg!(right, right_rect);
    assert_eq!(80, right_rect.x);
    assert_eq!(0, right_rect.y);
    assert_eq!(20, right_rect.w);
    assert_eq!(100, right_rect.h);

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
}

impl DisplayTargets {
    pub fn new(canvas: WindowCanvas) -> DisplayTargets {

        let texture_creator = canvas.texture_creator();

        let over_sample = 5;
        let background_panel = Panel::from_dims(&texture_creator, MAP_WIDTH as u32, MAP_HEIGHT as u32, 1);

        let map_panel = Panel::from_dims(&texture_creator, MAP_WIDTH as u32, MAP_HEIGHT as u32, 1);

        let info_panel = Panel::from_dims(&texture_creator, 20, 15, 1);

        let inventory_panel = Panel::from_dims(&texture_creator, 20, 15, 1);

        let player_panel = Panel::from_dims(&texture_creator, 20, 20, 1);

        let menu_panel = Panel::from_dims(&texture_creator, 20, 20, 1);

        let canvas_panel = Panel::with_canvas((SCREEN_WIDTH / FONT_WIDTH as u32, SCREEN_HEIGHT / FONT_HEIGHT as u32), canvas);

        return DisplayTargets {
            canvas_panel,
            texture_creator,
            background_panel,
            map_panel,
            player_panel,
            info_panel,
            menu_panel,
            inventory_panel,
        };
    }
}

pub struct DisplayState {
    // TODO try to remove by turning into a spritesheet
    // currently this is the only thing that makes display states hard to create
    pub font_map: FontMap,

    pub sprites: IndexMap<SpriteKey, SpriteSheet>,
    pub next_sprite_key: i64,

    pub effects: Vec<Effect>,

    pub animations: IndexMap<AnimKey, Animation>,
    pub next_anim_key: i64,


    pub drawn_sprites: IndexMap<EntityId, Sprite>,
    pub impressions: Vec<Impression>,

    pub prev_turn_fov: Vec<EntityId>,
    pub current_turn_fov: Vec<EntityId>,
}

impl DisplayState {
    pub fn new(font_map: FontMap) -> DisplayState {

        return DisplayState {
            font_map,
            sprites: IndexMap::new(),
            next_sprite_key: 0,
            effects: Vec::new(),
            animations: IndexMap::new(),
            next_anim_key: 0,
            drawn_sprites: IndexMap::new(),
            impressions: Vec::new(),
            prev_turn_fov: Vec::new(),
            current_turn_fov: Vec::new(),
        };
    }

    pub fn lookup_spritekey(&self, name: &str) -> Option<SpriteKey> {
        for (key, sprite_sheet) in self.sprites.iter() {
            if sprite_sheet.name == *name {
                return Some(*key);
            }
        }

        return None;
    }

    pub fn draw_sprite(&mut self,
                       canvas: &mut WindowCanvas,
                       sprite: Sprite,
                       pos: Pos,
                       color: Color,
                       cell_dims: (u32, u32)) {

        let sprite_index;
        let sprite_key;
        match sprite {
            Sprite::Sprite(index, key) => {
                sprite_index = index as usize;
                sprite_key = key;
            },

            Sprite::Char(chr) => {
                sprite_index = chr as usize;
                // TODO move expect into lookup_spritekey and return non-optional
                sprite_key =
                    self.lookup_spritekey("tiles")
                        .expect("Could not find rexpaint file in renderer!");
            },
        }

        let sprite = &mut self.sprites[&sprite_key];
        sprite.draw_sprite_at_cell(canvas, sprite_index, pos, cell_dims, color, 0.0);
    }

    pub fn play_effect(&mut self, effect: Effect) {
        self.effects.push(effect);
    }
}

pub struct Display {
    pub state: DisplayState,
    pub targets: DisplayTargets,
}

// TODO determine which functions should be moved to DisplayTargets or DisplayState
impl Display {
    pub fn new(font_map: FontMap,
               canvas: WindowCanvas) -> Display {
        return Display { state: DisplayState::new(font_map),
                         targets: DisplayTargets::new(canvas),
        };
    }

    pub fn update_display(&mut self) {
        self.targets.canvas_panel.target.present();
    }

    pub fn add_spritesheet(&mut self, name: String, texture: Texture, rows: usize) {
        let sprite_sheet = SpriteSheet::new(name, texture, rows);
        let sprite_key = self.state.next_sprite_key;
        self.state.next_sprite_key += 1;
        self.state.sprites.insert(sprite_key, sprite_sheet);
    }

    /// Create a sprite by looking up a texture and constructing the
    /// SpriteAnim structure.
    pub fn new_sprite(&self, name: String, speed: f32) -> Option<SpriteAnim> {
        if let Some(sprite_key) = self.state.lookup_spritekey(&name) {
            let max_index = self.state.sprites[&sprite_key].num_sprites;
            return Some(SpriteAnim::make_anim(name, sprite_key, max_index as f32, speed));
        }

        //panic!(format!("Tried to play sprite {}, but could not find animation!", name));
        return None;
    }

    pub fn font_sprite(&self, chr: char) -> Option<SpriteAnim> {
        if let Some(sprite_key) = self.state.lookup_spritekey(&"font".to_string()) {
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
                     color: Color) {
        for (index, chr) in text.chars().enumerate() {
            let chr_pos = Pos::new(pos.x + index as i32, pos.y);
            self.draw_char(chr, chr_pos, color);
        }
    }

    pub fn draw_text_list(&mut self,
                          text_list: &Vec<String>,
                          pos: Pos,
                          color: Color) {
        let mut y_pos = 0;
        for text in text_list.iter() {
            self.draw_text(text, Pos::new(pos.x, pos.y + y_pos), color);
            y_pos += 1;
        }
    }

    pub fn draw_char(&mut self,
                     chr: char,
                     pos: Pos,
                     color: Color) {
        //self.draw_char_with_rotation(chr, pos, color, area, 0.0);
    }

    // TODO this should be on panels, not always the canvas
    pub fn highlight_tile(&mut self,
                          pos: Pos,
                          color: Color) {
        self.targets.canvas_panel.target.set_blend_mode(BlendMode::Blend);
        self.targets.canvas_panel.target.set_draw_color(Sdl2Color::RGBA(color.r, color.g, color.b, color.a));

        /* TODO add back in
        let dst_rect = area.char_rect(pos.x, pos.y);
        self.targets.canvas_panel.target.fill_rect(dst_rect).unwrap();
        */
    }

    // TODO this should be on panels, not always the canvas
    pub fn draw_tile_edge(&mut self, pos: Pos, color: Color, dir: Cardinal) {
        /* TODO add back in
        self.targets.canvas_panel.target.set_blend_mode(BlendMode::Blend);
        self.targets.canvas_panel.target.set_draw_color(Sdl2Color::RGBA(color.r, color.g, color.b, color.a));

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

        self.targets.canvas_panel.target.fill_rect(side_rect).unwrap();
        */
    }


    /// Create and play a looping sprite
    pub fn loop_sprite(&mut self, sprite_name: &str, speed: f32) -> AnimKey {
        let sprite_anim = self.new_sprite(sprite_name.to_string(), speed).unwrap();
        
        let anim = Animation::Loop(sprite_anim);

        let key = self.play_animation(anim);

        return key;
    }

    pub fn get_idle_animation(&mut self, entity_id: EntityId, data: &mut GameData, config: &Config) -> Option<AnimKey> {
        let player_id = data.find_player().unwrap();

        if entity_id == player_id {
            let key;
            if data.using(entity_id, Item::Dagger) {
                key = self.loop_sprite("player_idle_dagger", config.idle_speed);
            } else if data.using(entity_id, Item::Hammer) {
                key = self.loop_sprite("player_idle_hammer", config.idle_speed);
            } else if data.using(entity_id, Item::Shield) {
                key = self.loop_sprite("player_idle_shield", config.idle_speed);
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
    }

    pub fn process_message(&mut self, msg: Msg, data: &mut GameData, config: &Config) {
        match msg {
            Msg::Sound(cause_id, source_pos, radius, should_animate) => {
                if should_animate {
                    // NOTE this is a duplicate computation, also done in logic message processing
                    let sound_aoe =
                        data.map.aoe_fill(AoeEffect::Sound, source_pos, radius);

                    let player_id = data.find_player().unwrap();
                    let player_pos = data.entities.pos[&player_id];

                    // only play the sound effect if the player position is included
                    let sound_hits_player = sound_aoe.positions().iter().any(|pos| *pos == player_pos);
                    let sound_from_monster = data.entities.typ.get(&cause_id) == Some(&EntityType::Enemy);
                    let player_can_see_source = data.is_in_fov(player_id, source_pos, config);
                    let visible_monster_sound = sound_from_monster && player_can_see_source;
                    if !visible_monster_sound && sound_hits_player {
                        let sound_effect = Effect::Sound(sound_aoe, 0.0);
                        self.state.play_effect(sound_effect);
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

            Msg::PickedUp(entity_id, _item_id) => {
                if let Some(anim_key) = self.get_idle_animation(entity_id, data, config) {
                    data.entities.set_animation(entity_id, anim_key);
                }
            }

            Msg::Moved(entity_id, movement, _pos) => {
                if !matches!(movement.typ, MoveType::Pass) {
                    if let Some(anim_key) = self.get_idle_animation(entity_id, data, config) {
                        data.entities.set_animation(entity_id, anim_key);
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

            Msg::HammerSwing(entity_id, _pos) => {
                if data.entities.typ[&entity_id] == EntityType::Player {
                    let attack_sprite =
                        self.new_sprite("player_attack_hammer".to_string(), config.player_attack_hammer_speed)
                                          .unwrap();
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
                        self.new_sprite("player_attack_dagger".to_string(), config.player_attack_speed)
                                          .unwrap();
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
                        self.new_sprite("player_attack".to_string(), config.player_attack_speed)
                                          .unwrap();
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

            Msg::SpawnedObject(entity_id, _typ, _pos, _name) => {
                let mut anim_desc = None;
                if data.entities.typ[&entity_id] == EntityType::Player {
                    anim_desc = Some(("player_idle", config.idle_speed));
                } else if data.entities.name[&entity_id] == EntityName::Key {
                    anim_desc = Some(("key", config.key_speed));
                } else if data.entities.name[&entity_id] == EntityName::Spike {
                    anim_desc = Some(("spikes", config.idle_speed));
                } else if data.entities.name[&entity_id] == EntityName::Pawn {
                    anim_desc = Some(("elf_idle", config.idle_speed));
                } else if data.entities.name[&entity_id] == EntityName::Gol {
                    anim_desc = Some(("gol_idle", config.idle_speed));
                }

                if let Some((name, speed)) = anim_desc {
                    let anim_key = self.loop_sprite(name, speed);
                    data.entities.animation[&entity_id].push_front(anim_key);
                }
            }

            Msg::PlayerTurn() => {
                let player_id = data.find_player().unwrap();

                self.state.prev_turn_fov.clear();
                self.state.prev_turn_fov.extend(self.state.current_turn_fov.iter());
                self.state.current_turn_fov.clear();

                for entity_id in data.entities.ids.clone() {
                    let pos = data.entities.pos[&entity_id];
                    if entity_id != player_id && data.is_in_fov(player_id, pos, config) {
                        self.state.current_turn_fov.push(entity_id);
                    }
                }

                for entity_id in self.state.prev_turn_fov.iter() {
                    if data.entities.typ.get(entity_id) != Some(&EntityType::Enemy) {
                        continue;
                    }

                    let pos = data.entities.pos[entity_id];
                    if !data.is_in_fov(player_id, pos, config) {
                        if let Some(sprite) = self.state.drawn_sprites.get(entity_id) {
                            self.state.impressions.push(Impression::new(*sprite, pos));
                        }
                    }
                }

                /* Remove impressions that are currently visible */
                let mut impressions_visible = Vec::new();
                for (index, impression) in self.state.impressions.iter().enumerate() {
                    if data.is_in_fov(player_id, impression.pos, config) {
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

pub struct FontMap {
    map: HashMap<char, Texture>,
    width: u32,
    height: u32,
}

impl FontMap {
    pub fn new(ttf_context: &Sdl2TtfContext, texture_creator: &TextureCreator<WindowContext>, file_name: String, font_size: u16) -> FontMap {
        let font = ttf_context.load_font("resources/Monoid.ttf", font_size).expect("Could not load font file!");

        let mut font_map: HashMap<char, Texture> = HashMap::new();
        let mut width = 0;
        let mut height = 0;
        for chr_ix in 1..=255u8 {
            let chr_surface = font.render_latin1(&[chr_ix]).solid(sdl2::pixels::Color::WHITE).unwrap();
            let char_texture = chr_surface.as_texture(&texture_creator).unwrap();

            let query = char_texture.query();
            width = query.width;
            height = query.height;

            font_map.insert(chr_ix as char, char_texture);
        }

        return FontMap {map: font_map, width, height };
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

    // TODO these functions should take a Panel<&mut WindowCanvas> and not take cell_dims
    // separately
    pub fn draw_text_list(&mut self,
                         canvas: &mut WindowCanvas,
                         text_list: &Vec<String>,
                         cell: Pos,
                         cell_dims: (u32, u32),
                         color: Color) {
        let (_, height) = cell_dims;
        for (index, text) in text_list.iter().enumerate() {
            let text_cell = Pos::new(cell.x, cell.y + index as i32);
            self.draw_text(canvas, text, text_cell, cell_dims, color);
        }
    }

    pub fn draw_text(&mut self,
                     canvas: &mut WindowCanvas,
                     text: &str,
                     cell: Pos,
                     cell_dims: (u32, u32),
                     color: Color) {
        let mut pos = cell;
        for chr in text.chars() {
            self.draw_sprite_at_cell(canvas, chr as usize, pos, cell_dims, color, 0.0);
            pos.x += 1;
        }
    }

    // consider removing and just converting chars to usize
    pub fn draw_char(&mut self,
                     canvas: &mut WindowCanvas,
                     chr: char,
                     cell: Pos,
                     cell_dims: (u32, u32),
                     color: Color) {
        self.draw_sprite_at_cell(canvas, chr as usize, cell, cell_dims, color, 0.0);
    }

    pub fn draw_sprite_at_cell(&mut self,
                               canvas: &mut WindowCanvas,
                               index: usize,
                               cell: Pos,
                               cell_dims: (u32, u32),
                               color: Color,
                               rotation: f64) {
        let (cell_width, cell_height) = cell_dims;

        let pos = Pos::new(cell.x * cell_width as i32, cell.y * cell_height as i32);

        self.draw_sprite_full(canvas, index, pos, cell_dims, color, rotation);
    }

    pub fn draw_sprite_full(&mut self,
                            canvas: &mut WindowCanvas,
                            index: usize,
                            pos: Pos,
                            cell_dims: (u32, u32),
                            color: Color,
                            rotation: f64) {
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

        canvas.set_blend_mode(BlendMode::Blend);
        self.texture.set_color_mod(color.r, color.g, color.b);
        self.texture.set_alpha_mod(color.a);

        canvas.copy_ex(&self.texture,
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

// TODO redo with spritesheet font
pub fn draw_text_with_font(canvas: &mut WindowCanvas,
                           font_map: &mut FontMap,
                           text: &str,
                           pos: Pos,
                           color: Color) {
    /*
    let total_width = font_map.width * text.len() as u32;
    let tile_rect = area.char_rect(pos.x, pos.y);

    let each_width = tile_rect.w / text.len() as i32;

    let y = tile_rect.y + (tile_rect.h / 2) - (font_map.height as i32 / 2);
    let x_offset = (tile_rect.w - total_width as i32) / 2;
    for (index, chr) in text.chars().enumerate() {
        let tex = font_map.map.get_mut(&chr).unwrap();

        let x = x_offset + tile_rect.x + index as i32 * font_map.width as i32;
        let dst = Rect::new(x,
                            y,
                            font_map.width,
                            font_map.height);

        tex.set_color_mod(color.r, color.g, color.b);
        tex.set_alpha_mod(color.a);

        canvas.copy(tex,
                    None,
                    Some(dst)).unwrap();
    }
    */
}

/* TODO check if still needed
pub fn draw_char_with_font(canvas: &mut WindowCanvas,
                           font_map: &mut FontMap,
                           chr: char,
                           pos: Pos,
                           color: Color,
                           area: &Area) {
    let tex = font_map.map.get_mut(&chr).unwrap();

    let tile_rect = area.char_rect(pos.x, pos.y);

    let dst = Rect::new(tile_rect.x + (tile_rect.w / 2) - (font_map.width as i32 / 2),
                        tile_rect.y + (tile_rect.h / 2) - (font_map.height as i32 / 2),
                        font_map.width,
                        font_map.height);

    tex.set_color_mod(color.r, color.g, color.b);
    tex.set_alpha_mod(color.a);

    canvas.copy(tex,
                None,
                Some(dst)).unwrap();
}
*/

pub fn draw_outline_tile(canvas: &mut WindowCanvas,
                         cell: Pos,
                         cell_dims: (u32, u32),
                         color: Color) {
    canvas.set_blend_mode(BlendMode::Blend);
    canvas.set_draw_color(Sdl2Color::RGBA(color.r, color.g, color.b, color.a));

    let rect = Rect::new(cell.x * cell_dims.0 as i32 + 1,
                         cell.y * cell_dims.1 as i32 + 1,
                         cell_dims.0,
                         cell_dims.1);

    canvas.draw_rect(rect).unwrap();
}

pub fn draw_tile_highlight(canvas: &mut WindowCanvas,
                           cell_dims: (u32, u32),
                           cell: Pos,
                           color: Color) {
    canvas.set_blend_mode(BlendMode::Blend);
    canvas.set_draw_color(Sdl2Color::RGBA(color.r, color.g, color.b, color.a));

    let rect = Rect::new(cell.x * cell_dims.0 as i32,
                         cell.y * cell_dims.1 as i32,
                         cell_dims.0,
                         cell_dims.1);

    canvas.fill_rect(rect).unwrap();
}

