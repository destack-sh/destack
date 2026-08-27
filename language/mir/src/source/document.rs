use destack_core::FxIndexMap;
use destack_serde::Reflect;
use destack_source::{SourceIndex, Span};
use serde::{Deserialize, Serialize};

use crate::source::Token;
use crate::{FieldSpan, FunctionHeaderSpans, TypeDeclarationSpans, TypedValueSpan};

/// One textual MIR document.
#[derive(Clone, Serialize, Deserialize, Reflect)]
pub(crate) struct Document {
    /// Source ranges and anchors keyed by MIR node id.
    pub(crate) index: SourceIndex,
    /// The complete source text.
    pub(crate) text: String,
    /// The complete token stream.
    pub(crate) tokens: Vec<Token>,
    /// Leading comment spans keyed by dense node index.
    pub(crate) leading_comments: Vec<Option<Span>>,
    /// Attribute spans keyed by MIR node id.
    pub(crate) attributes: FxIndexMap<u32, Vec<Span>>,
    /// Declaration keyword spans keyed by MIR node id.
    pub(crate) keywords: FxIndexMap<u32, Span>,
    /// Function parameter spans keyed by MIR node id.
    pub(crate) function_parameters: FxIndexMap<u32, Vec<TypedValueSpan>>,
    /// Function header spans keyed by MIR node id.
    pub(crate) function_headers: FxIndexMap<u32, FunctionHeaderSpans>,
    /// Type field spans keyed by MIR node id.
    pub(crate) type_fields: FxIndexMap<u32, Vec<FieldSpan>>,
    /// Type declaration spans keyed by MIR node id.
    pub(crate) type_declarations: FxIndexMap<u32, TypeDeclarationSpans>,
}

impl Document {
    /// Create one textual MIR document.
    pub(crate) fn new(text: String, tokens: Vec<Token>) -> Self {
        Self {
            index: SourceIndex::new(),
            text,
            tokens,
            leading_comments: Vec::new(),
            attributes: FxIndexMap::default(),
            keywords: FxIndexMap::default(),
            function_parameters: FxIndexMap::default(),
            function_headers: FxIndexMap::default(),
            type_fields: FxIndexMap::default(),
            type_declarations: FxIndexMap::default(),
        }
    }
}
