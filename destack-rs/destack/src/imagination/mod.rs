//! destack.imagination@2025.08.15.1

#![destack::partial(destack.imagination, file)]
#![allow(unused_imports)]
#![allow(unreachable_pub)]

pub use crate::imagination::animation::{
    Easing, Effect, EffectType, OffscreenBehavior, RepeatType, SpringType, TextSplitType,
    Transition, TransitionType,
};
pub use crate::imagination::style::{
    Border, BorderType, Color, ColorHue, ColorIntent, ColorShade, ColorType, Fill, FillPosition,
    FillSize, FillType, Font, FontSize, FontType, FontWeight, Gradient, GradientStop, GradientType,
    Shadow, ShadowPosition, ShadowType, Stroke, StrokeCap, StrokePath, StrokePoint, StrokeType,
    TextAlign, TextDecoration, TextTransform,
};

pub mod animation;
pub mod audio;
pub mod document;
pub mod image;
pub mod model;
pub mod style;
pub mod video;
