use std::str::FromStr;
use std::sync::Arc;

use destack_core::Color;
use destack_dir::{Keyword, TokenLiteral, TokenSpan, TokenType};
use destack_source::{File, SourceColorizer};

use super::Lexer;

/// Map a token to a color for terminal syntax highlighting.
///
/// This is purely lexical - it doesn't resolve symbols.
/// For resolved semantic highlighting, use workspace's semantic tokens.
fn token_to_color(file: &File, token: &TokenSpan, bright: bool) -> Option<Color> {
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
            let span_str = file.get_span_str(token.span).unwrap_or_default();
            if Keyword::from_str(span_str).is_ok() {
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
/// Takes a file, byte range (start, end), and brightness flag. When `bright`
/// is true, neutral tokens use BrightWhite; when false, they use dim White.
pub fn colorize_slice(file: &File, start_byte: u32, end_byte: u32, bright: bool) -> String {
    let source = file.text();
    let start = start_byte as usize;
    let end = end_byte as usize;

    if start >= source.len() || end > source.len() || start >= end {
        return source.get(start..end).unwrap_or("").to_string();
    }

    let (tokens, _side_tokens, _eof) = Lexer::lex(Arc::new(file.clone()), Default::default());

    let mut result = String::with_capacity((end - start) * 2);
    let mut last_end = start;

    for token_span in &tokens {
        let tok_start = token_span.span.start as usize;
        let tok_end = token_span.span.end as usize;

        if tok_end <= start {
            continue;
        }
        if tok_start >= end {
            break;
        }

        let visible_start = tok_start.max(start);
        let visible_end = tok_end.min(end);

        if visible_start > last_end {
            result.push_str(&source[last_end..visible_start]);
        }

        let token_str = &source[visible_start..visible_end];

        if let Some(color) = token_to_color(file, token_span, bright) {
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

/// Colorize source code using lexical tokens.
/// Returns a string with ANSI color codes (using bright colors).
pub fn colorize_source(file: &File) -> String {
    colorize_slice(file, 0, file.text().len() as u32, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use destack_source::{FileType, Uri};

    #[test]
    fn test_colorize_source_simple() {
        let file = File::from_text(
            destack_source::FileId::new(0),
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
}
