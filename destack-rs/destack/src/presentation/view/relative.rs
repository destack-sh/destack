//! destack.presentation.view.relative

#![destack::partial(destack.presentation.view.relative, file)]

#[destack::generated(Length, -, block)]
/// An absolute or relative length value.
pub struct Length {
    pub unit: LengthType,
    pub value: i16,
}

#[destack::generated(Offset2, -, block)]
/// A 2-dimensional position value (relative or absolute).
pub struct Offset2 {
    pub r#type: Anchor,
    pub top: Length,
    pub left: Length,
    pub width: Length,
    pub height: Length,
}

#[destack::generated(Grid2, -, block)]
/// A 2-dimensional grid configuration value.
pub struct Grid2 {
    pub columns: i16,
    pub rows: i16,
    pub column_width: Option<Length>,
    pub column_min_width: Option<Length>,
    pub row_height: Option<Length>,
}

#[destack::generated(GridSpan2, -, block)]
/// A 2-dimensional grid span value.
pub struct GridSpan2 {
    pub columns: i16,
    pub rows: i16,
}

#[destack::generated(Inset2, -, block)]
/// A 2-dimensional insets value (base + side overrides).
pub struct Inset2 {
    pub top: i16,
    pub left: i16,
    pub right: i16,
    pub bottom: i16,
}

#[destack::generated(Corner2, -, block)]
/// A 2-dimensional corners value (base + corner overrides).
pub struct Corner2 {
    pub top_left: i16,
    pub top_right: i16,
    pub bottom_left: i16,
    pub bottom_right: i16,
}

#[destack::generated(Axis2, -, block)]
/// A 2-dimensional axis value (base + x/y overrides).
pub struct Axis2 {
    pub x: i16,
    pub y: i16,
}

#[destack::generated(Axis3, -, block)]
/// A 3-dimensional axis value (base + x/y/z overrides).
pub struct Axis3 {
    pub x: i16,
    pub y: i16,
    pub z: i16,
}

#[destack::generated(LengthType, -, block)]
/// The unit of a length value.
pub enum LengthType {
    /// Pixel
    Pixel = 1,
    /// Percent
    Percent = 2,
    /// Fit
    Fit = 3,
    /// Fill
    Fill = 4,
}

#[destack::generated(Layout, -, block)]
/// The layout of elements.
pub enum Layout {
    /// Stack
    Stack = 1,
    /// Grid
    Grid = 2,
}

#[destack::generated(Distribute, -, block)]
/// The distribution of elements.
pub enum Distribute {
    /// Start
    Start = 1,
    /// Center
    Center = 2,
    /// End
    End = 3,
    /// Viewport Between
    SpaceBetween = 4,
    /// Viewport Around
    SpaceAround = 5,
    /// Viewport Evenly
    SpaceEvenly = 6,
}

#[destack::generated(Align, -, block)]
/// The alignment of elements.
pub enum Align {
    /// Start
    Start = 1,
    /// Center
    Center = 2,
    /// End
    End = 3,
}

#[destack::generated(Direction, -, block)]
/// The direction of elements.
pub enum Direction {
    /// Horizontal
    Horizontal = 1,
    /// Vertical
    Vertical = 2,
}

#[destack::generated(Overflow, -, block)]
/// The overflow behavior of elements.
pub enum Overflow {
    /// Hidden
    Hidden = 2,
    /// Visible
    Visible = 3,
    /// Scroll
    Scroll = 4,
}

#[destack::generated(Anchor, -, block)]
/// The position of elements.
pub enum Anchor {
    /// Relative to parent
    Relative = 1,
    /// Absolute in parent
    Absolute = 2,
    /// Fixed to root
    Fixed = 3,
    /// Sticky to parent
    Sticky = 4,
}
