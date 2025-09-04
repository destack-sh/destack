use destack_language_token::{TokenSpan, TokenType};

use crate::{Doc, NodeId, ParseResult, Parser};

impl<'a> Parser<'a> {
    /// Peek a doc comment (incl. `///` or `/**`).
    #[inline]
    pub fn peek_doc(&self) -> ParseResult<&TokenSpan> {
        if let Ok(doc_line_comment) = self.peek_token(TokenType::DocLineComment) {
            Ok(doc_line_comment)
        } else {
            self.peek_token(TokenType::DocBlockComment)
        }
    }

    /// Eat a doc comment (incl. `///` or `/**`).
    /// Successive doc comments are concatenated.
    ///
    /// Examples:
    /// ```
    /// /// Simple doc.
    ///
    /// /// This is a doc comment
    /// /// And it will be merged with this one
    ///
    /// /** inline doc comment */ /** followed by another (also merged) */
    /// ```
    pub fn eat_doc(&mut self) -> ParseResult<NodeId<Doc>> {
        let start = self.mark();

        // eat all doc comments
        let mut doc_tokens: Vec<TokenSpan> = vec![];
        loop {
            if self.peek_token(TokenType::DocLineComment).is_ok() {
                let doc_line_comment = self.eat_token(TokenType::DocLineComment)?;
                doc_tokens.push(*doc_line_comment);
                // eat any separating newlines so we can detect the next doc token
                let _ = self.eat_newlines_maybe();
            } else if self.peek_token(TokenType::DocBlockComment).is_ok() {
                let doc_block_comment = self.eat_token(TokenType::DocBlockComment)?;
                doc_tokens.push(*doc_block_comment);
                // eat any separating newlines so we can detect the next doc token
                let _ = self.eat_newlines_maybe();
            } else {
                break;
            }
        }

        // merge docs as one normalized string
        // - preserve internal newlines
        // - remove leading whitespace per line
        // - ensure exactly one newline between successive tokens
        let mut parts: Vec<String> = Vec::with_capacity(doc_tokens.len());
        for token in &doc_tokens {
            let raw = self.get_span_str(token.span);
            // drop trailing newlines so we can control separation
            let without_trailing_newlines = raw.trim_end_matches(['\r', '\n']);
            // clean leading whitespace on each line, preserve internal newlines
            let cleaned = without_trailing_newlines
                .lines()
                .map(|line| line.trim_start())
                .collect::<Vec<&str>>()
                .join("\n");
            parts.push(cleaned);
        }
        let doc_string_id = self.strings.intern(parts.join("\n"));

        // make doc node
        let doc_id = self.tree.allocate(
            Doc {
                string: doc_string_id,
            },
            self.get_span_from(start),
        );
        Ok(doc_id)
    }

    /// Eat a doc comment iff. it exists (incl. `///` or `/**`).
    /// Successive doc comments are concatenated.
    #[inline]
    pub fn eat_doc_maybe(&mut self) -> ParseResult<Option<NodeId<Doc>>> {
        if self.peek_doc().is_ok() {
            let doc = self.eat_doc()?;
            Ok(Some(doc))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use destack_language_token::{SourceFile, tokenize_semantic};

    use crate::Parser;

    /// Parse a single line doc comment and keep exact text (incl. prefix)
    #[test]
    fn test_parse_single_line_doc_comment() {
        let input = r###"
/// Simple doc
"###;
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        parser.eat_newline().unwrap();

        // /// Simple doc
        let doc_id = parser.eat_doc().unwrap();
        let doc = parser.tree.get(doc_id);
        assert_eq!(doc.string, parser.strings.intern("/// Simple doc"));
    }

    /// Merge successive line doc comments with a newline between tokens
    #[test]
    fn test_parse_multiple_line_doc_comments_merge() {
        let input = r###"
/// First line
/// Second line
"###;
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        parser.eat_newline().unwrap();

        // /// First line + /// Second line
        let doc_id = parser.eat_doc().unwrap();
        let doc = parser.tree.get(doc_id);
        assert_eq!(
            doc.string,
            parser.strings.intern("/// First line\n/// Second line")
        );
    }

    /// Parse a single block doc comment and keep exact text (incl. delimiters)
    #[test]
    fn test_parse_single_block_doc_comment() {
        let input = r###"
/** inline block doc */
"###;
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        parser.eat_newline().unwrap();

        // /** inline block doc */
        let doc_id = parser.eat_doc().unwrap();
        let doc = parser.tree.get(doc_id);
        assert_eq!(doc.string, parser.strings.intern("/** inline block doc */"));
    }

    /// eat_doc_maybe returns None if no doc comment is present
    #[test]
    fn test_eat_doc_maybe_none_when_absent() {
        let input = r###"
x
"###;
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        parser.eat_newline().unwrap();

        let maybe_doc = parser.eat_doc_maybe().unwrap();
        assert!(maybe_doc.is_none());
    }

    /// Merge mixed successive doc comments (line + block + line)
    #[test]
    fn test_merge_mixed_doc_comments() {
        let input = r###"
/// A
/** B */
/// C
"###;
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        parser.eat_newline().unwrap();

        // /// A + /** B */ + /// C
        let doc_id = parser.eat_doc().unwrap();
        let doc = parser.tree.get(doc_id);
        assert_eq!(doc.string, parser.strings.intern("/// A\n/** B */\n/// C"));
    }
}
