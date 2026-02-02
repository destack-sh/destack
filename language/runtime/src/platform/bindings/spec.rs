use serde::{Deserialize, Serialize};

use destack_base::fnv1a_128;

/// Replay behavior for external bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayPolicy {
    /// Record the call for replay and return replayed values in replay mode.
    Recordable,
    /// Reject the call in deterministic or replay modes.
    NonRecordable,
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

/// Specialized replay log variants for bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogKind {
    /// Log time-related effects with a dedicated record.
    Time,
    /// Log randomness with a dedicated record.
    Random,
    /// Log scheduler events with a dedicated record.
    Scheduler,
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
    /// Specialized log kind for replay.
    pub log_kind: Option<LogKind>,
}

impl BindingDescriptor {
    /// Create a binding descriptor with an explicit codec id.
    pub const fn with_codec_and_log(
        name: &'static str,
        signature: &'static str,
        codec: CodecId,
        effect_class: EffectClass,
        log_kind: Option<LogKind>,
    ) -> Self {
        let effect_mask = effect_mask_for_class(effect_class);
        Self {
            name,
            id: BindingId::from_name(name),
            signature: SignatureHash::from_signature(signature),
            codec,
            effect_class,
            effect_mask,
            log_kind,
        }
    }

    /// Create a binding descriptor with an explicit codec id.
    pub const fn with_codec(
        name: &'static str,
        signature: &'static str,
        codec: CodecId,
        effect_class: EffectClass,
    ) -> Self {
        Self::with_codec_and_log(name, signature, codec, effect_class, None)
    }

    /// Create a binding descriptor with the default codec.
    pub const fn new(
        name: &'static str,
        signature: &'static str,
        effect_class: EffectClass,
    ) -> Self {
        Self::with_codec(name, signature, CODEC_POSTCARD_V1, effect_class)
    }

    /// Create a pure binding descriptor.
    pub const fn pure(name: &'static str, signature: &'static str) -> Self {
        Self::new(name, signature, EffectClass::Pure)
    }

    /// Create a deterministic binding descriptor.
    pub const fn deterministic(name: &'static str, signature: &'static str) -> Self {
        Self::new(name, signature, EffectClass::Deterministic)
    }

    /// Create an external binding descriptor with an optional log kind.
    pub const fn external(
        name: &'static str,
        signature: &'static str,
        replay: ReplayPolicy,
        log_kind: Option<LogKind>,
    ) -> Self {
        Self::with_codec_and_log(
            name,
            signature,
            CODEC_POSTCARD_V1,
            EffectClass::External { replay },
            log_kind,
        )
    }
}
