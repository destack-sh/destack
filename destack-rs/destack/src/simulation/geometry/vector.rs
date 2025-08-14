#![destack::partial(vector, file)]

use core::ops::Add;

#[destack::generated(Vector2, struct, block)]
#[derive(Debug, Clone, Copy)]
pub struct Vector2 {
    pub x: f32,
    pub y: f32,
}

#[destack::partial(Vector2, impl, block)]
impl Vector2 {
    #[destack::stub(Vector2, x, block)]
    #[inline]
    /// Return the x component.
    pub const fn x(&self) -> f32 {
        self.x
    }
}

#[destack::partial(Vector2, Add:Vector2, block)]
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
