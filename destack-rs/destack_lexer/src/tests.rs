use super::*;
use crate::parse::strip_shebang;
use crate::token::{DocPosition, LiteralTokenType, NumberBase, RawStringError, Token, TokenType};
use crate::tokenizer::Tokenizer;

macro_rules! assert_tokens_eq {
    ($src:expr, $($expected:expr),* $(,)?) => {
        let tokens: Vec<_> = tokenize($src).collect();
        let expected = vec![$($expected),*];
        assert_eq!(tokens, expected, "tokenize({:?})", $src);
    };
}

fn check_raw_str(s: &str, expected: Result<u8, RawStringError>) {
    let s = &format!("r{s}");
    let mut cursor = Tokenizer::new(s);
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
        Err(RawStringError::NoTerminator {
            expected: 1,
            found: 0,
            possible_terminator_offset: None,
        }),
    );
    check_raw_str(
        r###"##"abc"#"###,
        Err(RawStringError::NoTerminator {
            expected: 2,
            found: 1,
            possible_terminator_offset: Some(7),
        }),
    );
    // we're looking for "# not just any #
    check_raw_str(
        r###"##"abc#"###,
        Err(RawStringError::NoTerminator {
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
        Err(RawStringError::InvalidStarter { bad_char: '~' }),
    );
}

#[test]
fn test_spread_and_arrows() {
    assert_tokens_eq!(
        "a...b => c->d :: x",
        Token {
            r#type: TokenType::Identifier,
            len: 1
        },
        Token {
            r#type: TokenType::DotDotDot,
            len: 3
        },
        Token {
            r#type: TokenType::Identifier,
            len: 1
        },
        Token {
            r#type: TokenType::Whitespace,
            len: 1
        },
        Token {
            r#type: TokenType::FatArrow,
            len: 2
        },
        Token {
            r#type: TokenType::Whitespace,
            len: 1
        },
        Token {
            r#type: TokenType::Identifier,
            len: 1
        },
        Token {
            r#type: TokenType::ThinArrow,
            len: 2
        },
        Token {
            r#type: TokenType::Identifier,
            len: 1
        },
        Token {
            r#type: TokenType::Whitespace,
            len: 1
        },
        Token {
            r#type: TokenType::DoubleColon,
            len: 2
        },
        Token {
            r#type: TokenType::Whitespace,
            len: 1
        },
        Token {
            r#type: TokenType::Identifier,
            len: 1
        },
    );
}

#[test]
fn test_unterminated_no_pound() {
    // https://github.com/rust-lang/rust/issues/70677
    check_raw_str(
        r#"""#,
        Err(RawStringError::NoTerminator {
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
        Err(RawStringError::TooManyDelimiters {
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
    assert_eq!(strip_shebang(input), Some(input.len()));

    let input = "#!    /bin/bash";
    assert_eq!(strip_shebang(input), Some(input.len()));

    let input = "#!    [attribute]";
    assert_eq!(strip_shebang(input), Some(input.len()));

    let input = "#! /* blah */  /bin/bash";
    assert_eq!(strip_shebang(input), Some(input.len()));

    let input = "#! /* blah */  [attribute]";
    assert_eq!(strip_shebang(input), Some(input.len()));

    let input = "#! // blah\n/bin/bash";
    assert_eq!(strip_shebang(input), Some(10)); // strip up to the newline

    let input = "#! // blah\n[attribute]";
    assert_eq!(strip_shebang(input), Some(10));

    let input = "#! /* blah\nblah\nblah */  /bin/bash";
    assert_eq!(strip_shebang(input), Some(10));

    let input = "#! /* blah\nblah\nblah */  [attribute]";
    assert_eq!(strip_shebang(input), Some(10));

    let input = "#!\n/bin/sh";
    assert_eq!(strip_shebang(input), Some(2));

    let input = "#!\n[attribute]";
    assert_eq!(strip_shebang(input), Some(2));

    // because shebangs are interpreted by the kernel, they must be on the first line
    let input = "\n#!/bin/bash";
    assert_eq!(strip_shebang(input), None);

    let input = "\n#![attribute]";
    assert_eq!(strip_shebang(input), None);
}

#[test]
fn test_smoke() {
    assert_tokens_eq!(
        "fn main() { println!(\"zebra\"); }\n",
        Token {
            r#type: TokenType::Identifier,
            len: 2
        },
        Token {
            r#type: TokenType::Whitespace,
            len: 1
        },
        Token {
            r#type: TokenType::Identifier,
            len: 4
        },
        Token {
            r#type: TokenType::OpenParenthesis,
            len: 1
        },
        Token {
            r#type: TokenType::CloseParenthesis,
            len: 1
        },
        Token {
            r#type: TokenType::Whitespace,
            len: 1
        },
        Token {
            r#type: TokenType::OpenBrace,
            len: 1
        },
        Token {
            r#type: TokenType::Whitespace,
            len: 1
        },
        Token {
            r#type: TokenType::Identifier,
            len: 7
        },
        Token {
            r#type: TokenType::Bang,
            len: 1
        },
        Token {
            r#type: TokenType::OpenParenthesis,
            len: 1
        },
        Token {
            r#type: TokenType::Literal {
                kind: LiteralTokenType::String { terminated: true },
                suffix_start: 7
            },
            len: 7
        },
        Token {
            r#type: TokenType::CloseParenthesis,
            len: 1
        },
        Token {
            r#type: TokenType::Semi,
            len: 1
        },
        Token {
            r#type: TokenType::Whitespace,
            len: 1
        },
        Token {
            r#type: TokenType::CloseBrace,
            len: 1
        },
        Token {
            r#type: TokenType::Whitespace,
            len: 1
        },
    );
}

#[test]
fn test_comment_flavors() {
    assert_tokens_eq!(
        r"
// line
//// line as well
/// outer doc line
//! inner doc line
",
        Token {
            r#type: TokenType::Whitespace,
            len: 1
        },
        Token {
            r#type: TokenType::LineComment { doc_style: None },
            len: 7
        },
        Token {
            r#type: TokenType::Whitespace,
            len: 1
        },
        Token {
            r#type: TokenType::LineComment { doc_style: None },
            len: 17
        },
        Token {
            r#type: TokenType::Whitespace,
            len: 1
        },
        Token {
            r#type: TokenType::LineComment {
                doc_style: Some(DocPosition::Outer)
            },
            len: 18
        },
        Token {
            r#type: TokenType::Whitespace,
            len: 1
        },
        Token {
            r#type: TokenType::LineComment {
                doc_style: Some(DocPosition::Inner)
            },
            len: 18
        },
        Token {
            r#type: TokenType::Whitespace,
            len: 1
        },
    );
}

#[test]
fn test_characters() {
    assert_tokens_eq!(
        "'a' ' ' '\\n'",
        Token {
            r#type: TokenType::Literal {
                kind: LiteralTokenType::Character { terminated: true },
                suffix_start: 3
            },
            len: 3
        },
        Token {
            r#type: TokenType::Whitespace,
            len: 1
        },
        Token {
            r#type: TokenType::Literal {
                kind: LiteralTokenType::Character { terminated: true },
                suffix_start: 3
            },
            len: 3
        },
        Token {
            r#type: TokenType::Whitespace,
            len: 1
        },
        Token {
            r#type: TokenType::Literal {
                kind: LiteralTokenType::Character { terminated: true },
                suffix_start: 4
            },
            len: 4
        },
    );
}

