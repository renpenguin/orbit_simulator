use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use crate::app::{
    ClickMode,
    simulation::{Planet, TRAIL_SCALE, Vec2},
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SelectionMode {
    Selected,
    Translating {
        original_pos: Vec2,
    },
    Scaling {
        original_mass: f64,
        original_distance_sq: f64,
    },
    Aiming {
        original_velocity: Vec2,
    },
}

pub enum Selection {
    Some {
        mode: SelectionMode,
        planet: Weak<RefCell<Planet>>,
        initial_mouse_pos: Vec2,
    },
    None,
}

impl Selection {
    pub fn new(click_mode: ClickMode, planet: &Rc<RefCell<Planet>>, mouse_pos: Vec2) -> Self {
        let planet_data = planet.borrow();
        #[rustfmt::skip]
        let mode = match click_mode {
            ClickMode::Select => SelectionMode::Selected,
            ClickMode::Translate => SelectionMode::Translating { original_pos: planet_data.pos },
            ClickMode::Scale => SelectionMode::Scaling {
                original_mass: planet_data.mass,
                original_distance_sq: (mouse_pos - planet_data.pos).length_sq(),
            },
            ClickMode::Aim => SelectionMode::Aiming { original_velocity: planet_data.vel },
            _ => return Self::None,
        };

        Self::Some {
            mode,
            planet: Rc::downgrade(planet),
            initial_mouse_pos: mouse_pos,
        }
    }

    pub fn mouse_motion(&mut self, mouse_pos: Vec2) {
        // Ensure that a selection exists
        #[rustfmt::skip]
        let Self::Some { mode, planet, initial_mouse_pos } = self else {
            return;
        };

        // Access the planet reference, and make sure it's still valid
        let Some(planet) = planet.upgrade() else {
            *self = Self::None;
            return;
        };
        let mut planet = planet.borrow_mut();

        match mode {
            // Don't change anything if the planet is only selected
            SelectionMode::Selected => (),
            // Move planet
            SelectionMode::Translating { original_pos } => {
                planet.pos = *original_pos + mouse_pos - *initial_mouse_pos;
            }
            // Scale planet
            SelectionMode::Scaling {
                original_mass,
                original_distance_sq,
            } => {
                // Current distance to planet / original distance to planet
                let scale_ratio = (planet.pos - mouse_pos).length_sq() / *original_distance_sq;
                planet.mass = *original_mass * scale_ratio;
            }
            // Aim planet
            SelectionMode::Aiming { original_velocity } => {
                planet.vel = *original_velocity - (mouse_pos - *initial_mouse_pos) / TRAIL_SCALE;
            }
        }
    }
}
