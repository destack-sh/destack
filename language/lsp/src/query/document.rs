use tspp_lsp_server::jsonrpc;
use tspp_lsp_types as lsp;
use tspp_query as query;

use super::{Document, DocumentSet, IntoLsp};
use crate::server::internal_error;

/// Semantic token types in legend order.
const SEMANTIC_TOKEN_TYPES: [lsp::SemanticTokenType; 15] = [
    lsp::SemanticTokenType::NAMESPACE,
    lsp::SemanticTokenType::TYPE,
    lsp::SemanticTokenType::CLASS,
    lsp::SemanticTokenType::ENUM,
    lsp::SemanticTokenType::INTERFACE,
    lsp::SemanticTokenType::STRUCT,
    lsp::SemanticTokenType::TYPE_PARAMETER,
    lsp::SemanticTokenType::PARAMETER,
    lsp::SemanticTokenType::VARIABLE,
    lsp::SemanticTokenType::PROPERTY,
    lsp::SemanticTokenType::ENUM_MEMBER,
    lsp::SemanticTokenType::FUNCTION,
    lsp::SemanticTokenType::METHOD,
    lsp::SemanticTokenType::COMMENT,
    lsp::SemanticTokenType::DECORATOR,
];

/// Semantic token modifiers in legend order.
const SEMANTIC_TOKEN_MODIFIERS: [lsp::SemanticTokenModifier; 9] = [
    lsp::SemanticTokenModifier::DECLARATION,
    lsp::SemanticTokenModifier::READONLY,
    lsp::SemanticTokenModifier::STATIC,
    lsp::SemanticTokenModifier::DEPRECATED,
    lsp::SemanticTokenModifier::ABSTRACT,
    lsp::SemanticTokenModifier::ASYNC,
    lsp::SemanticTokenModifier::MODIFICATION,
    lsp::SemanticTokenModifier::DOCUMENTATION,
    lsp::SemanticTokenModifier::DEFAULT_LIBRARY,
];

/// One delta encoded LSP semantic token stream.
pub(crate) struct SemanticTokenStream {
    /// Tokens emitted so far.
    tokens: Vec<lsp::SemanticToken>,
    /// The previous token line.
    previous_line: u32,
    /// The previous token character.
    previous_character: u32,
}

impl SemanticTokenStream {
    /// Create a semantic token stream.
    fn new(capacity: usize) -> Self {
        Self {
            tokens: Vec::with_capacity(capacity),
            previous_line: 0,
            previous_character: 0,
        }
    }

    /// Build the legend advertised to the client.
    pub(crate) fn legend() -> lsp::SemanticTokensLegend {
        lsp::SemanticTokensLegend {
            token_types: SEMANTIC_TOKEN_TYPES.to_vec(),
            token_modifiers: SEMANTIC_TOKEN_MODIFIERS.to_vec(),
        }
    }

    /// Return the legend index for one semantic token type.
    fn token_type(token_type: query::SemanticTokenType) -> u32 {
        match token_type {
            query::SemanticTokenType::Namespace => 0,
            query::SemanticTokenType::Type => 1,
            query::SemanticTokenType::Class => 2,
            query::SemanticTokenType::Enum => 3,
            query::SemanticTokenType::Interface => 4,
            query::SemanticTokenType::Struct => 5,
            query::SemanticTokenType::TypeParameter => 6,
            query::SemanticTokenType::Parameter => 7,
            query::SemanticTokenType::Variable => 8,
            query::SemanticTokenType::Property => 9,
            query::SemanticTokenType::EnumMember => 10,
            query::SemanticTokenType::Function => 11,
            query::SemanticTokenType::Method => 12,
            query::SemanticTokenType::Comment => 13,
            query::SemanticTokenType::Decorator => 14,
            // LSP has no label token type
            query::SemanticTokenType::Label => 8,
        }
    }

    /// Push one semantic token segment.
    fn push(
        &mut self,
        line: u32,
        character: u32,
        length: u32,
        token_type: u32,
        modifiers: u32,
    ) -> jsonrpc::Result<()> {
        // compute the position relative to the previous token
        let Some(delta_line) = line.checked_sub(self.previous_line) else {
            return Err(internal_error("semantic tokens are not in source order"));
        };
        let delta_start = if delta_line == 0 {
            character
                .checked_sub(self.previous_character)
                .ok_or_else(|| internal_error("semantic tokens are not in source order"))?
        } else {
            character
        };

        self.tokens.push(lsp::SemanticToken {
            delta_line,
            delta_start,
            length,
            token_type,
            token_modifiers_bitset: modifiers,
        });

        // retain the absolute position for the next token
        self.previous_line = line;
        self.previous_character = character;

        Ok(())
    }

