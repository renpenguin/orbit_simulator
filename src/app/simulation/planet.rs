use std::{
    cell::RefCell,
    ops::{Add, AddAssign, Div, Mul, Sub, SubAssign},
    rc::Rc,
};

use egui::Pos2;

const G: f64 = 2.0; // Change later to 6.67e-11

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, Copy, PartialEq)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

impl Vec2 {
    pub const ZERO: Self = Self::new(0.0, 0.0);

    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    // Return the magnitude of the vector squared
    pub fn length_sq(&self) -> f64 {
        self.x * self.x + self.y * self.y
    }
}

impl From<Vec2> for Pos2 {
    fn from(value: Vec2) -> Self {
        Self::new(value.x as f32, value.y as f32)
    }
}
impl From<Pos2> for Vec2 {
    fn from(value: Pos2) -> Self {
        Self::new(value.x as f64, value.y as f64)
    }
}impl From<egui::Vec2> for Vec2 {
    fn from(value: egui::Vec2) -> Self {
        Self::new(value.x as f64, value.y as f64)
    }
}

impl Add for Vec2 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}
impl AddAssign for Vec2 {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl Sub for Vec2 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}
impl SubAssign for Vec2 {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl Div<f64> for Vec2 {
    type Output = Self;

    fn div(self, rhs: f64) -> Self::Output {
        Self::new(self.x / rhs, self.y / rhs)
    }
}

impl Mul<Vec2> for f64 {
    type Output = Vec2;
    fn mul(self, rhs: Vec2) -> Self::Output {
        Vec2::new(self * rhs.x, self * rhs.y)
    }
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, Copy)]
pub struct Planet {
    pub pos: Vec2, // Position
    pub vel: Vec2, // Velocity
    pub mass: f64,
    pub locked: bool,
    pub popup_open: bool,
}

impl Planet {
    pub fn new(pos: Vec2, mass: f64) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self {
            pos,
            vel: Vec2::ZERO,
            mass,
            locked: false,
            popup_open: true,
        }))
    }

    // Calculate the radius of the planet
    pub fn radius(&self) -> f64 {
        const PLANET_DESCALE: f64 = 4.0;
        self.mass.sqrt() / PLANET_DESCALE
    }

    // Calculate a vector of the gravitational force towards the other planet
    pub fn calculate_force_between_planets(&self, other: &Self) -> Vec2 {
        let separation = other.pos - self.pos;
        // # We can calculate d^2 instead of d. This way we don't need to call sqrt()
        let distance_squared = separation.length_sq();

        // F_g = G * m_1 * m_2 / d^2
        let magnitude = G * self.mass * other.mass / distance_squared;
        // # Calculate angle from first planet to second planet
        let direction = separation.y.atan2(separation.x);

        magnitude * Vec2::new(direction.cos(), direction.sin())
    }

    // Return the resulting planet from a collision between two planets
    pub fn collide_planets(&self, other: &Self) -> Self {
        // m1v1 + m2v2
        let total_initial_momentum = self.mass * self.vel + other.mass * other.vel;
        let mass = self.mass + other.mass;

        // Weighted average position based on radii
        let separation = other.pos - self.pos;
        let threshold_distance = self.radius() + other.radius();
        let separation_ratio = other.radius() / threshold_distance;

        Self {
            pos: self.pos + separation_ratio * separation,
            vel: total_initial_momentum / mass, // Combined velocity based on combining masses
            mass,
            locked: self.locked || other.locked,
            popup_open: self.popup_open || other.popup_open,
        }
    }
}
