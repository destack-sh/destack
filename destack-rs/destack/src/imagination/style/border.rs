//! destack.imagination.style.border@2025.08.15.1

#![destack::partial(destack.imagination.style.border, file)]

use crate::BorderType;
use crate::Color;
use crate::Inset2;

#[destack::generated(Border, , block)]
/// A border value.
pub struct Border {
    r#type: BorderType,
    color: Option<Color>,
    width: Option<Inset2>,
    template: Option<i64 /* TODO */>,
}

#[destack::generated(BorderType, , block)]
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
