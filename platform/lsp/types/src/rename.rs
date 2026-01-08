use crate::{Range, TextDocumentPositionParams, WorkDoneProgressOptions, WorkDoneProgressParams};
use serde::{Deserialize, Serialize};

/// Parameters for a rename request.
#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameParams {
    /// Text document and position.
    #[serde(flatten)]
    pub text_document_position: TextDocumentPositionParams,

    /// The new name of the symbol. If the given name is not valid the
    /// request must return a [ResponseError](#ResponseError) with an
    /// appropriate message set.
    pub new_name: String,

    /// Work done progress params.
    #[serde(flatten)]
    pub work_done_progress_params: WorkDoneProgressParams,
}

/// Rename options.
#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameOptions {
    /// Renames should be checked and tested before being executed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prepare_provider: Option<bool>,

    /// Work done progress options.
    #[serde(flatten)]
    pub work_done_progress_options: WorkDoneProgressOptions,
}

/// Client capabilities for rename requests.
#[derive(Debug, Eq, PartialEq, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameClientCapabilities {
    /// Whether rename supports dynamic registration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamic_registration: Option<bool>,

    /// Client supports testing for validity of rename operations before execution.
    ///
    /// @since 3.12.0
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prepare_support: Option<bool>,

    /// Client supports the default behavior result.
    ///
    /// The value indicates the default behavior used by the
    /// client.
    ///
    /// @since 3.16.0
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prepare_support_default_behavior: Option<PrepareSupportDefaultBehavior>,

    /// Whether the client honors the change annotations in
    /// text edits and resource operations returned via the
    /// rename request's workspace edit by for example presenting
    /// the workspace edit in the user interface and asking
    /// for confirmation.
    ///
    /// @since 3.16.0
    #[serde(skip_serializing_if = "Option::is_none")]
    pub honors_change_annotations: Option<bool>,
}

/// The default behavior for prepare support.
#[derive(Eq, PartialEq, Copy, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PrepareSupportDefaultBehavior(i32);
lsp_enum! {
impl PrepareSupportDefaultBehavior {
    /// The client's default behavior is to select the identifier
    /// according the to language's syntax rule
    pub const IDENTIFIER: PrepareSupportDefaultBehavior = PrepareSupportDefaultBehavior(1);
}
}

/// Response for prepare rename request.
#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
#[serde(untagged)]
#[serde(rename_all = "camelCase")]
pub enum PrepareRenameResponse {
    /// A range to rename.
    Range(Range),
    /// A range with a placeholder for the new name.
    RangeWithPlaceholder {
        /// The range of the identifier to rename.
        range: Range,
        /// The placeholder text for the new name.
        placeholder: String,
    },
    /// Use default behavior.
    #[serde(rename_all = "camelCase")]
    DefaultBehavior {
        /// Whether to use the default behavior.
        default_behavior: bool,
    },
}
