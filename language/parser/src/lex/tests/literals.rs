use super::*;

/// Decimal bigint literals should lex as integer literals marked bigint.
#[test]
fn test_lex_bigint_literal() {
    assert_tokenize_eq_roundtrip!(
        "0n",
        Token::new(
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
        Token::new(TokenType::Newline, 1, None),
        // true
        Token::new(
            TokenType::Literal,
            4,
            Some(TokenLiteral::Boolean { value: true })
        ),
        Token::new(TokenType::Newline, 1, None),
        // false
        Token::new(
            TokenType::Literal,
            5,
            Some(TokenLiteral::Boolean { value: false })
        ),
        Token::new(TokenType::Newline, 1, None),
        // 'a'
        Token::new(
            TokenType::Literal,
            3,
            Some(TokenLiteral::String {
                is_terminated: true,
                has_invalid_escape: false,
            })
        ),
        Token::new(TokenType::Newline, 1, None),
        // "a"
        Token::new(
            TokenType::Literal,
            3,
            Some(TokenLiteral::String {
                is_terminated: true,
                has_invalid_escape: false,
            })
        ),
        Token::new(TokenType::Newline, 1, None),
        // 1234
        Token::new(
            TokenType::Literal,
            4,
            Some(TokenLiteral::Int {
                base: NumberBase::Decimal,
                is_empty: false,
                is_bigint: false,
            })
        ),
        // 0b101
        Token::new(TokenType::Newline, 1, None),
        Token::new(
            TokenType::Literal,
            5,
            Some(TokenLiteral::Int {
                base: NumberBase::Binary,
                is_empty: false,
                is_bigint: false,
            })
        ),
        Token::new(TokenType::Newline, 1, None),
        // 0xABC
        Token::new(
            TokenType::Literal,
            5,
            Some(TokenLiteral::Int {
                base: NumberBase::Hexadecimal,
                is_empty: false,
                is_bigint: false,
            })
        ),
        Token::new(TokenType::Newline, 1, None),
        // 1.0
        Token::new(
            TokenType::Literal,
            3,
            Some(TokenLiteral::Float {
                base: NumberBase::Decimal,
                is_empty_exponent: false
            })
        ),
        Token::new(TokenType::Newline, 1, None),
        // 1.0e10
        Token::new(
            TokenType::Literal,
            6,
            Some(TokenLiteral::Float {
                base: NumberBase::Decimal,
                is_empty_exponent: false
            })
        ),
        Token::new(TokenType::Newline, 1, None),
        // 2n
        Token::new(
            TokenType::Literal,
            2,
            Some(TokenLiteral::Int {
                base: NumberBase::Decimal,
                is_empty: false,
                is_bigint: true,
            })
        ),
        Token::new(TokenType::Newline, 1, None),
        // 0xABn
        Token::new(
            TokenType::Literal,
            5,
            Some(TokenLiteral::Int {
                base: NumberBase::Hexadecimal,
                is_empty: false,
                is_bigint: true,
            })
        ),
        Token::new(TokenType::Newline, 1, None),
        // 0b101n
        Token::new(
            TokenType::Literal,
            6,
            Some(TokenLiteral::Int {
                base: NumberBase::Binary,
                is_empty: false,
                is_bigint: true,
            })
        ),
        Token::new(TokenType::Newline, 1, None),
        // 0o77n
        Token::new(
            TokenType::Literal,
            5,
            Some(TokenLiteral::Int {
                base: NumberBase::Octal,
                is_empty: false,
                is_bigint: true,
            })
        ),
        Token::new(TokenType::Newline, 1, None),
    );
}

/// Decimal literals may use a dot followed directly by an exponent.
#[test]
fn test_lex_decimal_literal_with_dot_exponent() {
    assert_tokenize_eq_roundtrip!(
        "1.e1 0.e60",
        Token::new(
            TokenType::Literal,
            4,
            Some(TokenLiteral::Float {
                base: NumberBase::Decimal,
                is_empty_exponent: false
            })
        ),
        Token::new(TokenType::Whitespace, 1, None),
        Token::new(
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
        Token::new(
            TokenType::Literal,
            5,
            Some(TokenLiteral::Int {
                base: NumberBase::Binary,
                is_empty: false,
                is_bigint: false,
            })
        ),
        Token::new(TokenType::Whitespace, 1, None),
        Token::new(
            TokenType::Literal,
            4,
            Some(TokenLiteral::Int {
                base: NumberBase::Octal,
                is_empty: false,
                is_bigint: false,
            })
        ),
        Token::new(TokenType::Whitespace, 1, None),
        Token::new(
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
