use std::collections::BTreeMap;

use destack_dir::EnumBackingType;

/// Platform scope classification for bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BindingScope {
    /// Binding executes through direct host platform operations.
    Host,
    /// Binding executes entirely inside runtime policy and state.
    Runtime,
}

/// Blocking behavior classification for bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BindingBlocking {
    /// Binding always blocks under normal operation.
    Always,
    /// Binding never blocks and returns immediately.
    Never,
    /// Binding may block depending on flags, readiness, or host state.
    Sometimes,
}
/// Replay routing for generated bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BindingReplayKind {
    /// Regular binding replay behavior.
    Regular,
    /// Time read bindings with specialized replay.
    Time(TimeEventKind),
    /// Randomness bindings with specialized replay.
    Random(RandomEventKind),
}

/// Time event kinds supported by replay routing.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TimeEventKind {
    /// Virtual clock seed event.
    Seed,
    /// Monotonic clock sample.
    MonotonicSample,
    /// Wall clock read.
    WallClockRead,
    /// Timer scheduled event.
    TimerScheduled,
    /// Timer fired event.
    TimerFired,
    /// Timer canceled event.
    TimerCanceled,
    /// Sleep scheduled event.
    SleepScheduled,
    /// Sleep wake event.
    SleepWake,
}

/// Random event kinds supported by replay routing.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RandomEventKind {
    /// Stream seed or reseed event.
    Seed,
    /// Stream allocation event.
    Stream,
    /// Random bytes produced by the stream.
    Bytes,
    /// Random u64 produced by the stream.
    NextU64,
}

/// Replay behavior for external bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReplayPolicy {
    /// Record the call for replay and return replayed values in replay execution.
    Recordable,
    /// Reject the call in deterministic or replay execution modes.
    NonRecordable,
}

/// Replay payload policy for recorded bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReplayPayload {
    /// Record only the result value.
    ResultsOnly,
    /// Record arguments and results for verification.
    ArgumentsAndResults,
}

/// Effect classification for external bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EffectClass {
    /// No observable side effects.
    Pure,
    /// Deterministic effects that do not require external I/O.
    Deterministic,
    /// External side effects governed by replay policy.
    External {
        /// Replay behavior for the effect.
        replay: ReplayPolicy,
    },
}

/// Binding metadata extracted from builtin sources.
#[derive(Debug, Clone)]
pub(crate) struct BindingEntry {
    /// Declaration name used by runtime implementation functions.
    pub implementation_name: String,
    /// Declaration documentation extracted from builtin sources.
    pub documentation: Option<String>,
    /// Canonical signature string for stability checks.
    pub signature: String,
    /// Parameter metadata for the binding.
    pub parameters: Vec<BindingParameter>,
    /// Return binding type for generated wrappers.
    pub return_binding: BindingType,
    /// Whether the binding returns a Result wrapper.
    pub return_is_result: bool,
    /// Effect classification for replay and policy.
    pub effect_class: EffectClass,
    /// Replay routing for the binding.
    pub replay_kind: BindingReplayKind,
    /// Replay payload capability for recorded bindings.
    pub replay_payload: ReplayPayload,
    /// Required platform capabilities for this binding.
    pub requires: Vec<String>,
    /// Host platforms where this binding is supported.
    pub host_platforms: Vec<String>,
    /// Platform scope for this binding.
    pub scope: BindingScope,
    /// Blocking behavior for this binding.
    pub blocking: BindingBlocking,
}

/// Return metadata extracted from a binding signature.
#[derive(Debug, Clone)]
pub(crate) struct BindingReturn {
    /// Return binding type for generated wrappers.
    pub binding_type: BindingType,
    /// Whether the binding returns a Result wrapper.
    pub is_result: bool,
}

/// Catalog grouped by domain.
pub(crate) type BindingCatalog = BTreeMap<String, BTreeMap<String, BindingEntry>>;

/// Parameter metadata extracted from signatures.
#[derive(Debug, Clone)]
pub(crate) struct BindingParameter {
    /// Parameter name for diagnostics.
    pub name: String,
    /// Parameter type text, if declared.
    pub type_text: Option<String>,
    /// Parameter binding type for generated wrappers.
    pub binding_type: BindingType,
}

/// Field metadata for structured binding types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BindingField {
    /// Field name as declared in the source type.
    pub name: String,
    /// Field binding type for generated wrappers.
    pub binding_type: BindingType,
}

/// Enum variant metadata for binding types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BindingEnumVariant {
    /// Variant name.
    pub name: String,
    /// Variant value.
    pub value: BindingEnumValue,
}

/// Enum literal values used for binding decoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BindingEnumValue {
    /// Integer-backed variant.
    Int(i64),
    /// String-backed variant.
    String(String),
}

/// Supported binding types for generated decoders.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BindingType {
    /// No return value or parameter.
    Void,
    /// Boolean value.
    Bool,
    /// Signed integer with bit width.
    Int(u8),
    /// Unsigned integer with bit width.
    UInt(u8),
    /// Floating point with bit width.
    Float(u8),
    /// UTF 16 or UTF 8 string value.
    String,
    /// Slice of UTF-8 strings.
    StringSlice,
    /// Slice of binding values.
    Slice(Box<BindingType>),
    /// Owned array of binding values.
    Array(Box<BindingType>),
    /// Nominal newtype wrapper.
    Newtype {
        /// Nominal type name.
        name: String,
        /// Owning platform domain.
        domain: String,
        /// Underlying binding type.
        inner: Box<BindingType>,
    },
    /// Named struct type with fixed fields.
    Struct {
        /// Struct type name.
        name: String,
        /// Owning platform domain.
        domain: String,
        /// Ordered field definitions.
        fields: Vec<BindingField>,
    },
    /// Enum type with an explicit backing representation.
    Enum {
        /// Enum type name.
        name: String,
        /// Owning platform domain.
        domain: String,
        /// Enum backing representation.
        backing: EnumBackingType,
        /// Declared enum variants.
        variants: Vec<BindingEnumVariant>,
    },
}
