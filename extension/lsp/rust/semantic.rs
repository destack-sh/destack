//! SemanticTokens.

use crate::protocol::types as lsp;
use dyst_language_ast::SemanticType;

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
