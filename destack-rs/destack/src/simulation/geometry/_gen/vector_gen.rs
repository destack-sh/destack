#![destack::generated(vector, file)]

use core::ops::Sub;

use crate::Vector2;

#[destack::partial(Vector2, Sub:Vector2, block)]
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

impl Vector2 {
    pub fn extra_method(&self) -> f32 {
        self.x + self.y
    }
}
