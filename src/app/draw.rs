use egui::{Color32, Pos2};
use std::{cell::RefCell, f32::consts::FRAC_PI_2, rc::Rc};

use crate::app::simulation::{Planet, TRAIL_SCALE};

const SHORTCUTS: [(&str, &str); 18] = [
    ("Ctrl N", "Create new"),
    ("Ctrl O", "Open"),
    ("Ctrl S", "Save"),
    ("Ctrl Shift S", "Save as"),
    ("Ctrl +", "Zoom in"),
    ("Ctrl -", "Zoom out"),
    ("Ctrl 0", "Reset size"),
    ("Ctrl /", "Show shortcuts"),
    ("Space", "Start/stop"),
    ("1-6", "Select a tool"),
    ("RMB", "Open popup"),
    ("Escape", "Cancel edit"),
    ("Enter", "Confirm edit"),
    ("G", "Move selected"),
    ("S", "Scale selected"),
    ("V", "Aim selected"),
    ("A", "Spawn new planet"),
    ("X", "Delete selected"),
];

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
            #[rustfmt::skip]
			egui::Grid::new("shortcuts_cheatsheet").spacing(egui::Vec2::splat(8.0)).show(ui, |ui| {
				for (shortcut, description) in SHORTCUTS {
					let widget_width = ui.horizontal(|ui| {
						ui.label(egui::RichText::new(shortcut).monospace().size(24.0));
						ui.label(egui::RichText::new(description).size(16.0));
						ui.add_space(8.0);
					}).response.rect.width();

					// If there is not enough space for another widget, start a new row
					if ui.cursor().left() + widget_width > shortcuts_rect.right() {
						ui.end_row();
					}
				}
			});
        });
}

pub fn planet(painter: &egui::Painter, planet: &Planet, screen_position: Pos2) {
    let radius = planet.radius() as f32;

    painter.circle_filled(screen_position, radius, Color32::WHITE);

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

pub fn planet_popup(ctx: &egui::Context, planet_ref: &Rc<RefCell<Planet>>) {
    let mut planet = planet_ref.borrow_mut();

    // Skip if planet popup should not be visible
    if !planet.popup_open {
        return;
    }
    let mut is_open = true;

    // Get a unique ID for each planet, using its address in memory
    let planet_id = planet_ref.as_ptr().addr().to_string().into();

    // Draw popup fields
    egui::Window::new("Planet info")
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
