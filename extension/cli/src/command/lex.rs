use clap::{ArgGroup, Args};
use dyst_ast::{SemanticType, TokenSpan, TokenType};
use dyst_parser::{Lexer, is_semantic};
use dyst_source::{File, FileRegistry, FileSystem, LanguageOptions, PhysicalFileSystem};

use crate::command::{SourceArg, get_string_or_file};
use crate::console;
use crate::console::table;

#[derive(Args, Debug, Clone)]
#[command(group(
    ArgGroup::new("source")
        .args(["file", "string"])
        .required(true)
        .multiple(false)
))]
pub struct LexArgs {
    /// Read input from file.
    #[arg(long)]
    pub file: Option<String>,

    /// Read input from provided string.
    #[arg(long)]
    pub string: Option<String>,

    /// Parse a file with the given format (default: ds).
    #[arg(long = "type", alias = "format", value_name = "FORMAT")]
    pub format: Option<String>,

    /// Only show semantic tokens.
    #[arg(long)]
    pub only_semantic: bool,

    /// Don't show whitespace tokens.
    #[arg(long)]
    pub no_whitespace: bool,

    /// Truncate lexeme preview to n chars.
    #[arg(long, default_value_t = 80)]
    pub max_lexeme: usize,
}

/// Tokenize input and show a colored table with locations.
pub fn run(args: &LexArgs) -> i32 {
    let fs = PhysicalFileSystem::new();
    let files = FileRegistry::new();
    let file_id = match get_string_or_file(
        &fs,
        &files,
        SourceArg {
            file: args.file.as_deref(),
            string: args.string.as_deref(),
            format: args.format.as_deref(),
        },
    ) {
        Ok(Some(file_id)) => file_id,
        Ok(None) => {
            console::error("error: failed to resolve source input");
            return 1;
        }
        Err(e) => {
            console::error(&format!("error: {e}"));
            return 1;
        }
    };
    let file = files.get(file_id);
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
    let language = LanguageOptions::default();
    let (tokens, _) = Lexer::lex(file.id, text, language);
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
/// Use None for semantic types we do not wish to color.
/// Pick visually distinct colors for each semantic class where possible.
fn get_token_color(source: &File, token: &TokenSpan) -> &'static str {
    let semantic_type = SemanticType::from_token(source, token);
    match semantic_type {
        // whitespace and identifier get no color
        SemanticType::Whitespace => "0",
        SemanticType::Identifier => "0",
        // blue for keywords
        SemanticType::Keyword => "94",
        // yellow for number literals
        SemanticType::LiteralNumbery => "93",
        // green for string literals
        SemanticType::LiteralStringy => "92",
        // cyan for parentheses
        SemanticType::Parenthesis => "96",
        // magenta for symbols
        SemanticType::Symbol => "35",
        // bright cyan for operators
        SemanticType::Operator => "96",
        // bright green for doc comments
        SemanticType::Doc => "92",
        // dim for comments
        SemanticType::Comment => "2",
        // bold magenta for modifiers
        SemanticType::Modifier => "95;1",
        // bright magenta for macros
        SemanticType::Macro => "95",
        // cyan for types
        SemanticType::Type => "36",
        // bright blue for functions
        SemanticType::Function => "94;1",
        // bright white for parameters
        SemanticType::Parameter => "97",
        // yellow for arguments
        SemanticType::Argument => "93",
        // dim cyan for variables
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
