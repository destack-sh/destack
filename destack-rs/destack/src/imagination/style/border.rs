//! destack.imagination.style.border@2025.08.15.1

#![destack::partial(destack.imagination.style.border, file)]

use crate::{Inset2, Color};

#[destack::generated(Border, struct, block)]
/// A border value.
pub struct Border {
    r#type: BorderType,
    color: Color,
    width: Inset2,
    template: i64 /* TODO */ 
}

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
    Double = 13
}