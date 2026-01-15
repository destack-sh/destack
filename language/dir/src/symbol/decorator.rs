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
    /// The taint markers.
    pub taints: Vec<TaintMarker>,
    /// The tag markers.
    pub tags: Vec<TagMarker>,
}
