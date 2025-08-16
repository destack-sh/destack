//! destack.imagination.style.shadow@2025.08.15.1

#![destack::partial(destack.imagination.style.shadow, file)]

use crate::{Axis2, Color};

#[destack::generated(Shadow, -, block)]
/// A shadow value.
pub struct Shadow {
    pub r#type: ShadowType,
    pub template: Option<i64 /* TODO */>,
    pub color: Option<Color>,
    pub position: ShadowPosition,
    pub offset: Option<Axis2>,
    pub blur: Option<f32>,
    pub spread: Option<f32>,
    pub diffusion: Option<f32>,
}

#[destack::generated(ShadowType, -, block)]
/// Built-in shadow types.
pub enum ShadowType {
    /// A box shadow
    Box = 10,
    /// A realistic shadow
    Realistic = 11,
}

#[destack::generated(ShadowPosition, -, block)]
/// Built-in shadow positions.
pub enum ShadowPosition {
    /// An outside shadow
    Outside = 1,
    /// An inside shadow
    Inside = 2,
}
