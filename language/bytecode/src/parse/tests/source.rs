use destack_source::FileId;

use crate::{Lexer, TokenType};

/// Lex declarations, dotted operations, and control arrows into exact token categories.
#[test]
fn test_lex_bytecode_source() {
    let source =
        "export function run(r0: int32): int32 {\n    r1: int32 = invoke work(r0) => l0 | l1\n}";
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
            (TokenType::Identifier, "export"),
            (TokenType::Identifier, "function"),
            (TokenType::Identifier, "run"),
            (TokenType::OpenParenthesis, "("),
            (TokenType::Identifier, "r0"),
            (TokenType::Colon, ":"),
            (TokenType::Identifier, "int32"),
            (TokenType::CloseParenthesis, ")"),
            (TokenType::Colon, ":"),
            (TokenType::Identifier, "int32"),
            (TokenType::OpenBrace, "{"),
            (TokenType::Identifier, "r1"),
            (TokenType::Colon, ":"),
            (TokenType::Identifier, "int32"),
            (TokenType::Equal, "="),
            (TokenType::Identifier, "invoke"),
            (TokenType::Identifier, "work"),
            (TokenType::OpenParenthesis, "("),
            (TokenType::Identifier, "r0"),
            (TokenType::CloseParenthesis, ")"),
            (TokenType::FatArrow, "=>"),
            (TokenType::Identifier, "l0"),
            (TokenType::Pipe, "|"),
            (TokenType::Identifier, "l1"),
            (TokenType::CloseBrace, "}"),
            (TokenType::End, ""),
        ]
    );
}
