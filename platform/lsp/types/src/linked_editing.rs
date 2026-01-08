use serde::{Deserialize, Serialize};

use crate::{
    DynamicRegistrationClientCapabilities, Range, StaticRegistrationOptions,
    TextDocumentPositionParams, TextDocumentRegistrationOptions, WorkDoneProgressOptions,
    WorkDoneProgressParams,
};

/// Client capabilities for linked editing ranges.
pub type LinkedEditingRangeClientCapabilities = DynamicRegistrationClientCapabilities;

/// Options for linked editing ranges.
#[derive(Debug, Eq, PartialEq, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkedEditingRangeOptions {
    /// Work done progress options.
    #[serde(flatten)]
    pub work_done_progress_options: WorkDoneProgressOptions,
}

/// Registration options for linked editing ranges.
#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkedEditingRangeRegistrationOptions {
    /// Text document registration options.
    #[serde(flatten)]
    pub text_document_registration_options: TextDocumentRegistrationOptions,

    /// Linked editing range options.
    #[serde(flatten)]
    pub linked_editing_range_options: LinkedEditingRangeOptions,

    /// Static registration options.
    #[serde(flatten)]
    pub static_registration_options: StaticRegistrationOptions,
}

/// Server capabilities for linked editing ranges.
#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum LinkedEditingRangeServerCapabilities {
    /// Simple boolean capability.
    Simple(bool),
    /// Options-based capability.
    Options(LinkedEditingRangeOptions),
    /// Registration options capability.
    RegistrationOptions(LinkedEditingRangeRegistrationOptions),
}

/// Parameters for linked editing range requests.
#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkedEditingRangeParams {
    /// Text document position parameters.
    #[serde(flatten)]
    pub text_document_position_params: TextDocumentPositionParams,

    /// Work done progress parameters.
    #[serde(flatten)]
    pub work_done_progress_params: WorkDoneProgressParams,
}

/// The result of a linked editing range request.
#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkedEditingRanges {
    /// A list of ranges that can be renamed together.
    /// The ranges must have identical length and contain identical text content.
    /// The ranges cannot overlap.
    pub ranges: Vec<Range>,

    /// An optional word pattern (regular expression) that describes valid contents for
    /// the given ranges. If no pattern is provided, the client configuration's word
    /// pattern will be used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub word_pattern: Option<String>,
}
