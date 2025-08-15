//! destack.imagination.animation.transition@2025.08.15.1

#![destack::partial(destack.imagination.animation.transition, file)]

#[destack::generated(Transition, , block)]
/// A transition value.
pub struct Transition {
    r#type: TransitionType,
    template: Option<i64 /* TODO */>,
    delay: Option<f32>,
    duration: Option<f32>,
    ease: Vec<f32>,
    stiffness: Option<f32>,
    damping: Option<f32>,
    mass: Option<f32>,
    bounce: Option<f32>,
    spring_type: Option<SpringType>,
}

#[destack::generated(TransitionType, , block)]
/// Built-in transition types.
pub enum TransitionType {
    Tween = 10,
    Spring = 11,
}

#[destack::generated(SpringType, , block)]
/// Built-in spring types.
pub enum SpringType {
    Time = 1,
    Physical = 2,
}
