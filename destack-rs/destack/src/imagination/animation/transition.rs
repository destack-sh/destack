//! destack.imagination.animation.transition@2025.08.15.1

#![destack::partial(destack.imagination.animation.transition, file)]

#[destack::generated(Transition, struct, block)]
/// A transition value.
pub struct Transition {

}

#[destack::generated(TransitionType, enum, block)]
/// Built-in transition types.
pub enum TransitionType {
    TWEEN = 10,
    SPRING = 11
}

#[destack::generated(SpringType, enum, block)]
/// Built-in spring types.
pub enum SpringType {
    TIME = 1,
    PHYSICAL = 2
}