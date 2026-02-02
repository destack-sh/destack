use std::collections::BTreeMap;

use destack_dir::EnumBackingType;

/// Binding metadata extracted from builtin sources.
#[derive(Debug, Clone)]
pub(crate) struct BindingEntry {
    /// Canonical signature string for stability checks.
    pub signature: String,
    /// Parameter metadata for the binding.
    pub params: Vec<BindingParam>,
    /// Return binding type for generated wrappers.
    pub return_binding: BindingType,
    /// Whether the binding returns a Result wrapper.
    pub return_is_result: bool,
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
pub(crate) struct BindingParam {
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
