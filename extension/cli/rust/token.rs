use destack_terminal::{CommandArguments, console, table};
use dyst_ast::{TokenSpan, TokenType};
use dyst_parser::{is_semantic, tokenize_with_spans};

use crate::source::read_source;

const DEFAULT_MAX_LEXEME_LEN: usize = 80;

pub const HELP: &str = r"Tokenize source with spans.
	--file <path>      Read input from file
	--string <string>  Read input from provided string
	--no-color         Disable ANSI colors
	--no-pager         Print directly instead of use less -R
    --only-semantic    Only show semantic tokens
    --no-whitespace    Don't show whitespace tokens
	--max-lexeme <n>   Truncate lexeme preview to n chars";

/// Tokenize input and show a colored table with locations.
pub fn run(ctx: CommandArguments) -> i32 {
    let source = match read_source(&ctx) {
        Ok(source) => source,
        Err(error) => {
            console::error(&format!("Read input error: {error}"));
            return 1;
        }
    };

    let use_color = !ctx.flag("no-color");
    let use_pager = !ctx.flag("no-pager");
    let max_tokeneme_len = ctx
        .option("max-lexeme")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(DEFAULT_MAX_LEXEME_LEN);
    let only_semantic = ctx.flag("only-semantic");
    let no_whitespace = ctx.flag("no-whitespace");

    let headers = vec![
        "Index".to_string(),
        "Line".to_string(),
        "Col".to_string(),
        "Type".to_string(),
        "Lexeme".to_string(),
        "Length".to_string(),
    ];
    let mut rows: Vec<Vec<String>> = Vec::new();
    let filter = if only_semantic {
        is_semantic
    } else if no_whitespace {
        |t| t != TokenType::Whitespace
    } else {
        |_| true
    };
    let (tokens, _) = tokenize_with_spans(source.id, &source.content);
    let tokens: Vec<TokenSpan> = tokens
        .into_iter()
        .filter(|token| filter(token.token.ty))
        .collect();

    for (index, token) in tokens.iter().enumerate() {
        let start_offset = token.span.start as usize;
        let end_offset = token.span.end as usize;
        let len = end_offset - start_offset;

        let mut line = 1;
        let mut col = 1;
        for ch in source.content[..start_offset].chars() {
            if ch == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }

        let slice = &source.content[start_offset..end_offset.min(source.len as usize)];

        let kind_str = format_token(token.token.ty, use_color);
        let lexeme_preview = truncate_tokeneme(slice, max_tokeneme_len, token.token.ty, use_color);
        let index_str = if use_color {
            console::color(&index.to_string(), "35")
        } else {
            index.to_string()
        };
        let line_str = if use_color {
            console::color(&line.to_string(), "36")
        } else {
            line.to_string()
        };
        let col_str = if use_color {
            console::color(&col.to_string(), "34")
        } else {
            col.to_string()
        };
        let len_str = if use_color {
            console::color(&len.to_string(), "2")
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

    let secondary_headers = vec![
        "[0".to_string(),
        "[1".to_string(),
        "[1".to_string(),
        "".to_string(),
        "preview".to_string(),
        "bytes".to_string(),
    ];
    let secondary: Option<&[String]> = Some(&secondary_headers);

    let table_str = table::render_table(&headers, &rows, true, 2, secondary);
    let framed_table_str = console::frame(&table_str, Some("Lexer Tokens"), 2);

    let previous_mode = console::color_mode();
    if !use_color {
        console::disable_color();
    }

    let display_result = if use_pager {
        console::page_or_print(&framed_table_str)
    } else {
        console::print(&framed_table_str);
        Ok(())
    };

    if let Err(error) = display_result {
        console::error(&format!("failed to display tokens: {error}"));
    }

    if !use_color {
        console::set_color_mode(previous_mode);
    }

    0
}

fn format_token(kind: TokenType, use_color: bool) -> String {
    let base = format_token_kind(kind);
    if !use_color {
        return base;
    }
    let color = get_token_color(kind);
    console::color(&base, color)
}

fn format_token_kind(kind: TokenType) -> String {
    format!("{kind:?}")
}

fn get_token_color(kind: TokenType) -> &'static str {
    match kind {
        TokenType::Newline | TokenType::Whitespace | TokenType::End => "2",
        TokenType::Unknown => "31",
        TokenType::LineComment | TokenType::BlockComment => "2",
        TokenType::DocLineComment | TokenType::DocBlockComment => "32",
        TokenType::Identifier => "36",
        TokenType::InvalidIdentifier => "31",
        TokenType::UnknownLiteralPrefix => "31",
        TokenType::Literal => "35",
        TokenType::Wildcard => "37",
        TokenType::Colon => "37",
        TokenType::Semicolon => "37",
        TokenType::Comma => "37",
        TokenType::Dot => "37",
        TokenType::Range => "37",
        TokenType::RangeWide => "37",
        TokenType::Arrow => "95",
        TokenType::ArrowWide => "95",
        TokenType::OpenParenthesis => "33",
        TokenType::CloseParenthesis => "33",
        TokenType::OpenBrace => "33",
        TokenType::CloseBrace => "33",
        TokenType::OpenBracket => "33",
        TokenType::CloseBracket => "33",
        TokenType::At => "95",
        TokenType::Tag => "95",
        TokenType::ElementwiseNot => "96",
        TokenType::Maybe => "95",
        TokenType::Coalesce => "95",
        TokenType::Virtual => "95",
        TokenType::Not => "95",
        TokenType::Multiply => "93",
        TokenType::WrappingMultiply => "93",
        TokenType::SaturatingMultiply => "93",
        TokenType::Divide => "93",
        TokenType::Remainder => "93",
        TokenType::Add => "93",
        TokenType::WrappingAdd => "93",
        TokenType::SaturatingAdd => "93",
        TokenType::Subtract => "93",
        TokenType::WrappingSubtract => "93",
        TokenType::SaturatingSubtract => "93",
        TokenType::Increment => "93",
        TokenType::Decrement => "93",
        TokenType::ShiftLeft => "96",
        TokenType::SaturatingShiftLeft => "96",
        TokenType::ElementwiseAnd => "96",
        TokenType::ElementwiseXor => "96",
        TokenType::ElementwiseOr => "96",
        TokenType::LogicalAnd => "94",
        TokenType::LogicalOr => "94",
        TokenType::GreaterThan => "92",
        TokenType::LessThan => "92",
        TokenType::GreaterThanOrEqual => "92",
        TokenType::LessThanOrEqual => "92",
        TokenType::Equal => "92",
        TokenType::EqualWide => "92",
        TokenType::NotEqual => "92",
        TokenType::NotEqualWide => "92",
        TokenType::Assign => "91",
        TokenType::ElementwiseOrAssign => "91",
        TokenType::ElementwiseAndAssign => "91",
        TokenType::ElementwiseXorAssign => "91",
        TokenType::ShiftLeftAssign => "91",
        TokenType::SaturatingShiftLeftAssign => "91",
        TokenType::ShiftRightAssign => "91",
        TokenType::AddAssign => "91",
        TokenType::WrappingAddAssign => "91",
        TokenType::SaturatingAddAssign => "91",
        TokenType::SubtractAssign => "91",
        TokenType::WrappingSubtractAssign => "91",
        TokenType::SaturatingSubtractAssign => "91",
        TokenType::MultiplyAssign => "91",
        TokenType::WrappingMultiplyAssign => "91",
        TokenType::SaturatingMultiplyAssign => "91",
        TokenType::DivideAssign => "91",
        TokenType::RemainderAssign => "91",
        TokenType::LogicalAndAssign => "91",
        TokenType::LogicalOrAssign => "91",
    }
}

fn truncate_tokeneme(s: &str, max_len: usize, token_type: TokenType, color: bool) -> String {
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

    match token_type {
        TokenType::Whitespace => console::color(&visible, "2"),
        _ => console::color(&visible, get_token_color(token_type)),
    }
}
