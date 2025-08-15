//! destack.imagination.style.gradient@2025.08.15.1

#![destack::partial(destack.imagination.style.gradient, file)]

use crate::Axis2;
use crate::Color;

#[destack::generated(Gradient, struct, block)]
/// A gradient value.
pub struct Gradient {
    r#type: GradientType,
    template: i64, /* TODO */
    angle: f32,
    stops: Vec<GradientStop>,
    center_anchor: Axis2,
}

#[destack::generated(GradientStop, struct, block)]
/// A gradient stop with color and position.
pub struct GradientStop {
    color: Color,
    position: f32,
}

#[destack::generated(GradientType, enum, block)]
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
