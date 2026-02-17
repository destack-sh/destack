use serde::{Deserialize, Serialize};

use crate::replay::{RandomEventKind, TimeEventKind};
use destack_base::fnv1a_128;

/// Replay behavior for external bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayPolicy {
    /// Record the call for replay and return replayed values in replay execution.
    Recordable,
    /// Reject the call in deterministic or replay execution modes.
    NonRecordable,
}

/// Replay payload policy for recorded bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplayPayload {
    /// Record only the result value.
    Results,
    /// Record arguments and results for verification.
    ArgumentsAndResults,
}

/// Effect classification for external bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectClass {
    /// No observable side effects.
    Pure,
    /// Deterministic effects that do not require external I/O.
    Deterministic,
    /// External side effects governed by replay policy.
    External { replay: ReplayPolicy },
}

/// Replay routing for external bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BindingReplayKind {
    /// Record a generic binding call payload.
    Regular,
    /// Record a time event with a specific kind.
    Time(TimeEventKind),
    /// Record a random event with a specific kind.
    Random(RandomEventKind),
}

/// Platform scope classification for runtime bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BindingScope {
    /// Binding executes via direct host platform operations.
    Os,
    /// Binding executes entirely inside runtime-managed state.
    Runtime,
    /// Binding may cross both runtime and host platform boundaries.
    Hybrid,
}

/// Blocking behavior classification for runtime bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BindingBlocking {
    /// Binding always blocks under normal execution.
    Always,
    /// Binding never blocks and returns immediately.
    Never,
    /// Binding may block depending on runtime or host readiness.
    Sometimes,
}

/// Bitmask describing binding effect classes.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BindingEffectMask(
    /// Raw mask bits.
    pub u8,
);

impl BindingEffectMask {
    /// Mask for pure bindings.
    pub const PURE: Self = Self(1 << 0);
    /// Mask for deterministic bindings.
    pub const DETERMINISTIC: Self = Self(1 << 1);
    /// Mask for recordable external bindings.
    pub const EXTERNAL_RECORDABLE: Self = Self(1 << 2);
    /// Mask for non-recordable external bindings.
    pub const EXTERNAL_NONRECORDABLE: Self = Self(1 << 3);

    /// Return whether this mask allows the given effect.
    pub const fn allows(self, effect: BindingEffectMask) -> bool {
        (self.0 & effect.0) != 0
    }
}

const fn effect_mask_for_class(effect_class: EffectClass) -> BindingEffectMask {
    match effect_class {
        EffectClass::Pure => BindingEffectMask::PURE,
        EffectClass::Deterministic => BindingEffectMask::DETERMINISTIC,
        EffectClass::External {
            replay: ReplayPolicy::Recordable,
        } => BindingEffectMask::EXTERNAL_RECORDABLE,
        EffectClass::External {
            replay: ReplayPolicy::NonRecordable,
        } => BindingEffectMask::EXTERNAL_NONRECORDABLE,
    }
}

/// Return the current compile target host platform name.
fn current_host_platform_name() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        return "windows";
    }

    #[cfg(target_os = "android")]
    {
        return "android";
    }

    #[cfg(target_os = "linux")]
    {
        return "linux";
    }

    #[cfg(target_os = "macos")]
    {
        return "macos";
    }

    #[cfg(target_os = "freebsd")]
    {
        return "freebsd";
    }

    #[cfg(target_os = "openbsd")]
    {
        return "openbsd";
    }

    #[cfg(target_os = "netbsd")]
    {
        return "netbsd";
    }

    #[cfg(target_os = "dragonfly")]
    {
        return "dragonfly";
    }

    #[cfg(target_os = "solaris")]
    {
        return "solaris";
    }

    #[cfg(target_os = "illumos")]
    {
        return "illumos";
    }

    #[cfg(target_os = "haiku")]
    {
        return "haiku";
    }

    #[cfg(target_os = "fuchsia")]
    {
        return "fuchsia";
    }

    #[cfg(target_os = "redox")]
    {
        return "redox";
    }

    #[cfg(target_os = "hermit")]
    {
        return "hermit";
    }

    #[cfg(target_os = "ios")]
    {
        return "ios";
    }

    #[cfg(target_os = "wasi")]
    {
        return "wasi";
    }

    #[cfg(target_os = "emscripten")]
    {
        return "emscripten";
    }

    #[cfg(target_os = "none")]
    {
        return "baremetal";
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
        target_os = "wasi",
        target_os = "emscripten",
        target_os = "none"
    )))]
    {
        "unknown"
    }
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

/// Default codec used for binding payloads.
pub const CODEC_POSTCARD_V1: CodecId = CodecId::from_name("postcard-v1");

