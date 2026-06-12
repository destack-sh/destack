use std::sync::Arc;

use crate::{Lexer, Parser, ParserOptions, ParserTokenHistory, ParserTriviaMode};
use destack_core::StringPool;
use destack_dir::render_tokens;
pub(in crate::lex) use destack_dir::{NumberBase, Token, TokenLiteral, TokenSpan, TokenType};
pub(in crate::lex) use destack_source::LanguageType;
use destack_source::{File, FileId, FileType, Span, Uri};

/// The lexer entry point used by a roundtrip assertion.
#[derive(Debug, Clone, Copy)]
pub(in crate::lex) enum LexMode {
    /// Run the lexer directly.
    Normal,
    /// Drive tree literal tokenization through the parser.
    Tree,
}

/// Lex the given source input string into its constituent tokens and side tokens.
pub(in crate::lex) fn lex_source(
    input: &str,
    language: LanguageType,
) -> (Vec<TokenSpan>, Vec<TokenSpan>, TokenSpan) {
    // build a synthetic file
    let file = File::from_text(
        FileId::new(0),
        "<string>".to_string(),
        Uri::from_string("<string>"),
        None,
        FileType::Destack,
        input.to_string(),
    );

    Lexer::lex(Arc::new(file), language)
}

/// Lex source and strip source positions from semantic and side tokens.
pub(in crate::lex) fn lex_source_tokens(
    input: &str,
    language: LanguageType,
) -> (Vec<Token>, Vec<Token>) {
    let (semantic_tokens, side_tokens, _) = lex_source(input, language);

    // normalize semantic tokens
    let semantic_tokens = semantic_tokens
        .into_iter()
        .map(|token| token_without_source(token.token))
        .collect();

    // normalize side tokens
    let side_tokens = side_tokens
        .into_iter()
        .map(|token| token_without_source(token.token))
        .collect();

    (semantic_tokens, side_tokens)
}

/// Strip the line boundary marker from one token.
pub(in crate::lex) fn token_without_line_boundary(token: Token) -> Token {
    token.with_on_new_line(false)
}

/// Strip source positions from one token.
fn token_without_source(token: Token) -> Token {
    Token::new(token.ty(), 0, token.len(), token.literal())
}

/// Strip only the source start from one token.
pub(in crate::lex) fn token_without_start(token: Token) -> Token {
    Token::new(token.ty(), 0, token.len(), token.literal()).with_on_new_line(token.is_on_new_line())
}

/// Build one expected token at source offset zero.
pub(in crate::lex) fn token(ty: TokenType, len: u32, literal: Option<TokenLiteral>) -> Token {
    Token::new(ty, 0, len, literal)
}

/// Build one expected EOF token at source offset zero.
pub(in crate::lex) fn eof() -> Token {
    Token::eof(0)
}

/// Apply source positions to an expected token sequence.
fn position_expected_tokens(input: &str, tokens: Vec<Token>) -> Vec<Token> {
    let mut start = 0;
    let end = input.len() as u32;

    tokens
        .into_iter()
        .map(|token| {
            let token_start = if token.is(TokenType::End) { end } else { start };
            start += token.len();

            Token::new(token.ty(), token_start, token.len(), token.literal())
                .with_on_new_line(token.is_on_new_line())
        })
        .collect()
}

/// Assert one string literal token with the requested escape validity.
pub(in crate::lex) fn assert_single_string_literal_token(
    input: &str,
    language: LanguageType,
    has_invalid_escape: bool,
) {
    let (semantic_tokens, side_tokens) = lex_source_tokens(input, language);
    let expected_tokens = vec![
        token(
            TokenType::Literal,
            input.len() as u32,
            Some(TokenLiteral::String {
                is_terminated: true,
                has_invalid_escape,
            }),
        ),
        eof(),
    ];

    // compare semantic and side token partitions
    assert_eq!(semantic_tokens, expected_tokens);
    assert_eq!(side_tokens, vec![]);
}

