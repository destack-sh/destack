//! destack.imagination.style.stroke@2025.08.15.1

#![destack::partial(destack.imagination.style.stroke, file)]

use crate::{Color, Easing, Vector2};

#[destack::generated(Stroke, -, block)]
/// A stroke value.
pub struct Stroke {
    pub r#type: StrokeType,
    pub template: Option<i64>,
    pub size: u8,
    pub thinning: f32,
    pub smoothing: f32,
    pub streamline: f32,
    pub easing: Easing,
    pub color: Option<Color>,
    pub start: Option<StrokeCap>,
    pub end: Option<StrokeCap>,
}

#[destack::generated(StrokeCap, -, block)]
/// A stroke cap.
pub struct StrokeCap {
    pub cap: bool,
    pub taper: bool,
    pub easing: Easing,
}

#[destack::generated(StrokePath, -, block)]
/// A stroke path.
pub struct StrokePath {
    pub points: Vec<StrokePoint>,
}

#[destack::generated(StrokePoint, -, block)]
/// A computed point in a stroke.
pub struct StrokePoint {
    pub point: Vector2,
    pub original_point: Vector2,
    pub pressure: f32,
    pub direction: Vector2,
    pub distance: f32,
    pub running_length: f32,
    pub radius: f32,
}

#[destack::generated(StrokeType, -, block)]
/// StrokeType
pub enum StrokeType {
    /// A solid stroke
    Solid = 1,
    /// A dashed stroke
    Dashed = 2,
    /// A dotted stroke
    Dotted = 3,
    /// A freehand stroke
    Freehand = 4,
}
