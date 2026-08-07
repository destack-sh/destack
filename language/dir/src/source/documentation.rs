use destack_core::StringId;
use destack_serde::Reflect;
use destack_source::Span;
use serde::{Deserialize, Serialize};

use crate::{GenericParameter, LocalNodeId, LocalNodeIdAny, Parameter};

/// Parsed documentation attached to one DIR node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Documentation {
    /// The complete authored documentation span.
    pub span: Span,
    /// The Markdown before block tags.
    pub markdown: StringId,
    /// The parsed block tags in source order.
    pub tags: Vec<DocumentationTag>,
}

/// One parsed documentation block tag.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum DocumentationTag {
    /// Documentation for one exact value parameter.
    Parameter {
        /// The documented parameter.
        parameter: LocalNodeId<Parameter>,
        /// The parameter Markdown.
        markdown: StringId,
    },
    /// Documentation for one exact generic parameter.
    TypeParameter {
        /// The documented generic parameter.
        parameter: LocalNodeId<GenericParameter>,
        /// The parameter Markdown.
        markdown: StringId,
    },
    /// One usage example.
    Example {
        /// The example Markdown.
        markdown: StringId,
    },
}

impl DocumentationTag {
    /// Return whether this tag contains an example.
    #[inline]
    pub fn is_example(self) -> bool {
        matches!(self, Self::Example { .. })
    }

    /// Return the exact node documented by this tag.
    #[inline]
    pub fn target(self) -> Option<LocalNodeIdAny> {
        match self {
            Self::Parameter { parameter, .. } => Some(parameter.into_any()),
            Self::TypeParameter { parameter, .. } => Some(parameter.into_any()),
            Self::Example { .. } => None,
        }
    }

    /// Return this tag's Markdown.
    #[inline]
    pub fn markdown(self) -> StringId {
        match self {
            Self::Parameter { markdown, .. }
            | Self::TypeParameter { markdown, .. }
            | Self::Example { markdown, .. } => markdown,
        }
    }
}
