use super::*;

/// Braced unicode escapes should lex as identifier tokens.
#[test]
fn test_lex_unicode_escape_braced_single() {
    // \u{41} = 6 chars -> identifier "A"
    assert_tokenize_eq_roundtrip!(r"\u{41}", Token::new(TokenType::Identifier, 6, None),);
}

/// Braced unicode escapes with suffix characters should lex as one identifier token.
#[test]
fn test_lex_unicode_escape_braced_with_suffix() {
    // \u{41}BC = 8 chars -> identifier "ABC"
    assert_tokenize_eq_roundtrip!(r"\u{41}BC", Token::new(TokenType::Identifier, 8, None),);
}

/// ES5 unicode escapes should lex as identifier tokens.
#[test]
fn test_lex_unicode_escape_es5_style() {
    // \u0041 = 6 chars -> identifier "A"
    assert_tokenize_eq_roundtrip!(r"\u0041", Token::new(TokenType::Identifier, 6, None),);
}

/// ES5 unicode escapes with suffix characters should lex as one identifier token.
#[test]
fn test_lex_unicode_escape_es5_with_suffix() {
    // \u0041BC = 8 chars -> identifier "ABC"
    assert_tokenize_eq_roundtrip!(r"\u0041BC", Token::new(TokenType::Identifier, 8, None),);
}

/// Multiple unicode escapes should lex as one identifier token.
#[test]
fn test_lex_unicode_escape_multiple() {
    // \u{41}\u{42}\u{43} = 18 chars -> identifier "ABC"
    assert_tokenize_eq_roundtrip!(
        r"\u{41}\u{42}\u{43}",
        Token::new(TokenType::Identifier, 18, None),
    );
}

/// Unicode escapes should lex as identifiers inside declarations.
#[test]
fn test_lex_unicode_escape_in_let() {
    // let \u{41}BC; = "let" (3) + " " (1) + "\u{41}BC" (8) + ";" (1)
    assert_tokenize_eq_roundtrip!(
        r"let \u{41}BC;",
        Token::new(TokenType::Identifier, 3, None),
        Token::new(TokenType::Whitespace, 1, None),
        Token::new(TokenType::Identifier, 8, None),
        Token::new(TokenType::Semicolon, 1, None),
    );
}

/// Identifiers may continue with unicode escape suffixes.
#[test]
fn test_lex_identifier_with_unicode_escape_suffix() {
    // AB\u{43} = 8 chars -> identifier "ABC"
    assert_tokenize_eq_roundtrip!(r"AB\u{43}", Token::new(TokenType::Identifier, 8, None),);
}

/// Invalid unicode escapes with two digits should split into identifier, unknown, and identifier tokens.
#[test]
fn test_lex_invalid_unicode_escape_too_short() {
    // a\u11z: "a" (ident), "\u11" becomes Unknown, "z" (ident)
    // The backslash consumes as much as it can as Unknown
    assert_tokenize_eq_roundtrip!(
        r"a\u11z",
        Token::new(TokenType::Identifier, 1, None), // "a"
        Token::new(TokenType::Unknown, 4, None),    // "\u11"
        Token::new(TokenType::Identifier, 1, None), // "z"
    );
}

/// Invalid unicode escapes with one digit should split into identifier, unknown, and identifier tokens.
#[test]
fn test_lex_invalid_unicode_escape_one_digit() {
    // a\u1z: "a" (ident), "\u1" becomes Unknown, "z" (ident)
    assert_tokenize_eq_roundtrip!(
        r"a\u1z",
        Token::new(TokenType::Identifier, 1, None), // "a"
        Token::new(TokenType::Unknown, 3, None),    // "\u1"
        Token::new(TokenType::Identifier, 1, None), // "z"
    );
}

/// Unicode escapes for backslash should not continue identifiers.
#[test]
fn test_lex_identifier_unicode_escape_backslash_is_not_continuation() {
    // x\u005c: "x" (ident), "\u005c" (unknown)
    assert_tokenize_eq_roundtrip!(
        r"x\u005c",
        Token::new(TokenType::Identifier, 1, None), // "x"
        Token::new(TokenType::Unknown, 6, None),    // "\u005c"
    );
}

/// Unicode escapes for asterisk should not continue identifiers.
#[test]
fn test_lex_identifier_unicode_escape_asterisk_is_not_continuation() {
    // x\u002a: "x" (ident), "\u002a" (unknown)
    assert_tokenize_eq_roundtrip!(
        r"x\u002a",
        Token::new(TokenType::Identifier, 1, None), // "x"
        Token::new(TokenType::Unknown, 6, None),    // "\u002a"
    );
}
