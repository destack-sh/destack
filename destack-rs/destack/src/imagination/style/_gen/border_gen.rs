//! destack.imagination.style.border

#![destack::generated(destack.imagination.style.border, file)]

use crate::{Border, BorderType};

#[destack::generated(Border, Debug, block)]
impl std::fmt::Debug for Border {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Border")
    }
}

#[destack::generated(BorderType, Debug, block)]
impl std::fmt::Debug for BorderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BorderType::Style => write!(f, "STYLE"),
            BorderType::Solid => write!(f, "SOLID"),
            BorderType::Dashed => write!(f, "DASHED"),
            BorderType::Dotted => write!(f, "DOTTED"),
            BorderType::Double => write!(f, "DOUBLE"),
        }
    }
}
