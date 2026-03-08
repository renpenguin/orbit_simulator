use egui::Color32;
use std::{
    cell::{Ref, RefCell},
    rc::Rc,
};

use crate::app::{
    draw_ui,
    simulation::{Planet, Vec2},
};

/// Return the stationary and orbiting planet, if there are two total planets and the
pub fn get_planet_setup<'a>(
    planets: &'a [Rc<RefCell<Planet>>],
) -> Option<(Ref<'a, Planet>, Ref<'a, Planet>)> {
    // Ensure there are only two planets in the simulation
    if planets.len() != 2 {
        return None;
    }
    let first = planets[0].borrow();
    let second = planets[1].borrow();

    // Ensure there is one locked planet and one unlocked planet
    if first.locked && !second.locked {
        Some((first, second))
    } else if second.locked && !first.locked {
        Some((second, first))
    } else {
        None // not locked-unlocked pair
    }
}

/// A structure to store information for the visualisation of Kepler's Second Law
#[derive(Debug)]
pub enum K2L {
    Recording {
        sweep_separations: Vec<Vec2>,
        /// Time interval, recorded area
        logged_areas: Vec<(usize, f64)>,
        time_interval: usize,
        swept_area: f64,
        stationary_body_pos: Vec2,
    },

    IncorrectSetup,

    Disabled,
}

impl K2L {
    pub fn new_some() -> Self {
        Self::Recording {
            sweep_separations: Vec::new(),
            logged_areas: Vec::new(),
            time_interval: 100,
            swept_area: 0.0,
            stationary_body_pos: Vec2::ZERO,
        }
    }

    pub fn is_disabled(&self) -> bool {
        matches!(*self, Self::Disabled)
    }

    pub fn sweep_area(&mut self, planets: &[Rc<RefCell<Planet>>]) {
        if self.is_disabled() {
            return;
        }

        let Some((stationary, orbiting)) = get_planet_setup(planets) else {
            *self = Self::IncorrectSetup;
            return;
        };

        if matches!(*self, Self::IncorrectSetup) {
            *self = Self::new_some();
        }

        let Self::Recording {
            sweep_separations,
            logged_areas,
            time_interval,
            swept_area,
            stationary_body_pos,
        } = self
        else {
            unreachable!()
        };

        *stationary_body_pos = stationary.pos;

        // When the time interval has passed, log the swept area and reset the counters
        if sweep_separations.len() >= *time_interval {
            logged_areas.push((*time_interval, *swept_area));
            *swept_area = 0.0;
            sweep_separations.clear();
        }

        // Record the current separation
        sweep_separations.push(orbiting.pos - stationary.pos);

        // Add the area swept this frame to the area swept so far (use last two recorded separations)
        if let Some([sep1, sep2]) = sweep_separations.last_chunk() {
            *swept_area += sep1.cross(sep2).abs() / 2.0;
        }
    }

    pub fn draw_area(&self, painter: &egui::Painter, sim_to_screen: impl Fn(Vec2) -> egui::Pos2) {
        let Self::Recording {
            sweep_separations,
            stationary_body_pos,
            ..
        } = self
        else {
            return;
        };

        if sweep_separations.len() < 2 {
            return;
        }

        let screen_centre_pos = sim_to_screen(*stationary_body_pos);
        let mut previous_vertex = sim_to_screen(*stationary_body_pos + sweep_separations[0]);

        for separation in sweep_separations.iter().skip(1) {
            let vertex = sim_to_screen(*stationary_body_pos + *separation);

            // Construct triangle
            let mut mesh = egui::Mesh::default();
            mesh.colored_vertex(previous_vertex, Color32::GRAY);
            mesh.colored_vertex(vertex, Color32::GRAY);
            mesh.colored_vertex(screen_centre_pos, Color32::GRAY);
            mesh.add_triangle(0, 1, 2);

            painter.add(mesh); // Draw triangle

            previous_vertex = vertex;
        }
    }

    pub fn draw_popup(&mut self, ctx: &egui::Context) {
        if self.is_disabled() {
            return;
        }

        let mut is_open = true;
        egui::Window::new("Kepler's Second Law")
            .open(&mut is_open)
            .resizable(false)
            .default_width(0.0)
            .show(ctx, |ui| {
                let Self::Recording {
                    sweep_separations,
                    logged_areas,
                    time_interval,
                    swept_area,
                    ..
                } = self else {
                    ui.label("Error: Please ensure that there are\nonly two planets in the simulation,\nand that just one of them is locked\n in place");
                    return;
                };

                egui::Grid::new("Kepler's Second Law").show(ui, |ui| {
                    ui.label("Time interval");
                    ui.add(
                        egui::DragValue::new(time_interval)
                            .custom_formatter(|n, _| format!("{n:.0}"))
                            .suffix(" frames")
                    );
                    ui.end_row();

                    ui.label("Current area so far");
                    ui.label(draw_ui::format_3sf(*swept_area, 0..=1));
                    ui.end_row();

                    ui.label("Past recorded areas:");
                    if ui.button("Reset").clicked() {
                        sweep_separations.clear();
                        logged_areas.clear();
                        *swept_area = 0.0;
                    }
                });

                ui.separator();

                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .max_height(70.0)
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        for (interval, area) in &*logged_areas {
                            ui.label(format!("{area:.3} swept in {interval} frames"));
                        }
                        if logged_areas.is_empty() {
                            ui.label("Recording...");
                        }
                    });
            });
        if !is_open {
            *self = Self::Disabled;
        }
    }
}
