//! destack.imagination.animation.transition@2025.08.15.1

#![destack::generated(destack.imagination.animation.transition, file)]

use crate::SpringType;
use crate::TransitionType;

#[destack::generated(TransitionType, Debug, block)]
impl std::fmt::Debug for TransitionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransitionType::Tween => write!(f, "TWEEN"),
            TransitionType::Spring => write!(f, "SPRING"),
        }
    }
}

#[destack::generated(SpringType, Debug, block)]
impl std::fmt::Debug for SpringType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpringType::Time => write!(f, "TIME"),
            SpringType::Physical => write!(f, "PHYSICAL"),
        }
    }
}
