//! destack.presentation.view.relative@2025.08.15.1

#![destack::partial(destack.presentation.view.relative, file)]

#[destack::generated(Length, struct, block)]
/// An absolute or relative length value.
pub struct Length {

}

#[destack::generated(Offset2, struct, block)]
/// A 2-dimensional position value (relative or absolute).
pub struct Offset2 {

}

#[destack::generated(Grid2, struct, block)]
/// A 2-dimensional grid configuration value.
pub struct Grid2 {

}

#[destack::generated(GridSpan2, struct, block)]
/// A 2-dimensional grid span value.
pub struct GridSpan2 {

}

#[destack::generated(Inset2, struct, block)]
/// A 2-dimensional insets value (base + side overrides).
pub struct Inset2 {

}

#[destack::generated(Corner2, struct, block)]
/// A 2-dimensional corners value (base + corner overrides).
pub struct Corner2 {

}

#[destack::generated(Axis2, struct, block)]
/// A 2-dimensional axis value (base + x/y overrides).
pub struct Axis2 {

}

#[destack::generated(Axis3, struct, block)]
/// A 3-dimensional axis value (base + x/y/z overrides).
pub struct Axis3 {

}

#[destack::generated(LengthType, enum, block)]
/// The unit of a length value.
pub enum LengthType {
    /// Pixel
    PIXEL = 1,
    /// Percent
    PERCENT = 2,
    /// Fit
    FIT = 3,
    /// Fill
    FILL = 4
}

#[destack::generated(Layout, enum, block)]
/// The layout of elements.
pub enum Layout {
    /// Stack
    STACK = 1,
    /// Grid
    GRID = 2
}

#[destack::generated(Distribute, enum, block)]
/// The distribution of elements.
pub enum Distribute {
    /// Start
    START = 1,
    /// Center
    CENTER = 2,
    /// End
    END = 3,
    /// Viewport Between
    SPACE_BETWEEN = 4,
    /// Viewport Around
    SPACE_AROUND = 5,
    /// Viewport Evenly
    SPACE_EVENLY = 6
}

#[destack::generated(Align, enum, block)]
/// The alignment of elements.
pub enum Align {
    /// Start
    START = 1,
    /// Center
    CENTER = 2,
    /// End
    END = 3
}

#[destack::generated(Direction, enum, block)]
/// The direction of elements.
pub enum Direction {
    /// Horizontal
    HORIZONTAL = 1,
    /// Vertical
    VERTICAL = 2
}

#[destack::generated(Overflow, enum, block)]
/// The overflow behavior of elements.
pub enum Overflow {
    /// Hidden
    HIDDEN = 2,
    /// Visible
    VISIBLE = 3,
    /// Scroll
    SCROLL = 4
}

#[destack::generated(Anchor, enum, block)]
/// The position of elements.
pub enum Anchor {
    /// Relative to parent
    RELATIVE = 1,
    /// Absolute in parent
    ABSOLUTE = 2,
    /// Fixed to root
    FIXED = 3,
    /// Sticky to parent
    STICKY = 4
}