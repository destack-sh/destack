//! destack.presentation.view@2025.08.15.1

#![destack::partial(destack.presentation.view, file)]
#![allow(unused_imports)]
#![allow(unreachable_pub)]

pub use crate::presentation::view::relative::{
    Align, Anchor, Axis2, Axis3, Corner2, Direction, Distribute, Grid2, GridSpan2, Inset2, Layout,
    Length, LengthType, Offset2, Overflow,
};

pub mod _gen;
pub mod content;
pub mod frame;
pub mod input;
pub mod label;
pub mod layout;
pub mod number;
pub mod relative;
pub mod slider;
pub mod split;
pub mod text;
pub mod view;
