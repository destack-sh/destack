use super::{NumberBase, TokenLiteral, TokenType, assert_tokenize_eq_roundtrip, token};

/// Decimal bigint literals should lex as integer literals marked bigint.
#[test]
fn test_lex_bigint_literal() {
    assert_tokenize_eq_roundtrip!(
        "0n",
        token(
            TokenType::Literal,
            2,
            Some(TokenLiteral::Int {
                base: NumberBase::Decimal,
                is_empty: false,
                is_bigint: true,
            })
        ),
    );
}

/// Scalar literal forms should lex to their literal token payloads.
#[test]
fn test_lex_scalar_literal_forms() {
    assert_tokenize_eq_roundtrip!(
        r####"
true
false
'a'
"a"
1234
0b101
0xABC
1.0
1.0e10
2n
0xABn
0b101n
0o77n
"####,
        token(TokenType::Newline, 1, None),
        // true
        token(
            TokenType::Literal,
            4,
            Some(TokenLiteral::Boolean { value: true })
        ),
        token(TokenType::Newline, 1, None),
        // false
        token(
            TokenType::Literal,
            5,
            Some(TokenLiteral::Boolean { value: false })
        ),
        token(TokenType::Newline, 1, None),
        // 'a'
        token(
            TokenType::Literal,
            3,
            Some(TokenLiteral::String {
                is_terminated: true,
                has_invalid_escape: false,
            })
        ),
        token(TokenType::Newline, 1, None),
        // "a"
        token(
            TokenType::Literal,
            3,
            Some(TokenLiteral::String {
                is_terminated: true,
                has_invalid_escape: false,
            })
        ),
        token(TokenType::Newline, 1, None),
        // 1234
        token(
            TokenType::Literal,
            4,
            Some(TokenLiteral::Int {
                base: NumberBase::Decimal,
                is_empty: false,
                is_bigint: false,
            })
        ),
        // 0b101
        token(TokenType::Newline, 1, None),
        token(
            TokenType::Literal,
            5,
            Some(TokenLiteral::Int {
                base: NumberBase::Binary,
                is_empty: false,
                is_bigint: false,
            })
        ),
        token(TokenType::Newline, 1, None),
        // 0xABC
        token(
            TokenType::Literal,
            5,
            Some(TokenLiteral::Int {
                base: NumberBase::Hexadecimal,
                is_empty: false,
                is_bigint: false,
            })
        ),
        token(TokenType::Newline, 1, None),
        // 1.0
        token(
            TokenType::Literal,
            3,
            Some(TokenLiteral::Float {
                base: NumberBase::Decimal,
                is_empty_exponent: false
            })
        ),
        token(TokenType::Newline, 1, None),
        // 1.0e10
        token(
            TokenType::Literal,
            6,
            Some(TokenLiteral::Float {
                base: NumberBase::Decimal,
                is_empty_exponent: false
            })
        ),
        token(TokenType::Newline, 1, None),
        // 2n
        token(
            TokenType::Literal,
            2,
            Some(TokenLiteral::Int {
                base: NumberBase::Decimal,
                is_empty: false,
                is_bigint: true,
            })
        ),
        token(TokenType::Newline, 1, None),
        // 0xABn
        token(
            TokenType::Literal,
            5,
            Some(TokenLiteral::Int {
                base: NumberBase::Hexadecimal,
                is_empty: false,
                is_bigint: true,
            })
        ),
        token(TokenType::Newline, 1, None),
        // 0b101n
        token(
            TokenType::Literal,
            6,
            Some(TokenLiteral::Int {
                base: NumberBase::Binary,
                is_empty: false,
                is_bigint: true,
            })
        ),
        token(TokenType::Newline, 1, None),
        // 0o77n
        token(
            TokenType::Literal,
            5,
            Some(TokenLiteral::Int {
                base: NumberBase::Octal,
                is_empty: false,
                is_bigint: true,
            })
        ),
        token(TokenType::Newline, 1, None),
    );
}

/// Decimal literals may use a dot followed directly by an exponent.
#[test]
fn test_lex_decimal_literal_with_dot_exponent() {
    assert_tokenize_eq_roundtrip!(
        "1.e1 0.e60",
        token(
            TokenType::Literal,
            4,
            Some(TokenLiteral::Float {
                base: NumberBase::Decimal,
                is_empty_exponent: false
            })
        ),
        token(TokenType::Whitespace, 1, None),
        token(
            TokenType::Literal,
            5,
            Some(TokenLiteral::Float {
                base: NumberBase::Decimal,
                is_empty_exponent: false
            })
        ),
    );
}

/// Uppercase radix prefixes should lex as integer literal prefixes.
#[test]
fn test_lex_uppercase_radix_prefixes() {
    assert_tokenize_eq_roundtrip!(
        "0B101 0O77 0XFF",
        token(
            TokenType::Literal,
            5,
            Some(TokenLiteral::Int {
                base: NumberBase::Binary,
                is_empty: false,
                is_bigint: false,
            })
        ),
        token(TokenType::Whitespace, 1, None),
        token(
            TokenType::Literal,
            4,
            Some(TokenLiteral::Int {
                base: NumberBase::Octal,
                is_empty: false,
                is_bigint: false,
            })
        ),
        token(TokenType::Whitespace, 1, None),
        token(
            TokenType::Literal,
            4,
            Some(TokenLiteral::Int {
                base: NumberBase::Hexadecimal,
                is_empty: false,
                is_bigint: false,
            })
        ),
    );
}
