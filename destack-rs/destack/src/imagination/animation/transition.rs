//! destack.imagination.animation.transition@2025.08.15.1

#![destack::partial(destack.imagination.animation.transition, file)]

#[destack::generated(Transition, struct, block)]
/// A transition value.
pub struct Transition {}

#[destack::generated(TransitionType, enum, block)]
/// Built-in transition types.
pub enum TransitionType {
    Tween = 10,
    Spring = 11,
}

#[destack::generated(SpringType, enum, block)]
/// Built-in spring types.
pub enum SpringType {
    Time = 1,
    Physical = 2,
}
