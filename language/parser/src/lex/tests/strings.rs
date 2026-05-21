use super::*;

/// Single-quoted one-character strings should lex as terminated string literals.
#[test]
fn test_lex_single_quoted_single_character_strings() {
    assert_tokenize_eq_roundtrip!(
        "'a' ' ' '\\n'",
        Token::new(
            TokenType::Literal,
            3,
            Some(TokenLiteral::String {
                is_terminated: true,
                has_invalid_escape: false,
            })
        ),
        Token::new(TokenType::Whitespace, 1, None),
        Token::new(
            TokenType::Literal,
            3,
            Some(TokenLiteral::String {
                is_terminated: true,
                has_invalid_escape: false,
            })
        ),
        Token::new(TokenType::Whitespace, 1, None),
        Token::new(
            TokenType::Literal,
            4,
            Some(TokenLiteral::String {
                is_terminated: true,
                has_invalid_escape: false,
            })
        ),
    );
}

/// Single-quoted string forms should lex as terminated string literals.
#[test]
fn test_lex_single_quoted_strings() {
    assert_tokenize_eq_roundtrip!(
        "'ab' 'multi word' '../ivm/catch.ts'",
        Token::new(
            TokenType::Literal,
            4,
            Some(TokenLiteral::String {
                is_terminated: true,
                has_invalid_escape: false,
            })
        ),
        Token::new(TokenType::Whitespace, 1, None),
        Token::new(
            TokenType::Literal,
            12,
            Some(TokenLiteral::String {
                is_terminated: true,
                has_invalid_escape: false,
            })
        ),
        Token::new(TokenType::Whitespace, 1, None),
        Token::new(
            TokenType::Literal,
            17,
            Some(TokenLiteral::String {
                is_terminated: true,
                has_invalid_escape: false,
            })
        ),
    );
}

/// String unicode escapes should allow long leading zero sequences.
#[test]
fn test_lex_string_unicode_escape_with_long_leading_zeros() {
    assert_tokenize_eq_roundtrip!(
        "\"\\u{00000000034}\"",
        Token::new(
            TokenType::Literal,
            17,
            Some(TokenLiteral::String {
                is_terminated: true,
                has_invalid_escape: false,
            })
        ),
    );
}

/// Template strings without interpolation should lex as complete template tokens.
#[test]
fn test_lex_template_strings() {
    assert_tokenize_eq_roundtrip!(
        "`plain` `two words`",
        Token::new(TokenType::TemplateString, 7, None),
        Token::new(TokenType::Whitespace, 1, None),
        Token::new(TokenType::TemplateString, 11, None),
    );
}

/// Tagged template strings should lex the tag and template separately.
#[test]
fn test_lex_tagged_template_strings() {
    assert_tokenize_eq_roundtrip!(
        "tag`item`",
        Token::new(TokenType::Identifier, 3, None),
        Token::new(TokenType::TemplateString, 6, None),
    );
}

/// Tagged templates should keep legacy octal escape text inside template tokens.
#[test]
fn test_lex_tagged_template_with_legacy_octal_escape() {
    assert_tokenize_eq_roundtrip!(
        r"String.raw`\1`",
        Token::new(TokenType::Identifier, 6, None),
        Token::new(TokenType::Dot, 1, None),
        Token::new(TokenType::Identifier, 3, None),
        Token::new(TokenType::TemplateString, 4, None),
    );
}

/// Template strings with multiple interpolations should emit start, middle, and end tokens.
#[test]
fn test_lex_template_strings_with_interpolation_mixed() {
    assert_tokenize_eq_roundtrip!(
        "`a ${b} c ${d} e`",
        Token::new(TokenType::TemplateStringStart, 5, None),
        Token::new(TokenType::Identifier, 1, None),
        Token::new(TokenType::TemplateStringMiddle, 6, None),
        Token::new(TokenType::Identifier, 1, None),
        Token::new(TokenType::TemplateStringEnd, 4, None),
    );
}
/// Template strings with one interpolation should emit start and end tokens.
#[test]
fn test_lex_template_strings_with_interpolation() {
    assert_tokenize_eq_roundtrip!(
        "`${stmt}`",
        Token::new(TokenType::TemplateStringStart, 3, None),
        Token::new(TokenType::Identifier, 4, None),
        Token::new(TokenType::TemplateStringEnd, 2, None),
    );
}

