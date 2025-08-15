//! destack.imagination.style.gradient@2025.08.14.0

#![destack::partial(destack.imagination.style.gradient, file)]

#[destack::generated(Gradient, struct, block)]
/// A gradient value.
pub struct Gradient {

}

#[destack::generated(GradientStop, struct, block)]
/// A gradient stop with color and position.
pub struct GradientStop {

}

#[destack::generated(GradientType, enum, block)]
/// Built-in gradient types.
pub enum GradientType {
    /// A linear gradient
    LINEAR = 10,
    /// A radial gradient
    RADIAL = 11,
    /// A conic gradient
    CONIC = 12,
    /// A diamond gradient
    DIAMOND = 13
}