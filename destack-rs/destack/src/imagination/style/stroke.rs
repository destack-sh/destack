//! destack.imagination.style.stroke@2025.08.15.1

#![destack::partial(destack.imagination.style.stroke, file)]

use crate::Color;
use crate::Easing;
use crate::Vector2;

#[destack::generated(Stroke, , block)]
/// A stroke value.
pub struct Stroke {
    r#type: StrokeType,
    template: Option<i64 /* TODO */>,
    size: u8,
    thinning: f32,
    smoothing: f32,
    streamline: f32,
    easing: Easing,
    color: Option<Color>,
    start: Option<StrokeCap>,
    end: Option<StrokeCap>,
}

#[destack::generated(StrokeCap, , block)]
/// A stroke cap.
pub struct StrokeCap {
    cap: bool,
    taper: bool,
    easing: Easing,
}

#[destack::generated(StrokePath, , block)]
/// A stroke path.
pub struct StrokePath {
    points: Vec<StrokePoint>,
}

#[destack::generated(StrokePoint, , block)]
/// A computed point in a stroke.
pub struct StrokePoint {
    point: Vector2,
    original_point: Vector2,
    pressure: f32,
    direction: Vector2,
    distance: f32,
    running_length: f32,
    radius: f32,
}

#[destack::generated(StrokeType, , block)]
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
