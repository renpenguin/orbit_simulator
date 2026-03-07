use std::rc::Rc;

use crate::app::{
    self, App, ClickMode,
    selection::{Selection, SelectionMode},
    simulation::{Planet, Vec2},
};

impl App {
    /// If the passed event is a shortcut, handle it.
    pub fn handle_shortcut(&mut self, event: &egui::Event) {
        #[rustfmt::skip]
        let egui::Event::Key { key, modifiers, pressed: true, repeat: false, .. } = event else {
            return;
        };

        // Report pressed keys
        // println!("Pressed {}{:?}", if modifiers.ctrl { "CTRL " } else { "" }, key);

        match key {
            #[cfg(not(target_arch = "wasm32"))]
            egui::Key::S if modifiers.ctrl && modifiers.shift => self.save_as(),

            egui::Key::S if modifiers.ctrl => self.save(),

            #[cfg(target_arch = "wasm32")]
            egui::Key::O if modifiers.ctrl => self.load_web(),
            #[cfg(not(target_arch = "wasm32"))]
            egui::Key::O if modifiers.ctrl => self.load_native(),

            egui::Key::N if modifiers.ctrl => {
                self.simulation = app::Simulation::default();
                self.trail_manager = app::TrailManager::default();
                #[cfg(not(target_arch = "wasm32"))]
                {
                    self.save_file = None;
                    self.error_message = None;
                }
            }

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

    pub fn handle_mouse_inputs(&mut self, input_state: &egui::InputState) {
        let Some(mouse_screen_pos) = input_state.pointer.latest_pos() else {
            return;
        };

        // Map screen coordinates to position in painter
        let mouse_pos = self.screen_point_to_sim(mouse_screen_pos);

        self.handle_sim_mouse_input(&input_state.pointer, mouse_pos);
        self.selection.mouse_motion(mouse_pos);
        self.handle_selection_shortcut(input_state, mouse_pos);

        // For App.handle_context_menu()
        if input_state.pointer.secondary_clicked() {
            self.last_right_click_pos = mouse_pos;
        }

        // Zoom control
        let zoom_factor = (0.01 * input_state.smooth_scroll_delta.y).exp() as f64;
        if zoom_factor != 1.0 {
            let original_zoom = self.viewport_zoom;
            self.viewport_zoom *= zoom_factor;

            let mouse_pos = Vec2::from(mouse_screen_pos);
            self.viewport_focus += (original_zoom.recip() - self.viewport_zoom.recip()) * mouse_pos;
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

            let mut clicked_planet = self
                .simulation
                .try_find_planet_at_pos(mouse_pos, 4.0 / self.viewport_zoom);
            // If no planet clicked directly and in velocity mode
            if clicked_planet.is_none() && self.click_mode == ClickMode::Velocity {
                for (idx, planet) in self.simulation.get_planets().enumerate() {
                    if planet.vel == Vec2::ZERO {
                        continue;
                    }

                    let tail_pos = planet.pos - app::simulation::TRAIL_SCALE * planet.vel;
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

    pub fn handle_context_menu(&mut self, response: &egui::Response) {
        response.context_menu(|ui| {
            let click_pos = self.last_right_click_pos;
            let planet_under_mouse = self
                .simulation
                .try_find_planet_at_pos(click_pos, 4.0 / self.viewport_zoom);
            if let Some(planet_idx) = planet_under_mouse {
                let clicked_planet = &self.simulation.planets[planet_idx];

                let nowrap_button =
                    egui::Button::new("Planet info").wrap_mode(egui::TextWrapMode::Extend);
                if ui.add(nowrap_button)
                    .on_hover_text_at_pointer("Show a floating window containing information about the planet").clicked() {
                    let mut planet = clicked_planet.borrow_mut();
                    planet.popup_open = !planet.popup_open;
                }

                let followed_planet_id = self.followed_planet
                    .as_ref()
                    .and_then(|planet_ref| planet_ref.upgrade())
                    .map(|planet| egui::Id::new(planet.as_ptr().addr()));

                if followed_planet_id == Some(egui::Id::new(clicked_planet.as_ptr().addr())) {
                    // If clicked on currently followed planet
                    if ui.button("Unfollow").clicked() {
                        self.followed_planet = None;
                    }
                } else if ui.button("Follow").clicked() {
                    // If clicked on planet not currently followed
                    self.followed_planet = Some(Rc::downgrade(&self.simulation.planets[planet_idx]));
                }
            } else if self.followed_planet.is_some() && ui.button("Unfollow").clicked() {
                // If right-clicked in empty space
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
