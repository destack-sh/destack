use super::*;

/// Line comments and doc line comments should keep their distinct token kinds.
#[test]
fn test_lex_line_and_doc_line_comments() {
    assert_tokenize_eq_roundtrip!(
        r"
// comment
//// comment as well
/// doc comment
",
        Token::new(TokenType::Newline, 1, None),
        Token::new(TokenType::LineComment, 10, None),
        Token::new(TokenType::Newline, 1, None),
        Token::new(TokenType::LineComment, 20, None),
        Token::new(TokenType::Newline, 1, None),
        Token::new(TokenType::DocLineComment, 15, None),
        Token::new(TokenType::Newline, 1, None),
    );
}

/// Block comments and doc block comments should keep their distinct token kinds.
#[test]
fn test_lex_block_and_doc_block_comments() {
    assert_tokenize_eq_roundtrip!(
        "/* abc */ /** doc */",
        Token::new(TokenType::BlockComment, 9, None),
        Token::new(TokenType::Whitespace, 1, None),
        Token::new(TokenType::DocBlockComment, 10, None),
    );
}

/// Block comments should stop at the first closing delimiter in Destack mode.
#[test]
fn test_lex_block_comment_no_nesting_in_value_block_mode() {
    let source = "/* a /* b */ c */ d";
    let (semantic_tokens, side_tokens) = lex_source_tokens(source, LanguageType::Destack);
    assert_eq!(
        semantic_tokens,
        vec![
            Token::new(TokenType::Identifier, 1, None),
            Token::new(TokenType::Multiply, 1, None),
            Token::new(TokenType::Divide, 1, None),
            Token::new(TokenType::Identifier, 1, None),
            Token::end(),
        ]
    );
    assert_eq!(
        side_tokens,
        vec![
            Token::new(TokenType::BlockComment, 12, None),
            Token::new(TokenType::Whitespace, 1, None),
            Token::new(TokenType::Whitespace, 1, None),
            Token::new(TokenType::Whitespace, 1, None),
        ]
    );
}

/// Block comments should stop at the first closing delimiter in TypeScript mode.
#[test]
fn test_lex_block_comment_no_nesting_in_typed_mode() {
    let source = "/* a /* b */ c */ d";
    let (semantic_tokens, side_tokens) = lex_source_tokens(source, LanguageType::TypeScript);
    assert_eq!(
        semantic_tokens,
        vec![
            Token::new(TokenType::Identifier, 1, None),
            Token::new(TokenType::Multiply, 1, None),
            Token::new(TokenType::Divide, 1, None),
            Token::new(TokenType::Identifier, 1, None),
            Token::end(),
        ]
    );
    assert_eq!(
        side_tokens,
        vec![
            Token::new(TokenType::BlockComment, 12, None),
            Token::new(TokenType::Whitespace, 1, None),
            Token::new(TokenType::Whitespace, 1, None),
            Token::new(TokenType::Whitespace, 1, None),
        ]
    );
}

/// Block comments should stop at the first closing delimiter in JavaScript mode.
#[test]
fn test_lex_block_comment_no_nesting_in_untyped_mode() {
    let source = "/* a /* b */ c */ d";
    let (semantic_tokens, side_tokens) = lex_source_tokens(source, LanguageType::JavaScript);
    assert_eq!(
        semantic_tokens,
        vec![
            Token::new(TokenType::Identifier, 1, None),
            Token::new(TokenType::Multiply, 1, None),
            Token::new(TokenType::Divide, 1, None),
            Token::new(TokenType::Identifier, 1, None),
            Token::end(),
        ]
    );
    assert_eq!(
        side_tokens,
        vec![
            Token::new(TokenType::BlockComment, 12, None),
            Token::new(TokenType::Whitespace, 1, None),
            Token::new(TokenType::Whitespace, 1, None),
            Token::new(TokenType::Whitespace, 1, None),
        ]
    );
}

/// Unterminated empty block comments should produce one unknown token.
#[test]
fn test_lex_block_comment_unterminated() {
    assert_tokenize_eq_roundtrip!("/*", Token::new(TokenType::Unknown, 2, None),);
}

/// Unterminated block comments with content should produce one unknown token.
#[test]
fn test_lex_block_comment_unterminated_with_content() {
    assert_tokenize_eq_roundtrip!("/* some text", Token::new(TokenType::Unknown, 12, None),);
}

/// Four slash line comments should remain ordinary line comments.
#[test]
fn test_lex_four_slash_line_comment_is_not_doc_comment() {
    assert_tokenize_eq_roundtrip!(
        "//// not doc but line",
        Token::new(TokenType::LineComment, 21, None),
    );
}

/// Three star block comments should remain ordinary block comments.
#[test]
fn test_lex_three_star_block_comment_is_not_doc_comment() {
    assert_tokenize_eq_roundtrip!(
        "/*** not doc ***/",
        Token::new(TokenType::BlockComment, 17, None),
    );
}

/// Doc block comments should stop at the first closing delimiter.
#[test]
fn test_lex_doc_block_comment_no_nesting() {
    // doc block comments stop at the first closing delimiter even with inner /*
    assert_tokenize_eq_roundtrip!(
        "/** a /* b c */ d",
        Token::new(TokenType::DocBlockComment, 15, None),
        Token::new(TokenType::Whitespace, 1, None),
        Token::new(TokenType::Identifier, 1, None),
    );
}
