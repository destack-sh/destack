use core::fmt;
use core::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

#[destack::generated(Vector2, struct, block)]
#[derive(Debug, Clone, Copy)]
pub struct Vector2 {
    pub x: f32,
    pub y: f32,
}

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

#[destack::partial(Vector2, impl, block)]
impl Vector2 {
    #[destack::generated(Vector2, ZERO, line)]
    pub const ZERO: Vector2 = Vector2 { x: 0.0, y: 0.0 };

    #[destack::generated(Vector2, ONE, line)]
    pub const ONE: Vector2 = Vector2 { x: 1.0, y: 1.0 };

    #[destack::generated(Vector2, X_AXIS, line)]
    pub const X_AXIS: Vector2 = Vector2 { x: 1.0, y: 0.0 };

    #[destack::generated(Vector2, Y_AXIS, line)]
    pub const Y_AXIS: Vector2 = Vector2 { x: 0.0, y: 1.0 };

    #[destack::stub(Vector2, x, function_stub)]
    #[inline]
    /// Return the x component.
    pub const fn x(&self) -> f32 {
        self.x
    }
}

// Arithmetic ops with another Vector2
#[destack::partial(Vector2, Add, block)]
impl Add for Vector2 {
    type Output = Vector2;
    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Vector2 {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl Sub for Vector2 {
    type Output = Vector2;
    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        Vector2 {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl Mul for Vector2 {
    type Output = Vector2;
    #[inline]
    fn mul(self, rhs: Self) -> Self::Output {
        Vector2 {
            x: self.x * rhs.x,
            y: self.y * rhs.y,
        }
    }
}

impl Div for Vector2 {
    type Output = Vector2;
    #[inline]
    fn div(self, rhs: Self) -> Self::Output {
        Vector2 {
            x: self.x / rhs.x,
            y: self.y / rhs.y,
        }
    }
}

// Arithmetic ops with scalar on RHS
impl Add<f32> for Vector2 {
    type Output = Vector2;
    #[inline]
    fn add(self, rhs: f32) -> Self::Output {
        Vector2 {
            x: self.x + rhs,
            y: self.y + rhs,
        }
    }
}

impl Sub<f32> for Vector2 {
    type Output = Vector2;
    #[inline]
    fn sub(self, rhs: f32) -> Self::Output {
        Vector2 {
            x: self.x - rhs,
            y: self.y - rhs,
        }
    }
}

impl Mul<f32> for Vector2 {
    type Output = Vector2;
    #[inline]
    fn mul(self, rhs: f32) -> Self::Output {
        Vector2 {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

impl Div<f32> for Vector2 {
    type Output = Vector2;
    #[inline]
    fn div(self, rhs: f32) -> Self::Output {
        Vector2 {
            x: self.x / rhs,
            y: self.y / rhs,
        }
    }
}

// Arithmetic ops with scalar on LHS
impl Add<Vector2> for f32 {
    type Output = Vector2;
    #[inline]
    fn add(self, rhs: Vector2) -> Self::Output {
        rhs + self
    }
}

impl Sub<Vector2> for f32 {
    type Output = Vector2;
    #[inline]
    fn sub(self, rhs: Vector2) -> Self::Output {
        Vector2 {
            x: self - rhs.x,
            y: self - rhs.y,
        }
    }
}

impl Mul<Vector2> for f32 {
    type Output = Vector2;
    #[inline]
    fn mul(self, rhs: Vector2) -> Self::Output {
        rhs * self
    }
}

impl Div<Vector2> for f32 {
    type Output = Vector2;
    #[inline]
    fn div(self, rhs: Vector2) -> Self::Output {
        Vector2 {
            x: self / rhs.x,
            y: self / rhs.y,
        }
    }
}

// Assignment variants
impl AddAssign for Vector2 {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl SubAssign for Vector2 {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl MulAssign for Vector2 {
    #[inline]
    fn mul_assign(&mut self, rhs: Self) {
        self.x *= rhs.x;
        self.y *= rhs.y;
    }
}

impl DivAssign for Vector2 {
    #[inline]
    fn div_assign(&mut self, rhs: Self) {
        self.x /= rhs.x;
        self.y /= rhs.y;
    }
}

impl AddAssign<f32> for Vector2 {
    #[inline]
    fn add_assign(&mut self, rhs: f32) {
        self.x += rhs;
        self.y += rhs;
    }
}

impl SubAssign<f32> for Vector2 {
    #[inline]
    fn sub_assign(&mut self, rhs: f32) {
        self.x -= rhs;
        self.y -= rhs;
    }
}

impl MulAssign<f32> for Vector2 {
    #[inline]
    fn mul_assign(&mut self, rhs: f32) {
        self.x *= rhs;
        self.y *= rhs;
    }
}

impl DivAssign<f32> for Vector2 {
    #[inline]
    fn div_assign(&mut self, rhs: f32) {
        self.x /= rhs;
        self.y /= rhs;
    }
}

impl Neg for Vector2 {
    type Output = Vector2;
    #[inline]
    fn neg(self) -> Self::Output {
        Vector2 {
            x: -self.x,
            y: -self.y,
        }
    }
}

impl From<(f32, f32)> for Vector2 {
    #[inline]
    fn from(value: (f32, f32)) -> Self {
        Vector2 {
            x: value.0,
            y: value.1,
        }
    }
}

impl From<[f32; 2]> for Vector2 {
    #[inline]
    fn from(value: [f32; 2]) -> Self {
        Vector2 {
            x: value[0],
            y: value[1],
        }
    }
}

impl From<Vector2> for (f32, f32) {
    #[inline]
    fn from(value: Vector2) -> Self {
        (value.x, value.y)
    }
}

impl From<Vector2> for [f32; 2] {
    #[inline]
    fn from(value: Vector2) -> Self {
        [value.x, value.y]
    }
}
