//! Annotation parsing.

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
        let mut combined_tokens = Vec::with_capacity(self.tokens.len() + self.trivia_tokens.len());
        combined_tokens.extend(self.tokens.clone());
        combined_tokens.extend(self.trivia_tokens.clone());
        combined_tokens.sort_by_key(|token| token.span.start);

        // nocheckin todo!: Parser.process_annotations
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
