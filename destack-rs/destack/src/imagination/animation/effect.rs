//! destack.imagination.animation.effect@2025.08.15.1

#![destack::partial(destack.imagination.animation.effect, file)]

use crate::Axis3;
use crate::Duration;
use crate::Transition;
use crate::Vector2;

#[destack::generated(Effect, struct, block)]
/// An effect value.
pub struct Effect {
    r#type: EffectType,
    template: i64, /* TODO */
    opacity: f32,
    offset: Vector2,
    scale: f32,
    rotate: Axis3,
    skew: Vector2,
    perspective: f32,
    delay: Duration,
    duration: f32,
    threshold: f32,
    once: bool,
    repeat: RepeatType,
    split: TextSplitType,
    offscreen: OffscreenBehavior,
    transition: Transition,
}

#[destack::generated(RepeatType, enum, block)]
/// RepeatType
pub enum RepeatType {
    /// Restart from beginning
    Loop = 1,
    /// Yoyo back and forth
    Reverse = 2,
    /// Mirror keyframes
    Mirror = 3,
}

#[destack::generated(EffectType, enum, block)]
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

#[destack::generated(TextSplitType, enum, block)]
/// TextSplitType
pub enum TextSplitType {
    /// Split by character
    Char = 1,
    /// Split by word
    Word = 2,
    /// Split by line
    Line = 3,
}

#[destack::generated(OffscreenBehavior, enum, block)]
/// What happens when the element is offscreen.
pub enum OffscreenBehavior {
    /// Play the animation
    Play = 1,
    /// Pause the animation
    Pause = 2,
}
