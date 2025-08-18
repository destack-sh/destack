//! destack.basics.script.trigger

#![destack::partial(destack.basics.script.trigger, file)]

#[destack::generated(TriggerType, -, block)]
/// TriggerType
pub enum TriggerType {
    /// A Trigger that runs on an Event
    Event = 1,
}
