use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

/// One checked declaration signature.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
#[serde(rename_all = "camelCase")]
pub struct Signature {
    /// The formatted UTF-8 text.
    pub text: String,
    /// The declared name interval when one exists.
    pub name: Option<TextRange>,
}

/// One UTF-8 byte interval inside text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
#[serde(rename_all = "camelCase")]
pub struct TextRange {
    /// The inclusive byte offset.
    pub start: u32,
    /// The exclusive byte offset.
    pub end: u32,
}
