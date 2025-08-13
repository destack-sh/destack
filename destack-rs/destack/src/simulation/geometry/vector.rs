use core::fmt;
use core::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

#[destack::generated(Vector2, struct, full)]
#[derive(Debug, Clone, Copy)]
pub struct Vector2 {
    pub x: f32,
    pub y: f32,
}

#[destack::generated(Vector2, PartialEq, full)]
impl PartialEq for Vector2 {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y
    }
}

#[destack::generated(Vector2, Default, full)]
impl Default for Vector2 {
    fn default() -> Self {
        Self::ZERO
    }
}

#[destack::generated(Vector2, Display, full)]
impl fmt::Display for Vector2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

#[destack::generated(Vector2, partial)]
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

    #[inline]
    /// Return the y component.
    pub const fn y(&self) -> f32 {
        self.y
    }

    /// Return a vector with absolute-valued components.
    #[inline]
    pub fn abs(self) -> Self {
        Self {
            x: self.x.abs(),
            y: self.y.abs(),
        }
    }

    /// Return the perpendicular vector rotated 90 degrees counterclockwise.
    #[inline]
    pub fn perp(self) -> Self {
        Self {
            x: -self.y,
            y: self.x,
        }
    }

    /// Compute dot product with another vector.
    #[inline]
    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y
    }

    /// Compute the 2D cross product z-component (useful for angles/orientation).
    #[inline]
    pub fn cross_z(self, other: Self) -> f32 {
        self.x * other.y - self.y * other.x
    }

    /// Return squared magnitude.
    #[inline]
    pub fn magnitude2(self) -> f32 {
        self.dot(self)
    }

    /// Return magnitude (length).
    #[inline]
    pub fn magnitude(self) -> f32 {
        self.magnitude2().sqrt()
    }

    /// Return a normalized vector. Returns ZERO if the vector has zero length.
    #[inline]
    pub fn normalize(self) -> Self {
        let m = self.magnitude();
        if m > 0.0 { self / m } else { Self::ZERO }
    }

    /// Compute Euclidean distance to another vector.
    #[inline]
    pub fn distance(self, other: Self) -> f32 {
        (self - other).magnitude()
    }

    /// Compute squared Euclidean distance to another vector.
    #[inline]
    pub fn distance2(self, other: Self) -> f32 {
        (self - other).magnitude2()
    }

    /// Compute the signed angle to another vector in radians in range (-PI, PI].
    /// Uses atan2 of cross and dot for numerical robustness.
    #[inline]
    pub fn angle(self, other: Self) -> f32 {
        self.cross_z(other).atan2(self.dot(other))
    }

    /// Linearly interpolate between this vector and another by t in [0, 1].
    #[inline]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        self * (1.0 - t) + other * t
    }

    /// Rotate this point around `center` by `angle` radians (CCW).
    #[inline]
    pub fn rot_with(self, center: Self, angle: f32) -> Self {
        let s = angle.sin();
        let c = angle.cos();
        let p = self - center;
        let x = c * p.x - s * p.y;
        let y = s * p.x + c * p.y;
        Vector2 { x, y } + center
    }

    /// Return the number of components (always 2).
    #[inline]
    pub const fn len_components(&self) -> usize {
        2
    }
}

// Arithmetic ops with another Vector2
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
