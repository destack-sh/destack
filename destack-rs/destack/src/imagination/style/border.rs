//! destack.imagination.style.border@2025.08.15.1

#![destack::partial(destack.imagination.style.border, file)]

#[destack::generated(Border, struct, block)]
/// A border value.
pub struct Border {

}

#[destack::generated(BorderType, enum, block)]
/// Built-in border types.
pub enum BorderType {
    /// A border style
    STYLE = 2,
    /// A solid border
    SOLID = 10,
    /// A dashed border
    DASHED = 11,
    /// A dotted border
    DOTTED = 12,
    /// A double border
    DOUBLE = 13
}