//! destack.imagination.style.shadow@2025.08.15.1

#![destack::partial(destack.imagination.style.shadow, file)]

use crate::{Axis2, Color};

#[destack::generated(Shadow, struct, block)]
/// A shadow value.
pub struct Shadow {
    r#type: ShadowType,
    template: i64 /* TODO */ ,
    color: Color,
    position: ShadowPosition,
    offset: Axis2,
    blur: f32,
    spread: f32,
    diffusion: f32
}

#[destack::generated(ShadowType, enum, block)]
/// Built-in shadow types.
pub enum ShadowType {
    /// A box shadow
    Box = 10,
    /// A realistic shadow
    Realistic = 11
}

#[destack::generated(ShadowPosition, enum, block)]
/// Built-in shadow positions.
pub enum ShadowPosition {
    /// An outside shadow
    Outside = 1,
    /// An inside shadow
    Inside = 2
}