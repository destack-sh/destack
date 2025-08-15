//! destack.basics.script.log@2025.08.15.1

#![destack::partial(destack.basics.script.log, file)]

#[destack::generated(LogLevel, enum, block)]
/// LogLevel
pub enum LogLevel {
    /// A Trace
    TRACE = 1,
    /// A Debug
    DEBUG = 2,
    /// An Info
    INFO = 3,
    /// A Warning
    WARNING = 4,
    /// An Error
    ERROR = 5,
    /// A Panic
    PANIC = 6
}