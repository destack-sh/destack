use crate::cli::parse::read_parse_input;
use crate::console::parse::CommandArguments;
use crate::console::{console, table};
use dyst_language_source::SourceId;
use dyst_language_token::{TokenType, tokenize_semantic};

use super::DEFAULT_MAX_LEXEME_LEN;

/// Run the lexer subcommand: tokenize input and show a colored table with locations.
pub(crate) fn parse_token(ctx: CommandArguments) -> i32 {
    // input
    let input = match read_parse_input(&ctx) {
        Ok(s) => s,
        Err(e) => {
            console::error(&format!("Read input error: {e}"));
            return 1;
        }
    };

    // options
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
    let tokens = tokenize_semantic(SourceId::new(0), &input);

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
        // structural
        TokenType::Newline | TokenType::Whitespace | TokenType::End => "2", // dim
        TokenType::Unknown => "31",                                         // red

        // comments
        TokenType::LineComment | TokenType::BlockComment => "2", // dim
        TokenType::DocLineComment | TokenType::DocBlockComment => "32", // green

        // identifiers / literals
        TokenType::Identifier => "36",           // cyan
        TokenType::InvalidIdentifier => "31",    // red
        TokenType::UnknownLiteralPrefix => "31", // red
        TokenType::Literal => "35",              // magenta

        // symbols
        TokenType::Wildcard => "37",  // white
        TokenType::Colon => "37",     // white
        TokenType::Semicolon => "37", // white
        TokenType::Comma => "37",     // white
        TokenType::Dot => "37",       // white
        TokenType::Range => "37",     // white
        TokenType::RangeWide => "37", // white
        TokenType::Pound => "95",     // bright magenta
        TokenType::Empty => "95",     // bright magenta
        TokenType::EmptyWide => "95", // bright magenta
        TokenType::Arrow => "95",     // bright magenta
        TokenType::BadArrow => "95",  // bright magenta

        // parentheses
        TokenType::OpenParenthesis => "33",  // yellow
        TokenType::CloseParenthesis => "33", // yellow
        TokenType::OpenBrace => "33",        // yellow
        TokenType::CloseBrace => "33",       // yellow
        TokenType::OpenBracket => "33",      // yellow
        TokenType::CloseBracket => "33",     // yellow

        TokenType::At => "95",         // bright magenta
        TokenType::BitwiseNot => "96", // bright cyan
        TokenType::Question => "95",   // bright magenta
        TokenType::Dollar => "95",     // bright magenta
        TokenType::Bang => "95",       // bright magenta

        // multiplication
        TokenType::Multiply => "93",           // bright yellow
        TokenType::WrappingMultiply => "93",   // bright yellow
        TokenType::SaturatingMultiply => "93", // bright yellow
        TokenType::Divide => "93",             // bright yellow
        TokenType::Remainder => "93",          // bright yellow

        // addition
        TokenType::Add => "93",                // bright yellow
        TokenType::WrappingAdd => "93",        // bright yellow
        TokenType::SaturatingAdd => "93",      // bright yellow
        TokenType::Subtract => "93",           // bright yellow
        TokenType::WrappingSubtract => "93",   // bright yellow
        TokenType::SaturatingSubtract => "93", // bright yellow

        // shift
        TokenType::ShiftLeft => "96",           // bright cyan
        TokenType::SaturatingShiftLeft => "96", // bright cyan
        TokenType::ShiftRight => "96",          // bright cyan

        // bitwise
        TokenType::BitwiseAnd => "96", // bright cyan
        TokenType::BitwiseXor => "96", // bright cyan
        TokenType::BitwiseOr => "96",  // bright cyan

        // logical
        TokenType::LogicalAnd => "94", // bright blue
        TokenType::LogicalOr => "94",  // bright blue

        // comparison
        TokenType::GreaterThan => "92",        // bright green
        TokenType::LessThan => "92",           // bright green
        TokenType::GreaterThanOrEqual => "92", // bright green
        TokenType::LessThanOrEqual => "92",    // bright green
        TokenType::Equal => "92",              // bright green
        TokenType::NotEqual => "92",           // bright green

        // assignment
        TokenType::Assign => "91",                    // bright red
        TokenType::BitwiseOrAssign => "91",           // bright red
        TokenType::BitwiseAndAssign => "91",          // bright red
        TokenType::BitwiseXorAssign => "91",          // bright red
        TokenType::ShiftLeftAssign => "91",           // bright red
        TokenType::SaturatingShiftLeftAssign => "91", // bright red
        TokenType::ShiftRightAssign => "91",          // bright red
        TokenType::AddAssign => "91",                 // bright red
        TokenType::WrappingAddAssign => "91",         // bright red
        TokenType::SaturatingAddAssign => "91",       // bright red
        TokenType::SubtractAssign => "91",            // bright red
        TokenType::WrappingSubtractAssign => "91",    // bright red
        TokenType::SaturatingSubtractAssign => "91",  // bright red
        TokenType::MultiplyAssign => "91",            // bright red
        TokenType::WrappingMultiplyAssign => "91",    // bright red
        TokenType::SaturatingMultiplyAssign => "91",  // bright red
        TokenType::DivideAssign => "91",              // bright red
        TokenType::RemainderAssign => "91",           // bright red
        TokenType::LogicalAndAssign => "91",          // bright red
        TokenType::LogicalOrAssign => "91",           // bright red
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
