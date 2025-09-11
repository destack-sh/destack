//! Comment and doc parsing.

use crate::Parser;

// nocheckin: docs in statements/expressions and associate with items
//  (also keep regular comments for pretty printing? some side-table AST?)

impl<'a> Parser<'a> {
    /// Process documentation and comments to respective AST nodes.
    pub fn process_comments_and_docs(&mut self) {
        assert!(
            self.pos() == self.tokens.len(),
            "process_comments_and_docs must be called at the end of parsing"
        );
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