    /// Finish the semantic token stream.
    fn finish(self) -> Vec<lsp::SemanticToken> {
        self.tokens
    }
}

impl Document {
    /// Build delta encoded LSP semantic tokens.
    pub(crate) fn semantic_tokens(
        &self,
        tokens: &[query::SemanticToken],
    ) -> jsonrpc::Result<Vec<lsp::SemanticToken>> {
        let mut stream = SemanticTokenStream::new(tokens.len());

        // emit tokens in document order
        for token in tokens {
            if token.span.file != self.id() {
                return Err(internal_error(format!(
                    "semantic token {:?} does not belong to source file {:?}",
                    token.span,
                    self.id()
                )));
            }

            // resolve token positions
            let start = self.position(token.span.start)?;
            let end = self.position(token.span.end)?;

            // cache type and modifiers for split segments
            let token_type = SemanticTokenStream::token_type(token.token_type);
            let modifiers = token.modifiers.bits();

            // emit single line tokens
            if start.line == end.line {
                let Some(length) = end.character.checked_sub(start.character) else {
                    return Err(internal_error(format!(
                        "semantic token has reversed span: {:?}",
                        token.span
                    )));
                };
                if length == 0 {
                    return Err(internal_error(format!(
                        "semantic token has an empty span: {:?}",
                        token.span
                    )));
                }
                stream.push(start.line, start.character, length, token_type, modifiers)?;
                continue;
            }

            // emit first line segment
            let line_span = self.file().get_line_span(start.line).ok_or_else(|| {
                internal_error(format!(
                    "semantic token starts outside source file: {:?}",
                    token.span
                ))
            })?;
            let length = self.utf16_length(token.span.start, line_span.end)?;
            if length > 0 {
                stream.push(start.line, start.character, length, token_type, modifiers)?;
            }

            // emit middle line segments
            for line in (start.line + 1)..end.line {
                let line_span = self.file().get_line_span(line).ok_or_else(|| {
                    internal_error(format!(
                        "semantic token crosses a missing source line: {:?}",
                        token.span
                    ))
                })?;
                let length = self.utf16_length(line_span.start, line_span.end)?;
                if length > 0 {
                    stream.push(line, 0, length, token_type, modifiers)?;
                }
            }

            // emit last line segment
            if end.character > 0 {
                stream.push(end.line, 0, end.character, token_type, modifiers)?;
            }
        }

        Ok(stream.finish())
    }

    /// Return the UTF-16 length of one byte range.
    fn utf16_length(&self, start: u32, end: u32) -> jsonrpc::Result<u32> {
        if start > end || end > self.file().len {
            return Err(internal_error(format!(
                "source range {start}..{end} is outside file {:?} with length {}",
                self.id(),
                self.file().len
            )));
        }
        let Some(slice) = self.file().text().get(start as usize..end as usize) else {
            return Err(internal_error(format!(
                "source range {start}..{end} is not on character boundaries in file {:?}",
                self.id()
            )));
        };

        Ok(slice.encode_utf16().count() as u32)
    }
}

#[allow(deprecated)]
impl IntoLsp for query::SymbolKind {
    type Lsp = lsp::SymbolKind;