#[test]
fn test_raw_string() {
    assert_tokens_eq!(
        "r###\"\"#a\\b\x00c\"\"###",
        Token {
            r#type: TokenType::Literal {
                kind: LiteralTokenType::RawString { n_hashes: Some(3) },
                suffix_start: 17
            },
            len: 17
        },
    );
}

#[test]
fn test_literal_suffixes() {
    assert_tokens_eq!(
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
            r#type: TokenType::Whitespace,
            len: 1
        },
        Token {
            r#type: TokenType::Literal {
                kind: LiteralTokenType::Character { terminated: true },
                suffix_start: 3
            },
            len: 3
        },
        Token {
            r#type: TokenType::Whitespace,
            len: 1
        },
        Token {
            r#type: TokenType::Literal {
                kind: LiteralTokenType::Byte { terminated: true },
                suffix_start: 4
            },
            len: 4
        },
        Token {
            r#type: TokenType::Whitespace,
            len: 1
        },
        Token {
            r#type: TokenType::Literal {
                kind: LiteralTokenType::String { terminated: true },
                suffix_start: 3
            },
            len: 3
        },
        Token {
            r#type: TokenType::Whitespace,
            len: 1
        },
        Token {
            r#type: TokenType::Literal {
                kind: LiteralTokenType::ByteString { terminated: true },
                suffix_start: 4
            },
            len: 4
        },
        Token {
            r#type: TokenType::Whitespace,
            len: 1
        },
        Token {
            r#type: TokenType::Literal {
                kind: LiteralTokenType::Integer {
                    base: NumberBase::Decimal,
                    empty_int: false
                },
                suffix_start: 4
            },
            len: 4
        },
        Token {
            r#type: TokenType::Whitespace,
            len: 1
        },
        Token {
            r#type: TokenType::Literal {
                kind: LiteralTokenType::Integer {
                    base: NumberBase::Binary,
                    empty_int: false
                },
                suffix_start: 5
            },
            len: 5
        },
        Token {
            r#type: TokenType::Whitespace,
            len: 1
        },
        Token {
            r#type: TokenType::Literal {
                kind: LiteralTokenType::Integer {
                    base: NumberBase::Hexadecimal,
                    empty_int: false
                },
                suffix_start: 5
            },
            len: 5
        },
        Token {
            r#type: TokenType::Whitespace,
            len: 1
        },
        Token {
            r#type: TokenType::Literal {
                kind: LiteralTokenType::Float {
                    base: NumberBase::Decimal,
                    empty_exponent: false
                },
                suffix_start: 3
            },
            len: 3
        },
        Token {
            r#type: TokenType::Whitespace,
            len: 1
        },
        Token {
            r#type: TokenType::Literal {
                kind: LiteralTokenType::Float {
                    base: NumberBase::Decimal,
                    empty_exponent: false
                },
                suffix_start: 6
            },
            len: 6
        },
        Token {
            r#type: TokenType::Whitespace,
            len: 1
        },
        Token {
            r#type: TokenType::Literal {
                kind: LiteralTokenType::Integer {
                    base: NumberBase::Decimal,
                    empty_int: false
                },
                suffix_start: 1
            },
            len: 3
        },
        Token {
            r#type: TokenType::Whitespace,
            len: 1
        },
        Token {
            r#type: TokenType::Literal {
                kind: LiteralTokenType::RawString { n_hashes: Some(3) },
                suffix_start: 12
            },
            len: 18
        },
        Token {
            r#type: TokenType::Whitespace,
            len: 1
        },
        Token {
            r#type: TokenType::Literal {
                kind: LiteralTokenType::RawByteString { n_hashes: Some(3) },
                suffix_start: 13
            },
            len: 19
        },
        Token {
            r#type: TokenType::Whitespace,
            len: 1
        },
    );
}

