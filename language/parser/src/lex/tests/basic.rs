use super::{
    LanguageType, Token, TokenLiteral, TokenType, assert_tokenize_eq_roundtrip, eof, lex_source,
    lex_source_tokens, token, token_without_start,
};

/// Semantic tokens should record whether they begin after a line boundary.
#[test]
fn test_lex_tracks_semantic_token_line_boundaries() {
    let (semantic_tokens, _, _) = lex_source("a b\nc", LanguageType::default());
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

/// Hashbang lines should lex as side line comments in JavaScript and TypeScript.
#[test]
fn test_lex_hashbang_as_line_comment() {
    let source = "#!/usr/bin/env node\nimport value from 'pkg';\n";
    let expected_semantic_tokens = vec![
        token(TokenType::Identifier, 6, None),
        token(TokenType::Identifier, 5, None),
        token(TokenType::Identifier, 4, None),
        token(
            TokenType::Literal,
            5,
            Some(TokenLiteral::String {
                is_terminated: true,
                has_invalid_escape: false,
            }),
        ),
        token(TokenType::Semicolon, 1, None),
        eof(),
    ];
    let expected_side_tokens = vec![
        token(TokenType::LineComment, 19, None),
        token(TokenType::Newline, 1, None),
        token(TokenType::Whitespace, 1, None),
        token(TokenType::Whitespace, 1, None),
        token(TokenType::Whitespace, 1, None),
        token(TokenType::Newline, 1, None),
    ];

    // typed source
    let (semantic_tokens, side_tokens) = lex_source_tokens(source, LanguageType::TypeScript);
    assert_eq!(semantic_tokens, expected_semantic_tokens);
    assert_eq!(side_tokens, expected_side_tokens);

    // untyped source
    let (semantic_tokens, side_tokens) = lex_source_tokens(source, LanguageType::JavaScript);
    assert_eq!(semantic_tokens, expected_semantic_tokens);
    assert_eq!(side_tokens, expected_side_tokens);
}

/// Punctuated call-like source should roundtrip through ordinary tokenization.
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
