//! destack.basics.script.timer@2025.08.15.1

#![destack::generated(destack.basics.script.timer, file)]

use crate::TimerType;

#[destack::generated(TimerType, Debug, block)]
impl std::fmt::Debug for TimerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimerType::Once => write!(f, "ONCE"),
            TimerType::Recurring => write!(f, "RECURRING"),
        }
    }
}
