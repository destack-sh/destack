use tspp_source::FileId;

use crate::{Lexer, TokenType};

/// Lex declarations, dotted operations, and control arrows into exact token categories.
#[test]
fn test_lex_bytecode_source() {
    let source = "function f0 {\ninvoke r1, f1(r0) => b0 | b1\n}";
    let tokens = Lexer::lex(FileId::new(7), source);
    let tokens = tokens
        .iter()
        .filter(|token| !token.is_trivia())
        .map(|token| {
            (
                token.ty,
                &source[token.span.start as usize..token.span.end as usize],
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(
        tokens,
        vec![
            (TokenType::Identifier, "function"),
            (TokenType::Identifier, "f0"),
            (TokenType::OpenBrace, "{"),
            (TokenType::Identifier, "invoke"),
            (TokenType::Identifier, "r1"),
            (TokenType::Comma, ","),
            (TokenType::Identifier, "f1"),
            (TokenType::OpenParenthesis, "("),
            (TokenType::Identifier, "r0"),
            (TokenType::CloseParenthesis, ")"),
            (TokenType::FatArrow, "=>"),
            (TokenType::Identifier, "b0"),
            (TokenType::Pipe, "|"),
            (TokenType::Identifier, "b1"),
            (TokenType::CloseBrace, "}"),
            (TokenType::End, ""),
        ]
    );
}
