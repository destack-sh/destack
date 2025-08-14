use core::fmt;

use crate::Vector2;

#[destack::generated(Vector2, PartialEq, block)]
impl PartialEq for Vector2 {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y
    }
}

#[destack::generated(Vector2, Default, block)]
impl Default for Vector2 {
    fn default() -> Self {
        Self::ZERO
    }
}

#[destack::generated(Vector2, Display, block)]
impl fmt::Display for Vector2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

#[destack::generated(Vector2, impl, block)]
impl Vector2 {
    #[destack::generated(Vector2, ZERO, line)]
    pub const ZERO: Vector2 = Vector2 { x: 0.0, y: 0.0 };

    #[destack::generated(Vector2, ONE, line)]
    pub const ONE: Vector2 = Vector2 { x: 1.0, y: 1.0 };

    #[destack::generated(Vector2, X_AXIS, line)]
    pub const X_AXIS: Vector2 = Vector2 { x: 1.0, y: 0.0 };

    #[destack::generated(Vector2, Y_AXIS, line)]
    pub const Y_AXIS: Vector2 = Vector2 { x: 0.0, y: 1.0 };
}