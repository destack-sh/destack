//! SemanticTokens.

use crate::protocol::types as lsp;
use dyst_language_ast::{BlockFormat, NodeVisitor, Parser, SemanticTokenMap, SemanticType};
use dyst_language_session::Session;
use dyst_language_source::{Source, SourceId};
use dyst_language_token::{TokenSpan, TokenType};

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

/// Get semantic tokens for the given text.
pub fn get_semantic_tokens(text: &str) -> Vec<lsp::SemanticToken> {
    // parse
    let session = Session::new();
    let source = Source::from_string(SourceId::new(0), "<semantic>".to_string(), text.to_string());
    let mut parser = Parser::from_source(&source, &session);
    let statements = parser.with_recovery(
        parser.mark(),
        |parser| parser.eat_block_body(BlockFormat::Implicit),
        Vec::new(),
        TokenType::End,
    );
    parser.process_annotations();

    // index
    let mut combined_tokens: Vec<TokenSpan> = parser
        .tokens
        .into_iter()
        .chain(parser.trivia_tokens)
        .collect::<Vec<_>>();
    combined_tokens.sort_by_key(|token| token.span.start);
    let mut builder = SemanticTokenMap::from_tokens(&source, &combined_tokens);
    for statement in statements {
        builder.visit_statement(&parser.tree, statement, parser.tree.get(statement));
    }

    get_lsp_semantic_tokens(&source, builder.tokens, &builder.semantic_types)
}

/// Map our SemanticTokenMap to lsp::SemanticToken.
pub(crate) fn get_lsp_semantic_tokens(
    source: &Source,
    tokens: &[TokenSpan],
    semantic_types: &[SemanticType],
) -> Vec<lsp::SemanticToken> {
    let mut lsp_tokens: Vec<lsp::SemanticToken> = Vec::new();

    let mut prev_line = 0;
    let mut prev_start = 0;

    for (i, token_span) in tokens.iter().enumerate() {
        // skip whitespace tokens for LSP
        if semantic_types[i] == SemanticType::Whitespace {
            continue;
        }

        let Some((start_line, start_col)) = source.get_position(token_span.span.start) else {
            // skip tokens with invalid positions
            continue;
        };

        let delta_line = start_line - prev_line;
        let delta_start = if delta_line == 0 {
            start_col - prev_start
        } else {
            start_col
        };

        // find token type index in SEMANTIC_TOKEN_TYPES
        let lsp_semantic_type = get_lsp_semantic_type(semantic_types[i]);
        let token_type = SEMANTIC_TOKEN_TYPES
            .iter()
            .position(|t| *t == lsp_semantic_type)
            .unwrap_or(0) as u32;

        lsp_tokens.push(lsp::SemanticToken {
            delta_line,
            delta_start,
            length: token_span.token.len,
            token_type,
            token_modifiers_bitset: 0,
        });

        prev_line = start_line;
        prev_start = start_col;
    }

    lsp_tokens
}

/// Map our SemanticType to a lsp::SemanticTokenType.
pub(crate) fn get_lsp_semantic_type(semantic_type: SemanticType) -> lsp::SemanticTokenType {
    match semantic_type {
        // lexical
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
        // semantic
        SemanticType::Modifier => lsp::SemanticTokenType::MODIFIER,
        SemanticType::Macro => lsp::SemanticTokenType::MACRO,
        SemanticType::Type => lsp::SemanticTokenType::TYPE,
        SemanticType::Function => lsp::SemanticTokenType::FUNCTION,
        SemanticType::Parameter => lsp::SemanticTokenType::PARAMETER,
        SemanticType::Argument => lsp::SemanticTokenType::PARAMETER,
        SemanticType::Variable => lsp::SemanticTokenType::VARIABLE,
    }
}
