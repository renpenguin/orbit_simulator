use egui::Color32;
use std::{
    cell::RefCell,
    collections::HashSet,
    rc::{Rc, Weak},
};

use crate::app::{Planet, Selection, Vec2};

const MAX_TRAIL_LENGTH: usize = 500;
const RECORD_INTERVAL: usize = 3;

/// A circular queue for storing the last N positions
pub struct TrailPositions {
    planet: Weak<RefCell<Planet>>,
    positions: [Vec2; MAX_TRAIL_LENGTH],
    front: usize,
    back: usize,
}

impl TrailPositions {
    pub fn new(planet: &Rc<RefCell<Planet>>) -> Self {
        let mut positions = [Vec2::ZERO; MAX_TRAIL_LENGTH];
        positions[0] = planet.borrow().pos;
        Self {
            planet: Rc::downgrade(planet),
            positions,
            front: 0,
            back: 0,
        }
    }

    // Adds the planet's current position to the trail. Does nothing if
    pub fn enqueue_position(&mut self) {
        let Some(planet) = self.planet.upgrade() else {
            return;
        };
        let position = planet.borrow().pos;

        self.back = (self.back + 1) % MAX_TRAIL_LENGTH;
        if self.front == self.back {
            self.front = (self.front + 1) % MAX_TRAIL_LENGTH;
        }

        self.positions[self.back] = position;
    }

    /// Remove an item from the front of the positions circular queue. Return true if there are no positions left to remove
    pub fn dequeue_until_empty(&mut self) -> bool {
        // Only one item left. No line to draw, so return `true` to indicate empty
        if self.front == self.back {
            return true;
        }
        // Shift the front forward to remove a position that would be read in `App.draw_trails`.
        self.front = (self.front + 1) % MAX_TRAIL_LENGTH;
        false
    }
}

#[derive(Default)]
pub struct TrailManager {
    pub trails: Vec<TrailPositions>,
    // loop-counts up from 0 to `RECORD_INTERVAL`, records interval when equal to 0
    frame: usize,
}

impl TrailManager {
    pub fn planets_moved(&mut self, planets: &[Rc<RefCell<Planet>>]) {
        // Only record trail positions every `RECORD_INTERVAL` frames
        self.frame = (self.frame + 1) % RECORD_INTERVAL;
        if self.frame != 0 {
            return;
        }

        // Process existing trails. Remove trails whose planets have been deleted
        let mut trail_idx = 0;
        while let Some(trail) = self.trails.get_mut(trail_idx) {
            trail.enqueue_position();

            let planet_exists = trail.planet.upgrade().is_some();
            if !planet_exists {
                let is_empty = trail.dequeue_until_empty();
                if is_empty {
                    self.trails.swap_remove(trail_idx); // Next trail takes place of current
                    continue;
                }
            }
            trail_idx += 1;
        }

        // A list of the address of every planet with a related TrailPostions object
        let trailed_planet_addresses: HashSet<*mut Planet> = self
            .trails
            .iter()
            .filter_map(|t| t.planet.upgrade().map(|planet| planet.as_ptr()))
            .collect();

        // Add new planets to the trails list
        for planet in planets {
            if !trailed_planet_addresses.contains(&planet.as_ptr()) {
                self.trails.push(TrailPositions::new(planet));
            }
        }
    }
}

impl crate::App {
    pub fn draw_trails(&self, painter: &egui::Painter) {
        for trail in &self.trail_manager.trails {
            // Create a list to store the screen point of every trail position
            let mut line_points = Vec::with_capacity(MAX_TRAIL_LENGTH);
            line_points.push(self.sim_point_to_screen(trail.positions[trail.front]));

            let mut position_idx = trail.front;
            while position_idx != trail.back {
                position_idx = (position_idx + 1) % MAX_TRAIL_LENGTH;
                line_points.push(self.sim_point_to_screen(trail.positions[position_idx]));
            }

            let stroke = self.viewport_zoom.clamp(0.4, 2.0) as f32;

            // Trails are dark grey by default, but faded light blue if the planet is selected
            let color = if let Selection::Some { planet, .. } = &self.selection {
                if planet.ptr_eq(&trail.planet) {
                    Color32::from_rgb(0x54, 0x78, 0x84)
                } else {
                    Color32::DARK_GRAY
                }
            } else {
                Color32::DARK_GRAY
            };

            // Draw line
            painter.line(line_points, (stroke, color));
        }
    }
}
