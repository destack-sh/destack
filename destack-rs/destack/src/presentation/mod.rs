//! destack.presentation@2025.08.15.1

#![destack::partial(destack.presentation, file)]
#![allow(unused_imports)]
#![allow(unreachable_pub)]

pub use crate::presentation::view::{
    Align, Anchor, Axis2, Axis3, Corner2, Direction, Distribute, Grid2, GridSpan2, Inset2, Layout,
    Length, LengthType, Offset2, Overflow,
};

pub mod scene;
pub mod view;
