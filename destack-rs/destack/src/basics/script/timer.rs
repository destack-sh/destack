//! destack.basics.script.timer

#![destack::partial(destack.basics.script.timer, file)]

#[destack::generated(TimerType, -, block)]
/// TimerType
pub enum TimerType {
    /// A one-time timer
    Once = 1,
    /// A recurring timer
    Recurring = 2,
}
