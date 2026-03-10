use destack_source::Uri;

use crate::TargetId;

use super::{OutputContent, OutputId, OutputScope, OutputVersion};

/// Generated output from code generation or linking.
#[derive(Debug, Clone)]
pub struct Output {
    /// The output id.
    pub id: OutputId,
    /// The output version.
    pub version: OutputVersion,
    /// The output scope.
    pub scope: OutputScope,
    /// The target id.
    pub target: TargetId,
    /// The output URI.
    pub uri: Uri,
    /// The generated content.
    pub content: OutputContent,
    /// Related source output.
    pub source: Option<OutputId>,
}
