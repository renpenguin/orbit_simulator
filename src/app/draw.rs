use egui::{Color32, Pos2};
use std::{cell::RefCell, f32::consts::FRAC_PI_2, rc::Rc};

use crate::app::simulation::{Planet, TRAIL_SCALE, Vec2};

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

pub fn planet(painter: &egui::Painter, planet: &Planet, screen_position: Pos2, planet_name: &str) {
    let radius = planet.radius() as f32;

    painter.circle_filled(screen_position, radius, Color32::WHITE);

    painter.text(
        screen_position - egui::Vec2::new(0.0, radius + 8.0),
        egui::Align2::CENTER_BOTTOM,
        planet_name,
        egui::FontId::proportional(8.0),
        Color32::GRAY,
    );

    let angle = planet.vel.y.atan2(planet.vel.x) as f32;
    let side_offset = radius * egui::Vec2::angled(angle + FRAC_PI_2);

    if planet.locked {
        return;
    }

    painter.line(
        vec![
            screen_position + side_offset,
            screen_position - side_offset,
            screen_position - TRAIL_SCALE as f32 * egui::Vec2::from(planet.vel),
            screen_position + side_offset,
        ],
        (1.0, Color32::GRAY),
    );
}

pub fn planet_popup(ctx: &egui::Context, planet_ref: &Rc<RefCell<Planet>>, planet_name: &str) {
    let mut planet = planet_ref.borrow_mut();

    // Skip if planet popup should not be visible
    if !planet.popup_open {
        return;
    }
    let mut is_open = true;

    // Get a unique ID for each planet, using its address in memory
    let planet_id = planet_ref.as_ptr().addr().to_string().into();

    // Draw popup fields
    egui::Window::new(format!("{planet_name} info"))
        .id(planet_id)
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

                ui.label("Mass");
                ui.add(egui::DragValue::new(&mut planet.mass).range(f64::EPSILON..=f64::MAX));
                ui.end_row();

                ui.label("Lock position");
                ui.checkbox(&mut planet.locked, "");
            });
        });

    // Update planet's popup setting, reflecting whether X button pressed
    planet.popup_open = is_open;
}
