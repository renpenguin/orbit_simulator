mod planet;
use std::{
    cell::{Ref, RefCell},
    rc::Rc,
};

pub use planet::{Planet, TRAIL_SCALE, Vec2};

#[derive(Debug)]
pub struct Simulation {
    pub planets: Vec<Rc<RefCell<Planet>>>,
    pub tick_rate: usize,
    pub is_k2l_enabled: bool,
    pub playing: bool,
}

impl Default for Simulation {
    fn default() -> Self {
        Self {
            planets: vec![],
            tick_rate: 1,
            is_k2l_enabled: false,
            playing: true,
        }
    }
}

impl Simulation {
    // Return an iterator (array) of immutable planets, pre-borrowed
    pub fn get_planets(&self) -> impl Iterator<Item = Ref<'_, Planet>> {
        self.planets.iter().map(|p| p.borrow())
    }

    // Return the index of the first planet found at the passed position
    pub fn try_find_planet_at_pos(&self, pos: Vec2) -> Option<usize> {
        for (i, body) in self.get_planets().enumerate() {
            if (pos - body.pos).length_sq() < body.radius().powi(2) {
                return Some(i);
            }
        }

        None
    }

    // Move planets based on gravity
    pub fn simulate_gravity(&self) {
        let planets_len = self.planets.len(); // Call only once, as the length does not change
        let mut forces = vec![Vec2::ZERO; planets_len];

        for first in 0..planets_len {
            for second in (first + 1)..planets_len {
                let force_on_first = self.planets[first]
                    .borrow()
                    .calculate_force_between_planets(&self.planets[second].borrow());
                forces[first] += force_on_first;
                forces[second] -= force_on_first;
            }
        }

        for (i, planet) in self.planets.iter().enumerate() {
            let mut planet = planet.borrow_mut();

            // Don't move locked planets
            if planet.locked {
                planet.vel = Vec2::ZERO;
                continue;
            }

            let velocity = planet.vel + forces[i] / planet.mass;
            planet.vel = velocity;
            planet.pos += velocity;
        }
    }

    pub fn handle_collisions(&mut self) {
        let planets_len = self.planets.len();
        // Stores true at index of planet to delete
        let mut planets_to_delete = vec![false; planets_len];

        for first in 0..planets_len {
            for second in (first + 1)..planets_len {
                if planets_to_delete[first] || planets_to_delete[second] {
                    continue;
                }

                let separation =
                    self.planets[second].borrow().pos - self.planets[first].borrow().pos;

                let threshold_distance =
                    self.planets[first].borrow().radius() + self.planets[second].borrow().radius();

                if separation.length_sq() < threshold_distance.powi(2) {
                    // Turn first planet into result planet of collision
                    let combined_planet = self.planets[first]
                        .borrow()
                        .collide_planets(&self.planets[second].borrow());
                    self.planets[first] = Rc::new(RefCell::new(combined_planet));
                    // Mark second planet for deletion
                    planets_to_delete[second] = true;
                }
            }
        }

        // Remove all planets marked for deletion
        let mut planet_idx = 0;
        while planet_idx < planets_to_delete.len() {
            if planets_to_delete[planet_idx] {
                self.planets.swap_remove(planet_idx);
                planets_to_delete.swap_remove(planet_idx);
            } else {
                planet_idx += 1;
            }
        }
    }
}
