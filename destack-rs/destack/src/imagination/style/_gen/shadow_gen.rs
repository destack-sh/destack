//! destack.imagination.style.shadow@2025.08.15.1

#![destack::generated(destack.imagination.style.shadow, file)]

use crate::{Shadow, ShadowPosition, ShadowType};

#[destack::generated(Shadow, Debug, block)]
impl std::fmt::Debug for Shadow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Shadow")
    }
}

#[destack::generated(ShadowType, Debug, block)]
impl std::fmt::Debug for ShadowType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShadowType::Box => write!(f, "BOX"),
            ShadowType::Realistic => write!(f, "REALISTIC"),
        }
    }
}

#[destack::generated(ShadowPosition, Debug, block)]
impl std::fmt::Debug for ShadowPosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShadowPosition::Outside => write!(f, "OUTSIDE"),
            ShadowPosition::Inside => write!(f, "INSIDE"),
        }
    }
}
