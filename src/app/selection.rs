use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use crate::app::{
    ClickMode,
    simulation::{Planet, TAIL_SCALE, Vec2},
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SelectionMode {
    Selecting,
    Moving {
        original_pos: Vec2,
    },
    Resizing {
        original_mass: f64,
        original_distance_sq: f64,
    },
    Aiming {
        original_velocity: Vec2,
        snap_to_mouse: bool,
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
            ClickMode::Select => SelectionMode::Selecting,
            ClickMode::Move => SelectionMode::Moving { original_pos: planet_data.pos },
            ClickMode::Resize => SelectionMode::Resizing {
                original_mass: planet_data.mass,
                original_distance_sq: (mouse_pos - planet_data.pos).length_sq(),
            },
            ClickMode::Velocity => SelectionMode::Aiming { original_velocity: planet_data.vel, snap_to_mouse: true },
            _ => return Self::None,
        };

        Self::Some {
            mode,
            planet: Rc::downgrade(planet),
            initial_mouse_pos: mouse_pos,
        }
    }

    pub fn new_vel_unsnapped_to_mouse(planet: &Rc<RefCell<Planet>>, mouse_pos: Vec2) -> Self {
        Self::Some {
            mode: SelectionMode::Aiming {
                original_velocity: planet.borrow().vel,
                snap_to_mouse: false,
            },
            planet: Rc::downgrade(planet),
            initial_mouse_pos: mouse_pos,
        }
    }

    pub fn is_none(&self) -> bool {
        matches!(*self, Self::None)
    }

    /// Return a strong reference to a planet, if there is a selection and the planet exists
    pub fn extract_planet(&mut self) -> Option<Rc<RefCell<Planet>>> {
        // Only continue if there is a selection
        let Self::Some { planet, .. } = self else {
            return None;
        };

        // If the planet has been deleted, forget the selection
        if planet.strong_count() == 0 {
            *self = Self::None;
            return None;
        }

        planet.upgrade()
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
            SelectionMode::Selecting => (),
            // Move planet
            SelectionMode::Moving { original_pos } => {
                planet.pos = *original_pos + mouse_pos - *initial_mouse_pos;
            }
            // Resize planet
            SelectionMode::Resizing {
                original_mass,
                original_distance_sq,
            } => {
                let current_distance_sq = (planet.pos - mouse_pos).length_sq();
                if *original_distance_sq == 0.0 {
                    *original_distance_sq = current_distance_sq;
                } else {
                    // Current distance to planet / original distance to planet
                    let scale_ratio = current_distance_sq / *original_distance_sq;
                    planet.mass = *original_mass * scale_ratio;
                }
            }
            // Aim planet
            SelectionMode::Aiming {
                original_velocity,
                snap_to_mouse,
            } => {
                if *snap_to_mouse {
                    // Aim with tool + mouse click
                    planet.vel = (planet.pos - mouse_pos) / TAIL_SCALE;
                } else {
                    // Aim with Select + V
                    planet.vel = *original_velocity - (mouse_pos - *initial_mouse_pos) / TAIL_SCALE;
                }
            }
        }
    }
}
