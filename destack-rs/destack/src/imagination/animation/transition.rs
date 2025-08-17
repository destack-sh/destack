//! destack.imagination.animation.transition@2025.08.15.1

#![destack::partial(destack.imagination.animation.transition, file)]

#[destack::generated(Transition, -, block)]
/// A transition value.
pub struct Transition {
    pub r#type: TransitionType,
    pub template: Option<i64>,
    pub delay: Option<f32>,
    pub duration: Option<f32>,
    pub ease: Vec<f32>,
    pub stiffness: Option<f32>,
    pub damping: Option<f32>,
    pub mass: Option<f32>,
    pub bounce: Option<f32>,
    pub spring_type: Option<SpringType>,
}

#[destack::generated(TransitionType, -, block)]
/// Built-in transition types.
pub enum TransitionType {
    Tween = 10,
    Spring = 11,
}

#[destack::generated(SpringType, -, block)]
/// Built-in spring types.
pub enum SpringType {
    Time = 1,
    Physical = 2,
}
