//! destack.imagination.style.stroke@2025.08.15.1

#![destack::partial(destack.imagination.style.stroke, file)]

#[destack::generated(Stroke, struct, block)]
/// A stroke value.
pub struct Stroke {

}

#[destack::generated(StrokeCap, struct, block)]
/// A stroke cap.
pub struct StrokeCap {

}

#[destack::generated(StrokePath, struct, block)]
/// A stroke path.
pub struct StrokePath {

}

#[destack::generated(StrokePoint, struct, block)]
/// A computed point in a stroke.
pub struct StrokePoint {

}

#[destack::generated(StrokeType, enum, block)]
/// StrokeType
pub enum StrokeType {
    /// A solid stroke
    SOLID = 1,
    /// A dashed stroke
    DASHED = 2,
    /// A dotted stroke
    DOTTED = 3,
    /// A freehand stroke
    FREEHAND = 4
}