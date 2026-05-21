use super::*;

/// Destack punctuation should lex to the expected operator and delimiter tokens.
#[test]
fn test_lex_basic_destack_punctuation() {
    assert_tokenize_eq_roundtrip!(
        "a..b => c->d x _ : ? ! @ ~",
        // a
        Token::new(TokenType::Identifier, 1, None),
        // ..
        Token::new(TokenType::Range, 2, None),
        // b
        Token::new(TokenType::Identifier, 1, None),
        // (space)
        Token::new(TokenType::Whitespace, 1, None),
        // =>
        Token::new(TokenType::ArrowWide, 2, None),
        // (space)
        Token::new(TokenType::Whitespace, 1, None),
        // c
        Token::new(TokenType::Identifier, 1, None),
        // ->
        Token::new(TokenType::Arrow, 2, None),
        // d
        Token::new(TokenType::Identifier, 1, None),
        // (space)
        Token::new(TokenType::Whitespace, 1, None),
        // x
        Token::new(TokenType::Identifier, 1, None),
        // (space)
        Token::new(TokenType::Whitespace, 1, None),
        // _
        Token::new(TokenType::Identifier, 1, None),
        // (space)
        Token::new(TokenType::Whitespace, 1, None),
        // :
        Token::new(TokenType::Colon, 1, None),
        // (space)
        Token::new(TokenType::Whitespace, 1, None),
        // ?
        Token::new(TokenType::Maybe, 1, None),
        // (space)
        Token::new(TokenType::Whitespace, 1, None),
        // !
        Token::new(TokenType::Not, 1, None),
        // (space)
        Token::new(TokenType::Whitespace, 1, None),
        // @
        Token::new(TokenType::At, 1, None),
        // (space)
        Token::new(TokenType::Whitespace, 1, None),
        // ~
        Token::new(TokenType::ElementwiseNot, 1, None),
    );
}

/// Range punctuation should only produce range tokens in Destack mode.
#[test]
fn test_lex_range_tokens_in_destack_only() {
    let (semantic_tokens, _) = lex_source_tokens("a..b ..= c", LanguageType::default());
    assert_eq!(
        semantic_tokens,
        vec![
            Token::new(TokenType::Identifier, 1, None),
            Token::new(TokenType::Range, 2, None),
            Token::new(TokenType::Identifier, 1, None),
            Token::new(TokenType::RangeInclusive, 3, None),
            Token::new(TokenType::Identifier, 1, None),
            Token::end(),
        ],
    );

    let (semantic_tokens, _) = lex_source_tokens("a..b ..= c", LanguageType::TypeScript);
    assert_eq!(
        semantic_tokens,
        vec![
            Token::new(TokenType::Identifier, 1, None),
            Token::new(TokenType::Dot, 1, None),
            Token::new(TokenType::Dot, 1, None),
            Token::new(TokenType::Identifier, 1, None),
            Token::new(TokenType::Dot, 1, None),
            Token::new(TokenType::Dot, 1, None),
            Token::new(TokenType::Assign, 1, None),
            Token::new(TokenType::Identifier, 1, None),
            Token::end(),
        ],
    );
}

/// Comparison and equality operators should lex to their specific token kinds.
#[test]
fn test_lex_comparisons_and_equals() {
    assert_tokenize_eq_roundtrip!(
        "a==b != c <= d >= e < f > g",
        Token::new(TokenType::Identifier, 1, None),
        Token::new(TokenType::Equal, 2, None),
        Token::new(TokenType::Identifier, 1, None),
        Token::new(TokenType::Whitespace, 1, None),
        Token::new(TokenType::NotEqual, 2, None),
        Token::new(TokenType::Whitespace, 1, None),
        Token::new(TokenType::Identifier, 1, None),
        Token::new(TokenType::Whitespace, 1, None),
        Token::new(TokenType::LessThanOrEqual, 2, None),
        Token::new(TokenType::Whitespace, 1, None),
        Token::new(TokenType::Identifier, 1, None),
        Token::new(TokenType::Whitespace, 1, None),
        Token::new(TokenType::GreaterThanOrEqual, 2, None),
        Token::new(TokenType::Whitespace, 1, None),
        Token::new(TokenType::Identifier, 1, None),
        Token::new(TokenType::Whitespace, 1, None),
        Token::new(TokenType::LessThan, 1, None),
        Token::new(TokenType::Whitespace, 1, None),
        Token::new(TokenType::Identifier, 1, None),
        Token::new(TokenType::Whitespace, 1, None),
        Token::new(TokenType::GreaterThan, 1, None),
        Token::new(TokenType::Whitespace, 1, None),
        Token::new(TokenType::Identifier, 1, None),
    );
}

/// Logical assignment operators should lex as combined operator tokens.
#[test]
fn test_lex_logical_assignments() {
    assert_tokenize_eq_roundtrip!(
        "a&&=b ||= c",
        Token::new(TokenType::Identifier, 1, None),
        Token::new(TokenType::LogicalAndAssign, 3, None),
        Token::new(TokenType::Identifier, 1, None),
        Token::new(TokenType::Whitespace, 1, None),
        Token::new(TokenType::LogicalOrAssign, 3, None),
        Token::new(TokenType::Whitespace, 1, None),
        Token::new(TokenType::Identifier, 1, None),
    );
}

/// Coalesce assignment should lex as one combined operator token.
#[test]
fn test_lex_coalesce_assignment() {
    assert_tokenize_eq_roundtrip!(
        "a??=b",
        Token::new(TokenType::Identifier, 1, None),
        Token::new(TokenType::CoalesceAssign, 3, None),
        Token::new(TokenType::Identifier, 1, None),
    );
}
