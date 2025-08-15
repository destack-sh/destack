//! destack.imagination.style.fill@2025.08.15.1

#![destack::partial(destack.imagination.style.fill, file)]

#[destack::generated(Fill, struct, block)]
/// A fill value.
pub struct Fill {

}

#[destack::generated(FillType, enum, block)]
/// FillType
pub enum FillType {
    /// A solid fill
    SOLID = 10,
    /// A gradient fill
    GRADIENT = 11,
    /// An image fill
    IMAGE = 12
}

#[destack::generated(FillPosition, enum, block)]
/// FillPosition
pub enum FillPosition {
    /// A top left fill position
    TOP_LEFT = 1,
    /// A top center fill position
    TOP_CENTER = 2,
    /// A top right fill position
    TOP_RIGHT = 3,
    /// A left fill position
    LEFT = 10,
    /// A center fill position
    CENTER = 11,
    /// A right fill position
    RIGHT = 12,
    /// A bottom left fill position
    BOTTOM_LEFT = 20,
    /// A bottom center fill position
    BOTTOM_CENTER = 21,
    /// A bottom right fill position
    BOTTOM_RIGHT = 22
}

#[destack::generated(FillSize, enum, block)]
/// FillSize
pub enum FillSize {
    /// A fill size
    FILL = 1,
    /// A stretch size
    STRETCH = 2,
    /// A fit size
    FIT = 3,
    /// A tile size
    TILE = 4
}