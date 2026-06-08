// generated bridge target, do not edit

use destack_bridge_language as bridge;

use napi_derive::napi;

/// External file id crossing bridge boundaries.
#[derive(Debug)]
#[napi(object)]
pub struct FileId {
    /// Canonical lowercase hex file id.
    pub id: String,
}

impl FileId {
    /// Convert one bridge value into one NAPI value.
    pub(crate) fn from_bridge(value: bridge::FileId) -> Self {
        Self { id: value.id }
    }
}

/// External file content id crossing bridge boundaries.
#[derive(Debug)]
#[napi(object)]
pub struct FileContentId {
    /// Canonical lowercase hex file content id.
    pub id: String,
}

impl FileContentId {
    /// Convert one bridge value into one NAPI value.
    pub(crate) fn from_bridge(value: bridge::FileContentId) -> Self {
        Self { id: value.id }
    }
}

/// Full file content crossing bridge boundaries.
#[derive(Debug)]
#[napi(object)]
pub struct FileContent {
    /// Payload variant label.
    pub kind: String,
    /// Text content.
    pub text_content: Option<String>,
    /// Binary content.
    pub binary_content: Option<Vec<u8>>,
}

impl FileContent {
    /// Convert one bridge payload enum into one NAPI payload enum.
    pub(crate) fn from_bridge(value: bridge::FileContent) -> Self {
        match value {
            bridge::FileContent::Text { content } => Self {
                kind: "text".to_string(),
                text_content: Some(content),
                binary_content: None,
            },
            bridge::FileContent::Binary { content } => Self {
                kind: "binary".to_string(),
                binary_content: Some(content),
                text_content: None,
            },
        }
    }
}
