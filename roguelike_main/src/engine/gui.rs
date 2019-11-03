use ggez::graphics;
use ggez::Context;

use gfx_core::{handle::RenderTargetView, memory::Typed};

use gfx_device_gl;

use imgui::*;
use imgui_gfx_renderer::*;

use std::time::Instant;

use roguelike_core::types::*;
use roguelike_core::map::*;
use roguelike_core::constants::*;

use crate::engine::style::*;


pub struct Gui {
    pub imgui: imgui::Context,
    pub renderer: Renderer<gfx_core::format::Rgba8, gfx_device_gl::Resources>,
    last_frame: Instant,
    mouse_state: MouseState,
    show_popup: bool,
}

impl Gui {
    pub fn new(ctx: &mut Context) -> Self {
        // Create the imgui object
        let mut imgui = imgui::Context::create();
        imgui.set_ini_filename(None);

        set_style_dark(imgui.style_mut());

        let (factory, gfx_device, _, _, _) = graphics::gfx_objects(ctx);

        // Shaders
        let shaders = {
            let version = gfx_device.get_info().shading_language;
            if version.is_embedded {
                if version.major >= 3 {
                    Shaders::GlSlEs300
                } else {
                    Shaders::GlSlEs100
                }
            } else if version.major >= 4 {
                Shaders::GlSl400
            } else if version.major >= 3 {
                Shaders::GlSl130
            } else {
                Shaders::GlSl110
            }
        };

        // Renderer
        let renderer = Renderer::init(&mut imgui, &mut *factory, shaders).unwrap();

        // Create instace
        Self {
            imgui,
            renderer,
            last_frame: Instant::now(),
            mouse_state: MouseState::default(),
            show_popup: false,
        }
    }

    pub fn render(&mut self, ctx: &mut Context, map: &Map, objects: &[Object], mouse_state: &mut MouseState, dims: (usize, usize), pos: (usize, usize)) {
        // Update mouse
        self.update_mouse(mouse_state);

        // Create new frame
        let now = Instant::now();
        let delta = now - self.last_frame;
        let delta_s =
            delta.as_secs() as f32 + delta.subsec_nanos() as f32 / 1_000_000_000.0;
        self.last_frame = now;

        let (w, h) = graphics::drawable_size(ctx);
        self.imgui.io_mut().display_size = [w, h];
        self.imgui.io_mut().display_framebuffer_scale = [1.0, 1.0];
        self.imgui.io_mut().delta_time = delta_s;

        let ui = self.imgui.frame();

        {
            let mouse_pos = ((mouse_state.pos.0 / FONT_WIDTH) + 1,
                             (mouse_state.pos.1 / FONT_HEIGHT) + 1);

            // Window
            let ui_width = dims.0 as f32;
            let ui_height = dims.1 as f32;
            Window::new(im_str!("Lower Panel"))
                .size([ui_width, ui_height], imgui::Condition::FirstUseEver)
                .position([pos.0 as f32, pos.1 as f32], imgui::Condition::FirstUseEver)
                .movable(false)
                .collapsible(false)
                .bg_alpha(0.0)
                .title_bar(false)
                .resizable(false)
                .build(&ui, || {
                    ui.text(im_str!("Debug Inspector"));
                    ui.separator();

                    ui.text(im_str!(
                            "Player Position: ({:},{:})",
                            objects[0].x,
                            objects[0].y
                    ));
                    ui.text(im_str!(
                            "Player Momentum: {:}",
                            objects[0].momentum.unwrap().magnitude(),
                    ));

                    ui.text(im_str!("Tile:"));
                    ui.same_line(0.0);
                    ui.text(im_str!(
                            "({:?}, {:?}, {:})",
                            map[objects[0].pos()].tile_type,
                            map[objects[0].pos()].chr,
                            map[objects[0].pos()].blocked,
                            ));
                    ui.text(im_str!(
                            "left: {:?}, bottom: {:?}",
                            map[objects[0].pos()].left_wall,
                            map[objects[0].pos()].bottom_wall));

                    let ids = 
                        objects.iter()
                        .enumerate()
                        .filter(|(_, obj)| obj.pos() == mouse_pos)
                        .map(|(index, _)| index)
                        .collect::<Vec<_>>();
                    for id in ids {
                        if objects[id].alive {
                            ui.text(im_str!("{}",
                                            objects[id].name,
                                            ));
                            if let Some(fighter) = objects[id].fighter {
                                ui.same_line(0.0);
                                ui.text(im_str!("hp {}/{}", fighter.hp, fighter.max_hp));
                            }
                            if let Some(behave) = objects[id].behavior {
                                ui.text(im_str!("state {:?}", behave));
                            }
                            break;
                        }
                    }
                });
        }

        if self.show_popup {
            ui.open_popup(im_str!("popup"));
        }

        // Render
        let (factory, _, encoder, _, render_target) = graphics::gfx_objects(ctx);
        let draw_data = ui.render();
        self
            .renderer
            .render(
                &mut *factory,
                encoder,
                &mut RenderTargetView::new(render_target.clone()),
                draw_data,
                )
            .unwrap();
    }

    fn update_mouse(&mut self, mouse_state: &mut MouseState) {
        self.imgui.io_mut().mouse_pos = [mouse_state.pos.0 as f32, mouse_state.pos.1 as f32];

        self.imgui.io_mut().mouse_down = [
            mouse_state.pressed.0,
            mouse_state.pressed.1,
            mouse_state.pressed.2,
            false,
            false,
        ];

        self.imgui.io_mut().mouse_wheel = mouse_state.wheel;
        mouse_state.wheel = 0.0;
    }

    pub fn update_mouse_pos(&mut self, x: f32, y: f32, mouse_state: &mut MouseState) {
        mouse_state.pos = (x as i32, y as i32);
    }

    pub fn update_mouse_down(&mut self, pressed: (bool, bool, bool), mouse_state: &mut MouseState) {
        mouse_state.pressed = pressed;

        if pressed.0 {
            self.show_popup = false;
        }
    }
}
