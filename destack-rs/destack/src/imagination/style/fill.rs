//! destack.imagination.style.fill@2025.08.15.1

#![destack::partial(destack.imagination.style.fill, file)]

#[destack::generated(Fill, struct, block)]
/// A fill value.
pub struct Fill {}

#[destack::generated(FillType, enum, block)]
/// FillType
pub enum FillType {
    /// A solid fill
    Solid = 10,
    /// A gradient fill
    Gradient = 11,
    /// An image fill
    Image = 12,
}

#[destack::generated(FillPosition, enum, block)]
/// FillPosition
pub enum FillPosition {
    /// A top left fill position
    TopLeft = 1,
    /// A top center fill position
    TopCenter = 2,
    /// A top right fill position
    TopRight = 3,
    /// A left fill position
    Left = 10,
    /// A center fill position
    Center = 11,
    /// A right fill position
    Right = 12,
    /// A bottom left fill position
    BottomLeft = 20,
    /// A bottom center fill position
    BottomCenter = 21,
    /// A bottom right fill position
    BottomRight = 22,
}

#[destack::generated(FillSize, enum, block)]
/// FillSize
pub enum FillSize {
    /// A fill size
    Fill = 1,
    /// A stretch size
    Stretch = 2,
    /// A fit size
    Fit = 3,
    /// A tile size
    Tile = 4,
}
