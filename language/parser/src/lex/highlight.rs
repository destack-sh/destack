use std::sync::Arc;

use destack_ast::SemanticType;
use destack_source::{Color, File, SourceColorizer};

use super::Lexer;

/// Map a SemanticType to a Color for syntax highlighting.
///
/// When `bright` is true, uses full brightness (BrightWhite for neutral tokens).
/// When false, uses dimmed colors (White for neutral tokens).
pub fn semantic_type_to_color(semantic_type: SemanticType, bright: bool) -> Option<Color> {
    let neutral = if bright {
        Color::BrightWhite
    } else {
        Color::White
    };
    match semantic_type {
        SemanticType::Whitespace => None,
        SemanticType::Identifier => Some(neutral),
        SemanticType::Keyword => Some(Color::Magenta),
        SemanticType::LiteralNumbery => Some(Color::Cyan),
        SemanticType::LiteralStringy => Some(Color::Green),
        SemanticType::Parenthesis => Some(neutral),
        SemanticType::Symbol => Some(neutral),
        SemanticType::Operator => Some(Color::Yellow),
        SemanticType::Doc => Some(Color::BrightGreen),
        SemanticType::Comment => Some(Color::BrightBlue),
        SemanticType::Modifier => Some(Color::Magenta),
        SemanticType::Macro => Some(Color::BrightMagenta),
        SemanticType::Type => Some(Color::Cyan),
        SemanticType::Function => Some(Color::BrightYellow),
        SemanticType::Parameter => Some(Color::BrightCyan),
        SemanticType::Argument => Some(neutral),
        SemanticType::Variable => Some(neutral),
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

    // bounds check
    if start >= source.len() || end > source.len() || start >= end {
        return source.get(start..end).unwrap_or("").to_string();
    }

    let (tokens, _eof) = Lexer::lex(file.id, source, Default::default());

    let mut result = String::with_capacity((end - start) * 2);
    let mut last_end = start;

    for token_span in &tokens {
        let tok_start = token_span.span.start as usize;
        let tok_end = token_span.span.end as usize;

        // skip tokens entirely before our slice
        if tok_end <= start {
            continue;
        }

        // stop if we've passed our slice
        if tok_start >= end {
            break;
        }

        // clip token to slice boundaries
        let visible_start = tok_start.max(start);
        let visible_end = tok_end.min(end);

        // add any gap before this token (within our slice)
        if visible_start > last_end {
            result.push_str(&source[last_end..visible_start]);
        }

        // get the visible portion of this token
        let token_str = &source[visible_start..visible_end];

        // get semantic type and color
        let semantic_type = SemanticType::from_token(file, token_span);
        if let Some(color) = semantic_type_to_color(semantic_type, bright) {
            result.push_str(&color.apply(token_str));
        } else {
            result.push_str(token_str);
        }

        last_end = visible_end;
    }

    // add any remaining text within the slice
    if last_end < end {
        result.push_str(&source[last_end..end]);
    }

    result
}

/// Colorize source code using lexical tokens and semantic types.
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
        // should contain ANSI escape codes
        assert!(colorized.contains("\x1b["));
        // should still contain the original text
        assert!(colorized.contains("let"));
        assert!(colorized.contains("42"));
    }
}
