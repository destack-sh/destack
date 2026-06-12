use super::{LanguageType, TokenType, assert_tokenize_eq_roundtrip, eof, lex_source_tokens, token};

/// Destack punctuation should lex to the expected operator and delimiter tokens.
#[test]
fn test_lex_basic_destack_punctuation() {
    assert_tokenize_eq_roundtrip!(
        "a..b => c->d x _ : ? ! @ ~",
        // a
        token(TokenType::Identifier, 1, None),
        // ..
        token(TokenType::Range, 2, None),
        // b
        token(TokenType::Identifier, 1, None),
        // (space)
        token(TokenType::Whitespace, 1, None),
        // =>
        token(TokenType::ArrowWide, 2, None),
        // (space)
        token(TokenType::Whitespace, 1, None),
        // c
        token(TokenType::Identifier, 1, None),
        // ->
        token(TokenType::Arrow, 2, None),
        // d
        token(TokenType::Identifier, 1, None),
        // (space)
        token(TokenType::Whitespace, 1, None),
        // x
        token(TokenType::Identifier, 1, None),
        // (space)
        token(TokenType::Whitespace, 1, None),
        // _
        token(TokenType::Identifier, 1, None),
        // (space)
        token(TokenType::Whitespace, 1, None),
        // :
        token(TokenType::Colon, 1, None),
        // (space)
        token(TokenType::Whitespace, 1, None),
        // ?
        token(TokenType::Maybe, 1, None),
        // (space)
        token(TokenType::Whitespace, 1, None),
        // !
        token(TokenType::Not, 1, None),
        // (space)
        token(TokenType::Whitespace, 1, None),
        // @
        token(TokenType::At, 1, None),
        // (space)
        token(TokenType::Whitespace, 1, None),
        // ~
        token(TokenType::ElementwiseNot, 1, None),
    );
}

/// Range punctuation should only produce range tokens in Destack mode.
#[test]
fn test_lex_range_tokens_in_destack_only() {
    let (semantic_tokens, _) = lex_source_tokens("a..b ..= c", LanguageType::default());
    assert_eq!(
        semantic_tokens,
        vec![
            token(TokenType::Identifier, 1, None),
            token(TokenType::Range, 2, None),
            token(TokenType::Identifier, 1, None),
            token(TokenType::RangeInclusive, 3, None),
            token(TokenType::Identifier, 1, None),
            eof(),
        ],
    );

    let (semantic_tokens, _) = lex_source_tokens("a..b ..= c", LanguageType::TypeScript);
    assert_eq!(
        semantic_tokens,
        vec![
            token(TokenType::Identifier, 1, None),
            token(TokenType::Dot, 1, None),
            token(TokenType::Dot, 1, None),
            token(TokenType::Identifier, 1, None),
            token(TokenType::Dot, 1, None),
            token(TokenType::Dot, 1, None),
            token(TokenType::Assign, 1, None),
            token(TokenType::Identifier, 1, None),
            eof(),
        ],
    );
}

/// Comparison and equality operators should lex to their specific token kinds.
#[test]
fn test_lex_comparisons_and_equals() {
    assert_tokenize_eq_roundtrip!(
        "a==b != c <= d >= e < f > g",
        token(TokenType::Identifier, 1, None),
        token(TokenType::Equal, 2, None),
        token(TokenType::Identifier, 1, None),
        token(TokenType::Whitespace, 1, None),
        token(TokenType::NotEqual, 2, None),
        token(TokenType::Whitespace, 1, None),
        token(TokenType::Identifier, 1, None),
        token(TokenType::Whitespace, 1, None),
        token(TokenType::LessThanOrEqual, 2, None),
        token(TokenType::Whitespace, 1, None),
        token(TokenType::Identifier, 1, None),
        token(TokenType::Whitespace, 1, None),
        token(TokenType::GreaterThanOrEqual, 2, None),
        token(TokenType::Whitespace, 1, None),
        token(TokenType::Identifier, 1, None),
        token(TokenType::Whitespace, 1, None),
        token(TokenType::LessThan, 1, None),
        token(TokenType::Whitespace, 1, None),
        token(TokenType::Identifier, 1, None),
        token(TokenType::Whitespace, 1, None),
        token(TokenType::GreaterThan, 1, None),
        token(TokenType::Whitespace, 1, None),
        token(TokenType::Identifier, 1, None),
    );
}

/// Logical assignment operators should lex as combined operator tokens.
#[test]
fn test_lex_logical_assignments() {
    assert_tokenize_eq_roundtrip!(
        "a&&=b ||= c",
        token(TokenType::Identifier, 1, None),
        token(TokenType::LogicalAndAssign, 3, None),
        token(TokenType::Identifier, 1, None),
        token(TokenType::Whitespace, 1, None),
        token(TokenType::LogicalOrAssign, 3, None),
        token(TokenType::Whitespace, 1, None),
        token(TokenType::Identifier, 1, None),
    );
}

/// Coalesce assignment should lex as one combined operator token.
#[test]
fn test_lex_coalesce_assignment() {
    assert_tokenize_eq_roundtrip!(
        "a??=b",
        token(TokenType::Identifier, 1, None),
        token(TokenType::CoalesceAssign, 3, None),
        token(TokenType::Identifier, 1, None),
    );
}
