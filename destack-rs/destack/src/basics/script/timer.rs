//! destack.basics.script.timer@2025.08.15.1

#![destack::partial(destack.basics.script.timer, file)]

#[destack::generated(TimerType, , block)]
/// TimerType
pub enum TimerType {
    /// A one-time timer
    Once = 1,
    /// A recurring timer
    Recurring = 2,
}
