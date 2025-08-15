//! destack.imagination.style.border@2025.08.15.1

#![destack::partial(destack.imagination.style.border, file)]

#[destack::generated(Border, struct, block)]
/// A border value.
pub struct Border {}

#[destack::generated(BorderType, enum, block)]
/// Built-in border types.
pub enum BorderType {
    /// A border style
    Style = 2,
    /// A solid border
    Solid = 10,
    /// A dashed border
    Dashed = 11,
    /// A dotted border
    Dotted = 12,
    /// A double border
    Double = 13,
}
