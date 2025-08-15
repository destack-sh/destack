//! destack.imagination.animation.effect@2025.08.15.1

#![destack::partial(destack.imagination.animation.effect, file)]

use crate::Axis3;
use crate::Duration;
use crate::EffectType;
use crate::OffscreenBehavior;
use crate::RepeatType;
use crate::TextSplitType;
use crate::Transition;
use crate::Vector2;

#[destack::generated(Effect, , block)]
/// An effect value.
pub struct Effect {
    r#type: EffectType,
    template: Option<i64 /* TODO */>,
    opacity: Option<f32>,
    offset: Option<Vector2>,
    scale: Option<f32>,
    rotate: Option<Axis3>,
    skew: Option<Vector2>,
    perspective: Option<f32>,
    delay: Option<Duration>,
    duration: Option<f32>,
    threshold: Option<f32>,
    once: Option<bool>,
    repeat: Option<RepeatType>,
    split: Option<TextSplitType>,
    offscreen: Option<OffscreenBehavior>,
    transition: Option<Transition>,
}

#[destack::generated(RepeatType, , block)]
/// RepeatType
pub enum RepeatType {
    /// Restart from beginning
    Loop = 1,
    /// Yoyo back and forth
    Reverse = 2,
    /// Mirror keyframes
    Mirror = 3,
}

#[destack::generated(EffectType, , block)]
/// When the effect fires.
pub enum EffectType {
    /// Initial render in
    Appear = 10,
    /// Enters viewport
    Enter = 11,
    /// Leaves viewport
    Exit = 12,
    /// While hover
    Hover = 20,
    /// While tap
    Press = 21,
    /// While drag / drag
    Drag = 22,
    /// While focus
    Focus = 23,
    /// Continuous loop
    Loop = 30,
}

#[destack::generated(TextSplitType, , block)]
/// TextSplitType
pub enum TextSplitType {
    /// Split by character
    Char = 1,
    /// Split by word
    Word = 2,
    /// Split by line
    Line = 3,
}

#[destack::generated(OffscreenBehavior, , block)]
/// What happens when the element is offscreen.
pub enum OffscreenBehavior {
    /// Play the animation
    Play = 1,
    /// Pause the animation
    Pause = 2,
}
