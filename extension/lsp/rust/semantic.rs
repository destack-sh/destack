//! Semantic token LSP.

use dyst_ast::{
    Definition, NodeId, NodeTree, NodeVisitor, SemanticTokenIndex, SemanticType, TokenSpan,
};
use dyst_package::DocumentBody;
use dyst_source::{Source, Uri};
use tower_lsp_server::lsp_types as lsp;

use crate::{
    DestackLanguageServer, Workspace, byte_to_utf16_position, range_to_byte_span,
    token_length_utf16,
};

/// All semantic token types supported by the LSP server.
pub const SEMANTIC_TOKEN_TYPES: [lsp::SemanticTokenType; 23] = [
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
    lsp::SemanticTokenType::EVENT,
    lsp::SemanticTokenType::FUNCTION,
    lsp::SemanticTokenType::METHOD,
    lsp::SemanticTokenType::MACRO,
    lsp::SemanticTokenType::KEYWORD,
    lsp::SemanticTokenType::MODIFIER,
    lsp::SemanticTokenType::COMMENT,
    lsp::SemanticTokenType::STRING,
    lsp::SemanticTokenType::NUMBER,
    lsp::SemanticTokenType::REGEXP,
    lsp::SemanticTokenType::OPERATOR,
    lsp::SemanticTokenType::DECORATOR,
];

/// Build the legend advertised to the client.
pub fn legend() -> lsp::SemanticTokensLegend {
    lsp::SemanticTokensLegend {
        token_types: SEMANTIC_TOKEN_TYPES.to_vec(),
        token_modifiers: Vec::new(),
    }
}

/// Collect semantic tokens from source, optionally filtered by range.
pub fn collect_semantic_tokens(
    source: &Source,
    tokens: &Vec<TokenSpan>,
    tree: &NodeTree,
    root_definition_id: NodeId<Definition>,
    range: Option<&lsp::Range>,
) -> Option<Vec<lsp::SemanticToken>> {
    if tokens.is_empty() {
        return Some(Vec::new());
    }

    // build semantic type mapping from AST if available
    let mut semantic_index = SemanticTokenIndex::from_tokens(source, tokens);
    let definition = tree.get(root_definition_id);
    semantic_index.visit_definition(tree, root_definition_id, definition);

    // convert range to byte span for filtering
    let byte_span = if let Some(range) = range {
        match range_to_byte_span(source, range) {
            Some(span) => Some(span),
            None => return None,
        }
    } else {
        None
    };

    encode_semantic_tokens(source, tokens, &semantic_index.semantic_types, byte_span)
}

/// Encode tokens into LSP semantic token format.
fn encode_semantic_tokens(
    source: &Source,
    tokens: &[TokenSpan],
    semantic_types: &[SemanticType],
    byte_span: Option<(u32, u32)>,
) -> Option<Vec<lsp::SemanticToken>> {
    debug_assert_eq!(tokens.len(), semantic_types.len());

    let mut encoded: Vec<lsp::SemanticToken> = Vec::new();
    let mut previous_line = 0u32;
    let mut previous_column = 0u32;
    let mut is_first = true;

    for (token, semantic) in tokens.iter().zip(semantic_types.iter()) {
        let mapped_type = match get_semantic_type_index(*semantic) {
            Some(index) => index,
            None => continue,
        };

        // skip tokens outside the requested range
        if let Some((start, end)) = byte_span
            && (token.span.end <= start || token.span.start >= end)
        {
            continue;
        }

        // convert byte span to UTF-16 position and length
        let (line, column) = byte_to_utf16_position(source, token.span.start)?;
        let length = token_length_utf16(source, token);

        // compute deltas for LSP encoding
        let delta_line = if is_first {
            line
        } else {
            line.saturating_sub(previous_line)
        };
        let delta_start = if is_first || delta_line > 0 {
            column
        } else {
            column.saturating_sub(previous_column)
        };

        encoded.push(lsp::SemanticToken {
            delta_line,
            delta_start,
            length,
            token_type: mapped_type,
            token_modifiers_bitset: 0,
        });

        previous_line = line;
        previous_column = column;
        is_first = false;
    }

    Some(encoded)
}

/// Get the index of a SemanticType in our token types array.
fn get_semantic_type_index(semantic_type: SemanticType) -> Option<u32> {
    let lsp_type = match semantic_type {
        SemanticType::Keyword => lsp::SemanticTokenType::KEYWORD,
        SemanticType::Identifier => lsp::SemanticTokenType::VARIABLE,
        SemanticType::LiteralNumbery => lsp::SemanticTokenType::NUMBER,
        SemanticType::LiteralStringy => lsp::SemanticTokenType::STRING,
        SemanticType::Operator => lsp::SemanticTokenType::OPERATOR,
        SemanticType::Whitespace => lsp::SemanticTokenType::OPERATOR,
        SemanticType::Parenthesis => lsp::SemanticTokenType::OPERATOR,
        SemanticType::Symbol => lsp::SemanticTokenType::OPERATOR,
        SemanticType::Doc => lsp::SemanticTokenType::COMMENT,
        SemanticType::Comment => lsp::SemanticTokenType::COMMENT,
        SemanticType::Modifier => lsp::SemanticTokenType::MODIFIER,
        SemanticType::Macro => lsp::SemanticTokenType::MACRO,
        SemanticType::Type => lsp::SemanticTokenType::TYPE,
        SemanticType::Function => lsp::SemanticTokenType::FUNCTION,
        SemanticType::Parameter => lsp::SemanticTokenType::PARAMETER,
        SemanticType::Argument => lsp::SemanticTokenType::PARAMETER,
        SemanticType::Variable => lsp::SemanticTokenType::VARIABLE,
    };
    SEMANTIC_TOKEN_TYPES
        .iter()
        .position(|candidate| candidate == &lsp_type)
        .map(|idx| idx as u32)
}

impl DestackLanguageServer {
    /// Compute semantic tokens for a document.
    pub fn get_semantic_tokens_full(
        &self,
        workspace: &Workspace,
        uri: &Uri,
    ) -> Option<Vec<lsp::SemanticToken>> {
        let doc = workspace.get_document(uri)?;
        match &doc.body {
            DocumentBody::Text {
                source,
                all_tokens,
                ast,
                root_definition_id: module_id,
                ..
            } => collect_semantic_tokens(source, all_tokens, ast, *module_id, None),
            _ => None,
        }
    }

    /// Compute semantic tokens for a document within a range.
    pub fn get_semantic_tokens_range(
        &self,
        workspace: &Workspace,
        uri: &Uri,
        range: &lsp::Range,
    ) -> Option<Vec<lsp::SemanticToken>> {
        let doc = workspace.get_document(uri)?;
        match &doc.body {
            DocumentBody::Text {
                source,
                all_tokens,
                ast,
                root_definition_id: module_id,
                ..
            } => collect_semantic_tokens(source, all_tokens, ast, *module_id, Some(range)),
            _ => None,
        }
    }
}