/// Tokenize an input string in a roundtrip.
macro_rules! assert_tokenize_roundtrip {
    ($input:expr) => {
        // tokenize & render back to input string
        let tokens: Vec<_> = tokenize($input).collect();
        let rendered_input = render_tokens(&tokens, $input);

        // should match, do nice diff if not
        if rendered_input != $input {
            eprintln!("Roundtrip failed!");
            eprintln!("Expected:\n{}", $input);
            eprintln!("Got:\n{}", rendered_input);
            eprintln!("Diff:");
            for (i, (expected, actual)) in $input.chars().zip(rendered_input.chars()).enumerate() {
                if expected != actual {
                    eprintln!(
                        "  Position {}: expected {:?}, got {:?}",
                        i, expected, actual
                    );
                }
            }
            if $input.len() != rendered_input.len() {
                eprintln!(
                    "  Length mismatch: expected {}, got {}",
                    $input.len(),
                    rendered_input.len()
                );
            }
        }
        assert_eq!(rendered_input, $input);

        // tokenize *again* on
        let reparsed_tokens: Vec<_> = tokenize(&rendered_input).collect();
        if reparsed_tokens != tokens {
            eprintln!("Token roundtrip failed!");
            eprintln!("Original tokens: {:#?}", tokens);
            eprintln!("Reparsed tokens: {:#?}", reparsed_tokens);
        }
        assert_eq!(reparsed_tokens, tokens);
    };
}

#[test]
fn test_roundtrip_tetris() {
    let input = r##"
/// Base component for all tetris game objects
struct TetrisComponent {
	/// Game instance this object belongs to
	game_id: u32,
	/// Whether this object is active in the game
	is_active: bool = true,
}

/// A single cell in the tetris grid
struct TetrisCell {
	/// Whether the cell is occupied by a placed piece
	is_occupied: bool = false,
	/// Color of the piece in this cell
	color: Option<Color> = None,
	/// Shape type that occupies this cell
	shape_type: Option<TetrisShape> = None,
}"##;

    assert_tokenize_roundtrip!(input);
}

#[test]
fn test_roundtrip_view() {
    let input = r##"
entity MyCustomView extends View2D {
	fn render(self) {
        @if target == 'macos' {
            ButtonView::new({ test: f"Hi {self.name}!" })
        } @else {
            None
        }
	}
}"##;
    let tokens: Vec<_> = tokenize(input).collect();
    println!("{tokens:?}");

    assert_tokenize_roundtrip!(input);
}