/// Metadata describing a runtime binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BindingDescriptor {
    /// Stable external name for ABI resolution.
    pub name: &'static str,
    /// Stable binding id derived from the name.
    pub id: BindingId,
    /// Hash of the canonical signature string.
    pub signature: SignatureHash,
    /// Payload codec id for record/replay.
    pub codec: CodecId,
    /// Effect classification for policy and replay.
    pub effect_class: EffectClass,
    /// Effect mask for policy checks.
    pub effect_mask: BindingEffectMask,
    /// Replay routing for the binding.
    pub replay_kind: BindingReplayKind,
    /// Replay payload capability for recorded bindings.
    pub replay_payload: ReplayPayload,
    /// Required platform capabilities for this binding.
    pub requires: &'static [&'static str],
    /// Host platforms where this binding is supported.
    pub host_platforms: &'static [&'static str],
    /// Platform scope for this binding.
    pub scope: BindingScope,
    /// Blocking behavior for this binding.
    pub blocking: BindingBlocking,
}

#[allow(clippy::too_many_arguments)]
impl BindingDescriptor {
    /// Create a binding descriptor with an explicit codec id.
    pub const fn with_codec_and_replay_kind(
        name: &'static str,
        signature: &'static str,
        codec: CodecId,
        effect_class: EffectClass,
        replay_kind: BindingReplayKind,
        replay_payload: ReplayPayload,
    ) -> Self {
        Self::with_codec_and_replay_kind_with_requires(
            name,
            signature,
            codec,
            effect_class,
            replay_kind,
            replay_payload,
            &[],
        )
    }

