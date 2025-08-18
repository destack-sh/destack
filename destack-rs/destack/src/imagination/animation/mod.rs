//! destack.imagination.animation

#![destack::partial(destack.imagination.animation, file)]
#![allow(unused_imports)]
#![allow(unreachable_pub)]

pub use crate::imagination::animation::easing::Easing;
pub use crate::imagination::animation::effect::{
    Effect, EffectType, OffscreenBehavior, RepeatType, TextSplitType,
};
pub use crate::imagination::animation::transition::{SpringType, Transition, TransitionType};

pub mod _gen;
pub mod easing;
pub mod effect;
pub mod transition;