/// Assert one legacy string escape is invalid in every JavaScript-like language.
pub(in crate::lex) fn assert_legacy_string_escape_is_invalid_across_languages(input: &str) {
    for language in [
        LanguageType::default(),
        LanguageType::TypeScript,
        LanguageType::TypeScriptXml,
        LanguageType::JavaScript,
        LanguageType::JavaScriptXml,
    ] {
        assert_single_string_literal_token(input, language, true);
    }
}

/// Lex source through the parser so tree child tokenization is active.
pub(in crate::lex) fn lex_source_with_tree_literals(
    input: &str,
    language: LanguageType,
) -> (Vec<TokenSpan>, Vec<TokenSpan>, TokenSpan) {
    // build a synthetic file
    let file = File::from_text(
        FileId::new(0),
        "<string>".to_string(),
        Uri::from_string("<string>"),
        None,
        FileType::Destack,
        input.to_string(),
    );
    let file = Arc::new(file);

    // build the EOF token from the file end
    let eof_token = TokenSpan {
        token: Token::eof(file.len),
        span: Span::new(file.id, file.len, file.len),
    };

    // configure parser driven lexing
    let mut parser = Parser::lex_file_with_options(
        file,
        language,
        ParserOptions {
            trivia_mode: ParserTriviaMode::Full,
            token_history: ParserTokenHistory::Record,
            ..ParserOptions::default()
        },
        Arc::new(StringPool::new()),
    );

    // drive tree child tokenization like production code
    let _ = parser.parse_without_attaching_comments();
    let (semantic_tokens, side_tokens) = parser.take_token_spans();

    (semantic_tokens, side_tokens, eof_token)
}

/// Assert one tokenization and render roundtrip.
pub(in crate::lex) fn assert_tokens_roundtrip(
    input: &str,
    mut expected_tokens: Vec<Token>,
    mode: LexMode,
) {
    let language = LanguageType::default();
    let tokens = lex_roundtrip_tokens(input, language, mode);

    // add implicit EOF
    if !matches!(expected_tokens.last(), Some(token) if token.ty() == TokenType::End) {
        expected_tokens.push(eof());
    }

    // compare positioned tokens
    let expected_tokens = position_expected_tokens(input, expected_tokens);
    assert_eq!(tokens, expected_tokens);

    // render back to the original source
    let rendered_input = render_tokens(&tokens, input);
    assert_eq!(rendered_input, input);

    // require idempotent tokenization
    let tokens_again = lex_roundtrip_tokens(&rendered_input, language, mode);
    assert_eq!(tokens_again, tokens);
}

/// Lex source into one source ordered token stream.
fn lex_roundtrip_tokens(input: &str, language: LanguageType, mode: LexMode) -> Vec<Token> {
    let (semantic_tokens, side_tokens, _) = lex_spans(input, language, mode);
    let mut tokens: Vec<TokenSpan> = semantic_tokens.into_iter().chain(side_tokens).collect();

    // restore source order across semantic and side tokens
    tokens.sort_by_key(|token| token.span.start);

    tokens
        .into_iter()
        .map(|token| token_without_line_boundary(token.token))
        .collect()
}

/// Lex source through the selected test entry point.
fn lex_spans(
    input: &str,
    language: LanguageType,
    mode: LexMode,
) -> (Vec<TokenSpan>, Vec<TokenSpan>, TokenSpan) {
    match mode {
        LexMode::Normal => lex_source(input, language),
        LexMode::Tree => lex_source_with_tree_literals(input, language),
    }
}

/// Assert that direct lexer tokens match and render back to the source.
macro_rules! assert_tokenize_eq_roundtrip {
    ($src:expr, $($expected:expr),* $(,)?) => {
        super::assert_tokens_roundtrip($src, vec![$($expected),*], super::LexMode::Normal)
    };
}

/// Assert that tree lexer tokens match and render back to the source.
macro_rules! assert_tree_tokenize_eq_roundtrip {
    ($src:expr, $($expected:expr),* $(,)?) => {
        super::assert_tokens_roundtrip($src, vec![$($expected),*], super::LexMode::Tree)
    };
}

pub(in crate::lex) use assert_tokenize_eq_roundtrip;
pub(in crate::lex) use assert_tree_tokenize_eq_roundtrip;
