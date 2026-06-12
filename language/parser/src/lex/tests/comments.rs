use super::{LanguageType, TokenType, assert_tokenize_eq_roundtrip, eof, lex_source_tokens, token};

/// Line comments and doc line comments should keep their distinct token kinds.
#[test]
fn test_lex_line_and_doc_line_comments() {
    assert_tokenize_eq_roundtrip!(
        r"
// comment
//// comment as well
/// doc comment
",
        token(TokenType::Newline, 1, None),
        token(TokenType::LineComment, 10, None),
        token(TokenType::Newline, 1, None),
        token(TokenType::LineComment, 20, None),
        token(TokenType::Newline, 1, None),
        token(TokenType::DocLineComment, 15, None),
        token(TokenType::Newline, 1, None),
    );
}

/// Block comments and doc block comments should keep their distinct token kinds.
#[test]
fn test_lex_block_and_doc_block_comments() {
    assert_tokenize_eq_roundtrip!(
        "/* abc */ /** doc */",
        token(TokenType::BlockComment, 9, None),
        token(TokenType::Whitespace, 1, None),
        token(TokenType::DocBlockComment, 10, None),
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
            token(TokenType::Identifier, 1, None),
            token(TokenType::Multiply, 1, None),
            token(TokenType::Divide, 1, None),
            token(TokenType::Identifier, 1, None),
            eof(),
        ]
    );
    assert_eq!(
        side_tokens,
        vec![
            token(TokenType::BlockComment, 12, None),
            token(TokenType::Whitespace, 1, None),
            token(TokenType::Whitespace, 1, None),
            token(TokenType::Whitespace, 1, None),
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
            token(TokenType::Identifier, 1, None),
            token(TokenType::Multiply, 1, None),
            token(TokenType::Divide, 1, None),
            token(TokenType::Identifier, 1, None),
            eof(),
        ]
    );
    assert_eq!(
        side_tokens,
        vec![
            token(TokenType::BlockComment, 12, None),
            token(TokenType::Whitespace, 1, None),
            token(TokenType::Whitespace, 1, None),
            token(TokenType::Whitespace, 1, None),
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
            token(TokenType::Identifier, 1, None),
            token(TokenType::Multiply, 1, None),
            token(TokenType::Divide, 1, None),
            token(TokenType::Identifier, 1, None),
            eof(),
        ]
    );
    assert_eq!(
        side_tokens,
        vec![
            token(TokenType::BlockComment, 12, None),
            token(TokenType::Whitespace, 1, None),
            token(TokenType::Whitespace, 1, None),
            token(TokenType::Whitespace, 1, None),
        ]
    );
}

/// Unterminated empty block comments should produce one unknown token.
#[test]
fn test_lex_block_comment_unterminated() {
    assert_tokenize_eq_roundtrip!("/*", token(TokenType::Unknown, 2, None),);
}

/// Unterminated block comments with content should produce one unknown token.
#[test]
fn test_lex_block_comment_unterminated_with_content() {
    assert_tokenize_eq_roundtrip!("/* some text", token(TokenType::Unknown, 12, None),);
}

/// Four slash line comments should remain ordinary line comments.
#[test]
fn test_lex_four_slash_line_comment_is_not_doc_comment() {
    assert_tokenize_eq_roundtrip!(
        "//// not doc but line",
        token(TokenType::LineComment, 21, None),
    );
}

/// Three star block comments should remain ordinary block comments.
#[test]
fn test_lex_three_star_block_comment_is_not_doc_comment() {
    assert_tokenize_eq_roundtrip!(
        "/*** not doc ***/",
        token(TokenType::BlockComment, 17, None),
    );
}

/// Doc block comments should stop at the first closing delimiter.
#[test]
fn test_lex_doc_block_comment_no_nesting() {
    // doc block comments stop at the first closing delimiter even with inner /*
    assert_tokenize_eq_roundtrip!(
        "/** a /* b c */ d",
        token(TokenType::DocBlockComment, 15, None),
        token(TokenType::Whitespace, 1, None),
        token(TokenType::Identifier, 1, None),
    );
}
