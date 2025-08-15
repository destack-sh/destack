//! destack.imagination.animation.effect@2025.08.15.1

#![destack::partial(destack.imagination.animation.effect, file)]

#[destack::generated(Effect, struct, block)]
/// An effect value.
pub struct Effect {

}

#[destack::generated(RepeatType, enum, block)]
/// RepeatType
pub enum RepeatType {
    /// Restart from beginning
    LOOP = 1,
    /// Yoyo back and forth
    REVERSE = 2,
    /// Mirror keyframes
    MIRROR = 3
}

#[destack::generated(EffectType, enum, block)]
/// When the effect fires.
pub enum EffectType {
    /// Initial render in
    APPEAR = 10,
    /// Enters viewport
    ENTER = 11,
    /// Leaves viewport
    EXIT = 12,
    /// While hover
    HOVER = 20,
    /// While tap
    PRESS = 21,
    /// While drag / drag
    DRAG = 22,
    /// While focus
    FOCUS = 23,
    /// Continuous loop
    LOOP = 30
}

#[destack::generated(TextSplitType, enum, block)]
/// TextSplitType
pub enum TextSplitType {
    /// Split by character
    CHAR = 1,
    /// Split by word
    WORD = 2,
    /// Split by line
    LINE = 3
}

#[destack::generated(OffscreenBehavior, enum, block)]
/// What happens when the element is offscreen.
pub enum OffscreenBehavior {
    /// Play the animation
    PLAY = 1,
    /// Pause the animation
    PAUSE = 2
}