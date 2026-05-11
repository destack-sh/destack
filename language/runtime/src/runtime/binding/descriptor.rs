use serde::{Deserialize, Serialize};

use crate::runtime::action::{HostActionId, HostActionSet};
use crate::runtime::trace::EntropyKind;
use destack_core::fnv1a_128;

/// Default codec used for binding payloads.
pub const DEFAULT_BINDING_CODEC: CodecId = CodecId::from_name("postcard-v1");

/// Metadata describing a runtime binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BindingDescriptor {
    /// Stable external binding name.
    pub name: &'static str,
    /// Top-level binding namespace for runtime routing.
    pub namespace: &'static str,
    /// Stable binding id derived from the name.
    pub id: BindingId,
    /// Hash of the canonical signature string.
    pub signature: SignatureHash,
    /// Payload codec id for record/replay.
    pub codec: CodecId,
    /// Observable effect class.
    pub effect: BindingEffect,
    /// Replay routing for the binding.
    pub replay_kind: BindingReplayKind,
    /// Replay payload policy for recorded bindings.
    pub replay_payload: BindingReplayPayload,
    /// Required host actions for this binding.
    pub requires: &'static [&'static str],
    /// Platforms where this binding is supported.
    pub platforms: &'static [&'static str],
    /// Hosts where this binding is supported.
    pub hosts: &'static [&'static str],
    /// Provider that implements this binding.
    pub provider: BindingProvider,
    /// Execution context required by this binding.
    pub affinity: BindingAffinity,
}

#[allow(clippy::too_many_arguments)]
impl BindingDescriptor {
    /// Create a binding descriptor.
    pub const fn new(
        name: &'static str,
        signature: &'static str,
        effect: BindingEffect,
        replay_kind: BindingReplayKind,
        replay_payload: BindingReplayPayload,
        requires: &'static [&'static str],
        provider: BindingProvider,
        affinity: BindingAffinity,
    ) -> Self {
        Self {
            name,
            namespace: "",
            id: BindingId::from_name(name),
            signature: SignatureHash::from_signature(signature),
            codec: DEFAULT_BINDING_CODEC,
            effect,
            replay_kind,
            replay_payload,
            requires,
            platforms: &[],
            hosts: &[],
            provider,
            affinity,
        }
    }

    /// Return this descriptor with an explicit codec id.
    pub const fn with_codec(mut self, codec: CodecId) -> Self {
        self.codec = codec;
        self
    }

    /// Attach one top-level namespace to this binding descriptor.
    pub const fn with_namespace(mut self, namespace: &'static str) -> Self {
        self.namespace = namespace;
        self
    }

    /// Return the replay payload policy for this binding.
    pub const fn replay_payload(self) -> BindingReplayPayload {
        self.replay_payload
    }

    /// Return the required host actions for this binding.
    pub const fn requires(self) -> &'static [&'static str] {
        self.requires
    }

    /// Iterate required host action identifiers for this binding.
    pub fn required_action_ids(self) -> impl Iterator<Item = HostActionId> + 'static {
        self.requires.iter().copied().map(HostActionId::from_name)
    }

    /// Return the first required host action missing from one action set.
    pub fn missing_requirement(self, actions: &HostActionSet) -> Option<&'static str> {
        self.requires
            .iter()
            .copied()
            .find(|action| !actions.contains_name(action))
    }

    /// Return the platforms that support this binding.
    pub const fn platforms(self) -> &'static [&'static str] {
        self.platforms
    }

    /// Return this descriptor with explicit supported platforms.
    pub const fn with_platforms(mut self, platforms: &'static [&'static str]) -> Self {
        self.platforms = platforms;
        self
    }

    /// Return the hosts that support this binding.
    pub const fn hosts(self) -> &'static [&'static str] {
        self.hosts
    }

    /// Return this descriptor with explicit supported hosts.
    pub const fn with_hosts(mut self, hosts: &'static [&'static str]) -> Self {
        self.hosts = hosts;
        self
    }

    /// Return whether this binding supports one platform.
    pub fn supports_platform(self, platform: &str) -> bool {
        if self.platforms.is_empty() && self.hosts.is_empty() {
            return true;
        }

        self.platforms.contains(&platform)
    }

    /// Return whether this binding supports one host.
    pub fn supports_host(self, host: &str) -> bool {
        if self.platforms.is_empty() && self.hosts.is_empty() {
            return true;
        }

        self.hosts.contains(&host)
    }

    /// Return whether this binding supports the current target.
    pub fn supports_current_target(self) -> bool {
        self.supports_platform(current_platform_name()) || self.supports_host(current_host_name())
    }

    /// Return the provider for this binding.
    pub const fn provider(self) -> BindingProvider {
        self.provider
    }

    /// Return the execution context required by this binding.
    pub const fn affinity(self) -> BindingAffinity {
        self.affinity
    }
}

