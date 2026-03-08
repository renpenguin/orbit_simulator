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

    pub fn draw_planet_forces(&self, painter: &egui::Painter) {
        let planets_len = self.simulation.planets.len();
        let mut radii = Vec::with_capacity(planets_len);

        // Motion arrows and determine radii
        for planet in self.simulation.get_planets() {
            let radius = planet.radius();
            radii.push(radius);

            let vector_to_planet_edge = radius / planet.vel.length_sq().sqrt() * planet.vel;

            painter.arrow(
                self.sim_point_to_screen(planet.pos + vector_to_planet_edge),
                (self.viewport_zoom * TAIL_SCALE * planet.vel).into(),
                (1.0, egui::Color32::WHITE),
            );
        }

        // Forces
        for index_a in 0..planets_len {
            let planet_a = self.simulation.planets[index_a].borrow();

            for index_b in (index_a + 1)..planets_len {
                let planet_b = self.simulation.planets[index_b].borrow();

                let separation = planet_b.pos - planet_a.pos;
                let distance_squared = separation.length_sq();
                // F_g = G * m_1 * m_2 / d^2
                let magnitude = 10.0 * (planet_a.mass * planet_b.mass / distance_squared).ln_1p();

                let unit_vector = separation / distance_squared.sqrt();
                let arrow_vector = (self.viewport_zoom * magnitude * unit_vector).into();

                let planet_a_edge =
                    self.sim_point_to_screen(planet_a.pos + radii[index_a] * unit_vector);
                painter.arrow(planet_a_edge, arrow_vector, (1.0, egui::Color32::RED));

                let planet_b_edge =
                    self.sim_point_to_screen(planet_b.pos - radii[index_b] * unit_vector);
                painter.arrow(planet_b_edge, -arrow_vector, (1.0, egui::Color32::RED));
            }
        }
    }
}
