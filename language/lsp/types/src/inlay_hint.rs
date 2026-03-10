use crate::{
    Command, LSPAny, Location, MarkupContent, Position, Range, StaticRegistrationOptions,
    TextDocumentIdentifier, TextDocumentRegistrationOptions, TextEdit, WorkDoneProgressOptions,
    WorkDoneProgressParams,
};
use serde::{Deserialize, Serialize};

/// Server capabilities for inlay hints.
///
/// @since 3.17.0
#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
pub enum InlayHintServerCapabilities {
    /// Static inlay hint options.
    Options(InlayHintOptions),
    /// Dynamic inlay hint registration options.
    RegistrationOptions(InlayHintRegistrationOptions),
}

/// Inlay hint client capabilities.
///
/// @since 3.17.0
#[derive(Debug, Eq, PartialEq, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InlayHintClientCapabilities {
    /// Whether inlay hints support dynamic registration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamic_registration: Option<bool>,

    /// Indicates which properties a client can resolve lazily on a inlay
    /// hint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolve_support: Option<InlayHintResolveClientCapabilities>,
}

/// Inlay hint options used during static registration.
///
/// @since 3.17.0
#[derive(Debug, Eq, PartialEq, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InlayHintOptions {
    /// Work done progress options.
    #[serde(flatten)]
    pub work_done_progress_options: WorkDoneProgressOptions,

    /// The server provides support to resolve additional
    /// information for an inlay hint item.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolve_provider: Option<bool>,
}

/// Inlay hint options used during static or dynamic registration.
///
/// @since 3.17.0
#[derive(Debug, Eq, PartialEq, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InlayHintRegistrationOptions {
    /// Inlay hint options.
    #[serde(flatten)]
    pub inlay_hint_options: InlayHintOptions,

    /// Text document registration options.
    #[serde(flatten)]
    pub text_document_registration_options: TextDocumentRegistrationOptions,

    /// Static registration options.
    #[serde(flatten)]
    pub static_registration_options: StaticRegistrationOptions,
}

/// A parameter literal used in inlay hint requests.
///
/// @since 3.17.0
#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InlayHintParams {
    /// Work done progress params.
    #[serde(flatten)]
    pub work_done_progress_params: WorkDoneProgressParams,

    /// The text document.
    pub text_document: TextDocumentIdentifier,

    /// The visible document range for which inlay hints should be computed.
    pub range: Range,
}

/// Inlay hint information.
///
/// @since 3.17.0
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InlayHint {
    /// The position of this hint.
    pub position: Position,

    /// The label of this hint. A human readable string or an array of
    /// InlayHintLabelPart label parts.
    ///
    /// *Note* that neither the string nor the label part can be empty.
    pub label: InlayHintLabel,

    /// The kind of this hint. Can be omitted in which case the client
    /// should fall back to a reasonable default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<InlayHintKind>,

    /// Optional text edits that are performed when accepting this inlay hint.
    ///
    /// *Note* that edits are expected to change the document so that the inlay
    /// hint (or its nearest variant) is now part of the document and the inlay
    /// hint itself is now obsolete.
    ///
    /// Depending on the client capability `inlayHint.resolveSupport` clients
    /// might resolve this property late using the resolve request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_edits: Option<Vec<TextEdit>>,

    /// The tooltip text when you hover over this item.
    ///
    /// Depending on the client capability `inlayHint.resolveSupport` clients
    /// might resolve this property late using the resolve request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tooltip: Option<InlayHintTooltip>,

    /// Render padding before the hint.
    ///
    /// Note: Padding should use the editor's background color, not the
    /// background color of the hint itself. That means padding can be used
    /// to visually align/separate an inlay hint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub padding_left: Option<bool>,

    /// Render padding after the hint.
    ///
    /// Note: Padding should use the editor's background color, not the
    /// background color of the hint itself. That means padding can be used
    /// to visually align/separate an inlay hint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub padding_right: Option<bool>,

    /// A data entry field that is preserved on a inlay hint between
    /// a `textDocument/inlayHint` and a `inlayHint/resolve` request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<LSPAny>,
}

/// The label of an inlay hint.
///
/// @since 3.17.0
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum InlayHintLabel {
    /// A simple string label.
    String(String),
    /// An array of label parts for interactive labels.
    LabelParts(Vec<InlayHintLabelPart>),
}

impl From<String> for InlayHintLabel {
    #[inline]
    fn from(from: String) -> Self {
        Self::String(from)
    }
}

impl From<Vec<InlayHintLabelPart>> for InlayHintLabel {
    #[inline]
    fn from(from: Vec<InlayHintLabelPart>) -> Self {
        Self::LabelParts(from)
    }
}

/// The tooltip of an inlay hint.
///
/// @since 3.17.0
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum InlayHintTooltip {
    /// A simple string tooltip.
    String(String),
    /// A markup content tooltip.
    MarkupContent(MarkupContent),
}

impl From<String> for InlayHintTooltip {
    #[inline]
    fn from(from: String) -> Self {
        Self::String(from)
    }
}

impl From<MarkupContent> for InlayHintTooltip {
    #[inline]
    fn from(from: MarkupContent) -> Self {
        Self::MarkupContent(from)
    }
}

