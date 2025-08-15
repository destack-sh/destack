//! destack.presentation.view@2025.08.15.1

#![destack::partial(destack.presentation.view, file)]
#![allow(unused_imports)]

pub(crate) use crate::presentation::view::_gen::*;
pub(crate) use crate::presentation::view::content::*;
pub(crate) use crate::presentation::view::frame::*;
pub(crate) use crate::presentation::view::input::*;
pub(crate) use crate::presentation::view::label::*;
pub(crate) use crate::presentation::view::layout::*;
pub(crate) use crate::presentation::view::number::*;
pub use crate::presentation::view::relative::*;
pub(crate) use crate::presentation::view::slider::*;
pub(crate) use crate::presentation::view::split::*;
pub(crate) use crate::presentation::view::text::*;
pub(crate) use crate::presentation::view::view::*;

mod _gen;
mod content;
mod frame;
mod input;
mod label;
mod layout;
mod number;
mod relative;
mod slider;
mod split;
mod text;
mod view;