/// Adjacent template interpolations should emit middle tokens without text gaps.
#[test]
fn test_lex_template_strings_with_interpolation_adjacent() {
    assert_tokenize_eq_roundtrip!(
        "`${a}${b}${c}`",
        Token::new(TokenType::TemplateStringStart, 3, None),
        Token::new(TokenType::Identifier, 1, None),
        Token::new(TokenType::TemplateStringMiddle, 3, None),
        Token::new(TokenType::Identifier, 1, None),
        Token::new(TokenType::TemplateStringMiddle, 3, None),
        Token::new(TokenType::Identifier, 1, None),
        Token::new(TokenType::TemplateStringEnd, 2, None),
    );
}

/// Escaped interpolation prefixes should stay inside template text.
#[test]
fn test_lex_template_strings_with_escaped_interpolation_prefix() {
    assert_tokenize_eq_roundtrip!(
        r"`\${${value}}`",
        Token::new(TokenType::TemplateStringStart, 6, None),
        Token::new(TokenType::Identifier, 5, None),
        Token::new(TokenType::TemplateStringEnd, 3, None),
    );
}

/// Tagged templates with multiple interpolations should emit tag, start, middle, and end tokens.
#[test]
fn test_lex_tagged_template_strings_with_interpolation_mixed() {
    assert_tokenize_eq_roundtrip!(
        "tag`sum ${lhs} + ${rhs}`",
        Token::new(TokenType::Identifier, 3, None),
        Token::new(TokenType::TemplateStringStart, 7, None),
        Token::new(TokenType::Identifier, 3, None),
        Token::new(TokenType::TemplateStringMiddle, 6, None),
        Token::new(TokenType::Identifier, 3, None),
        Token::new(TokenType::TemplateStringEnd, 2, None),
    );
}

/// Tagged templates should track nested template interpolation braces.
#[test]
fn test_lex_tagged_template_strings_with_interpolation_nested() {
    assert_tokenize_eq_roundtrip!(
        "tag`sum ${text + {`${nested}`}}`",
        Token::new(TokenType::Identifier, 3, None),
        Token::new(TokenType::TemplateStringStart, 7, None),
        Token::new(TokenType::Identifier, 4, None),
        Token::new(TokenType::Whitespace, 1, None),
        Token::new(TokenType::Add, 1, None),
        Token::new(TokenType::Whitespace, 1, None),
        Token::new(TokenType::OpenBrace, 1, None),
        Token::new(TokenType::TemplateStringStart, 3, None),
        Token::new(TokenType::Identifier, 6, None),
        Token::new(TokenType::TemplateStringEnd, 2, None),
        Token::new(TokenType::CloseBrace, 1, None),
        Token::new(TokenType::TemplateStringEnd, 2, None),
    );
}

/// Unterminated single-quoted strings at EOF should lex without hanging.
#[test]
fn test_lex_unterminated_single_quote_eof() {
    let (semantic_tokens, side_tokens) = lex_source_tokens("'", LanguageType::default());

    assert_eq!(
        semantic_tokens,
        vec![
            Token::new(
                TokenType::Literal,
                1,
                Some(TokenLiteral::String {
                    is_terminated: false,
                    has_invalid_escape: false,
                }),
            ),
            Token::end(),
        ],
    );

    assert_eq!(side_tokens, vec![]);
}

/// Unterminated single-quoted strings ending in an escape should mark the escape invalid.
#[test]
fn test_lex_unterminated_single_quote_with_escape_eof() {
    let (semantic_tokens, side_tokens) = lex_source_tokens(r"'\x", LanguageType::default());

    assert_eq!(
        semantic_tokens,
        vec![
            Token::new(
                TokenType::Literal,
                3,
                Some(TokenLiteral::String {
                    is_terminated: false,
                    has_invalid_escape: true,
                }),
            ),
            Token::end(),
        ],
    );

    assert_eq!(side_tokens, vec![]);
}

/// Unterminated single-quoted strings ending in a backslash should mark the escape invalid.
#[test]
fn test_lex_unterminated_single_quote_with_trailing_slash_eof() {
    let (semantic_tokens, side_tokens) = lex_source_tokens("'\\", LanguageType::default());

    assert_eq!(
        semantic_tokens,
        vec![
            Token::new(
                TokenType::Literal,
                2,
                Some(TokenLiteral::String {
                    is_terminated: false,
                    has_invalid_escape: true,
                }),
            ),
            Token::end(),
        ],
    );

    assert_eq!(side_tokens, vec![]);
}

