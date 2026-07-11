use std::sync::Arc;

use destack_core::Color;
use destack_dir::{TokenLiteral, TokenSpan, TokenType};
use destack_source::{File, SourceColorizer};

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

    // combine both ordered token partitions for complete source highlighting
    let mut result = Lexer::lex_file(Arc::new(file.clone()));
    result.tokens.extend(result.side_tokens);
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
    use super::colorize_source;
    use destack_core::Color;
    use destack_source::{File, FileId, FileType, Uri};

    #[test]
    fn test_colorize_source_simple() {
        let file = File::from_text(
            FileId::new(0),
            "test.ds".to_string(),
            Uri::from_string("test.ds"),
            None,
            FileType::Destack,
            "let x = 42;".to_string(),
        );
        let colorized = colorize_source(&file);
        assert!(colorized.contains("\x1b["));
        assert!(colorized.contains("let"));
        assert!(colorized.contains("42"));
    }

    /// Colorize retained side tokens together with semantic tokens.
    #[test]
    fn test_colorize_source_comments() {
        let file = File::from_text(
            FileId::new(0),
            "test.ds".to_string(),
            Uri::from_string("test.ds"),
            None,
            FileType::Destack,
            "const value = 1; // note".to_string(),
        );
        let colorized = colorize_source(&file);
        let expected_comment = Color::BrightBlue.apply("// note");

        assert!(colorized.contains(&expected_comment));
    }
}
