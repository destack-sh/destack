use crate::StringId;
use serde::{Deserialize, Serialize};

/// Extern binding metadata for a symbol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternBinding {
    /// The external name override (defaults to the symbol name).
    pub name: Option<StringId>,
}

/// Intrinsic binding metadata for a symbol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntrinsicBinding {
    /// The intrinsic name override (defaults to the symbol name).
    pub name: Option<StringId>,
}

/// Language item binding metadata for a symbol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguageItemBinding {
    /// The language item name override (defaults to the symbol name).
    pub name: Option<StringId>,
}

/// Deprecated marker metadata for a symbol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeprecatedNotice {
    /// The deprecated message override.
    pub message: Option<StringId>,
}

/// Experimental marker metadata for a symbol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExperimentalNotice {
    /// The experimental message override.
    pub message: Option<StringId>,
}

/// Unroll hint metadata for a symbol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnrollHint {
    /// The suggested unroll factor.
    pub factor: Option<u32>,
}

/// Taint marker metadata for a symbol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaintMarker {
    /// The optional taint label.
    pub label: Option<StringId>,
}

/// Sink marker metadata for a symbol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SinkMarker {
    /// The optional sink label.
    pub label: Option<StringId>,
}

/// Sanitizer marker metadata for a symbol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SanitizerMarker {
    /// The optional sanitizer label.
    pub label: Option<StringId>,
}

/// Tag marker metadata for a symbol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagMarker {
    /// The optional tag label.
    pub label: Option<StringId>,
}

/// Lifetime annotation metadata for a symbol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LifetimeAnnotation {
    /// Borrow from specific parameter names.
    Parameters(Vec<StringId>),
    /// Borrow from static/global data only.
    Static,
}

/// Well-known decorator metadata attached to a symbol.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SymbolDecorators {
    /// The external binding override.
    pub extern_binding: Option<ExternBinding>,
    /// The intrinsic binding override.
    pub intrinsic_binding: Option<IntrinsicBinding>,
    /// The language item binding override.
    pub language_item: Option<LanguageItemBinding>,
    /// The deprecated marker info.
    pub deprecated: Option<DeprecatedNotice>,
    /// The experimental marker info.
    pub experimental: Option<ExperimentalNotice>,
    /// The lifetime annotation override.
    pub lifetime: Option<LifetimeAnnotation>,
    /// The no-managed hint.
    pub is_no_managed: bool,
    /// The stack-only hint.
    pub is_stack_only: bool,
    /// The inline hint.
    pub is_inline: bool,
    /// The noinline hint.
    pub is_noinline: bool,
    /// The loop unroll hint.
    pub unroll: Option<UnrollHint>,
    /// The hot-path hint.
    pub is_hot: bool,
    /// The cold-path hint.
    pub is_cold: bool,
    /// The likely-branch hint.
    pub is_likely: bool,
    /// The unlikely-branch hint.
    pub is_unlikely: bool,
    /// The must-use marker.
    pub is_must_use: bool,
    /// The pure marker.
    pub is_pure: bool,
    /// The tailcall hint.
    pub is_tailcall: bool,
    /// The unsafe marker.
    pub is_unsafe: bool,
    /// The transmute marker.
    pub is_transmute: bool,
    /// The taint markers.
    pub taints: Vec<TaintMarker>,
    /// The sink markers.
    pub sinks: Vec<SinkMarker>,
    /// The sanitizer markers.
    pub sanitizers: Vec<SanitizerMarker>,
    /// The tag markers.
    pub tags: Vec<TagMarker>,
}