/// Unterminated single-quoted strings with partial hex escapes should mark the escape invalid.
#[test]
fn test_lex_unterminated_single_quote_hex_escape() {
    let (semantic_tokens, side_tokens) = lex_source_tokens(r"'\x1", LanguageType::default());

    assert_eq!(
        semantic_tokens,
        vec![
            Token::new(
                TokenType::Literal,
                4,
                Some(TokenLiteral::String {
                    is_terminated: false,
                    has_invalid_escape: true,
                }),
            ),
            Token::end(),
        ],
    );

    assert_eq!(side_tokens, vec![]);
}

/// Unterminated single-quoted strings with octal escapes should mark the escape invalid.
#[test]
fn test_lex_unterminated_single_quote_octal_escape() {
    let (semantic_tokens, side_tokens) = lex_source_tokens(r"'\03", LanguageType::default());

    assert_eq!(
        semantic_tokens,
        vec![
            Token::new(
                TokenType::Literal,
                4,
                Some(TokenLiteral::String {
                    is_terminated: false,
                    has_invalid_escape: true,
                }),
            ),
            Token::end(),
        ],
    );

    assert_eq!(side_tokens, vec![]);
}

/// Single-quoted strings should terminate lexing before raw newlines.
#[test]
fn test_lex_single_quote_before_newline_is_unterminated() {
    let (semantic_tokens, side_tokens) = lex_source_tokens("'\n", LanguageType::default());

    assert_eq!(
        semantic_tokens,
        vec![
            Token::new(
                TokenType::Literal,
                1,
                Some(TokenLiteral::String {
                    is_terminated: false,
                    has_invalid_escape: false,
                }),
            ),
            Token::end(),
        ],
    );

    assert_eq!(side_tokens, vec![Token::new(TokenType::Newline, 1, None)]);
}

/// Double-quoted strings should terminate lexing before raw newlines.
#[test]
fn test_lex_double_quote_with_newline_is_unterminated() {
    let (semantic_tokens, side_tokens) =
        lex_source_tokens("\"hello\nworld\"", LanguageType::default());

    assert_eq!(
        semantic_tokens,
        vec![
            Token::new(
                TokenType::Literal,
                6,
                Some(TokenLiteral::String {
                    is_terminated: false,
                    has_invalid_escape: false,
                }),
            ),
            Token::new(TokenType::Identifier, 5, None),
            Token::new(
                TokenType::Literal,
                1,
                Some(TokenLiteral::String {
                    is_terminated: false,
                    has_invalid_escape: false,
                }),
            ),
            Token::end(),
        ],
    );

    assert_eq!(side_tokens, vec![Token::new(TokenType::Newline, 1, None)]);
}

/// Legacy escaped digit sequences should be invalid across language modes.
#[test]
fn test_lex_string_with_legacy_escaped_digit_is_invalid_across_languages() {
    assert_legacy_string_escape_is_invalid_across_languages("\"+4\\9 99 999 99\"");
}

/// Legacy octal escape sequences should be invalid across language modes.
#[test]
fn test_lex_string_with_legacy_octal_escape_is_invalid_across_languages() {
    assert_legacy_string_escape_is_invalid_across_languages("\"+5\\5 (99) 99999-9999\"");
}

/// Unterminated single-quoted strings inside parentheses should lex without hanging.
#[test]
fn test_lex_unterminated_single_quote_inside_parentheses() {
    let (semantic_tokens, side_tokens) = lex_source_tokens("(')", LanguageType::default());

    assert_eq!(
        semantic_tokens,
        vec![
            Token::new(TokenType::OpenParenthesis, 1, None),
            Token::new(
                TokenType::Literal,
                2,
                Some(TokenLiteral::String {
                    is_terminated: false,
                    has_invalid_escape: false,
                }),
            ),
            Token::end(),
        ],
    );

    assert_eq!(side_tokens, vec![]);
}

/// Unterminated single-quoted strings should be marked as unterminated.
#[test]
fn test_lex_unterminated_single_quote_is_marked() {
    let (semantic_tokens, side_tokens) = lex_source_tokens("'abc", LanguageType::default());

    assert_eq!(
        semantic_tokens,
        vec![
            Token::new(
                TokenType::Literal,
                4,
                Some(TokenLiteral::String {
                    is_terminated: false,
                    has_invalid_escape: false,
                }),
            ),
            Token::end(),
        ],
    );

    assert_eq!(side_tokens, vec![]);
}
