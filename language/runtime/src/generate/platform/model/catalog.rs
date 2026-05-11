use std::collections::BTreeMap;

use destack_dir::EnumBackingType;

/// Provider that implements one binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CatalogBindingProvider {
    /// Binding executes through direct platform operations.
    Host,
    /// Binding executes entirely inside runtime policy and state.
    Runtime,
}

/// Execution context required by one binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CatalogBindingAffinity {
    /// Binding has no execution context requirement.
    None,
    /// Binding requires the same worker context.
    Worker,
    /// Binding requires the process main context.
    Main,
}

/// Simulation support for one binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CatalogBindingSimulation {
    /// Binding cannot execute in a simulation world.
    Unsupported,
    /// Binding can execute in a simulation world.
    Supported,
}

/// Replay routing for generated bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CatalogBindingReplayKind {
    /// Binding call replay route.
    BindingCall,
    /// Entropy bindings with specialized replay.
    Entropy(CatalogEntropyKind),
}

/// Entropy event kinds supported by replay routing.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CatalogEntropyKind {
    /// Monotonic clock sample.
    TimeReadMonotonic,
    /// Wall clock read.
    TimeReadWall,
    /// Stream allocation event.
    RandomStreamCreate,
    /// Random bytes produced by the stream.
    RandomReadBytes,
    /// Random u64 produced by the stream.
    RandomReadU64,
}

/// Replay policy for external bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CatalogReplayPolicy {
    /// Record the call for replay and return replayed values in replay execution.
    Recordable,
    /// Reject the call in deterministic or replay execution modes.
    NonRecordable,
}

/// Replay payload policy for recorded bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CatalogReplayPayload {
    /// Record only the result value.
    ResultsOnly,
    /// Record arguments and results for verification.
    ArgumentsAndResults,
}

/// Effect kind for external bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CatalogEffect {
    /// No observable side effects.
    Pure,
    /// Deterministic effects that do not require external I/O.
    Deterministic,
    /// External side effects governed by replay policy.
    External {
        /// Replay policy for the effect.
        replay: CatalogReplayPolicy,
    },
}

/// Binding metadata extracted from library sources.
#[derive(Debug, Clone)]
pub(crate) struct BindingEntry {
    /// Declaration name used by runtime implementation functions.
    pub implementation_name: String,
    /// Declaration documentation extracted from library sources.
    pub documentation: Option<String>,
    /// Canonical signature string for stability checks.
    pub signature: String,
    /// Parameter metadata for the binding.
    pub parameters: Vec<BindingParameter>,
    /// Return binding type for generated wrappers.
    pub return_binding: BindingType,
    /// Whether the binding returns a Result wrapper.
    pub return_is_result: bool,
    /// Effect kind for replay and policy.
    pub effect: CatalogEffect,
    /// Replay routing for the binding.
    pub replay_kind: CatalogBindingReplayKind,
    /// Replay payload policy for recorded bindings.
    pub replay_payload: CatalogReplayPayload,
    /// Required host actions for this binding.
    pub requires: Vec<String>,
    /// Platforms where this binding is supported.
    pub platforms: Vec<String>,
    /// Hosts where this binding is supported.
    pub hosts: Vec<String>,
    /// Provider that implements this binding.
    pub provider: CatalogBindingProvider,
    /// Execution context required by this binding.
    pub affinity: CatalogBindingAffinity,
    /// Simulation support for this binding.
    pub simulation: CatalogBindingSimulation,
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

/// Constant catalog grouped by domain.
pub(crate) type ConstantCatalog = BTreeMap<String, BTreeMap<String, ConstantEntry>>;

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
    /// Field documentation extracted from library sources.
    pub documentation: Option<String>,
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

/// Tagged union variant metadata for binding types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BindingTaggedUnionVariant {
    /// Variant name.
    pub name: String,
    /// Variant payload type.
    pub binding_type: BindingType,
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
    /// Array of binding values.
    Array(Box<BindingType>),
    /// Optional binding value.
    Optional(Box<BindingType>),
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
    /// Tagged union type with named payload variants.
    TaggedUnion {
        /// Union type name.
        name: String,
        /// Owning platform domain.
        domain: String,
        /// Declared tagged union variants.
        variants: Vec<BindingTaggedUnionVariant>,
    },
}

impl BindingType {
    /// Report whether this binding type is one raw byte.
    pub(crate) fn is_byte_element(&self) -> bool {
        matches!(self, Self::UInt(8))
    }

    /// Report whether this binding type is a byte collection.
    pub(crate) fn is_byte_collection(&self) -> bool {
        match self {
            Self::Slice(inner) | Self::Array(inner) => inner.is_byte_element(),
            Self::Optional(inner) => inner.is_byte_collection(),
            _ => false,
        }
    }
}

/// Constant declaration metadata extracted from library sources.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConstantEntry {
    /// Constant name as declared in library sources.
    pub name: String,
    /// Constant documentation extracted from library sources.
    pub documentation: Option<String>,
    /// Constant binding type for generated ABI rendering.
    pub binding_type: BindingType,
    /// Constant value payload.
    pub value: ConstantValue,
}

/// Supported constant value payloads for generated ABI rendering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ConstantValue {
    /// Integer literal payload.
    Integer(i128),
}
