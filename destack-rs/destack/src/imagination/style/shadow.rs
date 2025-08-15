//! destack.imagination.style.shadow@2025.08.15.1

#![destack::partial(destack.imagination.style.shadow, file)]

use crate::Axis2;
use crate::Color;

#[destack::generated(Shadow, , block)]
/// A shadow value.
pub struct Shadow {
    r#type: ShadowType,
    template: Option<i64 /* TODO */>,
    color: Option<Color>,
    position: ShadowPosition,
    offset: Option<Axis2>,
    blur: Option<f32>,
    spread: Option<f32>,
    diffusion: Option<f32>,
}

#[destack::generated(ShadowType, , block)]
/// Built-in shadow types.
pub enum ShadowType {
    /// A box shadow
    Box = 10,
    /// A realistic shadow
    Realistic = 11,
}

#[destack::generated(ShadowPosition, , block)]
/// Built-in shadow positions.
pub enum ShadowPosition {
    /// An outside shadow
    Outside = 1,
    /// An inside shadow
    Inside = 2,
}
