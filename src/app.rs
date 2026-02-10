use egui::{Color32, Frame, Pos2, Rect, Sense, emath::RectTransform};
use web_time::{Duration, Instant};

mod draw;

mod selection;
use selection::{Selection, SelectionMode};

mod simulation;
use simulation::{Planet, Simulation, Vec2};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClickMode {
    Select,
    Translate,
    Scale,
    Aim,
    Spawn,
    Delete,
}

// Format to three significant figures
fn my_formatter(number: f64, _decimals: std::ops::RangeInclusive<usize>) -> String {
    if number == 0.0 {
        return String::from("0");
    }

    let a = number.abs();
    if (1.0e-2..1.0e4).contains(&a) {
        let n = 1.0 + a.log10().floor();

        let precision = (3.0 - n).max(0.0) as usize;

        format!("{number:.precision$}")
    } else {
        // 3 significant figures = 1 digit always before period, 2 digits after
        format!("{number:.2e}")
    }
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
pub struct App {
    click_mode: ClickMode,
    shortcuts_shown: bool,
    tutorial_page: Option<u8>,

    selection: Selection,

    simulation: Simulation,
    last_draw: Instant,
}

impl App {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_theme(egui::Theme::Dark);
        cc.egui_ctx.set_zoom_factor(1.4);
        cc.egui_ctx.style_mut(|style| {
            style.number_formatter = egui::style::NumberFormatter::new(my_formatter);
        });

        egui_material_icons::initialize(&cc.egui_ctx);

        Self {
            click_mode: ClickMode::Select,
            shortcuts_shown: false,
            tutorial_page: Some(0),
            selection: Selection::None,
            simulation: Simulation::default(),
            last_draw: Instant::now(),
        }
    }
}

impl eframe::App for App {
    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !ctx.wants_keyboard_input() {
            ctx.input(|i| {
                for event in &i.events {
                    self.handle_shortcut(event);
                }
            });
        }

        self.draw_top_panel(ctx);
        draw::shortcuts_screen(ctx, &mut self.shortcuts_shown);

        // Draw popups
        for planet_ref in &self.simulation.planets {
            draw::planet_popup(ctx, planet_ref);
        }

