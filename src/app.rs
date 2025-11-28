use egui::{Color32, Frame, Pos2, Rect, Sense, emath::RectTransform};
use web_time::{Duration, Instant};

mod simulation;
use simulation::Simulation;

const SHORTCUTS: [(&str, &str); 5] = [
    ("Ctrl /", "Open this screen"),
    ("Ctrl +", "Zoom in"),
    ("Ctrl -", "Zoom out"),
    ("Ctrl 0", "Reset size"),
    ("Space", "Toggle simulation"),
];

#[derive(Debug, PartialEq, Eq)]
enum ClickMode {
    Select,
    Translate,
    Scale,
    Spawn,
    Delete,
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct App {
    #[serde(skip)]
    click_mode: ClickMode,
    #[serde(skip)]
    shortcuts_shown: bool,

    simulation: Simulation,
    #[serde(skip)]
    last_draw: Instant,
}

impl Default for App {
    fn default() -> Self {
        Self {
            click_mode: ClickMode::Select,
            shortcuts_shown: false,
            simulation: Simulation::default(),
            last_draw: Instant::now(),
        }
    }
}

impl App {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        cc.egui_ctx.set_theme(egui::Theme::Dark);

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            Default::default()
        }
    }
}

impl eframe::App for App {
    /// Called by the framework to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    #[expect(clippy::too_many_lines)]
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Put your widgets into a `SidePanel`, `TopBottomPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        // Check new key presses
        ctx.input(|i| {
            for event in &i.events {
                match &event {
                    // Shortcut key
                    egui::Event::Key {
                        key: egui::Key::Slash,
                        pressed: true,
                        repeat: false,
                        modifiers,
                        ..
                    } if modifiers.contains(egui::Modifiers::CTRL) => {
                        self.shortcuts_shown = !self.shortcuts_shown;
                    }

                    _ => (),
                }
            }
        });

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:

            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New").clicked() {
                        println!("Start a new world");
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

                ui.add_space(20.0);

                ui.radio_value(&mut self.click_mode, ClickMode::Select, "Select");
                ui.radio_value(&mut self.click_mode, ClickMode::Translate, "Move");
                ui.radio_value(&mut self.click_mode, ClickMode::Scale, "Scale");
                ui.radio_value(&mut self.click_mode, ClickMode::Spawn, "New");
                ui.radio_value(&mut self.click_mode, ClickMode::Delete, "Delete");

                ui.add_space(20.0);

                ui.with_layout(
                    egui::Layout::right_to_left(egui::Align::Center),
                    egui::warn_if_debug_build,
                );
            });
        });

        // For selected or pinned planets
        // egui::Window::new("planet 0")

        // Define the size of the shortcuts window to always fill the central panel, with an 8-pixel margin.
        let mut shortcuts_rect = ctx.screen_rect();
        *shortcuts_rect.top_mut() += 24.0;
        *shortcuts_rect.bottom_mut() -= 48.0;
        *shortcuts_rect.right_mut() -= 15.0;
        shortcuts_rect = shortcuts_rect.shrink(8.0);

        // Shortcuts window
        egui::Window::new("Shortcuts Cheatsheet")
            .fixed_rect(shortcuts_rect)
            .collapsible(false)
            .open(&mut self.shortcuts_shown)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink(false)
                    .show(ui, |ui| {
                        egui::Grid::new("shortcuts_cheatsheet")
                            .spacing(egui::Vec2::splat(8.0))
                            .show(ui, |ui| {
                                for (shortcut, description) in SHORTCUTS {
                                    let widget_width = ui
                                        .horizontal(|ui| {
                                            ui.label(
                                                egui::RichText::new(shortcut)
                                                    .monospace()
                                                    .size(24.0),
                                            );
                                            ui.label(egui::RichText::new(description).size(16.0));
                                            ui.add_space(8.0);
                                        })
                                        .response
                                        .rect
                                        .width();

                                    // If there is not enough space for another widget, start a new row
                                    if (shortcuts_rect.width() - ui.cursor().left_top().x
                                        + shortcuts_rect.left())
                                        < widget_width
                                    {
                                        ui.end_row();
                                    }
                                }
                            });
                    });
            });

        // let delta_time = self.last_draw.elapsed().as_secs_f64();
        self.last_draw = Instant::now();

        // Simulate planets
        self.simulation.simulate_gravity();
        // self.simulation.handle_collisions();

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

                if response.hovered() {
                    ctx.input(|i| {
                        if let Some(click_pos) = i.pointer.press_origin() {
                            // Map screen coordinates to position in painter
                            let click_pos = to_screen.inverse().transform_pos(click_pos);

                            if i.pointer.primary_pressed() {
                                match self.click_mode {
                                    ClickMode::Spawn => self.simulation.spawn_planet_at(click_pos),
                                    ClickMode::Delete => {
                                        let mut planet_under_mouse = None;
                                        for (i, body) in self.simulation.planets.iter().enumerate() {
                                            let is_selectable = (click_pos - Pos2::from(body.pos))
                                                .length_sq()
                                                < 100.0;
                                            if is_selectable {
                                                planet_under_mouse = Some(i);
                                                break;
                                            }
                                        }
                                        if let Some(i) = planet_under_mouse {
                                            self.simulation.planets.swap_remove(i);
                                        }
                                    }
                                    _ => println!("This will do something eventually!"),
                                }
                            }
                        }
                    });
                }

                // Draw planets
                for planet in &self.simulation.planets {
                    painter.circle_filled(
                        to_screen.transform_pos(planet.pos.into()),
                        planet.radius() as f32,
                        Color32::WHITE,
                    );
                }
            });

        ctx.request_repaint_after(Duration::from_secs_f32(1.0 / 60.0));
    }
}
