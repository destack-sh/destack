//! destack.imagination.style

#![destack::partial(destack.imagination.style, file)]
#![allow(unused_imports)]
#![allow(unreachable_pub)]

pub use crate::imagination::style::border::{Border, BorderType};
pub use crate::imagination::style::color::{Color, ColorHue, ColorIntent, ColorShade, ColorType};
pub use crate::imagination::style::fill::{Fill, FillPosition, FillSize, FillType};
pub use crate::imagination::style::font::{
    Font, FontSize, FontType, FontWeight, TextAlign, TextDecoration, TextTransform,
};
pub use crate::imagination::style::gradient::{Gradient, GradientStop, GradientType};
pub use crate::imagination::style::shadow::{Shadow, ShadowPosition, ShadowType};
pub use crate::imagination::style::stroke::{
    Stroke, StrokeCap, StrokePath, StrokePoint, StrokeType,
};

pub mod _gen;
pub mod border;
pub mod color;
pub mod fill;
pub mod font;
pub mod gradient;
pub mod palette;
pub mod shadow;
pub mod stroke;
pub mod style;
pub mod theme;
