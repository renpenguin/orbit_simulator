use egui::{Color32, Frame, Pos2, Rect, Sense, emath::RectTransform};
use std::f32::consts::FRAC_PI_2;
use web_time::{Duration, Instant};

mod selection;
use selection::{Selection, SelectionMode};

mod simulation;
use simulation::Simulation;

use crate::app::simulation::{Planet, TRAIL_SCALE, Vec2};

const SHORTCUTS: [(&str, &str); 5] = [
    ("Ctrl /", "Open this screen"),
    ("Ctrl +", "Zoom in"),
    ("Ctrl -", "Zoom out"),
    ("Ctrl 0", "Reset size"),
    ("Space", "Toggle simulation"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClickMode {
    Select,
    Translate,
    Scale,
    Aim,
    Spawn,
    Delete,
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
pub struct App {
    click_mode: ClickMode,
    shortcuts_shown: bool,

    selection: Selection,

    simulation: Simulation,
    last_draw: Instant,
}

impl App {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_theme(egui::Theme::Dark);
        cc.egui_ctx.set_zoom_factor(1.4);

        Self {
            click_mode: ClickMode::Select,
            shortcuts_shown: false,
            selection: Selection::None,
            simulation: Simulation::default(),
            last_draw: Instant::now(),
        }
    }
}

impl eframe::App for App {
    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.input(|i| {
            for event in &i.events {
                self.handle_shortcut(event);
            }
        });

        self.draw_top_panel(ctx);
        self.draw_shortcuts_screen(ctx);

        // let delta_time = self.last_draw.elapsed().as_secs_f64();
        self.last_draw = Instant::now();

        // Simulate planets
        if self.simulation.playing {
            self.simulation.handle_collisions();
            self.simulation.simulate_gravity();
        }

        // Main planet space
        egui::CentralPanel::default()
            .frame(Frame::default().inner_margin(0.0).fill(Color32::BLACK))
            .show(ctx, |ui| {
                // Create a "canvas" for drawing on that's 100% x 300px
                let (response, painter) =
                    ui.allocate_painter(ui.available_size(), Sense::click_and_drag());

                // Get the relative position of our "canvas"
                let to_screen = RectTransform::from_to(
                    Rect::from_min_size(Pos2::ZERO, response.rect.size()),
                    response.rect,
                );

                // Handle mouse inputs
                if response.hovered() {
                    ctx.input(|input_state| {
                        if let Some(mouse_pos) = input_state.pointer.latest_pos() {
                            // Map screen coordinates to position in painter
                            let mouse_pos = to_screen.inverse().transform_pos(mouse_pos).into();

                            self.handle_sim_mouse_input(&input_state.pointer, mouse_pos);
                            self.handle_selection_shortcut(input_state, mouse_pos);
                        }
                    });
                }

                // Draw planets
                for planet in self.simulation.get_planets() {
                    let centre_pos = to_screen.transform_pos(planet.pos.into());
                    let radius = planet.radius() as f32;

                    painter.circle_filled(centre_pos, radius, Color32::WHITE);

                    let angle = planet.vel.y.atan2(planet.vel.x) as f32;
                    let side_offset = radius * egui::Vec2::angled(angle + FRAC_PI_2);

                    painter.line(
                        vec![
                            centre_pos + side_offset,
                            centre_pos - side_offset,
                            centre_pos - TRAIL_SCALE as f32 * egui::Vec2::from(planet.vel),
                            centre_pos + side_offset,
                        ],
                        (1.0, Color32::GRAY),
                    );
                }

                // Selection indicator
                if (self.click_mode == ClickMode::Select || self.click_mode == ClickMode::Spawn)
                    && let Some(planet) = self.selection.extract_planet()
                {
                    let planet = planet.borrow();
                    let centre_pos = to_screen.transform_pos(planet.pos.into());
                    let radius = planet.radius() as f32 + 4.0;
                    painter.line(
                        vec![
                            centre_pos + egui::Vec2::new(-radius, -radius),
                            centre_pos + egui::Vec2::new(radius, -radius),
                            centre_pos + egui::Vec2::new(radius, radius),
                            centre_pos + egui::Vec2::new(-radius, radius),
                            centre_pos + egui::Vec2::new(-radius, -radius),
                        ],
                        (1.0, Color32::LIGHT_BLUE),
                    );
                }
            });

        ctx.request_repaint_after(Duration::from_secs_f32(1.0 / 60.0));
    }
}

impl App {
    // If the passed event is a shortcut, handle it.
    fn handle_shortcut(&mut self, event: &egui::Event) {
        #[rustfmt::skip]
        let egui::Event::Key { key, modifiers, pressed: true, repeat: false, .. } = event else {
            return;
        };

        // Report pressed keys
        // println!("Pressed {}{:?}", if modifiers.ctrl { "CTRL " } else { "" }, key);

        match key {
            // Shortcut key
            egui::Key::Slash if modifiers.ctrl => {
                self.shortcuts_shown = !self.shortcuts_shown;
            }

            // Play/pause
            egui::Key::Space => {
                self.simulation.playing = !self.simulation.playing;
            }

            // ____X to delete the currently selected planet
            egui::Key::X => {
                let Some(planet_to_delete) = self.selection.extract_planet() else {
                    return;
                };

                let addr_to_delete = planet_to_delete.as_ptr().addr();
                for i in 0..self.simulation.planets.len() {
                    // Does this pointer have the same address as the selection's planet pointer?
                    if self.simulation.planets[i].as_ptr().addr() == addr_to_delete {
                        self.simulation.planets.swap_remove(i);
                        self.selection = Selection::None;
                        break;
                    }
                }
            }

            // 1..=6 number keys to set clickmode
            egui::Key::Num1 => self.click_mode = ClickMode::Select,
            egui::Key::Num2 => self.click_mode = ClickMode::Translate,
            egui::Key::Num3 => self.click_mode = ClickMode::Scale,
            egui::Key::Num4 => self.click_mode = ClickMode::Aim,
            egui::Key::Num5 => self.click_mode = ClickMode::Spawn,
            egui::Key::Num6 => self.click_mode = ClickMode::Delete,

            // Cancel operation
            egui::Key::Escape => {
                if let Selection::Some { mode, planet, .. } = &mut self.selection {
                    let Some(planet) = planet.upgrade() else {
                        self.selection = Selection::None;
                        return;
                    };
                    let mut planet = planet.borrow_mut();
                    match mode {
                        SelectionMode::Selected => (),
                        SelectionMode::Translating { original_pos } => {
                            planet.pos = *original_pos;
                        }
                        SelectionMode::Scaling { original_mass, .. } => {
                            planet.mass = *original_mass;
                        }
                        SelectionMode::Aiming { original_velocity } => {
                            planet.vel = *original_velocity;
                        }
                    }
                    *mode = SelectionMode::Selected;
                }
            }

            _ => (),
        }
    }

    fn handle_selection_shortcut(&mut self, input_state: &egui::InputState, mouse_pos: Vec2) {
        // Ignore Ctrl+_ (in particular Ctrl+S)
        if input_state.modifiers.ctrl {
            return;
        }

        // GSVA_
        // For GSV, only continue if there is a selection and the planet exists. Any old selection will be overwritten, confirming the old operation
        if input_state.key_pressed(egui::Key::G) {
            if let Some(planet) = self.selection.extract_planet() {
                self.selection = Selection::new(ClickMode::Translate, &planet, mouse_pos);
            };
        }
        if input_state.key_pressed(egui::Key::S) {
            if let Some(planet) = self.selection.extract_planet() {
                self.selection = Selection::new(ClickMode::Scale, &planet, mouse_pos);
            };
        }
        if input_state.key_pressed(egui::Key::V) {
            if let Some(planet) = self.selection.extract_planet() {
                self.selection = Selection::new(ClickMode::Aim, &planet, mouse_pos);
            };
        }
        if input_state.key_pressed(egui::Key::A) {
            // Create and select a new planet
            let planet = Planet::new(mouse_pos, 960.0);
            self.selection = Selection::new(ClickMode::Select, &planet, mouse_pos);
            self.simulation.planets.push(planet);
        }

        if input_state.key_pressed(egui::Key::Enter) {
            // Overwrites previous operation, confirming it. Akin to GSV
            if let Some(planet) = self.selection.extract_planet() {
                self.selection = Selection::new(ClickMode::Select, &planet, mouse_pos);
            };
        }
    }

    // Handle mouse inputs (clicking, moving) while over the simulation area
    fn handle_sim_mouse_input(&mut self, mouse_state: &egui::PointerState, mouse_pos: Vec2) {
        if mouse_state.primary_pressed() {
            // If an operation is in progress, confirm it and don't attempt to select a new planet
            if let Selection::Some { mode, .. } = &mut self.selection {
                if *mode != SelectionMode::Selected {
                    *mode = SelectionMode::Selected;
                    return;
                }
            }

            let clicked_planet = self.simulation.try_find_planet_at_pos(mouse_pos);

            match self.click_mode {
                ClickMode::Spawn => {
                    // Create and select a new planet
                    let new_planet = Planet::new(mouse_pos, 960.0);
                    self.selection = Selection::new(ClickMode::Select, &new_planet, mouse_pos);
                    self.simulation.planets.push(new_planet);
                }
                ClickMode::Delete => {
                    if let Some(i) = clicked_planet {
                        self.simulation.planets.swap_remove(i);
                        self.selection = Selection::None;
                    }
                }
                other => {
                    if let Some(i) = clicked_planet {
                        let planet = &self.simulation.planets[i];
                        self.selection = Selection::new(other, planet, mouse_pos);
                    } else {
                        self.selection = Selection::None;
                    }
                }
            }
        }

        // Complete operation if mouse released, selection exists and is not "Selected"
        if mouse_state.primary_released()
            && let Selection::Some { mode, .. } = &self.selection
            && *mode != SelectionMode::Selected
        {
            self.selection = Selection::None;
        }

        self.selection.mouse_motion(mouse_pos);
    }

    fn draw_top_panel(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New").clicked() {
                        self.simulation = Simulation::default();
                    }
                    if ui.button("Load").clicked() {
                        println!("Load a world from a file");
                    }
                    if ui.button("Save").clicked() {
                        println!("Save the world to a file");
                    }

                    // NOTE: No File->Quit on web pages!
                    if !cfg!(target_arch = "wasm32") {
                        ui.add(egui::Separator::default().grow(6.0));
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    }
                });

                ui.menu_button("Tools", |ui| {
                    if ui.button("Kepler's 2nd Law").clicked() {
                        println!("Start a new world");
                    }
                    if ui.button("Show forces action planets").clicked() {
                        println!("Load a world from a file");
                    }
                });

                if ui.button("Help").clicked() {
                    self.shortcuts_shown = !self.shortcuts_shown;
                }

                let pause_button_label = if self.simulation.playing {
                    "Playing"
                } else {
                    "Paused"
                };
                if ui.button(pause_button_label).clicked() {
                    self.simulation.playing = !self.simulation.playing;
                }

                ui.add_space(20.0);

                ui.selectable_value(&mut self.click_mode, ClickMode::Select, "Select");
                ui.selectable_value(&mut self.click_mode, ClickMode::Translate, "Move");
                ui.selectable_value(&mut self.click_mode, ClickMode::Scale, "Scale");
                ui.selectable_value(&mut self.click_mode, ClickMode::Aim, "Aim");
                ui.selectable_value(&mut self.click_mode, ClickMode::Spawn, "New");
                ui.selectable_value(&mut self.click_mode, ClickMode::Delete, "Delete");

                ui.add_space(20.0);

                ui.with_layout(
                    egui::Layout::right_to_left(egui::Align::Center),
                    egui::warn_if_debug_build,
                );
            });
        });
    }

    #[rustfmt::skip]
    fn draw_shortcuts_screen(&mut self, ctx: &egui::Context) {
        // Define the size of the shortcuts window to always fill the central panel, with an 8-pixel margin.
        let mut shortcuts_rect = ctx.screen_rect();
        *shortcuts_rect.top_mut() += 24.0;
        *shortcuts_rect.bottom_mut() -= 48.0;
        *shortcuts_rect.right_mut() -= 15.0;
        shortcuts_rect = shortcuts_rect.shrink(8.0);

        // Shortcuts window
        egui::Window::new("Shortcuts Cheatsheet").fixed_rect(shortcuts_rect).collapsible(false).open(&mut self.shortcuts_shown).vscroll(true).show(ctx, |ui| {
            egui::Grid::new("shortcuts_cheatsheet").spacing(egui::Vec2::splat(8.0)).show(ui, |ui| {
                for (shortcut, description) in SHORTCUTS {
                    let widget_width = ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(shortcut).monospace().size(24.0));
                        ui.label(egui::RichText::new(description).size(16.0));
                        ui.add_space(8.0);
                    }).response.rect.width();

                    // If there is not enough space for another widget, start a new row
                    let remaining_width = shortcuts_rect.width() - ui.cursor().left_top().x + shortcuts_rect.left();
                    if remaining_width < widget_width {
                        ui.end_row();
                    }
                }
            });
        });
    }
}
