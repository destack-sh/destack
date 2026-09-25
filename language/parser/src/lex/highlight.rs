use std::sync::Arc;

use tspp_core::Color;
use tspp_dir::{Comment, CommentKind, Token, TokenLiteral, TokenSpan, TokenType};
use tspp_source::{File, FileId, SourceColorizer};

use super::{Lexer, classify_keyword};

/// Return the terminal highlight color for one token.
///
/// This is purely lexical and does not resolve symbols.
fn classify_token_color(file: &File, token: &TokenSpan, bright: bool) -> Option<Color> {
    let neutral = if bright {
        Color::BrightWhite
    } else {
        Color::White
    };

    match token.token.ty() {
        TokenType::Newline | TokenType::Whitespace | TokenType::Unknown | TokenType::End => None,

        TokenType::LineComment | TokenType::BlockComment => Some(Color::BrightBlue),
        TokenType::DocLineComment | TokenType::DocBlockComment => Some(Color::BrightGreen),

        TokenType::Identifier | TokenType::InvalidIdentifier | TokenType::UnknownLiteralPrefix => {
            let span_str = file.span_str(token.span);
            if classify_keyword(span_str).is_some() {
                Some(Color::Magenta)
            } else {
                Some(neutral)
            }
        }

        TokenType::Literal => {
            if let Some(literal) = token.token.literal() {
                match literal {
                    TokenLiteral::Boolean { .. }
                    | TokenLiteral::Int { .. }
                    | TokenLiteral::Float { .. } => Some(Color::Cyan),
                    TokenLiteral::Character { .. }
                    | TokenLiteral::String { .. }
                    | TokenLiteral::RegexString { .. }
                    | TokenLiteral::TreeString => Some(Color::Green),
                }
            } else {
                Some(Color::Green)
            }
        }

        TokenType::TemplateStringStart
        | TokenType::TemplateStringMiddle
        | TokenType::TemplateStringEnd
        | TokenType::TemplateString => Some(Color::Green),

        // operators and punctuation
        _ => Some(Color::Yellow),
    }
}

/// Convert one canonical comment into its lexical token shape.
fn tokenize_comment(comment: Comment, file_id: FileId) -> TokenSpan {
    // select the lexical comment kind
    let token_type = match (comment.kind, comment.is_documentation()) {
        (CommentKind::Line, false) => TokenType::LineComment,
        (CommentKind::Line, true) => TokenType::DocLineComment,
        (CommentKind::SingleLineBlock | CommentKind::MultiLineBlock, false) => {
            TokenType::BlockComment
        }
        (CommentKind::SingleLineBlock | CommentKind::MultiLineBlock, true) => {
            TokenType::DocBlockComment
        }
    };

    // reconstruct the source token span
    let token = Token::simple(
        token_type,
        comment.span.start,
        comment.span.end - comment.span.start,
    );

    TokenSpan::new(token, file_id)
}

/// Create a source colorizer suitable for use with diagnostic printing.
pub fn source_colorizer() -> SourceColorizer {
    Arc::new(colorize_slice)
}

/// Colorize a slice of source code by byte range.
///
/// Takes a file, byte range, and brightness flag.
/// Neutral tokens use bright white when `bright` is true and dim white otherwise.
pub fn colorize_slice(file: &File, start_byte: u32, end_byte: u32, bright: bool) -> String {
    let source = file.text();
    let start = start_byte as usize;
    let end = end_byte as usize;

    if start >= source.len() || end > source.len() || start >= end {
        return source.get(start..end).unwrap_or("").to_string();
    }

    // combine semantic tokens and canonical comments for complete highlighting
    let mut result = Lexer::lex_file(Arc::new(file.clone()));
    result.tokens.extend(
        result
            .comments
            .into_iter()
            .map(|comment| tokenize_comment(comment, file.id)),
    );
    result.tokens.sort_unstable_by_key(|token| token.span.start);
    let tokens = result.tokens;

    let mut result = String::with_capacity((end - start) * 2);
    let mut last_end = start;

    for token_span in &tokens {
        let token_start = token_span.span.start as usize;
        let token_end = token_span.span.end as usize;

        if token_end <= start {
            continue;
        }
        if token_start >= end {
            break;
        }

        let visible_start = token_start.max(start);
        let visible_end = token_end.min(end);

        if visible_start > last_end {
            result.push_str(&source[last_end..visible_start]);
        }

        let token_str = &source[visible_start..visible_end];

        if let Some(color) = classify_token_color(file, token_span, bright) {
            result.push_str(&color.apply(token_str));
        } else {
            result.push_str(token_str);
        }

        last_end = visible_end;
    }

    if last_end < end {
        result.push_str(&source[last_end..end]);
    }

    result
}

