use crate::{
    App,
    app::simulation::{Planet, Vec2},
};

fn shortcuts_section(
    ui: &mut egui::Ui,
    title: &str,
    shortcuts: &[(&str, &str)],
    window_right_edge: f32,
) {
    ui.label(egui::RichText::new(title).heading().strong());

    #[rustfmt::skip]
    egui::Grid::new(title).spacing(egui::Vec2::splat(8.0)).show(ui, |ui| {
        for (keybind, description) in shortcuts {
            let widget_width = ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(*keybind)
                        .background_color(egui::Color32::from_rgb(40, 40, 40))
                        .monospace()
                        .size(20.0),
                );
                ui.label(egui::RichText::new(*description).size(20.0));
                ui.add_space(8.0);
            }).response.rect.width();

            // If there is not enough space for another widget, start a new row
            if ui.cursor().left() + widget_width > window_right_edge {
                ui.end_row();
            }
        }
    });

    ui.separator();
}

pub fn shortcuts_screen(ctx: &egui::Context, shortcuts_shown: &mut bool) {
    // Define the size of the shortcuts window to always fill the central panel, with an 8-pixel margin.
    let mut shortcuts_rect = ctx.screen_rect();
    *shortcuts_rect.top_mut() += 24.0;
    *shortcuts_rect.bottom_mut() -= 48.0;
    *shortcuts_rect.right_mut() -= 15.0;
    shortcuts_rect = shortcuts_rect.shrink(8.0);

    // Shortcuts window
    egui::Window::new("Shortcuts Cheatsheet")
        .order(egui::Order::Foreground)
        .fixed_rect(shortcuts_rect)
        .collapsible(false)
        .open(shortcuts_shown)
        .vscroll(true)
        .show(ctx, |ui| {
            shortcuts_section(
                ui,
                "Miscellaneous",
                &[
                    ("Ctrl /", "Shortcuts menu"),
                    ("RMB", "Context menu"),
                    ("Space", "Pause/resume"),
                ],
                shortcuts_rect.right(),
            );
            shortcuts_section(
                ui,
                "Editing planets",
                &[
                    ("1-6", "Select tool"),
                    ("M", "Move selected"),
                    ("R", "Resize selected"),
                    ("V", "Aim selected"),
                    ("I", "Insert planet"),
                    ("D", "Delete selected"),
                    ("Escape", "Cancel edit"),
                    ("Enter", "Confirm edit"),
                ],
                shortcuts_rect.right(),
            );
            shortcuts_section(
                ui,
                "Saving/loading state",
                &[
                    ("Ctrl N", "New simulation"),
                    ("Ctrl O", "Load from file"),
                    ("Ctrl S", "Save to file"),
                    ("Ctrl Shift S", "Save as"),
                ],
                shortcuts_rect.right(),
            );
            shortcuts_section(
                ui,
                "UI Scale",
                &[
                    ("Ctrl +", "Zoom in"),
                    ("Ctrl -", "Zoom out"),
                    ("Ctrl 0", "Reset size"),
                ],
                shortcuts_rect.right(),
            );
        });
}

impl App {
    pub fn tutorial_popup(&mut self, ctx: &egui::Context) {
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
}
