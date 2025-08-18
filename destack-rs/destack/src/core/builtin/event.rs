//! destack.core.builtin.event

#![destack::partial(destack.core.builtin.event, file)]

#[destack::generated(RuntimePlatform, -, block)]
/// The platform of the Runtime.
pub enum RuntimePlatform {
    /// Core platform
    Core = 100,
    /// Destack system (internal)
    System = 200,
    /// Server environment
    Server = 300,
    /// Web browser
    Web = 400,
    /// Mobile device
    Mobile = 500,
    /// Desktop computer
    Desktop = 600,
}

#[destack::generated(RuntimeLanguage, -, block)]
/// The language of the Runtime.
pub enum RuntimeLanguage {
    Python = 1,
    Typescript = 2,
    Rust = 3,
}

#[destack::generated(RuntimeType, -, block)]
/// The specific Runtime (RuntimeLanguage x RuntimePlatform).
pub enum RuntimeType {
    /// Destack Python library
    CorePython = 101,
    /// Destack TypeScript library
    CoreTypescript = 102,
    /// Destack Rust library
    CoreRust = 103,
    /// Destack TypeScript system runtime (internal)
    SystemTypescript = 202,
    /// Destack Rust system runtime (internal)
    SystemRust = 203,
    /// Destack Python server runtime
    ServerPython = 301,
    /// Destack TypeScript server runtime
    ServerTypescript = 302,
    /// Destack TypeScript web runtime
    WebTypescript = 402,
}

#[destack::generated(EventStatus, -, block)]
/// The consensus status of an Event.
pub enum EventStatus {
    /// Pending application on client
    Pending = 1,
    /// Optimistically staged on client
    Staged = 2,
    /// Successfully applied in system
    Approved = 10,
    /// Skipped and ignored in system
    Skipped = 11,
    /// Could not apply in system
    Failed = 12,
    /// Denied by the system
    Rejected = 13,
}
