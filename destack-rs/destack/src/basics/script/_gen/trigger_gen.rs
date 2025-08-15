//! destack.basics.script.trigger@2025.08.15.1

#![destack::generated(destack.basics.script.trigger, file)]

#[destack::generated(TriggerType, Debug, block)]
impl std::fmt::Debug for TriggerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TriggerType::Event => write!(f, "EVENT"),
        }
    }
}
