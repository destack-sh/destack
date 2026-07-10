use super::{
    Token, TokenLiteral, TokenType, assert_tokenize_eq_roundtrip, eof, lex_source, token,
    token_without_start,
};

/// Semantic tokens should record whether they begin after a line boundary.
#[test]
fn test_lex_tracks_semantic_token_line_boundaries() {
    let (semantic_tokens, _, _) = lex_source("a b\nc");
    let semantic_tokens: Vec<Token> = semantic_tokens
        .into_iter()
        .map(|token| token_without_start(token.token))
        .collect();

    assert_eq!(
        semantic_tokens,
        vec![
            token(TokenType::Identifier, 1, None).with_on_new_line(true),
            token(TokenType::Identifier, 1, None),
            token(TokenType::Identifier, 1, None).with_on_new_line(true),
            eof(),
        ],
    );
}

/// Unknown control characters should lex as unknown tokens before newlines.
#[test]
fn test_lex_unknown_control_character_before_newline() {
    assert_tokenize_eq_roundtrip!(
        "\u{3}\n",
        token(TokenType::Unknown, 1, None),
        token(TokenType::Newline, 1, None),
    );
}
#[test]
fn test_lex_punctuated_call_like_source() {
    assert_tokenize_eq_roundtrip!(
        "fn main() { println!(\"zebra\"); }\n",
        token(TokenType::Identifier, 2, None),
        token(TokenType::Whitespace, 1, None),
        token(TokenType::Identifier, 4, None),
        token(TokenType::OpenParenthesis, 1, None),
        token(TokenType::CloseParenthesis, 1, None),
        token(TokenType::Whitespace, 1, None),
        token(TokenType::OpenBrace, 1, None),
        token(TokenType::Whitespace, 1, None),
        token(TokenType::Identifier, 7, None),
        token(TokenType::Not, 1, None),
        token(TokenType::OpenParenthesis, 1, None),
        token(
            TokenType::Literal,
            7,
            Some(TokenLiteral::String {
                is_terminated: true,
                has_invalid_escape: false,
            })
        ),
        token(TokenType::CloseParenthesis, 1, None),
        token(TokenType::Semicolon, 1, None),
        token(TokenType::Whitespace, 1, None),
        token(TokenType::CloseBrace, 1, None),
        token(TokenType::Newline, 1, None),
    );
}
