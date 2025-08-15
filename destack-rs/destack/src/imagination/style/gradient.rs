//! destack.imagination.style.gradient@2025.08.15.1

#![destack::partial(destack.imagination.style.gradient, file)]

#[destack::generated(Gradient, struct, block)]
/// A gradient value.
pub struct Gradient {}

#[destack::generated(GradientStop, struct, block)]
/// A gradient stop with color and position.
pub struct GradientStop {}

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
