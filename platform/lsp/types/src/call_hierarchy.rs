use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    DynamicRegistrationClientCapabilities, PartialResultParams, Range, SymbolKind, SymbolTag,
    TextDocumentPositionParams, Uri, WorkDoneProgressOptions, WorkDoneProgressParams,
};

/// Client capabilities for call hierarchy requests.
pub type CallHierarchyClientCapabilities = DynamicRegistrationClientCapabilities;

/// Options for call hierarchy support.
#[derive(Debug, Eq, PartialEq, Clone, Default, Deserialize, Serialize, Copy)]
#[serde(rename_all = "camelCase")]
pub struct CallHierarchyOptions {
    /// Work done progress options.
    #[serde(flatten)]
    pub work_done_progress_options: WorkDoneProgressOptions,
}

/// Server capability for call hierarchy.
#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize, Copy)]
#[serde(untagged)]
pub enum CallHierarchyServerCapability {
    /// Simple boolean capability.
    Simple(bool),
    /// Detailed options.
    Options(CallHierarchyOptions),
}

impl From<CallHierarchyOptions> for CallHierarchyServerCapability {
    fn from(from: CallHierarchyOptions) -> Self {
        Self::Options(from)
    }
}

impl From<bool> for CallHierarchyServerCapability {
    fn from(from: bool) -> Self {
        Self::Simple(from)
    }
}

/// Parameters for preparing a call hierarchy request.
#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CallHierarchyPrepareParams {
    /// The text document and position.
    #[serde(flatten)]
    pub text_document_position_params: TextDocumentPositionParams,

    /// Work done progress parameters.
    #[serde(flatten)]
    pub work_done_progress_params: WorkDoneProgressParams,
}

/// An item in the call hierarchy.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CallHierarchyItem {
    /// The name of this item.
    pub name: String,

    /// The kind of this item.
    pub kind: SymbolKind,

    /// Tags for this item.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<SymbolTag>>,

    /// More detail for this item, e.g. the signature of a function.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,

    /// The resource identifier of this item.
    pub uri: Uri,

    /// The range enclosing this symbol not including leading/trailing whitespace but everything else, e.g. comments and code.
    pub range: Range,

    /// The range that should be selected and revealed when this symbol is being picked, e.g. the name of a function.
    /// Must be contained by the [`range`](#CallHierarchyItem.range).
    pub selection_range: Range,

    /// A data entry field that is preserved between a call hierarchy prepare and incoming calls or outgoing calls requests.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// Parameters for requesting incoming calls.
#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CallHierarchyIncomingCallsParams {
    /// The call hierarchy item to get incoming calls for.
    pub item: CallHierarchyItem,

    /// Work done progress parameters.
    #[serde(flatten)]
    pub work_done_progress_params: WorkDoneProgressParams,

    /// Partial result parameters.
    #[serde(flatten)]
    pub partial_result_params: PartialResultParams,
}

/// Represents an incoming call, e.g. a caller of a method or constructor.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CallHierarchyIncomingCall {
    /// The item that makes the call.
    pub from: CallHierarchyItem,

    /// The range at which at which the calls appears. This is relative to the caller
    /// denoted by [`this.from`](#CallHierarchyIncomingCall.from).
    pub from_ranges: Vec<Range>,
}

/// Parameters for requesting outgoing calls.
#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CallHierarchyOutgoingCallsParams {
    /// The call hierarchy item to get outgoing calls for.
    pub item: CallHierarchyItem,

    /// Work done progress parameters.
    #[serde(flatten)]
    pub work_done_progress_params: WorkDoneProgressParams,

    /// Partial result parameters.
    #[serde(flatten)]
    pub partial_result_params: PartialResultParams,
}

/// Represents an outgoing call, e.g. calling a getter from a method or a method from a constructor etc.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CallHierarchyOutgoingCall {
    /// The item that is called.
    pub to: CallHierarchyItem,

    /// The range at which this item is called. This is the range relative to the caller, e.g the item
    /// passed to [`provideCallHierarchyOutgoingCalls`](#CallHierarchyItemProvider.provideCallHierarchyOutgoingCalls)
    /// and not [`this.to`](#CallHierarchyOutgoingCall.to).
    pub from_ranges: Vec<Range>,
}