        self.tutorial_popup(ctx);

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
                    draw::planet(
                        &painter,
                        &planet,
                        to_screen.transform_pos(planet.pos.into()),
                    );
                }

                // Selection indicator
                if [ClickMode::Select, ClickMode::Spawn].contains(&self.click_mode)
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

        egui::Window::new("Pause button")
            .title_bar(false)
            .resizable(false)
            .frame(egui::Frame::NONE)
            .anchor(egui::Align2::RIGHT_TOP, (0.0, 16.0))
            .show(ctx, |ui| {
                let pause_button_label = if self.simulation.playing {
                    egui_material_icons::icons::ICON_PAUSE
                } else {
                    egui_material_icons::icons::ICON_PLAY_ARROW
                };
                let button = egui::Button::new(egui::RichText::new(pause_button_label).size(48.0))
                    .frame(false);
                if ui.add(button).clicked() {
                    self.simulation.playing = !self.simulation.playing;
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

        if mouse_state.secondary_pressed() {
            if let Some(planet_idx) = self.simulation.try_find_planet_at_pos(mouse_pos) {
                let mut planet = self.simulation.planets[planet_idx].borrow_mut();
                planet.popup_open = !planet.popup_open;
            }
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

                ui.menu_button("Help", |ui| {
                    if ui.button("Tutorial").clicked() {
                        self.tutorial_page = self.tutorial_page.map_or(Some(0), |_| None);
                    }
                    if ui.button("Shortcuts").clicked() {
                        self.shortcuts_shown = !self.shortcuts_shown;
                    }
                });

                ui.add_space(20.0);

                ui.selectable_value(&mut self.click_mode, ClickMode::Select, "1. Select");
                ui.selectable_value(&mut self.click_mode, ClickMode::Translate, "2. Move");
                ui.selectable_value(&mut self.click_mode, ClickMode::Scale, "3. Scale");
                ui.selectable_value(&mut self.click_mode, ClickMode::Aim, "4. Aim");
                ui.selectable_value(&mut self.click_mode, ClickMode::Spawn, "5. New");
                ui.selectable_value(&mut self.click_mode, ClickMode::Delete, "6. Delete");

                ui.add_space(20.0);

                ui.with_layout(
                    egui::Layout::right_to_left(egui::Align::Center),
                    egui::warn_if_debug_build,
                );
            });
        });
    }

    fn tutorial_popup(&mut self, ctx: &egui::Context) {
        let Some(page) = &mut self.tutorial_page else {
            return;
        };

        let mut is_tutorial_open = true;
        egui::Window::new("Tutorial")
            .open(&mut is_tutorial_open)
            .fixed_size((250.0, 100.0))
            .resizable(false)
            .collapsible(false)
            .pivot(egui::Align2::RIGHT_BOTTOM)
            .default_pos(ctx.screen_rect().right_bottom() - egui::Vec2::new(64.0, 64.0))
            .show(ctx, |ui| {
                const FIXED_PLANET: Planet = Planet { pos: Vec2::new(279.0, 192.0), vel: Vec2::ZERO, mass: 3.4e4, locked: true, popup_open: false };
                const ORBITING_PLANET: Planet = Planet { pos: Vec2::new(102.0, 206.0), vel: Vec2::new(0.681, -14.2), mass: 960.0, locked: false, popup_open: false };

                const LAST_PAGE: u8 = 4;
                *page = (*page).clamp(0, LAST_PAGE); // Make sure the page is a valid value
                match page {
                    0 => {
                        ui.label(egui::RichText::new("You can reopen this tutorial from the Help menu").strong());
                        ui.label("Welcome! This is a guide to using the simulator. Click the button below to spawn a demo, and press SPACE or the play button (top right) to start the simulation.");
                        if ui.button("Spawn demo").clicked() {
                            self.simulation.planets.clear();
                            self.simulation.planets.push(FIXED_PLANET.as_rc());
                            self.simulation.planets.push(ORBITING_PLANET.as_rc());
                            self.simulation.playing = false;
                        }
                    }
                    1 => {
                        ui.label("Press SPACE or the pause button to pause, and try to move the planets around. Switch to the Move tool in the top bar (or press 2!), and drag a planet to move it.");
                    }
                    2 => {
                        ui.label(egui::RichText::new("Other tools:").strong());
                        ui.label("Resize planets by dragging with Scale (3)\nAim planets by dragging with Aim (4)\nSpawn planets with New (5)\nDelete planets by clicking with Delete (6)");
                    }
                    3 => {
                        ui.label("The larger planet does not move because its position has been set as locked. Right click on the larger planet to see an info popup, and unselect \"Lock position\" to let it move.");
                        if ui.button("Reset").clicked() {
                            self.simulation.planets.clear();
                            self.simulation.planets.push(FIXED_PLANET.as_rc());
                            self.simulation.planets.push(ORBITING_PLANET.as_rc());
                            self.simulation.playing = false;
                        }
                    }
                    4 => {
                        ui.label("You can use tools from Select mode (1)! Select a planet by clicking on it, then use GSVAX to Move, Scale, Aim, Spawn and Delete. You can left click to confirm a change or press Escape to undo");
                    }
                    _ => (),
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Max), |ui| {
                    // Grey out the right button if on the last page
                    let right = ui.add_enabled(
                        *page < LAST_PAGE,
                        egui::Button::new(egui_material_icons::icons::ICON_ARROW_RIGHT).small(),
                    );
                    // Grey out the left button if page is 0
                    let left = ui.add_enabled(
                        *page != 0,
                        egui::Button::new(egui_material_icons::icons::ICON_ARROW_LEFT).small(),
                    );
                    if right.clicked() {
                        *page += 1;
                    }
                    if left.clicked() {
                        *page -= 1;
                    }
                })
            });

        if !is_tutorial_open {
            self.tutorial_page = None;
        }
    }
}
