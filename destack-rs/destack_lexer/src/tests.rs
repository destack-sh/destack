use super::*;
use crate::cursor::Cursor;
use crate::parse::strip_shebang;
use crate::token::{Base, DocStyle, LiteralKind, RawStrError, Token, TokenKind};

fn check_raw_str(s: &str, expected: Result<u8, RawStrError>) {
    let s = &format!("r{s}");
    let mut cursor = Cursor::new(s);
    cursor.bump();
    let res = cursor.raw_double_quoted_string(0);
    assert_eq!(res, expected);
}

#[test]
fn test_naked_raw_str() {
    check_raw_str(r#""abc""#, Ok(0));
}

#[test]
fn test_raw_no_start() {
    check_raw_str(r##""abc"#"##, Ok(0));
}

#[test]
fn test_too_many_terminators() {
    // this error is handled in the parser later
    check_raw_str(r###"#"abc"##"###, Ok(1));
}

#[test]
fn test_unterminated() {
    check_raw_str(
        r#"#"abc"#,
        Err(RawStrError::NoTerminator {
            expected: 1,
            found: 0,
            possible_terminator_offset: None,
        }),
    );
    check_raw_str(
        r###"##"abc"#"###,
        Err(RawStrError::NoTerminator {
            expected: 2,
            found: 1,
            possible_terminator_offset: Some(7),
        }),
    );
    // we're looking for "# not just any #
    check_raw_str(
        r###"##"abc#"###,
        Err(RawStrError::NoTerminator {
            expected: 2,
            found: 0,
            possible_terminator_offset: None,
        }),
    )
}

#[test]
fn test_invalid_start() {
    check_raw_str(
        r##"#~"abc"#"##,
        Err(RawStrError::InvalidStarter { bad_char: '~' }),
    );
}

#[test]
fn test_unterminated_no_pound() {
    // https://github.com/rust-lang/rust/issues/70677
    check_raw_str(
        r#"""#,
        Err(RawStrError::NoTerminator {
            expected: 0,
            found: 0,
            possible_terminator_offset: None,
        }),
    );
}

#[test]
fn test_too_many_hashes() {
    let max_count = u8::MAX;
    let hashes1 = "#".repeat(max_count as usize);
    let hashes2 = "#".repeat(max_count as usize + 1);
    let middle = "\"abc\"";
    let s1 = [&hashes1, middle, &hashes1].join("");
    let s2 = [&hashes2, middle, &hashes2].join("");

    // valid number of hashes (255 = 2^8 - 1 = u8::MAX)
    check_raw_str(&s1, Ok(255));

    // one more hash sign (256 = 2^8) becomes too many
    check_raw_str(
        &s2,
        Err(RawStrError::TooManyDelimiters {
            found: u32::from(max_count) + 1,
        }),
    );
}

// https://github.com/rust-lang/rust/issues/70528
#[test]
fn test_valid_shebang() {
    let input = "#!/bin/bash";
    assert_eq!(strip_shebang(input), Some(input.len()));

    let input = "#![attribute]";
    assert_eq!(strip_shebang(input), None);

    let input = "#!    /bin/bash";
    assert_eq!(strip_shebang(input), Some(input.len()));

    let input = "#!    [attribute]";
    assert_eq!(strip_shebang(input), None);

    let input = "#! /* blah */  /bin/bash";
    assert_eq!(strip_shebang(input), Some(input.len()));

    let input = "#! /* blah */  [attribute]";
    assert_eq!(strip_shebang(input), None);

    let input = "#! // blah\n/bin/bash";
    assert_eq!(strip_shebang(input), Some(10)); // strip up to the newline

    let input = "#! // blah\n[attribute]";
    assert_eq!(strip_shebang(input), None);

    let input = "#! /* blah\nblah\nblah */  /bin/bash";
    assert_eq!(strip_shebang(input), Some(10));

    let input = "#! /* blah\nblah\nblah */  [attribute]";
    assert_eq!(strip_shebang(input), None);

    let input = "#!\n/bin/sh";
    assert_eq!(strip_shebang(input), Some(2));

    let input = "#!\n[attribute]";
    assert_eq!(strip_shebang(input), None);

    // because shebangs are interpreted by the kernel, they must be on the first line
    let input = "\n#!/bin/bash";
    assert_eq!(strip_shebang(input), None);

    let input = "\n#![attribute]";
    assert_eq!(strip_shebang(input), None);
}

macro_rules! assert_tokens {
    ($src:expr, $($expected:expr),* $(,)?) => {
        let tokens: Vec<_> = tokenize($src).collect();
        let expected = vec![$($expected),*];
        assert_eq!(tokens, expected, "tokenize({:?})", $src);
    };
}

#[test]
fn test_smoke() {
    assert_tokens!(
        "/* my source file */ fn main() { println!(\"zebra\"); }\n",
        Token {
            kind: TokenKind::Whitespace,
            len: 1
        },
        Token {
            kind: TokenKind::Ident,
            len: 2
        },
        Token {
            kind: TokenKind::Whitespace,
            len: 1
        },
        Token {
            kind: TokenKind::Ident,
            len: 4
        },
        Token {
            kind: TokenKind::OpenParen,
            len: 1
        },
        Token {
            kind: TokenKind::CloseParen,
            len: 1
        },
        Token {
            kind: TokenKind::Whitespace,
            len: 1
        },
        Token {
            kind: TokenKind::OpenBrace,
            len: 1
        },
        Token {
            kind: TokenKind::Whitespace,
            len: 1
        },
        Token {
            kind: TokenKind::Ident,
            len: 7
        },
        Token {
            kind: TokenKind::Bang,
            len: 1
        },
        Token {
            kind: TokenKind::OpenParen,
            len: 1
        },
        Token {
            kind: TokenKind::Literal {
                kind: LiteralKind::Str { terminated: true },
                suffix_start: 7
            },
            len: 7
        },
        Token {
            kind: TokenKind::CloseParen,
            len: 1
        },
        Token {
            kind: TokenKind::Semi,
            len: 1
        },
        Token {
            kind: TokenKind::Whitespace,
            len: 1
        },
        Token {
            kind: TokenKind::CloseBrace,
            len: 1
        },
        Token {
            kind: TokenKind::Whitespace,
            len: 1
        },
    );
}

#[test]
fn test_comment_flavors() {
    assert_tokens!(
        r"
// line
//// line as well
/// outer doc line
//! inner doc line
",
        Token {
            kind: TokenKind::Whitespace,
            len: 1
        },
        Token {
            kind: TokenKind::LineComment { doc_style: None },
            len: 7
        },
        Token {
            kind: TokenKind::Whitespace,
            len: 1
        },
        Token {
            kind: TokenKind::LineComment { doc_style: None },
            len: 17
        },
        Token {
            kind: TokenKind::Whitespace,
            len: 1
        },
        Token {
            kind: TokenKind::LineComment {
                doc_style: Some(DocStyle::Outer)
            },
            len: 18
        },
        Token {
            kind: TokenKind::Whitespace,
            len: 1
        },
        Token {
            kind: TokenKind::LineComment {
                doc_style: Some(DocStyle::Inner)
            },
            len: 18
        },
        Token {
            kind: TokenKind::Whitespace,
            len: 1
        },
    );
}

#[test]
fn test_characters() {
    assert_tokens!(
        "'a' ' ' '\\n'",
        Token {
            kind: TokenKind::Literal {
                kind: LiteralKind::Char { terminated: true },
                suffix_start: 3
            },
            len: 3
        },
        Token {
            kind: TokenKind::Whitespace,
            len: 1
        },
        Token {
            kind: TokenKind::Literal {
                kind: LiteralKind::Char { terminated: true },
                suffix_start: 3
            },
            len: 3
        },
        Token {
            kind: TokenKind::Whitespace,
            len: 1
        },
        Token {
            kind: TokenKind::Literal {
                kind: LiteralKind::Char { terminated: true },
                suffix_start: 4
            },
            len: 4
        },
    );
}

#[test]
fn test_raw_string() {
    assert_tokens!(
        "r###\"\"#a\\b\x00c\"\"###",
        Token {
            kind: TokenKind::Literal {
                kind: LiteralKind::RawStr { n_hashes: Some(3) },
                suffix_start: 17
            },
            len: 17
        },
    );
}

#[test]
fn test_literal_suffixes() {
    assert_tokens!(
        r####"
'a'
b'a'
"a"
b"a"
1234
0b101
0xABC
1.0
1.0e10
2us
r###"raw"###suffix
br###"raw"###suffix
"####,
        Token {
            kind: TokenKind::Whitespace,
            len: 1
        },
        Token {
            kind: TokenKind::Literal {
                kind: LiteralKind::Char { terminated: true },
                suffix_start: 3
            },
            len: 3
        },
        Token {
            kind: TokenKind::Whitespace,
            len: 1
        },
        Token {
            kind: TokenKind::Literal {
                kind: LiteralKind::Byte { terminated: true },
                suffix_start: 4
            },
            len: 4
        },
        Token {
            kind: TokenKind::Whitespace,
            len: 1
        },
        Token {
            kind: TokenKind::Literal {
                kind: LiteralKind::Str { terminated: true },
                suffix_start: 3
            },
            len: 3
        },
        Token {
            kind: TokenKind::Whitespace,
            len: 1
        },
        Token {
            kind: TokenKind::Literal {
                kind: LiteralKind::ByteStr { terminated: true },
                suffix_start: 4
            },
            len: 4
        },
        Token {
            kind: TokenKind::Whitespace,
            len: 1
        },
        Token {
            kind: TokenKind::Literal {
                kind: LiteralKind::Int {
                    base: Base::Decimal,
                    empty_int: false
                },
                suffix_start: 4
            },
            len: 4
        },
        Token {
            kind: TokenKind::Whitespace,
            len: 1
        },
        Token {
            kind: TokenKind::Literal {
                kind: LiteralKind::Int {
                    base: Base::Binary,
                    empty_int: false
                },
                suffix_start: 5
            },
            len: 5
        },
        Token {
            kind: TokenKind::Whitespace,
            len: 1
        },
        Token {
            kind: TokenKind::Literal {
                kind: LiteralKind::Int {
                    base: Base::Hexadecimal,
                    empty_int: false
                },
                suffix_start: 5
            },
            len: 5
        },
        Token {
            kind: TokenKind::Whitespace,
            len: 1
        },
        Token {
            kind: TokenKind::Literal {
                kind: LiteralKind::Float {
                    base: Base::Decimal,
                    empty_exponent: false
                },
                suffix_start: 3
            },
            len: 3
        },
        Token {
            kind: TokenKind::Whitespace,
            len: 1
        },
        Token {
            kind: TokenKind::Literal {
                kind: LiteralKind::Float {
                    base: Base::Decimal,
                    empty_exponent: false
                },
                suffix_start: 6
            },
            len: 6
        },
        Token {
            kind: TokenKind::Whitespace,
            len: 1
        },
        Token {
            kind: TokenKind::Literal {
                kind: LiteralKind::Int {
                    base: Base::Decimal,
                    empty_int: false
                },
                suffix_start: 1
            },
            len: 3
        },
        Token {
            kind: TokenKind::Whitespace,
            len: 1
        },
        Token {
            kind: TokenKind::Literal {
                kind: LiteralKind::RawStr { n_hashes: Some(3) },
                suffix_start: 12
            },
            len: 18
        },
        Token {
            kind: TokenKind::Whitespace,
            len: 1
        },
        Token {
            kind: TokenKind::Literal {
                kind: LiteralKind::RawByteStr { n_hashes: Some(3) },
                suffix_start: 13
            },
            len: 19
        },
        Token {
            kind: TokenKind::Whitespace,
            len: 1
        },
    );
}
