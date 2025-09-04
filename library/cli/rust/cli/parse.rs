//! Lexes and parse Destack source code.

use std::fs;
use std::path::Path;

use crate::console::parse::{CommandApp, CommandArguments};
use crate::console::{console, table};
use destack_language_token::{TokenType, tokenize_semantic};

const DEFAULT_MAX_LEXEME_LEN: usize = 80;

/// Create the parse CLI app.
pub fn app() -> CommandApp {
    CommandApp::new("parse")
        .help("parser tools")
        .default_command("lex")
        .command(
            "lex",
            lex,
            Some(format!(
                "Parse source.
			--file <path>    Read input from file
			--string <string>  Read input from provided string
			--no-color       Disable ANSI colors
			--no-pager       Print directly instead of using less -R
			--max-lexeme <n> Truncate lexeme preview to n chars (default {DEFAULT_MAX_LEXEME_LEN})"
            )),
        )
}

/// Run the lexer subcommand: tokenize input and show a colored table with locations.
fn lex(ctx: CommandArguments) -> i32 {
    // resolve input
    let input = match resolve_input(&ctx) {
        Ok(s) => s,
        Err(e) => {
            console::error(&format!("input error: {e}"));
            return 1;
        }
    };

    let use_color = !ctx.flag("no-color");
    let use_pager = !ctx.flag("no-pager");
    let max_tokeneme_len: usize = ctx
        .option("max-lexeme")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(DEFAULT_MAX_LEXEME_LEN);

    // build table
    let headers = vec![
        "Index".to_string(),
        "Line".to_string(),
        "Col".to_string(),
        "Type".to_string(),
        "Lexeme".to_string(),
        "Length".to_string(),
    ];
    let mut rows: Vec<Vec<String>> = Vec::new();
    let tokens = tokenize_semantic(&input);

    for (index, tok) in tokens.iter().enumerate() {
        let start_offset = tok.span.start as usize;
        let end_offset = tok.span.end as usize;
        let len = end_offset - start_offset;

        // compute line and column by scanning from start of input to token start
        let mut line = 1;
        let mut col = 1;
        for ch in input[..start_offset].chars() {
            if ch == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }

        // extract token slice
        let slice = &input[start_offset..end_offset.min(input.len())];

        // prepare pretty fields
        let kind_str = format_token(tok.token.r#type, use_color);
        let lexeme_preview =
            truncate_tokeneme(slice, max_tokeneme_len, tok.token.r#type, use_color);
        let index_str = if use_color {
            console::color(&index.to_string(), "35") // magenta
        } else {
            index.to_string()
        };
        let line_str = if use_color {
            console::color(&line.to_string(), "36") // cyan
        } else {
            line.to_string()
        };
        let col_str = if use_color {
            console::color(&col.to_string(), "34") // blue
        } else {
            col.to_string()
        };
        let len_str = if use_color {
            console::color(&len.to_string(), "2") // dim
        } else {
            len.to_string()
        };

        rows.push(vec![
            index_str,
            line_str,
            col_str,
            kind_str,
            lexeme_preview,
            len_str,
        ]);
    }

    // prepare secondary headers
    let secondary_headers = vec![
        "[0".to_string(),
        "[1".to_string(),
        "[1".to_string(),
        "".to_string(),
        "preview".to_string(),
        "bytes".to_string(),
    ];
    let secondary: Option<&[String]> = Some(&secondary_headers);

    // render table
    let table_str = table::render_table(&headers, &rows, true, 2, secondary);
    let framed_table_str = console::frame(&table_str, Some("Lexer Tokens"), 2);

    // print table
    if use_pager {
        if console::page_with_less(&framed_table_str).is_err() {
            println!("{framed_table_str}");
        }
    } else {
        println!("{framed_table_str}");
    }

    0
}

/// Resolve the input to lex from the command arguments.
fn resolve_input(ctx: &CommandArguments) -> Result<String, String> {
    if let Some(path) = ctx.option("file") {
        return read_file_to_string(path).map_err(|e| format!("failed to read {path}: {e}"));
    }
    if let Some(string) = ctx.option("string") {
        return Ok(string.to_string());
    }
    Err("provide --file <path> or --string <string>".to_string())
}

/// Read a file to a string.
fn read_file_to_string(path: &str) -> Result<String, std::io::Error> {
    let p = Path::new(path);
    let data = fs::read_to_string(p)?;
    Ok(data)
}

/// Format a Token for display.
fn format_token(kind: TokenType, use_color: bool) -> String {
    let base = format_token_kind(kind);
    if !use_color {
        return base;
    }
    let color = get_token_color(kind);
    console::color(&base, color)
}

/// Format a token kind for display in a concise manner.
fn format_token_kind(kind: TokenType) -> String {
    format!("{kind:?}")
}

/// Get the color code for a token type.
fn get_token_color(kind: TokenType) -> &'static str {
    match kind {
        // regular comments
        TokenType::LineComment | TokenType::BlockComment => "2", // dim

        // doc comments
        TokenType::DocLineComment | TokenType::DocBlockComment => "32", // green

        // whitespaces / end / newline
        TokenType::Whitespace | TokenType::Newline | TokenType::End => "2", // dim

        // colon, semi, comma, dot, range, ellipsis
        TokenType::Colon
        | TokenType::Semicolon
        | TokenType::Comma
        | TokenType::Dot
        | TokenType::Range
        | TokenType::Ellipsis => "37", // white

        // all parens
        TokenType::OpenParenthesis
        | TokenType::CloseParenthesis
        | TokenType::OpenBrace
        | TokenType::CloseBrace
        | TokenType::OpenBracket
        | TokenType::CloseBracket => "33", // yellow

        // weird symbols like at pound arrows
        TokenType::At
        | TokenType::Pound
        | TokenType::BitwiseNot
        | TokenType::Question
        | TokenType::Dollar
        | TokenType::Bang
        | TokenType::Empty
        | TokenType::Arrow
        | TokenType::BadArrow => "95", // bright magenta

        // logical operators (excl. assignment)
        TokenType::LogicalAnd | TokenType::LogicalOr => "94", // bright blue

        // shifting and bitwise ops (excl. assignment)
        TokenType::BitwiseOr
        | TokenType::BitwiseAnd
        | TokenType::BitwiseXor
        | TokenType::ShiftLeft
        | TokenType::SaturatingShiftLeft
        | TokenType::ShiftRight => "96", // bright cyan

        // math operators (excl. assignment)
        TokenType::Add
        | TokenType::WrappingAdd
        | TokenType::SaturatingAdd
        | TokenType::Subtract
        | TokenType::WrappingSubtract
        | TokenType::SaturatingSubtract
        | TokenType::Multiply
        | TokenType::WrappingMultiply
        | TokenType::SaturatingMultiply
        | TokenType::Divide
        | TokenType::Remainder => "93", // bright yellow

        // assignments (all)
        TokenType::Assign
        | TokenType::BitwiseOrAssign
        | TokenType::BitwiseAndAssign
        | TokenType::BitwiseXorAssign
        | TokenType::ShiftLeftAssign
        | TokenType::SaturatingShiftLeftAssign
        | TokenType::ShiftRightAssign
        | TokenType::AddAssign
        | TokenType::WrappingAddAssign
        | TokenType::SaturatingAddAssign
        | TokenType::SubtractAssign
        | TokenType::WrappingSubtractAssign
        | TokenType::SaturatingSubtractAssign
        | TokenType::MultiplyAssign
        | TokenType::WrappingMultiplyAssign
        | TokenType::SaturatingMultiplyAssign
        | TokenType::DivideAssign
        | TokenType::RemainderAssign
        | TokenType::LogicalAndAssign
        | TokenType::LogicalOrAssign => "91", // bright red

        // comparison
        TokenType::GreaterThan
        | TokenType::LessThan
        | TokenType::GreaterThanOrEqual
        | TokenType::LessThanOrEqual
        | TokenType::Equal
        | TokenType::NotEqual => "92", // bright green

        // identifiers and literals
        TokenType::Identifier => "36",        // cyan
        TokenType::InvalidIdentifier => "31", // red
        TokenType::Literal => "35",           // magenta

        // unknown/error tokens
        TokenType::Unknown | TokenType::UnknownLiteralPrefix => "31", // red
    }
}

/// Truncate a lexeme to a maximum length and escape newlines and tabs.
fn truncate_tokeneme(s: &str, max_len: usize, token_type: TokenType, color: bool) -> String {
    // escape newlines and tabs for readability
    let mut out = String::new();
    for ch in s.chars() {
        match ch {
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            _ => out.push(ch),
        }
    }
    let visible: String = if out.chars().count() > max_len {
        let mut acc = String::new();
        for (idx, ch) in out.chars().enumerate() {
            if idx >= max_len {
                break;
            }
            acc.push(ch);
        }
        acc.push('…');
        acc
    } else {
        out
    };

    if !color {
        return visible;
    }

    // apply color based on token type
    match token_type {
        TokenType::Whitespace => {
            // dim whitespace-only previews
            console::color(&visible, "2")
        }
        _ => {
            // use the same color as the token type
            console::color(&visible, get_token_color(token_type))
        }
    }
}
