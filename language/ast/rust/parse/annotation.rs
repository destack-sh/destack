//! Annotation parsing.

use dyst_language_token::{TokenSpan, TokenType};

use crate::{AnnotationPosition, AnnotationStyle, Comment, Doc, Parser};

const ANNOTATION_TOKEN_TYPES: [TokenType; 4] = [
    TokenType::LineComment,
    TokenType::DocLineComment,
    TokenType::BlockComment,
    TokenType::DocBlockComment,
];

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
        let mut tokens = Vec::with_capacity(self.tokens.len());
        tokens.extend(self.tokens.clone());
        tokens.extend(
            self.trivia_tokens
                .iter()
                .filter(|token| token.token.r#type != TokenType::Whitespace),
        );
        tokens.sort_by_key(|token| token.span.start);
        if tokens.is_empty() {
            return;
        }

        // flush group
        let mut flush_group = |start: u32, end: u32, group_type: TokenType| {
            // check if it's an annotation
            let separator = match group_type {
                // line annotations with newlines
                TokenType::LineComment => "\n",
                TokenType::DocLineComment => "\n",
                // block annotations with spaces
                TokenType::BlockComment => " ",
                TokenType::DocBlockComment => " ",
                _ => return, // not a comment, bail
            };

            // get the next token to attach to
            let next_token = {
                // next token is valid target
                if let Some(next_token) = tokens.get(end as usize)
                    && next_token.token.r#type != TokenType::Newline
                    && !ANNOTATION_TOKEN_TYPES.contains(&next_token.token.r#type)
                {
                    next_token
                }
                // next next token is valid target with newline
                else if let Some(next_token) = tokens.get((end) as usize)
                    && next_token.token.r#type == TokenType::Newline
                    && let Some(next_next_token) = tokens.get((end + 1) as usize)
                    && next_next_token.token.r#type != TokenType::Newline
                    && !ANNOTATION_TOKEN_TYPES.contains(&next_next_token.token.r#type)
                {
                    next_token
                }
                // no valid target token, bail
                else {
                    return;
                }
            };

            // get the node to attach to
            let Some((next_node_id, _)) = self.tree.map.get_enclosing_span(next_token.span.start)
            else {
                return; // no valid target node, bail
            };

            // merge content
            let merged_content = tokens[start as usize..end as usize]
                .iter()
                .map(|token| self.clean_annotation(*token))
                .collect::<Vec<_>>()
                .join(separator);
            let string_id = self.strings.intern(merged_content);

            // append annotation node
            match group_type {
                TokenType::LineComment => {
                    let annotation_node_id = self.tree.allocate(
                        Comment {
                            string: string_id,
                            style: AnnotationStyle::Line,
                            position: AnnotationPosition::Prefix,
                        },
                        next_token.span,
                    );
                    self.tree.append_comment(next_node_id, annotation_node_id);
                }
                TokenType::DocLineComment => {
                    let annotation_node_id = self.tree.allocate(
                        Doc {
                            string: string_id,
                            style: AnnotationStyle::Line,
                            position: AnnotationPosition::Prefix,
                        },
                        next_token.span,
                    );
                    self.tree.append_doc(next_node_id, annotation_node_id);
                }
                TokenType::BlockComment => {
                    let annotation_node_id = self.tree.allocate(
                        Comment {
                            string: string_id,
                            style: AnnotationStyle::Block,
                            position: AnnotationPosition::Prefix,
                        },
                        next_token.span,
                    );
                    self.tree.append_comment(next_node_id, annotation_node_id);
                }
                TokenType::DocBlockComment => {
                    let annotation_node_id = self.tree.allocate(
                        Doc {
                            string: string_id,
                            style: AnnotationStyle::Block,
                            position: AnnotationPosition::Prefix,
                        },
                        next_token.span,
                    );
                    self.tree.append_doc(next_node_id, annotation_node_id);
                }
                _ => panic!("unexpected token type: {group_type:?}"),
            }
        };

        // process tokens in groups
        let mut current_start: u32 = 0;
        let mut current_type: TokenType = tokens[0].token.r#type;
        for (i, token) in tokens.iter().enumerate() {
            if current_type != token.token.r#type {
                flush_group(current_start, i as u32, current_type);
                current_start = i as u32;
                current_type = token.token.r#type;
            }
        }
        if current_start != tokens.len() as u32 {
            flush_group(current_start, tokens.len() as u32, current_type);
        }
    }

    /// Clean an annotation token into its inner string (newlines are preserved).
    /// Panics if the token is not an annotation (!).
    #[inline]
    fn clean_annotation(&self, token: TokenSpan) -> &str {
        match token.token.r#type {
            TokenType::LineComment => {
                // strip leading `//`
                let string = self.source.get_span_str(token.span);
                string.strip_prefix("//").unwrap_or(string)
            }
            TokenType::BlockComment => {
                // strip leading `/*` and trailing `*/`
                let string = self.source.get_span_str(token.span);
                string
                    .strip_prefix("/*")
                    .unwrap_or(string)
                    .strip_suffix("*/")
                    .unwrap_or(string)
            }
            TokenType::DocLineComment => {
                // strip leading `///`
                let string = self.source.get_span_str(token.span);
                string.strip_prefix("///").unwrap_or(string)
            }
            TokenType::DocBlockComment => {
                // strip leading `/**` and trailing `*/`
                let string = self.source.get_span_str(token.span);
                string
                    .strip_prefix("/**")
                    .unwrap_or(string)
                    .strip_suffix("*/")
                    .unwrap_or(string)
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
