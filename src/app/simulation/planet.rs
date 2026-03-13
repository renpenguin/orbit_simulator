use std::{
    cell::RefCell,
    ops::{Add, AddAssign, Div, Mul, Sub, SubAssign},
    rc::Rc,
};

use egui::Pos2;

// G in mAU^3 muM0^-1 day^-2
const G: f64 = 0.29591;
pub const TAIL_SCALE: f64 = 8.0;

pub fn get_planet_name_from_index(idx: usize) -> String {
    const NAMES_LEN: usize = 18;
    const NAMES: [&str; NAMES_LEN] = [
        "Alpha", "Beta", "Gamma", "Delta", "Epsilon", "Zeta", "Eta", "Theta", "Iota", "Kappa",
        "Lambda", "Mu", "Xi", "Omicron", "Rho", "Sigma", "Tau", "Omega",
    ];

    let mut name = String::from(NAMES[idx % NAMES_LEN]);

    let repeat = idx / NAMES_LEN;
    if repeat > 0 {
        name.push(' ');
        name.push_str(&repeat.to_string());
    }

    name
}

// Fast inverse-square-root function
pub fn inv_sqrt(x: f64) -> f64 {
    let half_x = 0.5 * x;
    let i = x.to_bits();
    let i = 0x5fe6eb50c7b537a9u64 - (i >> 1);
    let mut y = f64::from_bits(i);

    y = y * (1.5 - half_x * y * y);
    y * (1.5 - half_x * y * y)
}

#[derive(Debug, Clone, Copy, PartialEq)]
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

    pub fn cross(&self, other: &Self) -> f64 {
        self.x * other.y - self.y * other.x
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
}
impl From<egui::Vec2> for Vec2 {
    fn from(value: egui::Vec2) -> Self {
        Self::new(value.x as f64, value.y as f64)
    }
}
impl From<Vec2> for egui::Vec2 {
    fn from(value: Vec2) -> Self {
        Self::new(value.x as f32, value.y as f32)
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

#[derive(Debug, Clone)]
pub struct Planet {
    pub pos: Vec2, // Position
    pub vel: Vec2, // Velocity
    pub mass: f64,
    pub locked: bool,
    pub popup_open: bool,
}

impl Planet {
    pub fn new(pos: Vec2, mass: f64) -> Rc<RefCell<Self>> {
        Self {
            pos,
            vel: Vec2::ZERO,
            mass,
            locked: false,
            popup_open: false,
        }
        .as_rc()
    }

    /// Return the planet as a reference-counted object
    pub fn as_rc(&self) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(self.clone()))
    }

    /// Calculate the radius of the planet
    pub fn radius(&self) -> f64 {
        const PLANET_DESCALE: f64 = 8.0;
        self.mass.sqrt() / PLANET_DESCALE
    }

    /// Calculate a vector of the gravitational force towards the other planet
    pub fn calculate_force_between_planets(&self, other: &Self) -> Vec2 {
        let separation = other.pos - self.pos;
        // # We can calculate d^3 instead of d and d^2
        let distance_inverse_cubed = inv_sqrt(separation.length_sq()).powi(3);

        // F_g = G * m_1 * m_2 / d^2
        let magnitude = G * self.mass * other.mass; // / distance_squared (instead .powi(3) in final equation)

        // d = |DeltaX|, return |F_g| * DeltaX / d
        magnitude * distance_inverse_cubed * separation
    }

    /// Return the resulting planet from a collision between two planets
    pub fn collide_planets(&self, other: &Self) -> Self {
        // m1v1 + m2v2
        let total_initial_momentum = self.mass * self.vel + other.mass * other.vel;
        let mass = self.mass + other.mass;

        // Weighted average position based on radii
        let separation = other.pos - self.pos;
        let threshold_distance = self.radius() + other.radius();
        let separation_ratio = other.radius() / threshold_distance;

        let larger = if self.mass > other.mass { self } else { other };

        Self {
            pos: self.pos + separation_ratio * separation,
            vel: total_initial_momentum / mass, // Combined velocity based on combining masses
            mass,
            locked: larger.locked,
            popup_open: larger.popup_open,
        }
    }
}
