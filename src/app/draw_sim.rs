use crate::{
    App,
    app::{
        ClickMode,
        simulation::{self, TAIL_SCALE, Vec2},
    },
};

impl App {
    // Planets
    pub fn draw_planets(&self, painter: &egui::Painter) {
        for (planet_idx, planet) in self.simulation.get_planets().enumerate() {
            let planet_name = simulation::get_planet_name_from_index(planet_idx);
            let screen_position = self.sim_point_to_screen(planet.pos);

            let radius = (self.viewport_zoom * planet.radius()) as f32;

            painter.circle_filled(screen_position, radius, egui::Color32::WHITE);

            painter.text(
                screen_position - egui::Vec2::new(0.0, radius + 8.0),
                egui::Align2::CENTER_BOTTOM,
                planet_name,
                egui::FontId::proportional(8.0),
                egui::Color32::GRAY,
            );

            // Don't draw tail if planet is locked
            if planet.locked {
                continue;
            }

            let side_offset = if planet.vel == Vec2::ZERO {
                egui::Vec2::new(radius, 0.0)
            } else {
                radius * egui::Vec2::from(planet.vel).normalized().rot90()
            };

            painter.line(
                vec![
                    screen_position + side_offset,
                    screen_position - side_offset,
                    screen_position
                        - (self.viewport_zoom * TAIL_SCALE) as f32 * egui::Vec2::from(planet.vel),
                    screen_position + side_offset,
                ],
                (1.0, egui::Color32::GRAY),
            );

            // Planet tail select circle
            if self.click_mode == ClickMode::Velocity {
                let tail_length = (self.viewport_zoom * TAIL_SCALE) as f32;
                painter.circle_stroke(
                    screen_position - tail_length * egui::Vec2::from(planet.vel),
                    4.0,
                    (1.0, egui::Color32::LIGHT_BLUE),
                );
            }
        }
    }

    /// Selection indicator
    pub fn draw_selection_indicator(&mut self, painter: &egui::Painter) {
        let Some(planet_ref) = self.selection.extract_planet() else {
            return;
        };
        let planet = planet_ref.borrow();

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
            (1.0, egui::Color32::LIGHT_BLUE),
        );
    }
}
