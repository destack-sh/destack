//! destack.imagination.animation.effect@2025.08.15.1

#![destack::partial(destack.imagination.animation.effect, file)]

use crate::{Axis3, Duration, Transition, Vector2};

#[destack::generated(Effect, -, block)]
/// An effect value.
pub struct Effect {
    pub r#type: EffectType,
    pub template: Option<i64>,
    pub opacity: Option<f32>,
    pub offset: Option<Vector2>,
    pub scale: Option<f32>,
    pub rotate: Option<Axis3>,
    pub skew: Option<Vector2>,
    pub perspective: Option<f32>,
    pub delay: Option<Duration>,
    pub duration: Option<f32>,
    pub threshold: Option<f32>,
    pub once: Option<bool>,
    pub repeat: Option<RepeatType>,
    pub split: Option<TextSplitType>,
    pub offscreen: Option<OffscreenBehavior>,
    pub transition: Option<Transition>,
}

#[destack::generated(RepeatType, -, block)]
/// RepeatType
pub enum RepeatType {
    /// Restart from beginning
    Loop = 1,
    /// Yoyo back and forth
    Reverse = 2,
    /// Mirror keyframes
    Mirror = 3,
}

#[destack::generated(TextSplitType, -, block)]
/// TextSplitType
pub enum TextSplitType {
    /// Split by character
    Char = 1,
    /// Split by word
    Word = 2,
    /// Split by line
    Line = 3,
}

#[destack::generated(OffscreenBehavior, -, block)]
/// What happens when the element is offscreen.
pub enum OffscreenBehavior {
    /// Play the animation
    Play = 1,
    /// Pause the animation
    Pause = 2,
}

#[destack::generated(EffectType, -, block)]
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
