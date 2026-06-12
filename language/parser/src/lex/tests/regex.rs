use super::{
    LanguageType, TokenLiteral, TokenType, assert_tokenize_eq_roundtrip, eof, lex_source_tokens,
    token,
};

/// String literals should lex after import from even without separating whitespace.
#[test]
fn test_lex_string_literal_after_import_from_without_space() {
    // lex import with adjacent string literal
    assert_tokenize_eq_roundtrip!(
        "import Foo from'./bar'",
        token(TokenType::Identifier, 6, None),
        token(TokenType::Whitespace, 1, None),
        token(TokenType::Identifier, 3, None),
        token(TokenType::Whitespace, 1, None),
        token(TokenType::Identifier, 4, None),
        token(
            TokenType::Literal,
            7,
            Some(TokenLiteral::String {
                is_terminated: true,
                has_invalid_escape: false,
            })
        ),
    );
}

/// Slash after a switch header should remain divide punctuation.
#[test]
fn test_lex_divide_after_switch_header() {
    let (semantic_tokens, side_tokens) =
        lex_source_tokens("switch (x) /foo/", LanguageType::JavaScript);

    assert_eq!(
        semantic_tokens,
        vec![
            token(TokenType::Identifier, 6, None),
            token(TokenType::OpenParenthesis, 1, None),
            token(TokenType::Identifier, 1, None),
            token(TokenType::CloseParenthesis, 1, None),
            token(TokenType::Divide, 1, None),
            token(TokenType::Identifier, 3, None),
            token(TokenType::Divide, 1, None),
            eof(),
        ],
    );

    assert_eq!(
        side_tokens,
        vec![
            token(TokenType::Whitespace, 1, None),
            token(TokenType::Whitespace, 1, None),
        ],
    );
}

/// Slash after a catch header should remain divide punctuation.
#[test]
fn test_lex_divide_after_catch_header() {
    let (semantic_tokens, side_tokens) =
        lex_source_tokens("catch (e) /foo/", LanguageType::JavaScript);

    assert_eq!(
        semantic_tokens,
        vec![
            token(TokenType::Identifier, 5, None),
            token(TokenType::OpenParenthesis, 1, None),
            token(TokenType::Identifier, 1, None),
            token(TokenType::CloseParenthesis, 1, None),
            token(TokenType::Divide, 1, None),
            token(TokenType::Identifier, 3, None),
            token(TokenType::Divide, 1, None),
            eof(),
        ],
    );

    assert_eq!(
        side_tokens,
        vec![
            token(TokenType::Whitespace, 1, None),
            token(TokenType::Whitespace, 1, None),
        ],
    );
}

/// Regex-like source should remain raw slash and identifier tokens.
#[test]
fn test_lex_regex_like_source_stays_raw() {
    assert_tokenize_eq_roundtrip!(
        "/abc/ /foo/gi",
        token(TokenType::Divide, 1, None),
        token(TokenType::Identifier, 3, None),
        token(TokenType::Divide, 1, None),
        token(TokenType::Whitespace, 1, None),
        token(TokenType::Divide, 1, None),
        token(TokenType::Identifier, 3, None),
        token(TokenType::Divide, 1, None),
        token(TokenType::Identifier, 2, None),
    );
}

/// Regex-like expression source should remain raw slash and identifier tokens.
#[test]
fn test_lex_regex_like_source_in_expression_context_stays_raw() {
    let (semantic_tokens, side_tokens) = lex_source_tokens(
        "const f = () => /foo/.test(value)",
        LanguageType::JavaScript,
    );

    assert_eq!(
        semantic_tokens,
        vec![
            token(TokenType::Identifier, 5, None),
            token(TokenType::Identifier, 1, None),
            token(TokenType::Assign, 1, None),
            token(TokenType::OpenParenthesis, 1, None),
            token(TokenType::CloseParenthesis, 1, None),
            token(TokenType::ArrowWide, 2, None),
            token(TokenType::Divide, 1, None),
            token(TokenType::Identifier, 3, None),
            token(TokenType::Divide, 1, None),
            token(TokenType::Dot, 1, None),
            token(TokenType::Identifier, 4, None),
            token(TokenType::OpenParenthesis, 1, None),
            token(TokenType::Identifier, 5, None),
            token(TokenType::CloseParenthesis, 1, None),
            eof(),
        ],
    );

    assert_eq!(
        side_tokens,
        vec![
            token(TokenType::Whitespace, 1, None),
            token(TokenType::Whitespace, 1, None),
            token(TokenType::Whitespace, 1, None),
            token(TokenType::Whitespace, 1, None),
            token(TokenType::Whitespace, 1, None),
        ],
    );
}

/// Regex-like tree attribute source should remain raw slash and identifier tokens.
#[test]
fn test_lex_regex_like_tree_attribute_stays_raw() {
    assert_tokenize_eq_roundtrip!(
        r"
<input
    type=/text/i
    // comment
/>",
        token(TokenType::Newline, 1, None),
        token(TokenType::LessThan, 1, None),
        token(TokenType::Identifier, 5, None),
        token(TokenType::Newline, 1, None),
        token(TokenType::Whitespace, 4, None),
        token(TokenType::Identifier, 4, None),
        token(TokenType::Assign, 1, None),
        token(TokenType::Divide, 1, None),
        token(TokenType::Identifier, 4, None),
        token(TokenType::Divide, 1, None),
        token(TokenType::Identifier, 1, None),
        token(TokenType::Newline, 1, None),
        token(TokenType::Whitespace, 4, None),
        token(TokenType::LineComment, 10, None),
        token(TokenType::Newline, 1, None),
        token(TokenType::Divide, 1, None),
        token(TokenType::GreaterThan, 1, None),
    );
}
