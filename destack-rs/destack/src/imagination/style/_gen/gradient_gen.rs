//! destack.imagination.style.gradient

#![destack::generated(destack.imagination.style.gradient, file)]

use crate::{Gradient, GradientStop, GradientType};

#[destack::generated(Gradient, Debug, block)]
impl std::fmt::Debug for Gradient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Gradient")
    }
}

#[destack::generated(GradientStop, Debug, block)]
impl std::fmt::Debug for GradientStop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GradientStop")
    }
}

#[destack::generated(GradientType, Debug, block)]
impl std::fmt::Debug for GradientType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GradientType::Linear => write!(f, "LINEAR"),
            GradientType::Radial => write!(f, "RADIAL"),
            GradientType::Conic => write!(f, "CONIC"),
            GradientType::Diamond => write!(f, "DIAMOND"),
        }
    }
}