    /// Convert this symbol kind.
    fn into_lsp(self) -> lsp::SymbolKind {
        match self {
            query::SymbolKind::AssociatedConst | query::SymbolKind::Constant => {
                lsp::SymbolKind::CONSTANT
            }
            query::SymbolKind::AssociatedType => lsp::SymbolKind::TYPE_PARAMETER,
            query::SymbolKind::Class => lsp::SymbolKind::CLASS,
            query::SymbolKind::Constructor => lsp::SymbolKind::CONSTRUCTOR,
            query::SymbolKind::Enum => lsp::SymbolKind::ENUM,
            query::SymbolKind::EnumMember => lsp::SymbolKind::ENUM_MEMBER,
            query::SymbolKind::Extension => lsp::SymbolKind::CLASS,
            query::SymbolKind::Field => lsp::SymbolKind::FIELD,
            query::SymbolKind::Function => lsp::SymbolKind::FUNCTION,
            query::SymbolKind::Interface | query::SymbolKind::NewtypeInterface => {
                lsp::SymbolKind::INTERFACE
            }
            query::SymbolKind::Method => lsp::SymbolKind::METHOD,
            query::SymbolKind::Module => lsp::SymbolKind::MODULE,
            query::SymbolKind::Namespace => lsp::SymbolKind::NAMESPACE,
            query::SymbolKind::Newtype => lsp::SymbolKind::STRUCT,
            query::SymbolKind::Property => lsp::SymbolKind::PROPERTY,
            query::SymbolKind::Struct => lsp::SymbolKind::STRUCT,
            query::SymbolKind::TypeAlias => lsp::SymbolKind::TYPE_PARAMETER,
            query::SymbolKind::Variable => lsp::SymbolKind::VARIABLE,
        }
    }
}

#[allow(deprecated)]
impl Document {
    /// Encode one outline symbol as an LSP document symbol.
    pub(crate) fn document_symbol(
        &self,
        outline: &query::OutlineSymbol,
    ) -> jsonrpc::Result<lsp::DocumentSymbol> {
        let (range, selection_range) = self.ranges(outline.range, outline.selection_range)?;
        let kind = outline.kind.into_lsp();

        // build child symbols
        let children = if outline.children.is_empty() {
            None
        } else {
            let children = outline
                .children
                .iter()
                .map(|child| self.document_symbol(child))
                .collect::<jsonrpc::Result<Vec<_>>>()?;

            Some(children)
        };

        Ok(lsp::DocumentSymbol {
            name: outline.name.clone(),
            detail: outline.detail.clone(),
            kind,
            tags: None,
            deprecated: None,
            range,
            selection_range,
            children,
        })
    }
}

impl Document {
    /// Build one LSP folding range from a query result.
    pub(crate) fn folding_range(&self, range: query::FoldingRange) -> lsp::FoldingRange {
        lsp::FoldingRange {
            start_line: range.start_line,
            start_character: range.start_character,
            end_line: range.end_line,
            end_character: range.end_character,
            kind: range.kind.map(|kind| match kind {
                query::FoldingRangeKind::Comment => lsp::FoldingRangeKind::Comment,
                query::FoldingRangeKind::Imports => lsp::FoldingRangeKind::Imports,
                query::FoldingRangeKind::Region => lsp::FoldingRangeKind::Region,
            }),
            collapsed_text: range.collapsed_text,
        }
    }
}

impl Document {
    /// Encode one selection range as an LSP selection range.
    pub(crate) fn selection_range(
        &self,
        range: query::SelectionRange,
    ) -> jsonrpc::Result<lsp::SelectionRange> {
        let lsp_range = self.range(range.range)?;
        let parent = range
            .parent
            .map(|parent| self.selection_range(*parent).map(Box::new))
            .transpose()?;

        Ok(lsp::SelectionRange {
            range: lsp_range,
            parent,
        })
    }
}

impl Document {
    /// Encode one document highlight as an LSP document highlight.
    pub(crate) fn highlight(
        &self,
        highlight: &query::Highlight,
    ) -> jsonrpc::Result<lsp::DocumentHighlight> {
        let range = self.range(highlight.range)?;
        let kind = match highlight.kind {
            query::HighlightKind::Text => lsp::DocumentHighlightKind::TEXT,
            query::HighlightKind::Read => lsp::DocumentHighlightKind::READ,
            query::HighlightKind::Write => lsp::DocumentHighlightKind::WRITE,
        };

        Ok(lsp::DocumentHighlight {
            range,
            kind: Some(kind),
        })
    }
}

impl DocumentSet {
    /// Encode one document link as an LSP document link.
    pub(crate) fn link(&self, link: &query::Link) -> jsonrpc::Result<lsp::DocumentLink> {
        let range = self.document(link.range.file)?.range(link.range)?;
        let target = self.document(link.target)?.uri(self.project)?;

        Ok(lsp::DocumentLink {
            range,
            target: Some(target),
            tooltip: None,
            data: None,
        })
    }
}
