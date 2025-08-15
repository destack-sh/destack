//! destack.imagination.style.fill@2025.08.15.1

#![destack::generated(destack.imagination.style.fill, file)]

use crate::FillPosition;
use crate::FillSize;
use crate::FillType;

#[destack::generated(FillType, Debug, block)]
impl std::fmt::Debug for FillType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FillType::Solid => write!(f, "SOLID"),
            FillType::Gradient => write!(f, "GRADIENT"),
            FillType::Image => write!(f, "IMAGE"),
        }
    }
}

#[destack::generated(FillPosition, Debug, block)]
impl std::fmt::Debug for FillPosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FillPosition::TopLeft => write!(f, "TOP_LEFT"),
            FillPosition::TopCenter => write!(f, "TOP_CENTER"),
            FillPosition::TopRight => write!(f, "TOP_RIGHT"),
            FillPosition::Left => write!(f, "LEFT"),
            FillPosition::Center => write!(f, "CENTER"),
            FillPosition::Right => write!(f, "RIGHT"),
            FillPosition::BottomLeft => write!(f, "BOTTOM_LEFT"),
            FillPosition::BottomCenter => write!(f, "BOTTOM_CENTER"),
            FillPosition::BottomRight => write!(f, "BOTTOM_RIGHT"),
        }
    }
}

#[destack::generated(FillSize, Debug, block)]
impl std::fmt::Debug for FillSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FillSize::Fill => write!(f, "FILL"),
            FillSize::Stretch => write!(f, "STRETCH"),
            FillSize::Fit => write!(f, "FIT"),
            FillSize::Tile => write!(f, "TILE"),
        }
    }
}
