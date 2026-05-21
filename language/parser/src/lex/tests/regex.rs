use super::*;

/// String literals should lex after import from even without separating whitespace.
#[test]
fn test_lex_string_literal_after_import_from_without_space() {
    // lex import with adjacent string literal
    assert_tokenize_eq_roundtrip!(
        "import Foo from'./bar'",
        Token::new(TokenType::Identifier, 6, None),
        Token::new(TokenType::Whitespace, 1, None),
        Token::new(TokenType::Identifier, 3, None),
        Token::new(TokenType::Whitespace, 1, None),
        Token::new(TokenType::Identifier, 4, None),
        Token::new(
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
            Token::new(TokenType::Identifier, 6, None),
            Token::new(TokenType::OpenParenthesis, 1, None),
            Token::new(TokenType::Identifier, 1, None),
            Token::new(TokenType::CloseParenthesis, 1, None),
            Token::new(TokenType::Divide, 1, None),
            Token::new(TokenType::Identifier, 3, None),
            Token::new(TokenType::Divide, 1, None),
            Token::end(),
        ],
    );

    assert_eq!(
        side_tokens,
        vec![
            Token::new(TokenType::Whitespace, 1, None),
            Token::new(TokenType::Whitespace, 1, None),
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
            Token::new(TokenType::Identifier, 5, None),
            Token::new(TokenType::OpenParenthesis, 1, None),
            Token::new(TokenType::Identifier, 1, None),
            Token::new(TokenType::CloseParenthesis, 1, None),
            Token::new(TokenType::Divide, 1, None),
            Token::new(TokenType::Identifier, 3, None),
            Token::new(TokenType::Divide, 1, None),
            Token::end(),
        ],
    );

    assert_eq!(
        side_tokens,
        vec![
            Token::new(TokenType::Whitespace, 1, None),
            Token::new(TokenType::Whitespace, 1, None),
        ],
    );
}

/// Regex-like source should remain raw slash and identifier tokens.
#[test]
fn test_lex_regex_like_source_stays_raw() {
    assert_tokenize_eq_roundtrip!(
        "/abc/ /foo/gi",
        Token::new(TokenType::Divide, 1, None),
        Token::new(TokenType::Identifier, 3, None),
        Token::new(TokenType::Divide, 1, None),
        Token::new(TokenType::Whitespace, 1, None),
        Token::new(TokenType::Divide, 1, None),
        Token::new(TokenType::Identifier, 3, None),
        Token::new(TokenType::Divide, 1, None),
        Token::new(TokenType::Identifier, 2, None),
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
            Token::new(TokenType::Identifier, 5, None),
            Token::new(TokenType::Identifier, 1, None),
            Token::new(TokenType::Assign, 1, None),
            Token::new(TokenType::OpenParenthesis, 1, None),
            Token::new(TokenType::CloseParenthesis, 1, None),
            Token::new(TokenType::ArrowWide, 2, None),
            Token::new(TokenType::Divide, 1, None),
            Token::new(TokenType::Identifier, 3, None),
            Token::new(TokenType::Divide, 1, None),
            Token::new(TokenType::Dot, 1, None),
            Token::new(TokenType::Identifier, 4, None),
            Token::new(TokenType::OpenParenthesis, 1, None),
            Token::new(TokenType::Identifier, 5, None),
            Token::new(TokenType::CloseParenthesis, 1, None),
            Token::end(),
        ],
    );

    assert_eq!(
        side_tokens,
        vec![
            Token::new(TokenType::Whitespace, 1, None),
            Token::new(TokenType::Whitespace, 1, None),
            Token::new(TokenType::Whitespace, 1, None),
            Token::new(TokenType::Whitespace, 1, None),
            Token::new(TokenType::Whitespace, 1, None),
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
        Token::new(TokenType::Newline, 1, None),
        Token::new(TokenType::LessThan, 1, None),
        Token::new(TokenType::Identifier, 5, None),
        Token::new(TokenType::Newline, 1, None),
        Token::new(TokenType::Whitespace, 4, None),
        Token::new(TokenType::Identifier, 4, None),
        Token::new(TokenType::Assign, 1, None),
        Token::new(TokenType::Divide, 1, None),
        Token::new(TokenType::Identifier, 4, None),
        Token::new(TokenType::Divide, 1, None),
        Token::new(TokenType::Identifier, 1, None),
        Token::new(TokenType::Newline, 1, None),
        Token::new(TokenType::Whitespace, 4, None),
        Token::new(TokenType::LineComment, 10, None),
        Token::new(TokenType::Newline, 1, None),
        Token::new(TokenType::Divide, 1, None),
        Token::new(TokenType::GreaterThan, 1, None),
    );
}
