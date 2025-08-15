//! destack.imagination.animation.transition@2025.08.15.1

#![destack::partial(destack.imagination.animation.transition, file)]

#[destack::generated(Transition, struct, block)]
/// A transition value.
pub struct Transition {
    r#type: TransitionType,
    template: i64, /* TODO */
    delay: f32,
    duration: f32,
    ease: Vec<f32>,
    stiffness: f32,
    damping: f32,
    mass: f32,
    bounce: f32,
    spring_type: SpringType,
}

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
