use destack_ast::SemanticType;
use destack_source::{Color, File};

use super::Lexer;

/// Map a SemanticType to a Color for syntax highlighting.
pub fn semantic_type_to_color(semantic_type: SemanticType) -> Option<Color> {
    match semantic_type {
        SemanticType::Whitespace => None,
        SemanticType::Identifier => Some(Color::White),
        SemanticType::Keyword => Some(Color::Magenta),
        SemanticType::LiteralNumbery => Some(Color::Cyan),
        SemanticType::LiteralStringy => Some(Color::Green),
        SemanticType::Parenthesis => Some(Color::White),
        SemanticType::Symbol => Some(Color::White),
        SemanticType::Operator => Some(Color::Yellow),
        SemanticType::Doc => Some(Color::BrightGreen),
        SemanticType::Comment => Some(Color::BrightBlue),
        SemanticType::Modifier => Some(Color::Magenta),
        SemanticType::Macro => Some(Color::BrightMagenta),
        SemanticType::Type => Some(Color::Cyan),
        SemanticType::Function => Some(Color::BrightYellow),
        SemanticType::Parameter => Some(Color::BrightCyan),
        SemanticType::Argument => Some(Color::White),
        SemanticType::Variable => Some(Color::White),
    }
}

/// Colorize source code using lexical tokens and semantic types.
/// Returns a string with ANSI color codes.
pub fn colorize_source(file: &File) -> String {
    let source = file.text();
    let (tokens, _eof) = Lexer::lex(file.id, source, Default::default());

    let mut result = String::with_capacity(source.len() * 2);
    let mut last_end: usize = 0;

    for token_span in &tokens {
        let start = token_span.span.start as usize;
        let end = token_span.span.end as usize;

        // retain any gap between tokens (shouldn't happen, but just in case)
        if start > last_end {
            result.push_str(&source[last_end..start]);
        }

        // get the source text for this token
        let token_str = &source[start..end];

        // get semantic type and color
        let semantic_type = SemanticType::from_token(file, token_span);
        if let Some(color) = semantic_type_to_color(semantic_type) {
            result.push_str(&color.apply(token_str));
        } else {
            result.push_str(token_str);
        }

        last_end = end;
    }

    // add any remaining text after the last token
    if last_end < source.len() {
        result.push_str(&source[last_end..]);
    }

    result
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
