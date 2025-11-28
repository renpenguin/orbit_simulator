mod planet;
use std::{
    cell::{Ref, RefCell},
    rc::Rc,
};

use egui::Pos2;
pub use planet::{Planet, Vec2};

#[derive(serde::Deserialize, serde::Serialize, Debug)]
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

    pub fn spawn_planet_at(&mut self, pos: Pos2) {
        self.planets.push(Planet::new(pos.into(), 80.0));
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
            let vel = planet.vel + forces[i];
            planet.vel = vel;
            planet.pos += vel;
        }
    }

    pub fn collide_planets(&mut self) {
        todo!();
    }
}
