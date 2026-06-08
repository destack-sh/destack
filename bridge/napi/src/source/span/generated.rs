// generated bridge target, do not edit

use destack_bridge_language as bridge;

use napi_derive::napi;

use crate::FileId;

/// Source byte span crossing bridge boundaries.
#[derive(Debug)]
#[napi(object)]
pub struct Span {
    /// File containing this span.
    pub file: FileId,
    /// Inclusive start byte offset.
    pub start: u32,
    /// Exclusive end byte offset.
    pub end: u32,
}

impl Span {
    /// Convert one bridge value into one NAPI value.
    pub(crate) fn from_bridge(value: bridge::Span) -> Self {
        Self {
            file: FileId::from_bridge(value.file),
            start: value.start,
            end: value.end,
        }
    }
}

/// Source span with a display label.
#[derive(Debug)]
#[napi(object)]
pub struct LabeledSpan {
    /// Source span.
    pub span: Span,
    /// Display label.
    pub label: String,
}
