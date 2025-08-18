//! destack.basics.script.trigger

#![destack::generated(destack.basics.script.trigger, file)]

use crate::TriggerType;

#[destack::generated(TriggerType, Debug, block)]
impl std::fmt::Debug for TriggerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TriggerType::Event => write!(f, "EVENT"),
        }
    }
}
