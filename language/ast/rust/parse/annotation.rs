//! Annotation parsing.

use crate::{Doc, NodeId, ParseResult, Parser};
use dyst_language_source::Span;
use dyst_language_token::{TokenSpan, TokenType};

// nocheckin todo!: docs in statements/expressions and associate with items
//  (also keep regular comments for pretty printing? some side-table AST?)

impl<'a> Parser<'a> {
    /// Attach annotations to respective AST nodes.
    /// Must be called *after* parsing.
    ///
    /// Annotations of the same type are merged,
    ///  and annotations are attached to nodes immediately following them.
    /// One newline is ignored (both between annotations and between annotations and nodes).
    pub fn process_annotations(&mut self) {
        let mut combined_tokens = Vec::with_capacity(self.tokens.len() + self.trivia_tokens.len());
        combined_tokens.extend(self.tokens.clone());
        combined_tokens.extend(self.trivia_tokens.clone());
        combined_tokens.sort_by_key(|token| token.span.start);

        for ele in combined_tokens {
            println!("{:?}", ele);
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
    }
}
