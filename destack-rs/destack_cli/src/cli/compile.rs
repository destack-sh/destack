//! Compile-like command that currently lexes input and prints tokens.

use std::fs;
use std::path::Path;

use crate::console::parser::{CommandApp, CommandArguments};
use crate::console::{console, table};
use destack_lexer::{TokenType, tokenize};

const DEFAULT_MAX_LEXEME_LEN: usize = 80;

/// Create the compile CLI app.
pub fn app() -> CommandApp {
    CommandApp::new("compile").help("Compile tools").command(
        "lex",
        lex,
        Some(format!(
            "Lex source and show tokens.
			--file <path>    Read input from file
			--text <string>  Read input from provided string
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
    let max_lexeme_len: usize = ctx
        .option("max-lexeme")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(DEFAULT_MAX_LEXEME_LEN);

    // build table
    let headers = vec![
        "#".to_string(),
        "Line".to_string(),
        "Col".to_string(),
        "Bytes".to_string(),
        "Kind".to_string(),
        "Lexeme".to_string(),
    ];
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut offset_bytes: usize = 0;
    let mut line: usize = 1;
    let mut col: usize = 1;
    for (idx, tok) in tokenize(&input).enumerate() {
        // take substring for this token by byte length
        let len = tok.len as usize;
        let end = offset_bytes.saturating_add(len).min(input.len());
        let slice = &input[offset_bytes..end];

        // advance line/col based on slice
        for ch in slice.chars() {
            if ch == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }

        // prepare pretty fields
        let kind_str = format_token(tok.r#type, use_color);
        let lexeme_preview = truncate_lexeme(slice, max_lexeme_len, use_color);
        let idx_str = if use_color {
            console::color(&(idx + 1).to_string(), "2") // dim
        } else {
            (idx + 1).to_string()
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
            idx_str,
            line_str,
            col_str,
            len_str,
            kind_str,
            lexeme_preview,
        ]);

        offset_bytes = end;
    }

    // prepare secondary headers
    let secondary_headers = vec![
        "i".to_string(),
        "[1".to_string(),
        "[1".to_string(),
        "byte".to_string(),
        "type".to_string(),
        "preview".to_string(),
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
    if let Some(text) = ctx.option("text") {
        return Ok(text.to_string());
    }
    Err("provide --file <path> or --text <string>".to_string())
}

/// Read a file to a string.
fn read_file_to_string(path: &str) -> Result<String, std::io::Error> {
    let p = Path::new(path);
    let data = fs::read_to_string(p)?;
    Ok(data)
}

/// Format a Token for display.
fn format_token(kind: TokenType, color: bool) -> String {
    let base = concise_kind(kind);
    if !color {
        return base;
    }
    match kind {
        TokenType::LineComment { .. } => console::color(&base, "32"), // green
        TokenType::Whitespace => console::color(&base, "2"),          // dim
        TokenType::Identifier | TokenType::RawIdentifier => console::color(&base, "36"), // cyan
        TokenType::InvalidIdentifier => console::color(&base, "31"),  // red
        TokenType::Literal { .. } => console::color(&base, "35"),     // magenta
        TokenType::Unknown | TokenType::UnknownLiteralPrefix => console::color(&base, "33"), // yellow
        _ => console::color(&base, "34"), // blue for punctuators
    }
}

/// Format a token kind for display in a concise manner.
fn concise_kind(kind: TokenType) -> String {
    match kind {
        TokenType::Literal { r#type, .. } => match r#type {
            destack_lexer::LiteralTokenType::Integer { .. } => "Literal<Integer>".to_string(),
            destack_lexer::LiteralTokenType::Float { .. } => "Literal<Float>".to_string(),
            destack_lexer::LiteralTokenType::Character { .. } => "Literal<Character>".to_string(),
            destack_lexer::LiteralTokenType::Byte { .. } => "Literal<Byte>".to_string(),
            destack_lexer::LiteralTokenType::String { .. } => "Literal<String>".to_string(),
            destack_lexer::LiteralTokenType::ByteString { .. } => "Literal<ByteString>".to_string(),
            destack_lexer::LiteralTokenType::RawString { .. } => "Literal<RawString>".to_string(),
            destack_lexer::LiteralTokenType::RawByteString { .. } => {
                "Literal<RawByteString>".to_string()
            }
        },
        other => format!("{other:?}"),
    }
}

/// Truncate a lexeme to a maximum length and escape newlines and tabs.
fn truncate_lexeme(s: &str, max_len: usize, color: bool) -> String {
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
    // dim whitespace-only previews
    if color && !s.chars().all(|c| c.is_whitespace()) {
        console::color(&visible, "2") // dim
    } else {
        visible
    }
}
