use std::{cell::RefCell, rc::Weak};

mod draw_help;
mod draw_sim;
mod draw_ui;
mod input;
mod saveload;

mod selection;
use selection::Selection;

mod simulation;
use simulation::{Planet, Simulation, Vec2};

mod trails;
use trails::TrailManager;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClickMode {
    Select,
    Move,
    Resize,
    Velocity,
    Insert,
    Delete,
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
pub struct App {
    click_mode: ClickMode,
    shortcuts_shown: bool,
    tutorial_page: Option<u8>,

    selection: Selection,
    simulation: Simulation,
    trail_manager: TrailManager,

    followed_planet: Option<Weak<RefCell<Planet>>>,
    viewport_focus: Vec2,
    viewport_zoom: f64,

    #[cfg(not(target_arch = "wasm32"))]
    save_file: Option<std::path::PathBuf>,
    #[cfg(not(target_arch = "wasm32"))]
    error_message: Option<String>,

    #[cfg(target_arch = "wasm32")]
    load_task: Option<saveload::Task<Option<String>>>,

    last_right_click_pos: Vec2,
}

impl App {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_theme(egui::Theme::Dark);
        cc.egui_ctx.set_zoom_factor(1.4);
        cc.egui_ctx.style_mut(|style| {
            style.number_formatter = egui::style::NumberFormatter::new(draw_ui::format_3sf);
        });

        egui_material_icons::initialize(&cc.egui_ctx);

        Self {
            click_mode: ClickMode::Select,
            shortcuts_shown: false,
            tutorial_page: None, // Some(0) // TODO: only show tutorial on first run

            selection: Selection::None,
            simulation: Simulation::default(),
            trail_manager: TrailManager::default(),

            followed_planet: None,
            viewport_focus: Vec2::ZERO,
            viewport_zoom: 1.0,

            #[cfg(not(target_arch = "wasm32"))]
            save_file: None,
            #[cfg(not(target_arch = "wasm32"))]
            error_message: None,
            #[cfg(target_arch = "wasm32")]
            load_task: None,

            last_right_click_pos: Vec2::ZERO,
        }
    }

    fn sim_point_to_screen(&self, sim_point: Vec2) -> egui::Pos2 {
        egui::Pos2::from(self.viewport_zoom * (sim_point - self.viewport_focus))
    }
    fn screen_point_to_sim(&self, screen_point: egui::Pos2) -> Vec2 {
        Vec2::from(screen_point) / self.viewport_zoom + self.viewport_focus
    }
}

impl eframe::App for App {
    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Process global shortcuts
        if !ctx.wants_keyboard_input() {
            ctx.input(|i| {
                for event in &i.events {
                    self.handle_shortcut(event);
                }
            });
        }

        #[cfg(target_arch = "wasm32")]
        self.process_load_task();

        // Record the planet position before the simulation runs, if there is a focused planet and it still exists
        let old_followed_planet_pos = self
            .followed_planet
            .as_ref()
            .and_then(|planet_ref| planet_ref.upgrade())
            .map(|planet| planet.borrow().pos);

        // Run simulation
        self.simulation.update();

        // Record trails
        if self.simulation.playing {
            self.trail_manager.planets_moved(&self.simulation.planets);
        }

        // Record the planet position *after* the simulation runs and adjust the viewport focus to keep the planet in place
        if let Some(planet_ref) = &self.followed_planet {
            if let Some(planet) = planet_ref.upgrade() {
                let followed_planet_pos = planet.borrow().pos;

                self.viewport_focus +=
                    followed_planet_pos - old_followed_planet_pos.unwrap_or_else(|| unreachable!());
            } else {
                self.followed_planet = None;
            }
        }

        self.draw_top_panel(ctx);

        // Help screens
        draw_help::shortcuts_screen(ctx, &mut self.shortcuts_shown);
        self.tutorial_popup(ctx);

        #[cfg(not(target_arch = "wasm32"))]
        if let Some(message) = &self.error_message {
            let open = draw_ui::message_dialogue(ctx, message);
            if !open {
                self.error_message = None;
            }
        }

        // Planet popups
        for (planet_idx, planet_ref) in self.simulation.planets.iter().enumerate() {
            let planet_name = simulation::get_planet_name_from_index(planet_idx);
            draw_ui::planet_popup(ctx, planet_ref, &mut self.followed_planet, &planet_name);
        }

        // Main simulation space
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(egui::Color32::BLACK))
            .show(ctx, |ui| {
                // Create a "canvas" for drawing the simulation space
                let (response, painter) =
                    ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());

                // Handle mouse inputs
                if response.hovered() {
                    ctx.input(|input_state| {
                        self.handle_mouse_inputs(input_state);
                    });
                }

                // Context menu when painter space is secondary-clicked
                self.handle_context_menu(&response);

                // Draw trails
                self.draw_trails(&painter);

                // Draw planets
                self.draw_planets(&painter);
                if [ClickMode::Select, ClickMode::Insert].contains(&self.click_mode) {
                    self.draw_selection_indicator(&painter);
                }
            });

        ctx.request_repaint_after(web_time::Duration::from_secs_f32(1.0 / 60.0));
    }
}
