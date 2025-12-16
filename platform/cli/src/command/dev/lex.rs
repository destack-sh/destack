use clap::Args;
use destack_ast::{SemanticType, TokenSpan, TokenType};
use destack_parser::{Lexer, is_semantic};
use destack_source::{File, LanguageType};

use crate::common::{ProgramArgs, SingleInputArgs, load_source};
use crate::console;
use crate::console::table;

#[derive(Args, Debug, Clone)]
pub struct LexArgs {
    /// Input arguments.
    #[command(flatten)]
    pub input: SingleInputArgs,

    /// Only show semantic tokens.
    #[arg(long)]
    pub only_semantic: bool,

    /// Don't show whitespace tokens.
    #[arg(long)]
    pub no_whitespace: bool,

    /// Truncate lexeme preview to n chars.
    #[arg(long, default_value_t = 80)]
    pub max_lexeme: usize,

    /// The program options.
    #[command(flatten)]
    pub program: ProgramArgs,
}

/// Tokenize input and show a colored table with locations.
pub fn run(args: &LexArgs) -> i32 {
    let session = args.program.setup();

    // get the program from the session
    let program = session
        .programs
        .iter()
        .next()
        .map(|entry| entry.value().clone())
        .expect("session should have a program after setup");

    // determine input source
    let source = match args.input.to_source() {
        Ok(s) => s,
        Err(e) => {
            console::error(&format!("error: {e}"));
            return 1;
        }
    };

    // load source
    let file = match load_source(&program, &source) {
        Ok(f) => f,
        Err(e) => {
            console::error(&format!("error: {e}"));
            return 1;
        }
    };
    let text = file.text();

    let use_color = true;
    let max_tokeneme_len = args.max_lexeme;
    let only_semantic = args.only_semantic;
    let no_whitespace = args.no_whitespace;

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
    let language_type = LanguageType::from(file.ty);
    let (tokens, _) = Lexer::lex(file.id, text, language_type);
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
        for ch in text[..start_offset].chars() {
            if ch == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }

        let kind_str = format_token(&file, token, use_color);
        let lexeme_preview = truncate_tokeneme(&file, token, max_tokeneme_len, use_color);
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
    console::print(&framed_table_str);
    0
}

fn format_token(source: &File, token: &TokenSpan, use_color: bool) -> String {
    let base = format_token_kind(token.token.ty);
    if !use_color {
        return base;
    }
    let color = get_token_color(source, token);
    console::color(&base, color)
}

fn format_token_kind(kind: TokenType) -> String {
    format!("{kind:?}")
}

/// Compute the appropriate ANSI color code for a token's semantic type.
fn get_token_color(source: &File, token: &TokenSpan) -> &'static str {
    let semantic_type = SemanticType::from_token(source, token);
    match semantic_type {
        SemanticType::Whitespace => "0",
        SemanticType::Identifier => "0",
        SemanticType::Keyword => "94",
        SemanticType::LiteralNumbery => "93",
        SemanticType::LiteralStringy => "92",
        SemanticType::Parenthesis => "96",
        SemanticType::Symbol => "35",
        SemanticType::Operator => "96",
        SemanticType::Doc => "92",
        SemanticType::Comment => "2",
        SemanticType::Modifier => "95;1",
        SemanticType::Macro => "95",
        SemanticType::Type => "36",
        SemanticType::Function => "94;1",
        SemanticType::Parameter => "97",
        SemanticType::Argument => "93",
        SemanticType::Variable => "36;2",
    }
}

fn truncate_tokeneme(file: &File, token: &TokenSpan, max_len: usize, use_color: bool) -> String {
    let mut out = String::new();
    for ch in file.get_span_str(token.span).unwrap_or_default().chars() {
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

    if !use_color {
        return visible;
    }

    let color = get_token_color(file, token);
    console::color(&visible, color)
}
