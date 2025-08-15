//! destack.presentation.view.relative@2025.08.15.1

#![destack::partial(destack.presentation.view.relative, file)]

#[destack::generated(Length, struct, block)]
/// An absolute or relative length value.
pub struct Length {}

#[destack::generated(Offset2, struct, block)]
/// A 2-dimensional position value (relative or absolute).
pub struct Offset2 {}

#[destack::generated(Grid2, struct, block)]
/// A 2-dimensional grid configuration value.
pub struct Grid2 {}

#[destack::generated(GridSpan2, struct, block)]
/// A 2-dimensional grid span value.
pub struct GridSpan2 {}

#[destack::generated(Inset2, struct, block)]
/// A 2-dimensional insets value (base + side overrides).
pub struct Inset2 {}

#[destack::generated(Corner2, struct, block)]
/// A 2-dimensional corners value (base + corner overrides).
pub struct Corner2 {}

#[destack::generated(Axis2, struct, block)]
/// A 2-dimensional axis value (base + x/y overrides).
pub struct Axis2 {}

#[destack::generated(Axis3, struct, block)]
/// A 3-dimensional axis value (base + x/y/z overrides).
pub struct Axis3 {}

#[destack::generated(LengthType, enum, block)]
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

#[destack::generated(Layout, enum, block)]
/// The layout of elements.
pub enum Layout {
    /// Stack
    Stack = 1,
    /// Grid
    Grid = 2,
}

#[destack::generated(Distribute, enum, block)]
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

#[destack::generated(Align, enum, block)]
/// The alignment of elements.
pub enum Align {
    /// Start
    Start = 1,
    /// Center
    Center = 2,
    /// End
    End = 3,
}

#[destack::generated(Direction, enum, block)]
/// The direction of elements.
pub enum Direction {
    /// Horizontal
    Horizontal = 1,
    /// Vertical
    Vertical = 2,
}

#[destack::generated(Overflow, enum, block)]
/// The overflow behavior of elements.
pub enum Overflow {
    /// Hidden
    Hidden = 2,
    /// Visible
    Visible = 3,
    /// Scroll
    Scroll = 4,
}

#[destack::generated(Anchor, enum, block)]
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