/// Execution engine that is invoking one binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BindingEngine {
    /// Binding call is executed by the VM.
    Vm,
    /// Binding call is executed by native code.
    Native,
}

/// Provider that implements one binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BindingProvider {
    /// Binding is implemented by the host backend.
    Host,
    /// Binding is implemented by runtime state.
    Runtime,
}

/// Execution context required by one binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BindingAffinity {
    /// Binding has no execution context requirement.
    None,
    /// Binding requires the same worker context.
    Worker,
    /// Binding requires the process main context.
    Main,
}

/// Observable effect kind for one binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BindingEffect {
    /// Pure binding.
    Pure,
    /// Deterministic binding.
    Deterministic,
    /// Recordable external binding.
    ExternalRecordable,
    /// Non-recordable external binding.
    ExternalNonRecordable,
}

/// Runtime world selected for one binding call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum RuntimeWorld {
    /// Use host-backed platform bindings.
    #[default]
    Host,
    /// Use simulation-backed platform bindings.
    Simulation,
}

/// Runtime access decision for one binding call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum RuntimeAccess {
    /// Allow the binding call.
    #[default]
    Allow,
    /// Deny the binding call.
    Deny,
}

/// Replay payload policy for recorded bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BindingReplayPayload {
    /// Record only the result value.
    Results,
    /// Record arguments and results for verification.
    ArgumentsAndResults,
}

/// Replay routing for external bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BindingReplayKind {
    /// Record a generic binding call payload.
    BindingCall,
    /// Record one entropy event with one specific kind.
    Entropy(EntropyKind),
}

/// Stable identifier for a binding name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BindingId(pub u128);

impl BindingId {
    /// Build a binding id from a binding name.
    pub const fn from_name(name: &'static str) -> Self {
        Self(fnv1a_128(name.as_bytes()))
    }
}

/// Stable identifier for a binding payload codec.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CodecId(pub u128);

impl CodecId {
    /// Build a codec id from a codec name.
    pub const fn from_name(name: &'static str) -> Self {
        Self(fnv1a_128(name.as_bytes()))
    }
}

/// Stable hash for a binding signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SignatureHash(pub u128);

impl SignatureHash {
    /// Build a signature hash from a canonical signature string.
    pub const fn from_signature(signature: &'static str) -> Self {
        Self(fnv1a_128(signature.as_bytes()))
    }
}

/// Return the current compile target platform name.
pub(crate) fn current_platform_name() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "windows"
    }

    #[cfg(target_os = "android")]
    {
        "android"
    }

    #[cfg(target_os = "linux")]
    {
        "linux"
    }

    #[cfg(target_os = "macos")]
    {
        "macos"
    }

    #[cfg(target_os = "freebsd")]
    {
        "freebsd"
    }

    #[cfg(target_os = "openbsd")]
    {
        "openbsd"
    }

    #[cfg(target_os = "netbsd")]
    {
        "netbsd"
    }

    #[cfg(target_os = "dragonfly")]
    {
        "dragonfly"
    }

    #[cfg(target_os = "solaris")]
    {
        "solaris"
    }

    #[cfg(target_os = "illumos")]
    {
        "illumos"
    }

    #[cfg(target_os = "haiku")]
    {
        "haiku"
    }

    #[cfg(target_os = "fuchsia")]
    {
        "fuchsia"
    }

    #[cfg(target_os = "redox")]
    {
        "redox"
    }

    #[cfg(target_os = "hermit")]
    {
        "hermit"
    }

    #[cfg(target_os = "ios")]
    {
        "ios"
    }

    #[cfg(target_os = "none")]
    {
        "none"
    }

    #[cfg(not(any(
        target_os = "windows",
        target_os = "android",
        target_os = "linux",
        target_os = "macos",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly",
        target_os = "solaris",
        target_os = "illumos",
        target_os = "haiku",
        target_os = "fuchsia",
        target_os = "redox",
        target_os = "hermit",
        target_os = "ios",
        target_os = "none"
    )))]
    {
        "unknown"
    }
}

/// Return the current compile target host name.
pub(crate) fn current_host_name() -> &'static str {
    #[cfg(target_os = "wasi")]
    {
        "wasi"
    }

    #[cfg(target_os = "emscripten")]
    {
        "emscripten"
    }

    #[cfg(target_os = "none")]
    {
        "freestanding"
    }

    #[cfg(all(
        target_arch = "wasm32",
        not(any(target_os = "wasi", target_os = "emscripten", target_os = "none"))
    ))]
    {
        "browser"
    }

    #[cfg(not(any(
        target_arch = "wasm32",
        target_os = "wasi",
        target_os = "emscripten",
        target_os = "none"
    )))]
    {
        "native"
    }
}
