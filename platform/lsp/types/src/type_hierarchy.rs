use crate::{
    DynamicRegistrationClientCapabilities, LSPAny, PartialResultParams, Range,
    StaticRegistrationOptions, SymbolKind, SymbolTag, TextDocumentPositionParams,
    TextDocumentRegistrationOptions, Uri, WorkDoneProgressOptions, WorkDoneProgressParams,
};

use serde::{Deserialize, Serialize};

/// Client capabilities for the type hierarchy feature.
pub type TypeHierarchyClientCapabilities = DynamicRegistrationClientCapabilities;

/// Server options for the type hierarchy feature.
#[derive(Debug, Eq, PartialEq, Clone, Default, Deserialize, Serialize)]
pub struct TypeHierarchyOptions {
    /// Work done progress options.
    #[serde(flatten)]
    pub work_done_progress_options: WorkDoneProgressOptions,
}

/// Registration options for the type hierarchy feature.
#[derive(Debug, Eq, PartialEq, Clone, Default, Deserialize, Serialize)]
pub struct TypeHierarchyRegistrationOptions {
    /// Text document registration options.
    #[serde(flatten)]
    pub text_document_registration_options: TextDocumentRegistrationOptions,

    /// Type hierarchy options.
    #[serde(flatten)]
    pub type_hierarchy_options: TypeHierarchyOptions,

    /// Static registration options.
    #[serde(flatten)]
    pub static_registration_options: StaticRegistrationOptions,
}

/// Parameters for the type hierarchy prepare request.
#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
pub struct TypeHierarchyPrepareParams {
    /// The text document and position.
    #[serde(flatten)]
    pub text_document_position_params: TextDocumentPositionParams,

    /// Work done progress parameters.
    #[serde(flatten)]
    pub work_done_progress_params: WorkDoneProgressParams,
}

/// Parameters for the type hierarchy supertypes request.
#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
pub struct TypeHierarchySupertypesParams {
    /// The hierarchy item to get supertypes for.
    pub item: TypeHierarchyItem,

    /// Work done progress parameters.
    #[serde(flatten)]
    pub work_done_progress_params: WorkDoneProgressParams,

    /// Partial result parameters.
    #[serde(flatten)]
    pub partial_result_params: PartialResultParams,
}

/// Parameters for the type hierarchy subtypes request.
#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
pub struct TypeHierarchySubtypesParams {
    /// The hierarchy item to get subtypes for.
    pub item: TypeHierarchyItem,

    /// Work done progress parameters.
    #[serde(flatten)]
    pub work_done_progress_params: WorkDoneProgressParams,

    /// Partial result parameters.
    #[serde(flatten)]
    pub partial_result_params: PartialResultParams,
}

/// Represents an item in a type hierarchy.
#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeHierarchyItem {
    /// The name of this item.
    pub name: String,

    /// The kind of this item.
    pub kind: SymbolKind,

    /// Tags for this item.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<SymbolTag>,

    /// More detail for this item, e.g. the signature of a function.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,

    /// The resource identifier of this item.
    pub uri: Uri,

    /// The range enclosing this symbol not including leading/trailing whitespace
    /// but everything else, e.g. comments and code.
    pub range: Range,

    /// The range that should be selected and revealed when this symbol is being
    /// picked, e.g. the name of a function. Must be contained by the
    /// [`range`](#TypeHierarchyItem.range).
    pub selection_range: Range,

    /// A data entry field that is preserved between a type hierarchy prepare and
    /// supertypes or subtypes requests. It could also be used to identify the
    /// type hierarchy in the server, helping improve the performance on
    /// resolving supertypes and subtypes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<LSPAny>,
}
