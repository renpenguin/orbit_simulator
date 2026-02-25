use egui::{Color32, Frame, Pos2, Sense};
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};
use web_time::{Duration, Instant};

mod draw;

mod selection;
use selection::{Selection, SelectionMode};

mod simulation;
use simulation::{Planet, Simulation, TRAIL_SCALE, Vec2};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClickMode {
    Select,
    Move,
    Resize,
    Velocity,
    Insert,
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

    followed_planet: Option<Weak<RefCell<Planet>>>,
    viewport_focus: Vec2,
    viewport_zoom: f64,

    last_right_click_pos: Vec2,
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
            tutorial_page: None, // Some(0) // TODO: only show tutorial on first run

            selection: Selection::None,
            simulation: Simulation::default(),

            followed_planet: None,
            viewport_focus: Vec2::ZERO,
            viewport_zoom: 1.0,

            last_right_click_pos: Vec2::ZERO,
            last_draw: Instant::now(),
        }
    }

    fn sim_point_to_screen(&self, sim_point: Vec2) -> Pos2 {
        Pos2::from(self.viewport_zoom * (sim_point - self.viewport_focus))
    }
    fn screen_point_to_sim(&self, screen_point: Pos2) -> Vec2 {
        Vec2::from(screen_point) / self.viewport_zoom + self.viewport_focus
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
        for (planet_idx, planet_ref) in self.simulation.planets.iter().enumerate() {
            let planet_name = simulation::get_planet_name_from_index(planet_idx);
            draw::planet_popup(ctx, planet_ref, &mut self.followed_planet, &planet_name);
        }

        self.tutorial_popup(ctx);

        // let delta_time = self.last_draw.elapsed().as_secs_f64();
        self.last_draw = Instant::now();

        // Record the planet position before the simulation runs, if there is a focused planet and it still exists
        let old_followed_planet_pos = self
            .followed_planet
            .as_ref()
            .and_then(|planet_ref| planet_ref.upgrade())
            .map(|planet| planet.borrow().pos);

        // Simulate planets
        if self.simulation.playing {
            for _ in 0..self.simulation.tick_rate {
                self.simulation.handle_collisions();
                self.simulation.simulate_gravity();
            }
        }

        // Record the planet position *after* the simulation runs
        if let Some(planet_ref) = &self.followed_planet {
            if let Some(planet) = planet_ref.upgrade() {
                let followed_planet_pos = planet.borrow().pos;

                self.viewport_focus +=
                    followed_planet_pos - old_followed_planet_pos.unwrap_or_else(|| unreachable!());
            } else {
                self.followed_planet = None;
            }
        }

        // Main planet space
        egui::CentralPanel::default()
            .frame(Frame::default().inner_margin(0.0).fill(Color32::BLACK))
            .show(ctx, |ui| {
                // Create a "canvas" for drawing on that's 100% x 300px
                let (response, painter) =
                    ui.allocate_painter(ui.available_size(), Sense::click_and_drag());

                // Handle mouse inputs
                if response.hovered() {
                    ctx.input(|input_state| {
                        if let Some(mouse_screen_pos) = input_state.pointer.latest_pos() {
                            // Map screen coordinates to position in painter
                            let mouse_pos = self.screen_point_to_sim(mouse_screen_pos);

                            self.handle_sim_mouse_input(&input_state.pointer, mouse_pos);
                            self.handle_selection_shortcut(input_state, mouse_pos);

                            if input_state.pointer.secondary_clicked() {
                                self.last_right_click_pos = mouse_pos;
                            }

                            let zoom_factor =
                                (0.01 * input_state.smooth_scroll_delta.y).exp() as f64;
                            if zoom_factor != 1.0 {
                                let original_zoom = self.viewport_zoom;
                                self.viewport_zoom *= zoom_factor;

                                let mouse_pos = Vec2::from(mouse_screen_pos);
                                self.viewport_focus += (original_zoom.recip()
                                    - self.viewport_zoom.recip())
                                    * mouse_pos;
                            }
                        }
                    });
                }
                self.handle_context_menu(&response);

                // Draw planets
                for (planet_idx, planet) in self.simulation.get_planets().enumerate() {
                    let planet_name = simulation::get_planet_name_from_index(planet_idx);
                    let screen_pos = self.sim_point_to_screen(planet.pos);

                    draw::planet(
                        &painter,
                        &planet,
                        screen_pos,
                        &planet_name,
                        self.viewport_zoom,
                    );

                    // Planet tail select circle
                    if !planet.locked && self.click_mode == ClickMode::Velocity {
                        let tail_length = (self.viewport_zoom * TRAIL_SCALE) as f32;
                        painter.circle_stroke(
                            screen_pos - tail_length * egui::Vec2::from(planet.vel),
                            4.0,
                            (1.0, Color32::LIGHT_BLUE),
                        );
                    }
                }

                // Selection indicator
                if [ClickMode::Select, ClickMode::Insert].contains(&self.click_mode)
                    && let Some(planet) = self.selection.extract_planet()
                {
                    let planet = planet.borrow();
                    let centre_pos = self.sim_point_to_screen(planet.pos);
                    let radius = (self.viewport_zoom * planet.radius()) as f32 + 4.0;
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

            // ____D to delete the currently selected planet
            egui::Key::D => {
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
            egui::Key::Num2 => self.click_mode = ClickMode::Move,
            egui::Key::Num3 => {
                self.click_mode = ClickMode::Resize;
                self.simulation.playing = false; // Pause program when aiming or resizing
            }
            egui::Key::Num4 => {
                self.click_mode = ClickMode::Velocity;
                self.simulation.playing = false; // Pause program when aiming or resizing
            }
            egui::Key::Num5 => self.click_mode = ClickMode::Insert,
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
                        SelectionMode::Selecting => (),
                        SelectionMode::Moving { original_pos } => {
                            planet.pos = *original_pos;
                        }
                        SelectionMode::Resizing { original_mass, .. } => {
                            planet.mass = *original_mass;
                        }
                        SelectionMode::Aiming {
                            original_velocity, ..
                        } => {
                            planet.vel = *original_velocity;
                        }
                    }
                    *mode = SelectionMode::Selecting;
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

        // MRVI_
        // For MRV, only continue if there is a selection and the planet exists. Any old selection will be overwritten, confirming the old operation
        if input_state.key_pressed(egui::Key::M) {
            if let Some(planet) = self.selection.extract_planet() {
                self.selection = Selection::new(ClickMode::Move, &planet, mouse_pos);
            };
        }
        if input_state.key_pressed(egui::Key::R) {
            if let Some(planet) = self.selection.extract_planet() {
                self.selection = Selection::new(ClickMode::Resize, &planet, mouse_pos);
                self.simulation.playing = false; // Pause program when aiming or resizing
            };
        }
        if input_state.key_pressed(egui::Key::V) {
            if let Some(planet) = self.selection.extract_planet() {
                self.selection = Selection::new_vel_unsnapped_to_mouse(&planet, mouse_pos);
                self.simulation.playing = false; // Pause program when aiming or resizing
            };
        }
        if input_state.key_pressed(egui::Key::I) {
            // Create and select a new planet
            let planet = Planet::new(mouse_pos, 960.0);
            self.selection = Selection::new(ClickMode::Select, &planet, mouse_pos);
            self.simulation.planets.push(planet);
        }

        if input_state.key_pressed(egui::Key::Enter) {
            // Overwrites previous operation, confirming it. Akin to MRV
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
                if *mode != SelectionMode::Selecting {
                    *mode = SelectionMode::Selecting;
                    return;
                }
            }

            let mut clicked_planet = self.simulation.try_find_planet_at_pos(mouse_pos);
            // If no planet clicked directly and in velocity mode
            if clicked_planet.is_none() && self.click_mode == ClickMode::Velocity {
                for (idx, planet) in self.simulation.get_planets().enumerate() {
                    if planet.vel == Vec2::ZERO {
                        continue;
                    }

                    let tail_pos = planet.pos - TRAIL_SCALE * planet.vel;
                    if (tail_pos - mouse_pos).length_sq() < 16.0 {
                        clicked_planet = Some(idx);
                        break;
                    }
                }
            }

            match self.click_mode {
                ClickMode::Insert => {
                    self.selection = Selection::None;
                    // Insert planet only on release
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
        // If dragging with middle click or without a selection
        if mouse_state.middle_down() || (mouse_state.primary_down() && self.selection.is_none()) {
            self.viewport_focus -= Vec2::from(mouse_state.delta()) / self.viewport_zoom;
        }

        if mouse_state.primary_released() {
            // Complete operation if mouse released, selection exists and is not "Selected"
            if let Selection::Some { mode, .. } = &self.selection
                && *mode != SelectionMode::Selecting
            {
                self.selection = Selection::None;
            }

            // If released after not dragging in insert mode, create and select a new planet
            if self.click_mode == ClickMode::Insert && !mouse_state.is_decidedly_dragging() {
                let new_planet = Planet::new(mouse_pos, 960.0);
                self.selection = Selection::new(ClickMode::Select, &new_planet, mouse_pos);
                self.simulation.planets.push(new_planet);
            }
        }

        self.selection.mouse_motion(mouse_pos);
    }

    fn draw_top_panel(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New simulation").clicked() {
                        self.simulation = Simulation::default();
                    }
                    if ui.button("Load from file").clicked() {
                        println!("Load a world from a file");
                    }
                    if ui.button("Save to file").clicked() {
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
                    if ui.button("Kepler's 2nd Law")
                        .on_hover_text_at_pointer("Show a visualisation of Kepler's 2nd Law when there are only two planets and one is stationary")
                        .clicked()
                    {
                        println!("Start a new world");
                    }
                    if ui.button("Show forces acting on planets").clicked() {
                        println!("Load a world from a file");
                    }
                });

                ui.menu_button("Help", |ui| {
                    if ui.button("Tutorial").on_hover_text_at_pointer("Show the tutorial again").clicked() {
                        self.tutorial_page = self.tutorial_page.map_or(Some(0), |_| None);
                    }
                    if ui.button("Shortcuts").on_hover_text_at_pointer("Show shortcuts screen").clicked() {
                        self.shortcuts_shown = !self.shortcuts_shown;
                    }
                });

                ui.add_space(10.0);

                ui.selectable_value(&mut self.click_mode, ClickMode::Select, "1. Select")
                    .on_hover_text_at_pointer("Use the Select tool. Click on a planet to select it, then edit it using keyboard shortcuts (see the Help menu for shortcuts)");
                ui.selectable_value(&mut self.click_mode, ClickMode::Move, "2. Move")
                    .on_hover_text_at_pointer("Use the Move tool. Drag a planet to move it");

                let resize_button = ui.selectable_value(&mut self.click_mode, ClickMode::Resize, "3. Resize")
                    .on_hover_text_at_pointer("Use the Move tool. Grab the edge of a planet and drag to resize it");
                let velocity_button = ui.selectable_value(&mut self.click_mode, ClickMode::Velocity, "4. Velocity")
                    .on_hover_text_at_pointer("Use the Move tool. Click and drag a planet or its tail to adjust its speed and direction of motion");
                if resize_button.clicked() || velocity_button.clicked() {
                    self.simulation.playing = false; // Pause program when aiming or resizing
                }

                ui.selectable_value(&mut self.click_mode, ClickMode::Insert, "5. Insert")
                    .on_hover_text_at_pointer("Use the Insert tool. Click in the simulation space to spawn a planet");
                ui.selectable_value(&mut self.click_mode, ClickMode::Delete, "6. Delete")
                    .on_hover_text_at_pointer("Use the Delete tool. Click on a planet to delete it");

                ui.add_space(10.0);

                ui.add(egui::DragValue::new(&mut self.simulation.tick_rate).custom_formatter(|num, _| (num as usize).to_string()).prefix("Tick rate: ").speed(0.01).range(1..=100));

                ui.add_space(10.0);

                ui.with_layout(
                    egui::Layout::right_to_left(egui::Align::Center),
                    egui::warn_if_debug_build,
                );
            });
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
                if ui
                    .add(button)
                    .on_hover_text_at_pointer("Click to start/pause the simulation")
                    .clicked()
                {
                    self.simulation.playing = !self.simulation.playing;
                }
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
                        ui.label("Welcome! This is a guide to using the simulator. Click the button below to load a demo, and press SPACE or the play button (top right) to start the simulation.");
                        if ui.button("Load demo").on_hover_text_at_pointer("Load a ready-to-go sun-planet setup").clicked() {
                            self.simulation.planets.clear();
                            self.simulation.planets.push(FIXED_PLANET.as_rc());
                            self.simulation.planets.push(ORBITING_PLANET.as_rc());
                            self.simulation.playing = false;
                        }
                    }
                    1 => {
                        ui.label("Press SPACE or the pause button to pause, and try to move the planets around by switching to the Move tool in the top bar (or press the 2 key!), and drag a planet to move it.");
                    }
                    2 => {
                        ui.label(egui::RichText::new("Other tools:").strong());
                        ui.label("Resize planets by dragging with Resize (3)\nAim planets by dragging with Velocity (4)\nInsert planets with Insert (5)\nDelete planets by clicking with Delete (6)");
                    }
                    3 => {
                        ui.label("The larger planet does not move because its position has been set as locked. Right click on the larger planet, then select \"Planet Info\" to see an info popup, and unselect \"Lock position\" to let it move.");
                        if ui.button("Reset").on_hover_text_at_pointer("Reload the ready-to-go sun-planet setup").clicked() {
                            self.simulation.planets.clear();
                            self.simulation.planets.push(FIXED_PLANET.as_rc());
                            self.simulation.planets.push(ORBITING_PLANET.as_rc());
                            self.simulation.playing = false;
                        }
                    }
                    4 => {
                        ui.label("You can use tools from Select mode (1)! Select a planet by clicking on it, then use MRVID to Move, Resize, set Velocity of, Insert and Delete planets. You can left click to confirm a change or press Escape to undo");
                    }
                    _ => (),
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Max), |ui| {
                    // Grey out the right button if on the last page
                    let right = ui.add_enabled(
                        *page < LAST_PAGE,
                        egui::Button::new(egui_material_icons::icons::ICON_ARROW_RIGHT).small(),
                    ).on_hover_text_at_pointer("Go to next page");
                    if right.clicked() {
                        *page += 1;
                    }

                    ui.label(format!("Page {}/{}", *page + 1, LAST_PAGE + 1));

                    // Grey out the left button if on first page
                    let left = ui.add_enabled(
                        *page != 0,
                        egui::Button::new(egui_material_icons::icons::ICON_ARROW_LEFT).small(),
                    ).on_hover_text_at_pointer("Go to previous page");
                    if left.clicked() {
                        *page -= 1;
                    }
                })
            });

        if !is_tutorial_open {
            self.tutorial_page = None;
        }
    }

    fn handle_context_menu(&mut self, response: &egui::Response) {
        response.context_menu(|ui| {
            let click_pos = self.last_right_click_pos;
            let planet_under_mouse = self.simulation.try_find_planet_at_pos(click_pos);
            if let Some(planet_idx) = planet_under_mouse {
                let clicked_planet = &self.simulation.planets[planet_idx];

                let nowrap_button =
                    egui::Button::new("Planet info").wrap_mode(egui::TextWrapMode::Extend);
                if ui.add(nowrap_button)
                    .on_hover_text_at_pointer("Show a floating window containing information about the planet").clicked() {
                    let mut planet = clicked_planet.borrow_mut();
                    planet.popup_open = !planet.popup_open;
                }

                let followed_planet = self.followed_planet.as_ref().and_then(|planet_ref| planet_ref.upgrade());

                if let Some(planet) = followed_planet && planet.as_ptr().addr() == clicked_planet.as_ptr().addr() {
                    // If clicked on currently followed planet
                    if ui.button("Unfollow").clicked() {
                        self.followed_planet = None;
                    }
                // If clicked on planet not currently followed
                } else if ui.button("Follow").clicked() {
                    self.followed_planet = Some(Rc::downgrade(&self.simulation.planets[planet_idx]));
                }
            // If right-clicked in empty space
            } else if self.followed_planet.is_some() && ui.button("Unfollow").clicked() {
                self.followed_planet = None;
            }

            ui.selectable_value(&mut self.click_mode, ClickMode::Select, "Select")
                .on_hover_text_at_pointer("Use the Select tool. Click on a planet to select it, then edit it using keyboard shortcuts (see the Help menu for shortcuts)");
            ui.selectable_value(&mut self.click_mode, ClickMode::Move, "Move")
                .on_hover_text_at_pointer("Use the Move tool. Drag a planet to move it");

            let resize_button = ui.selectable_value(&mut self.click_mode, ClickMode::Resize, "Resize")
                .on_hover_text_at_pointer("Use the Move tool. Grab the edge of a planet and drag to resize it");
            let velocity_button = ui.selectable_value(&mut self.click_mode, ClickMode::Velocity, "Velocity")
                .on_hover_text_at_pointer("Use the Move tool. Click and drag a planet or its tail to adjust its speed and direction of motion");
            if resize_button.clicked() || velocity_button.clicked() {
                self.simulation.playing = false; // Pause program when aiming or resizing
            }

            if ui.button("Insert").on_hover_text_at_pointer("Insert a planet where the simulation space was right-clicked").clicked() {
                let new_planet = Planet::new(click_pos, 960.0);
                self.selection = Selection::new(ClickMode::Select, &new_planet, click_pos);
                self.simulation.planets.push(new_planet);
            }

            if let Some(planet_idx) = planet_under_mouse {
                if ui.button("Delete").on_hover_text_at_pointer("Delete the hovered planet").clicked() {
                    self.simulation.planets.swap_remove(planet_idx);
                }
            }
        });
    }
}