    /// Create a binding descriptor with an explicit codec id and required capabilities.
    pub const fn with_codec_and_replay_kind_with_requires(
        name: &'static str,
        signature: &'static str,
        codec: CodecId,
        effect_class: EffectClass,
        replay_kind: BindingReplayKind,
        replay_payload: ReplayPayload,
        requires: &'static [&'static str],
    ) -> Self {
        Self::with_codec_and_replay_kind_with_requires_and_behavior(
            name,
            signature,
            codec,
            effect_class,
            replay_kind,
            replay_payload,
            requires,
            BindingScope::Hybrid,
            BindingBlocking::Sometimes,
        )
    }

    /// Create a binding descriptor with explicit capability and behavior metadata.
    pub const fn with_codec_and_replay_kind_with_requires_and_behavior(
        name: &'static str,
        signature: &'static str,
        codec: CodecId,
        effect_class: EffectClass,
        replay_kind: BindingReplayKind,
        replay_payload: ReplayPayload,
        requires: &'static [&'static str],
        scope: BindingScope,
        blocking: BindingBlocking,
    ) -> Self {
        let effect_mask = effect_mask_for_class(effect_class);
        Self {
            name,
            id: BindingId::from_name(name),
            signature: SignatureHash::from_signature(signature),
            codec,
            effect_class,
            effect_mask,
            replay_kind,
            replay_payload,
            requires,
            host_platforms: &[],
            scope,
            blocking,
        }
    }

    /// Create a binding descriptor with an explicit codec id.
    pub const fn with_codec(
        name: &'static str,
        signature: &'static str,
        codec: CodecId,
        effect_class: EffectClass,
    ) -> Self {
        Self::with_codec_with_requires(name, signature, codec, effect_class, &[])
    }

    /// Create a binding descriptor with an explicit codec id and required capabilities.
    pub const fn with_codec_with_requires(
        name: &'static str,
        signature: &'static str,
        codec: CodecId,
        effect_class: EffectClass,
        requires: &'static [&'static str],
    ) -> Self {
        Self::with_codec_and_replay_kind_with_requires(
            name,
            signature,
            codec,
            effect_class,
            BindingReplayKind::Regular,
            ReplayPayload::Results,
            requires,
        )
    }

    /// Create a binding descriptor with the default codec.
    pub const fn new(
        name: &'static str,
        signature: &'static str,
        effect_class: EffectClass,
    ) -> Self {
        Self::new_with_requires(name, signature, effect_class, &[])
    }

    /// Create a binding descriptor with the default codec and required capabilities.
    pub const fn new_with_requires(
        name: &'static str,
        signature: &'static str,
        effect_class: EffectClass,
        requires: &'static [&'static str],
    ) -> Self {
        Self::with_codec_with_requires(name, signature, CODEC_POSTCARD_V1, effect_class, requires)
    }

    /// Create a pure binding descriptor.
    pub const fn pure(name: &'static str, signature: &'static str) -> Self {
        Self::pure_with_requires(name, signature, &[])
    }

    /// Create a pure binding descriptor with required capabilities.
    pub const fn pure_with_requires(
        name: &'static str,
        signature: &'static str,
        requires: &'static [&'static str],
    ) -> Self {
        Self::pure_with_requires_and_behavior(
            name,
            signature,
            requires,
            BindingScope::Hybrid,
            BindingBlocking::Sometimes,
        )
    }

    /// Create a pure binding descriptor with required capabilities and behavior metadata.
    pub const fn pure_with_requires_and_behavior(
        name: &'static str,
        signature: &'static str,
        requires: &'static [&'static str],
        scope: BindingScope,
        blocking: BindingBlocking,
    ) -> Self {
        Self::with_codec_and_replay_kind_with_requires_and_behavior(
            name,
            signature,
            CODEC_POSTCARD_V1,
            EffectClass::Pure,
            BindingReplayKind::Regular,
            ReplayPayload::Results,
            requires,
            scope,
            blocking,
        )
    }

    /// Create a deterministic binding descriptor.
    pub const fn deterministic(name: &'static str, signature: &'static str) -> Self {
        Self::deterministic_with_requires(name, signature, &[])
    }

    /// Create a deterministic binding descriptor with required capabilities.
    pub const fn deterministic_with_requires(
        name: &'static str,
        signature: &'static str,
        requires: &'static [&'static str],
    ) -> Self {
        Self::deterministic_with_requires_and_behavior(
            name,
            signature,
            requires,
            BindingScope::Hybrid,
            BindingBlocking::Sometimes,
        )
    }

    /// Create a deterministic binding descriptor with required capabilities and behavior metadata.
    pub const fn deterministic_with_requires_and_behavior(
        name: &'static str,
        signature: &'static str,
        requires: &'static [&'static str],
        scope: BindingScope,
        blocking: BindingBlocking,
    ) -> Self {
        Self::with_codec_and_replay_kind_with_requires_and_behavior(
            name,
            signature,
            CODEC_POSTCARD_V1,
            EffectClass::Deterministic,
            BindingReplayKind::Regular,
            ReplayPayload::Results,
            requires,
            scope,
            blocking,
        )
    }

    /// Create an external binding descriptor with explicit replay routing.
    pub const fn external(
        name: &'static str,
        signature: &'static str,
        replay: ReplayPolicy,
        replay_kind: BindingReplayKind,
    ) -> Self {
        Self::external_with_requires(name, signature, replay, replay_kind, &[])
    }

    /// Create an external binding descriptor with explicit replay routing and required capabilities.
    pub const fn external_with_requires(
        name: &'static str,
        signature: &'static str,
        replay: ReplayPolicy,
        replay_kind: BindingReplayKind,
        requires: &'static [&'static str],
    ) -> Self {
        Self::external_with_requires_and_behavior(
            name,
            signature,
            replay,
            replay_kind,
            requires,
            BindingScope::Hybrid,
            BindingBlocking::Sometimes,
        )
    }

    /// Create an external binding descriptor with explicit replay routing, capabilities, and behavior metadata.
    pub const fn external_with_requires_and_behavior(
        name: &'static str,
        signature: &'static str,
        replay: ReplayPolicy,
        replay_kind: BindingReplayKind,
        requires: &'static [&'static str],
        scope: BindingScope,
        blocking: BindingBlocking,
    ) -> Self {
        Self::external_with_payload_with_requires(
            name,
            signature,
            replay,
            replay_kind,
            ReplayPayload::Results,
            requires,
            scope,
            blocking,
        )
    }

    /// Create an external binding descriptor with a replay payload override.
    pub const fn external_with_payload(
        name: &'static str,
        signature: &'static str,
        replay: ReplayPolicy,
        replay_kind: BindingReplayKind,
        replay_payload: ReplayPayload,
    ) -> Self {
        Self::external_with_payload_with_requires(
            name,
            signature,
            replay,
            replay_kind,
            replay_payload,
            &[],
            BindingScope::Hybrid,
            BindingBlocking::Sometimes,
        )
    }

    /// Create an external binding descriptor with payload and required capabilities.
    pub const fn external_with_payload_with_requires(
        name: &'static str,
        signature: &'static str,
        replay: ReplayPolicy,
        replay_kind: BindingReplayKind,
        replay_payload: ReplayPayload,
        requires: &'static [&'static str],
        scope: BindingScope,
        blocking: BindingBlocking,
    ) -> Self {
        Self::with_codec_and_replay_kind_with_requires_and_behavior(
            name,
            signature,
            CODEC_POSTCARD_V1,
            EffectClass::External { replay },
            replay_kind,
            replay_payload,
            requires,
            scope,
            blocking,
        )
    }

    /// Return the replay payload capability for this binding.
    pub const fn replay_payload(self) -> ReplayPayload {
        self.replay_payload
    }

    /// Return the required platform capabilities for this binding.
    pub const fn requires(self) -> &'static [&'static str] {
        self.requires
    }

    /// Return the host platform availability list for this binding.
    pub const fn host_platforms(self) -> &'static [&'static str] {
        self.host_platforms
    }

    /// Return this descriptor with explicit host platform availability.
    pub const fn with_host_platforms(mut self, host_platforms: &'static [&'static str]) -> Self {
        self.host_platforms = host_platforms;
        self
    }

    /// Return whether this binding supports one host platform.
    pub fn supports_host_platform(self, platform: &str) -> bool {
        if self.host_platforms.is_empty() {
            return true;
        }

        self.host_platforms.iter().any(|name| *name == platform)
    }

    /// Return whether this binding supports the current host platform.
    pub fn supports_current_host(self) -> bool {
        self.supports_host_platform(current_host_platform_name())
    }

    /// Return the platform scope classification for this binding.
    pub const fn scope(self) -> BindingScope {
        self.scope
    }

    /// Return the blocking behavior classification for this binding.
    pub const fn blocking(self) -> BindingBlocking {
        self.blocking
    }
}
