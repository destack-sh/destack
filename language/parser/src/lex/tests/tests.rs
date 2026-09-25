use std::sync::Arc;

use crate::{CommentRetention, Lexer, ParseOptions, Parser, Tokenizer};
pub(in crate::lex) use tspp_dir::{NumberBase, Token, TokenLiteral, TokenSpan, TokenType};
use tspp_dir::{Tree, render_tokens};
use tspp_source::{File, FileId, FileType, LanguageType, ModuleId, PackageId, Span, Uri};

/// The lexer entry point used by a roundtrip assertion.
#[derive(Debug, Clone, Copy)]
pub(in crate::lex) enum LexMode {
    /// Run the lexer directly.
    Normal,
    /// Drive tree literal tokenization through the parser.
    Tree,
}

/// Lex the given source input string into its constituent tokens and trivia tokens.
pub(in crate::lex) fn lex_source(input: &str) -> (Vec<TokenSpan>, Vec<TokenSpan>, TokenSpan) {
    // build a synthetic file
    let file = File::from_text(
        FileId::new(0),
        "<string>".to_string(),
        Uri::from_string("<string>"),
        None,
        FileType::Tspp,
        input.to_string(),
    )
    .expect("test source should load");

    let file = Arc::new(file);
    let (semantic_tokens, eof_token) = Lexer::lex(file.clone());
    let trivia_tokens = raw_trivia_token_spans(file, &semantic_tokens);

    (semantic_tokens, trivia_tokens, eof_token)
}

/// Read source trivia tokens not covered by contextual semantic tokens.
fn raw_trivia_token_spans(file: Arc<File>, semantic_tokens: &[TokenSpan]) -> Vec<TokenSpan> {
    let mut tokenizer = Tokenizer::new(file.clone());
    let mut trivia_tokens = Vec::new();
    let mut semantic_index = 0usize;

    // retain raw trivia tokens outside contextual semantic ranges
    loop {
        let token = tokenizer.read_source_token();
        if token.is_semantic() {
            if token.is(TokenType::End) {
                break;
            }

            continue;
        }

        // advance to the first semantic token that may overlap this trivia token
        while semantic_tokens
            .get(semantic_index)
            .is_some_and(|semantic| semantic.token.end() <= token.start())
        {
            semantic_index += 1;
        }

        // contextual semantic tokens replace every raw token they cover
        let is_contextual = semantic_tokens.get(semantic_index).is_some_and(|semantic| {
            semantic.token.start() < token.end() && token.start() < semantic.token.end()
        });
        if !is_contextual {
            trivia_tokens.push(TokenSpan::new(token, file.id));
        }
    }

    trivia_tokens
}

/// Lex source and strip source positions from semantic and trivia tokens.
pub(in crate::lex) fn lex_source_tokens(input: &str) -> (Vec<Token>, Vec<Token>) {
    let (semantic_tokens, trivia_tokens, _) = lex_source(input);

    // normalize semantic tokens
    let semantic_tokens = semantic_tokens
        .into_iter()
        .map(|token| token_without_source(token.token))
        .collect();

    // normalize trivia tokens
    let trivia_tokens = trivia_tokens
        .into_iter()
        .map(|token| token_without_source(token.token))
        .collect();

    (semantic_tokens, trivia_tokens)
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
pub(in crate::lex) fn assert_single_string_literal_token(input: &str, has_invalid_escape: bool) {
    let (semantic_tokens, trivia_tokens) = lex_source_tokens(input);
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

    // compare semantic and trivia token partitions
    assert_eq!(semantic_tokens, expected_tokens);
    assert_eq!(trivia_tokens, vec![]);
}

/// Assert one legacy string escape is invalid.
pub(in crate::lex) fn assert_legacy_string_escape_is_invalid(input: &str) {
    assert_single_string_literal_token(input, true);
}

/// Lex source through the parser so tree child tokenization is active.
pub(in crate::lex) fn lex_source_with_tree_literals(
    input: &str,
) -> (Vec<TokenSpan>, Vec<TokenSpan>, TokenSpan) {
    // build a synthetic file
    let file = File::from_text(
        FileId::new(0),
        "<string>".to_string(),
        Uri::from_string("<string>"),
        None,
        FileType::Tspp,
        input.to_string(),
    )
    .expect("test source should load");
    let file = Arc::new(file);

    // build the EOF token from the file end
    let eof_token = TokenSpan {
        token: Token::eof(file.len),
        span: Span::new(file.id, file.len, file.len),
    };

    // configure parser driven lexing
    let module_id = ModuleId::new(PackageId::new(0), file.id.0);
    let mut parser = Parser::new(
        file.clone(),
        LanguageType::Tspp,
        Tree::new(module_id),
        ParseOptions {
            comment_retention: CommentRetention::All,
            ..ParseOptions::default()
        },
    );

    // drive tree child tokenization like production code
    let _ = parser.parse_roots();
    let semantic_tokens = parser.take_token_spans();
    let trivia_tokens = raw_trivia_token_spans(file, &semantic_tokens);

    (semantic_tokens, trivia_tokens, eof_token)
}

/// Assert one tokenization and render roundtrip.
pub(in crate::lex) fn assert_tokens_roundtrip(
    input: &str,
    mut expected_tokens: Vec<Token>,
    mode: LexMode,
) {
    let tokens = lex_roundtrip_tokens(input, mode);

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
    let tokens_again = lex_roundtrip_tokens(&rendered_input, mode);
    assert_eq!(tokens_again, tokens);
}

/// Lex source into one source ordered token stream.
fn lex_roundtrip_tokens(input: &str, mode: LexMode) -> Vec<Token> {
    let (semantic_tokens, trivia_tokens, _) = lex_spans(input, mode);
    let mut tokens: Vec<TokenSpan> = semantic_tokens.into_iter().chain(trivia_tokens).collect();

    // restore source order across semantic and trivia tokens
    tokens.sort_by_key(|token| token.span.start);

    tokens
        .into_iter()
        .map(|token| token_without_line_boundary(token.token))
        .collect()
}

/// Lex source through the selected test entry point.
fn lex_spans(input: &str, mode: LexMode) -> (Vec<TokenSpan>, Vec<TokenSpan>, TokenSpan) {
    match mode {
        LexMode::Normal => lex_source(input),
        LexMode::Tree => lex_source_with_tree_literals(input),
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
