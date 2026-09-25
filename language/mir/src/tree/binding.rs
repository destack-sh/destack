use serde::{Deserialize, Serialize};
use tspp_core::StringId;
use tspp_serde::Reflect;

/// One runtime binding declaration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct Binding {
    /// Stable binding name.
    pub name: StringId,
    /// Binding implementation owner.
    pub provider: BindingProvider,
    /// Observable effect class.
    pub effect: BindingEffect,
    /// Replay behavior.
    pub replay: BindingReplay,
    /// Required execution context.
    pub affinity: BindingAffinity,
    /// Whether calls through this binding park the calling fiber.
    pub is_park: bool,
    /// Required runtime actions.
    pub requires: Vec<StringId>,
    /// Supported platforms.
    pub platforms: Vec<StringId>,
    /// Supported platform families.
    pub families: Vec<StringId>,
    /// Supported hosts.
    pub hosts: Vec<StringId>,
}

impl Binding {
    /// Create one binding declaration with explicit required properties.
    pub fn new(name: StringId, provider: BindingProvider, effect: BindingEffect) -> Self {
        Self {
            name,
            provider,
            effect,
            replay: BindingReplay::Recordable,
            affinity: BindingAffinity::None,
            is_park: false,
            requires: Vec::new(),
            platforms: Vec::new(),
            families: Vec::new(),
            hosts: Vec::new(),
        }
    }
}

/// Observable effect class for one binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum BindingEffect {
    /// Pure function of its explicit arguments.
    Pure,
    /// Deterministic operation that may observe runtime state.
    Deterministic,
    /// Operation that observes or mutates external state.
    External,
}

impl BindingEffect {
    /// Parse one MIR binding effect name.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "pure" => Some(Self::Pure),
            "deterministic" => Some(Self::Deterministic),
            "external" => Some(Self::External),
            _ => None,
        }
    }

    /// Return the MIR binding effect name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Pure => "pure",
            Self::Deterministic => "deterministic",
            Self::External => "external",
        }
    }
}

/// Owner that implements one binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum BindingProvider {
    /// Host platform implementation.
    Host,
    /// TS++ runtime implementation.
    Runtime,
}

impl BindingProvider {
    /// Parse one MIR binding provider name.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "host" => Some(Self::Host),
            "runtime" => Some(Self::Runtime),
            _ => None,
        }
    }

    /// Return the MIR binding provider name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Host => "host",
            Self::Runtime => "runtime",
        }
    }
}

/// Replay behavior for one binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum BindingReplay {
    /// Calls may be recorded and replayed.
    Recordable,
    /// Calls cannot be replayed.
    Forbidden,
}

impl BindingReplay {
    /// Parse one MIR binding replay name.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "recordable" => Some(Self::Recordable),
            "forbidden" => Some(Self::Forbidden),
            _ => None,
        }
    }

    /// Return the MIR binding replay name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Recordable => "recordable",
            Self::Forbidden => "forbidden",
        }
    }
}

/// Execution context required by one binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum BindingAffinity {
    /// No execution context requirement.
    None,
    /// Current worker context.
    Worker,
    /// Process main context.
    Main,
}

impl BindingAffinity {
    /// Parse one MIR binding affinity name.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "none" => Some(Self::None),
            "worker" => Some(Self::Worker),
            "main" => Some(Self::Main),
            _ => None,
        }
    }

    /// Return the MIR binding affinity name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Worker => "worker",
            Self::Main => "main",
        }
    }
}
