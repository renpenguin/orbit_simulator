use egui::{Button, RichText};
use egui_material_icons::icons;
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use crate::app::{
    App, ClickMode,
    simulation::{Planet, Simulation},
};

/// Format to three significant figures
pub fn format_3sf(number: f64, _decimals: std::ops::RangeInclusive<usize>) -> String {
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

pub fn planet_popup(
    ctx: &egui::Context,
    planet_ref: &Rc<RefCell<Planet>>,
    followed_planet: &mut Option<Weak<RefCell<Planet>>>,
    planet_name: &str,
) {
    let mut planet = planet_ref.borrow_mut();

    // Skip if planet popup should not be visible
    if !planet.popup_open {
        return;
    }
    let mut is_open = true;

    // Get a unique ID for each planet, using its address in memory
    let planet_id = egui::Id::new(planet_ref.as_ptr().addr());

    // Draw popup fields
    egui::Window::new(format!("{planet_name} info"))
        .id(planet_id)
        .resizable(false)
        .open(&mut is_open)
        .show(ctx, |ui| {
            egui::Grid::new(planet_id).show(ui, |ui| {
                ui.label("Position");
                ui.add(egui::DragValue::new(&mut planet.pos.x));
                ui.add(egui::DragValue::new(&mut planet.pos.y));
                ui.end_row();

                ui.label("Velocity");
                ui.add(egui::DragValue::new(&mut planet.vel.x));
                ui.add(egui::DragValue::new(&mut planet.vel.y));
                ui.end_row();

                ui.label("Speed");
                let mut speed = planet.vel.length_sq().sqrt();
                ui.add_enabled(
                    speed != 0.0,
                    egui::DragValue::from_get_set(|set_value| {
                        if let Some(value) = set_value {
                            planet.vel = (value / speed) * planet.vel;
                            speed = value;
                        }
                        speed
                    })
                    .range(0.0..=f64::MAX),
                );
                ui.end_row();

                ui.label("Mass");
                ui.add(egui::DragValue::new(&mut planet.mass).range(f64::EPSILON..=f64::MAX));
                ui.end_row();

                ui.label("Lock position");
                ui.checkbox(&mut planet.locked, "")
                    .on_hover_text_at_pointer(
                        "Lock the planet in one place, forcing it to have a velocity of 0",
                    );
                ui.end_row();

                let followed_planet_id = followed_planet
                    .as_ref()
                    .and_then(|planet_ref| planet_ref.upgrade())
                    .map(|planet| egui::Id::new(planet.as_ptr().addr()));

                // If popup planet is currently followed
                if followed_planet_id == Some(planet_id) {
                    if ui.button("Unfollow").clicked() {
                        *followed_planet = None;
                    }
                } else if ui.button("Follow").clicked() {
                    *followed_planet = Some(Rc::downgrade(planet_ref));
                }
            });
        });

    // Update planet's popup setting, reflecting whether X button pressed
    planet.popup_open = is_open;
}

#[cfg(not(target_arch = "wasm32"))]
pub fn message_dialogue(ctx: &egui::Context, message: &str) -> bool {
    let mut is_open = true;

    egui::Window::new("Error")
        .resizable(false)
        .title_bar(false)
        .anchor(egui::Align2::CENTER_CENTER, (0.0, 0.0))
        .movable(false)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.label(RichText::new(message).size(16.0));
                if ui.button("Ok").clicked() {
                    is_open = false;
                }
            })
        });

    is_open
}

impl App {
    pub fn draw_top_panel(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New simulation").clicked() {
                        self.simulation = Simulation::default();
                        #[cfg(not(target_arch = "wasm32"))] {
                            self.save_file = None;
                            self.error_message = None;
                        }
                    }
                    if ui.button("Load from file").clicked() {
                        #[cfg(target_arch = "wasm32")]
                        self.load_web();
                        #[cfg(not(target_arch = "wasm32"))]
                        self.load_native();
                    }
                    if ui.button("Save simulation").clicked() {
                        self.save();
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    if ui.button("Save simulation as...").clicked() {
                        self.save_as();
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
                    icons::ICON_PAUSE
                } else {
                    icons::ICON_PLAY_ARROW
                };
                let button = Button::new(RichText::new(pause_button_label).size(48.0)).frame(false);
                if ui
                    .add(button)
                    .on_hover_text_at_pointer("Click to start/pause the simulation")
                    .clicked()
                {
                    self.simulation.playing = !self.simulation.playing;
                }
            });
    }
}
