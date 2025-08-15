//! destack.basics.script.log@2025.08.15.1

#![destack::partial(destack.basics.script.log, file)]

#[destack::generated(LogLevel, enum, block)]
/// LogLevel
pub enum LogLevel {
    /// A Trace
    Trace = 1,
    /// A Debug
    Debug = 2,
    /// An Info
    Info = 3,
    /// A Warning
    Warning = 4,
    /// An Error
    Error = 5,
    /// A Panic
    Panic = 6
}