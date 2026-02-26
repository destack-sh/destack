use serde::{Deserialize, Serialize};

use crate::{
    DynamicRegistrationClientCapabilities, PartialResultParams, TextDocumentPositionParams,
    TextDocumentRegistrationOptions, WorkDoneProgressOptions, WorkDoneProgressParams,
};

/// Client capabilities for the moniker request.
pub type MonikerClientCapabilities = DynamicRegistrationClientCapabilities;

/// Server capabilities for monikers.
#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum MonikerServerCapabilities {
    /// Basic moniker options.
    Options(MonikerOptions),
    /// Moniker registration options.
    RegistrationOptions(MonikerRegistrationOptions),
}

/// Moniker options.
#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
pub struct MonikerOptions {
    /// Work done progress options.
    #[serde(flatten)]
    pub work_done_progress_options: WorkDoneProgressOptions,
}

/// Moniker registration options.
#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonikerRegistrationOptions {
    /// Text document registration options.
    #[serde(flatten)]
    pub text_document_registration_options: TextDocumentRegistrationOptions,

    /// Moniker options.
    #[serde(flatten)]
    pub moniker_options: MonikerOptions,
}

/// Moniker uniqueness level to define scope of the moniker.
#[derive(Debug, Eq, PartialEq, Deserialize, Serialize, Copy, Clone)]
#[serde(rename_all = "camelCase")]
pub enum UniquenessLevel {
    /// The moniker is only unique inside a document
    Document,
    /// The moniker is unique inside a project for which a dump got created
    Project,
    /// The moniker is unique inside the group to which a project belongs
    Group,
    /// The moniker is unique inside the moniker scheme.
    Scheme,
    /// The moniker is globally unique
    Global,
}

/// The moniker kind.
#[derive(Debug, Eq, PartialEq, Deserialize, Serialize, Copy, Clone)]
#[serde(rename_all = "camelCase")]
pub enum MonikerKind {
    /// The moniker represent a symbol that is imported into a project
    Import,
    /// The moniker represent a symbol that is exported into a project
    Export,
    /// The moniker represents a symbol that is local to a project (e.g. a local
    /// variable of a function, a class not visible outside the project, ...)
    Local,
}

/// Parameters for the moniker request.
#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonikerParams {
    /// Text document and position.
    #[serde(flatten)]
    pub text_document_position_params: TextDocumentPositionParams,

    /// Work done progress params.
    #[serde(flatten)]
    pub work_done_progress_params: WorkDoneProgressParams,

    /// Partial result params.
    #[serde(flatten)]
    pub partial_result_params: PartialResultParams,
}

/// Moniker definition to match LSIF 0.5 moniker definition.
#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Moniker {
    /// The scheme of the moniker. For example tsc or .Net
    pub scheme: String,

    /// The identifier of the moniker. The value is opaque in LSIF however
    /// schema owners are allowed to define the structure if they want.
    pub identifier: String,

    /// The scope in which the moniker is unique
    pub unique: UniquenessLevel,

    /// The moniker kind if known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<MonikerKind>,
}