/// An inlay hint label part allows for interactive and composite labels
/// of inlay hints.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InlayHintLabelPart {
    /// The value of this label part.
    pub value: String,

    /// The tooltip text when you hover over this label part. Depending on
    /// the client capability `inlayHint.resolveSupport` clients might resolve
    /// this property late using the resolve request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tooltip: Option<InlayHintLabelPartTooltip>,

    /// An optional source code location that represents this
    /// label part.
    ///
    /// The editor will use this location for the hover and for code navigation
    /// features: This part will become a clickable link that resolves to the
    /// definition of the symbol at the given location (not necessarily the
    /// location itself), it shows the hover that shows at the given location,
    /// and it shows a context menu with further code navigation commands.
    ///
    /// Depending on the client capability `inlayHint.resolveSupport` clients
    /// might resolve this property late using the resolve request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<Location>,

    /// An optional command for this label part.
    ///
    /// Depending on the client capability `inlayHint.resolveSupport` clients
    /// might resolve this property late using the resolve request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<Command>,
}

/// The tooltip of an inlay hint label part.
///
/// @since 3.17.0
#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum InlayHintLabelPartTooltip {
    /// A simple string tooltip.
    String(String),
    /// A markup content tooltip.
    MarkupContent(MarkupContent),
}

impl From<String> for InlayHintLabelPartTooltip {
    #[inline]
    fn from(from: String) -> Self {
        Self::String(from)
    }
}

impl From<MarkupContent> for InlayHintLabelPartTooltip {
    #[inline]
    fn from(from: MarkupContent) -> Self {
        Self::MarkupContent(from)
    }
}

/// Inlay hint kinds.
///
/// @since 3.17.0
#[derive(Eq, PartialEq, Copy, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct InlayHintKind(i32);
lsp_enum! {
impl InlayHintKind {
    /// An inlay hint that for a type annotation.
    pub const TYPE: InlayHintKind = InlayHintKind(1);

    /// An inlay hint that is for a parameter.
    pub const PARAMETER: InlayHintKind = InlayHintKind(2);
}
}

/// Inlay hint client capabilities.
///
/// @since 3.17.0
#[derive(Debug, Eq, PartialEq, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InlayHintResolveClientCapabilities {
    /// The properties that a client can resolve lazily.
    pub properties: Vec<String>,
}

/// Client workspace capabilities specific to inlay hints.
///
/// @since 3.17.0
#[derive(Debug, Eq, PartialEq, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InlayHintWorkspaceClientCapabilities {
    /// Whether the client implementation supports a refresh request sent from
    /// the server to the client.
    ///
    /// Note that this event is global and will force the client to refresh all
    /// inlay hints currently shown. It should be used with absolute care and
    /// is useful for situation where a server for example detects a project wide
    /// change that requires such a calculation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_support: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Position;
    use crate::tests::test_serialization;

    #[test]
    fn test_inlay_hint_kind() {
        test_serialization(&InlayHintKind::TYPE, "1");
        test_serialization(&InlayHintKind::PARAMETER, "2");
    }

    #[test]
    fn test_inlay_hint_label_string() {
        let label = InlayHintLabel::String("type".into());
        let serialized = serde_json::to_string(&label).unwrap();
        assert_eq!(serialized, r#""type""#);
    }

    #[test]
    fn test_inlay_hint_label_parts() {
        let label = InlayHintLabel::LabelParts(vec![InlayHintLabelPart {
            value: "int".into(),
            ..Default::default()
        }]);
        let serialized = serde_json::to_string(&label).unwrap();
        assert_eq!(serialized, r#"[{"value":"int"}]"#);
    }

    #[test]
    fn test_inlay_hint() {
        let hint = InlayHint {
            position: Position::new(1, 5),
            label: InlayHintLabel::String(": int".into()),
            kind: Some(InlayHintKind::TYPE),
            text_edits: None,
            tooltip: None,
            padding_left: Some(true),
            padding_right: None,
            data: None,
        };
        let serialized = serde_json::to_string(&hint).unwrap();
        assert_eq!(
            serialized,
            r#"{"position":{"line":1,"character":5},"label":": int","kind":1,"paddingLeft":true}"#
        );
    }

    #[test]
    fn test_inlay_hint_params() {
        use crate::{Range, TextDocumentIdentifier, Uri, WorkDoneProgressParams};
        use std::str::FromStr;

        test_serialization(
            &InlayHintParams {
                work_done_progress_params: WorkDoneProgressParams::default(),
                text_document: TextDocumentIdentifier {
                    uri: Uri::from_str("file:///test.rs").unwrap(),
                },
                range: Range::new(Position::new(0, 0), Position::new(10, 0)),
            },
            r#"{"textDocument":{"uri":"file:///test.rs"},"range":{"start":{"line":0,"character":0},"end":{"line":10,"character":0}}}"#,
        );
    }

    #[test]
    fn test_inlay_hint_client_capabilities() {
        test_serialization(
            &InlayHintClientCapabilities {
                dynamic_registration: Some(true),
                resolve_support: Some(InlayHintResolveClientCapabilities {
                    properties: vec!["tooltip".into(), "textEdits".into()],
                }),
            },
            r#"{"dynamicRegistration":true,"resolveSupport":{"properties":["tooltip","textEdits"]}}"#,
        );
    }
}
