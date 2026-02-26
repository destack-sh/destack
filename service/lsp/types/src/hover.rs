use serde::{Deserialize, Serialize};

use crate::{
    MarkedString, MarkupContent, MarkupKind, Range, TextDocumentPositionParams,
    TextDocumentRegistrationOptions, WorkDoneProgressOptions, WorkDoneProgressParams,
};

/// Client capabilities for hover requests.
#[derive(Debug, Eq, PartialEq, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HoverClientCapabilities {
    /// Whether completion supports dynamic registration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamic_registration: Option<bool>,

    /// Client supports the follow content formats for the content
    /// property. The order describes the preferred format of the client.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_format: Option<Vec<MarkupKind>>,
}

/// Hover options.
#[derive(Debug, Eq, PartialEq, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HoverOptions {
    /// Work done progress options.
    #[serde(flatten)]
    pub work_done_progress_options: WorkDoneProgressOptions,
}

/// Registration options for hover requests.
#[derive(Debug, Eq, PartialEq, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HoverRegistrationOptions {
    /// Text document registration options.
    #[serde(flatten)]
    pub text_document_registration_options: TextDocumentRegistrationOptions,

    /// Hover options.
    #[serde(flatten)]
    pub hover_options: HoverOptions,
}

/// Hover provider capability.
#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum HoverProviderCapability {
    /// Simple boolean capability.
    Simple(bool),
    /// Options-based capability.
    Options(HoverOptions),
}

impl From<HoverOptions> for HoverProviderCapability {
    fn from(from: HoverOptions) -> Self {
        Self::Options(from)
    }
}

impl From<bool> for HoverProviderCapability {
    fn from(from: bool) -> Self {
        Self::Simple(from)
    }
}

/// Parameters for a hover request.
#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HoverParams {
    /// Text document position parameters.
    #[serde(flatten)]
    pub text_document_position_params: TextDocumentPositionParams,

    /// Work done progress parameters.
    #[serde(flatten)]
    pub work_done_progress_params: WorkDoneProgressParams,
}

/// The result of a hover request.
#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
pub struct Hover {
    /// The hover's content
    pub contents: HoverContents,
    /// An optional range is a range inside a text document
    /// that is used to visualize a hover, e.g. by changing the background color.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<Range>,
}

/// Hover contents could be single entry or multiple entries.
#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HoverContents {
    /// A single marked string.
    Scalar(MarkedString),
    /// An array of marked strings.
    Array(Vec<MarkedString>),
    /// Markup content.
    Markup(MarkupContent),
}
