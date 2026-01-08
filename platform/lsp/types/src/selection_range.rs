use serde::{Deserialize, Serialize};

use crate::{
    PartialResultParams, Position, Range, StaticTextDocumentRegistrationOptions,
    TextDocumentIdentifier, WorkDoneProgressOptions, WorkDoneProgressParams,
};

/// Client capabilities for selection range requests.
#[derive(Debug, Eq, PartialEq, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectionRangeClientCapabilities {
    /// Whether implementation supports dynamic registration for selection range
    /// providers. If this is set to `true` the client supports the new
    /// `SelectionRangeRegistrationOptions` return value for the corresponding
    /// server capability as well.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamic_registration: Option<bool>,
}

/// Selection range options.
#[derive(Debug, Eq, PartialEq, Clone, Default, Deserialize, Serialize)]
pub struct SelectionRangeOptions {
    /// Work done progress options.
    #[serde(flatten)]
    pub work_done_progress_options: WorkDoneProgressOptions,
}

/// Selection range registration options.
#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
pub struct SelectionRangeRegistrationOptions {
    /// Selection range options.
    #[serde(flatten)]
    pub selection_range_options: SelectionRangeOptions,

    /// Registration options.
    #[serde(flatten)]
    pub registration_options: StaticTextDocumentRegistrationOptions,
}

/// Server capability for selection range support.
#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum SelectionRangeProviderCapability {
    /// Simple boolean flag.
    Simple(bool),
    /// Selection range options.
    Options(SelectionRangeOptions),
    /// Selection range registration options.
    RegistrationOptions(SelectionRangeRegistrationOptions),
}

impl From<SelectionRangeRegistrationOptions> for SelectionRangeProviderCapability {
    fn from(from: SelectionRangeRegistrationOptions) -> Self {
        Self::RegistrationOptions(from)
    }
}

impl From<SelectionRangeOptions> for SelectionRangeProviderCapability {
    fn from(from: SelectionRangeOptions) -> Self {
        Self::Options(from)
    }
}

impl From<bool> for SelectionRangeProviderCapability {
    fn from(from: bool) -> Self {
        Self::Simple(from)
    }
}

/// A parameter literal used in selection range requests.
#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectionRangeParams {
    /// The text document.
    pub text_document: TextDocumentIdentifier,

    /// The positions inside the text document.
    pub positions: Vec<Position>,

    /// Work done progress params.
    #[serde(flatten)]
    pub work_done_progress_params: WorkDoneProgressParams,

    /// Partial result params.
    #[serde(flatten)]
    pub partial_result_params: PartialResultParams,
}

/// Represents a selection range.
#[derive(Debug, Eq, PartialEq, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectionRange {
    /// Range of the selection.
    pub range: Range,

    /// The parent selection range containing this range.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<Box<SelectionRange>>,
}
