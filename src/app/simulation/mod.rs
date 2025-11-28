mod planet;
use std::rc::Rc;

use egui::Pos2;
pub use planet::{Planet, Vec2};

#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct Simulation {
    pub planets: Vec<Rc<Planet>>,
    pub tick_rate: usize,
    pub is_k2l_enabled: bool,
    pub paused: bool,
}

impl Default for Simulation {
    fn default() -> Self {
        Self {
            planets: vec![],
            tick_rate: 1,
            is_k2l_enabled: false,
            paused: false,
        }
    }
}

impl Simulation {
    pub fn spawn_planet_at(&mut self, pos: Pos2) {
        self.planets.push(Planet::new(pos.into(), 80.0));
    }

    // Move planets based on gravity
    pub fn simulate_gravity(&mut self) {
        let planets_len = self.planets.len(); // Call only once, as the length does not change
        let mut forces = vec![Vec2::ZERO; planets_len];

        for first in 0..planets_len {
            for second in (first + 1)..planets_len {
                let force_on_first =
                    self.planets[first].calculate_force_between_planets(&self.planets[second]);
                forces[first] += force_on_first;
                forces[second] -= force_on_first;
            }
        }

        for planet in 0..planets_len {
            self.planets[planet].vel += forces[planet];
            self.planets[planet].pos += forces[planet];
        }
    }

    pub fn collide_planets(&mut self) {
        todo!();
    }
}
