//! destack.imagination.style.stroke

#![destack::generated(destack.imagination.style.stroke, file)]

use crate::{Stroke, StrokeCap, StrokePath, StrokePoint, StrokeType};

#[destack::generated(Stroke, Debug, block)]
impl std::fmt::Debug for Stroke {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Stroke")
    }
}

#[destack::generated(StrokeCap, Debug, block)]
impl std::fmt::Debug for StrokeCap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "StrokeCap")
    }
}

#[destack::generated(StrokePath, Debug, block)]
impl std::fmt::Debug for StrokePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "StrokePath")
    }
}

#[destack::generated(StrokePoint, Debug, block)]
impl std::fmt::Debug for StrokePoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "StrokePoint")
    }
}

#[destack::generated(StrokeType, Debug, block)]
impl std::fmt::Debug for StrokeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StrokeType::Solid => write!(f, "SOLID"),
            StrokeType::Dashed => write!(f, "DASHED"),
            StrokeType::Dotted => write!(f, "DOTTED"),
            StrokeType::Freehand => write!(f, "FREEHAND"),
        }
    }
}