/// Colorize source code using bright ANSI colors.
pub fn colorize_source(file: &File) -> String {
    colorize_slice(file, 0, file.text().len() as u32, true)
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use tspp_core::Color;
    use tspp_source::{
        Diagnostic, DiagnosticCollection, DiagnosticLabel, DiagnosticTarget, File, FileId,
        FileType, PrintOptions, Span, Uri, print_diagnostics,
    };

    use super::{colorize_source, source_colorizer};

    #[test]
    fn test_colorize_source_simple() {
        let file = File::from_text(
            FileId::new(0),
            "test.tspp".to_string(),
            Uri::from_string("test.tspp"),
            None,
            FileType::Tspp,
            "let x = 42;".to_string(),
        )
        .expect("test source should load");
        let colorized = colorize_source(&file);
        assert!(colorized.contains("\x1b["));
        assert!(colorized.contains("let"));
        assert!(colorized.contains("42"));
    }

    /// Highlight keyword and literal tokens in printed diagnostic source rows.
    #[test]
    fn test_highlight_printed_diagnostic_source_rows() {
        let file_id = FileId::new(1);
        let file = Arc::new(
            File::from_text(
                file_id,
                "<test>".to_string(),
                Uri::from_string("<test>"),
                None,
                FileType::Tspp,
                "const answer: int32 = \"text\";".to_string(),
            )
            .expect("test source should load"),
        );
        let span = Span::new(file_id, 22, 28);
        let diagnostic = Diagnostic::error(
            "not-assignable",
            "mismatch",
            DiagnosticLabel::message(file.blob(), DiagnosticTarget::Span(span), "mismatch"),
        );
        let diagnostics = DiagnosticCollection::from_diagnostics(vec![diagnostic]);

        // capture the printed lines with the lexical colorizer active
        let lines = Arc::new(Mutex::new(Vec::<String>::new()));
        let output = Arc::clone(&lines);
        let writer = Arc::new(move |line: &str| {
            output.lock().unwrap().push(line.to_string());
        });
        let file_for_id = |id| {
            if id == file_id {
                Some(Arc::clone(&file))
            } else {
                None
            }
        };
        let options = PrintOptions::new()
            .with_colorizer(source_colorizer())
            .with_skip_summary(true)
            .with_line_writer(writer);
        print_diagnostics(&file_for_id, &diagnostics, options).unwrap();
        let rendered = lines.lock().unwrap().join("\n");

        // the const keyword renders magenta and the string literal green
        assert!(
            rendered.contains("\u{1b}[35mconst"),
            "rendered: {rendered:?}"
        );
        assert!(
            rendered.contains("\u{1b}[32m\"text\""),
            "rendered: {rendered:?}"
        );
    }

    /// Colorize retained comments together with semantic tokens.
    #[test]
    fn test_colorize_source_comments() {
        let file = File::from_text(
            FileId::new(0),
            "test.tspp".to_string(),
            Uri::from_string("test.tspp"),
            None,
            FileType::Tspp,
            "const value = 1; // note".to_string(),
        )
        .expect("test source should load");
        let colorized = colorize_source(&file);
        let expected_comment = Color::BrightBlue.apply("// note");

        assert!(colorized.contains(&expected_comment));
    }
}
