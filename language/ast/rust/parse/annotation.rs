//! Annotation parsing.

use dyst_language_token::{TokenSpan, TokenType};

use crate::Parser;

impl<'a> Parser<'a> {
    /// Attach annotations to respective AST nodes.
    /// Must be called *after* parsing.
    ///
    /// Whitespace is completely ignored (except newlines, as usual).
    /// Annotations of the same type are merged,
    ///  and annotations are attached to nodes immediately following them.
    /// One newline is ignored (both between annotations and between annotations and nodes).
    /// Annotations without corresponding following nodes are "free floating"
    ///  (we allocate them but don't append them to any nodes).
    pub fn process_annotations(&mut self) {
        let mut combined_tokens = Vec::with_capacity(self.tokens.len());
        combined_tokens.extend(self.tokens.clone());
        combined_tokens.extend(
            self.trivia_tokens
                .iter()
                .filter(|token| token.token.r#type != TokenType::Whitespace),
        );
        combined_tokens.sort_by_key(|token| token.span.start);

        // nocheckin todo!: Parser.process_annotations
        let mut current_group_start: Option<u32> = None;
        let mut current_group_type: Option<TokenType> = None;
        for (i, token) in combined_tokens.iter().enumerate() {
            let prev_token = combined_tokens.get(i - 1);
            let next_token = combined_tokens.get(i + 1);

            if current_group_start.is_none() {
                current_group_start = Some(i as u32);
                current_group_type = Some(token.token.r#type);
            }

            if token.token.r#type == TokenType::LineComment
                || token.token.r#type == TokenType::BlockComment
                || token.token.r#type == TokenType::DocLineComment
                || token.token.r#type == TokenType::DocBlockComment
            {

                // ...
            } else {
                // ...
            }
        }
    }

    /// Clean an annotation token into its inner string (newlines are preserved).
    fn clean_annotation(&self, token: TokenSpan) -> &str {
        match token.token.r#type {
            TokenType::LineComment => {
                // strip leading `//`
                let string = self.source.get_span_str(token.span);
                let string = string.strip_prefix("//").unwrap_or(string);
                string
            }
            TokenType::BlockComment => {
                // strip leading `/*` and trailing `*/`
                let string = self.source.get_span_str(token.span);
                let string = string.strip_prefix("/*").unwrap_or(string);
                let string = string.strip_suffix("*/").unwrap_or(string);
                string
            }
            TokenType::DocLineComment => {
                // strip leading `///`
                let string = self.source.get_span_str(token.span);
                let string = string.strip_prefix("///").unwrap_or(string);
                string
            }
            TokenType::DocBlockComment => {
                // strip leading `/**` and trailing `*/`
                let string = self.source.get_span_str(token.span);
                let string = string.strip_prefix("/**").unwrap_or(string);
                let string = string.strip_suffix("*/").unwrap_or(string);
                string
            }
            _ => panic!("unexpected token type: {:?}", token.token.r#type),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::BlockFormat;
    use crate::parse::tests::TestParser;

    #[test]
    fn test_attach_docs() {
        let test = TestParser::new(
            r"
/// doc, floating

/// doc, new
/// continued
struct Floof {
    /// doc, struct field
    a: int32
}
        ",
        );
        let mut parser = test.parser();
        let statements = parser.eat_block_body(BlockFormat::Implicit).unwrap();
        parser.process_annotations();
    }

    #[test]
    fn test_attach_comments() {
        let test = TestParser::new(
            r"
// comment, floating

// comment 1
// comment 1.1
let x = 1 + 1

// comment 2
func() /* comment 3, detached */
        ",
        );
        let mut parser = test.parser();
        let statements = parser.eat_block_body(BlockFormat::Implicit).unwrap();
        parser.process_annotations();
    }
}
