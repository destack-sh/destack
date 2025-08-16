//! destack.imagination.style.gradient@2025.08.15.1

#![destack::partial(destack.imagination.style.gradient, file)]

use crate::{Axis2, Color};

#[destack::generated(Gradient, -, block)]
/// A gradient value.
pub struct Gradient {
    pub r#type: GradientType,
    pub template: Option<i64 /* TODO */>,
    pub angle: Option<f32>,
    pub stops: Vec<GradientStop>,
    pub center_anchor: Option<Axis2>,
}

#[destack::generated(GradientStop, -, block)]
/// A gradient stop with color and position.
pub struct GradientStop {
    pub color: Option<Color>,
    pub position: f32,
}

#[destack::generated(GradientType, -, block)]
/// Built-in gradient types.
pub enum GradientType {
    /// A linear gradient
    Linear = 10,
    /// A radial gradient
    Radial = 11,
    /// A conic gradient
    Conic = 12,
    /// A diamond gradient
    Diamond = 13,
}
