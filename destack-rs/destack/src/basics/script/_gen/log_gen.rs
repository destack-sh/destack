//! destack.basics.script.log@2025.08.15.1

#![destack::generated(destack.basics.script.log, file)]

use crate::LogLevel;

#[destack::generated(LogLevel, Debug, block)]
impl std::fmt::Debug for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Trace => write!(f, "TRACE"),
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warning => write!(f, "WARNING"),
            LogLevel::Error => write!(f, "ERROR"),
            LogLevel::Panic => write!(f, "PANIC"),
        }
    }
}
