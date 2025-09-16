//! Annotation parsing.

use std::borrow::Cow;

use dyst_language_source::Span;
use dyst_language_token::{TokenSpan, TokenType};

use crate::{ANNOTATION_TOKEN_TYPES, AnnotationPosition, AnnotationStyle, Comment, Doc, Parser};

impl<'a> Parser<'a> {
    /// Attach annotations to respective AST nodes.
    /// Must be called *after* parsing.
    ///
    /// Annotations of the same type are merged,
    ///  and annotations are attached to nodes immediately following them.
    /// One newline is ignored (both between annotations and between annotations and nodes).
    /// Whitespace is completely ignored (except newlines, as usual).
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
        let mut flush_annotation_group = |start: u32, end: u32, group_type: TokenType| {
            // check if it's an annotation
            let separator = match group_type {
                // line annotations with newlines
                TokenType::LineComment => "\n",
                TokenType::DocLineComment => "\n",
                // block annotations with spaces
                TokenType::BlockComment => " ",
                TokenType::DocBlockComment => " ",
                // not an annotation, bail
                _ => panic!("unexpected token type: {group_type:?}"),
            };

            // get the target token to attach to
            let target_token = {
                // next token is valid target
                if let Some(next_token) = tokens.get(end as usize)
                    && next_token.token.r#type != TokenType::Newline
                    && !ANNOTATION_TOKEN_TYPES.contains(&next_token.token.r#type)
                {
                    next_token
                }
                // next next token is valid target (with newline in between)
                else if let Some(next_token) = tokens.get((end) as usize)
                    && next_token.token.r#type == TokenType::Newline
                    && let Some(next_next_token) = tokens.get((end + 1) as usize)
                    && next_next_token.token.r#type != TokenType::Newline
                    && !ANNOTATION_TOKEN_TYPES.contains(&next_next_token.token.r#type)
                {
                    next_next_token
                }
                // no valid target token, bail
                else {
                    return;
                }
            };

            // get the biggest, "lowest" next node to attach to that starts right after the annotation
            // (we automatically get the "lowest" because indices are created bottom-up)
            let mut enclosing_spans = self
                .tree
                .map
                .get_enclosing_spans(target_token.span.start, target_token.span.end - 1)
                .into_iter()
                .filter(|span| target_token.span.start == span.span.start)
                .collect::<Vec<_>>();
            enclosing_spans.sort_by_key(|span| -(span.length as i64));
            let Some(target_node_id) = enclosing_spans.into_iter().next().map(|span| span.idx)
            else {
                return; // no valid target node, bail
            };

            // merge content
            let merged_content = tokens[start as usize..end as usize]
                .iter()
                .filter(|token| token.token.r#type == group_type)
                .map(|token| self.clean_annotation(*token))
                .collect::<Vec<_>>()
                .join(separator);
            let string_id = self.intern_string(&merged_content);
            let merged_span = Span::new(
                target_token.span.source,
                tokens[start as usize].span.start,
                tokens[end as usize].span.end,
            );

            // append annotation node
            match group_type {
                TokenType::LineComment => {
                    let annotation_node_id = self.tree.allocate(
                        Comment {
                            string: string_id,
                            style: AnnotationStyle::Line,
                            position: AnnotationPosition::Prefix,
                        },
                        merged_span,
                    );
                    self.tree.append_comment(target_node_id, annotation_node_id);
                }
                TokenType::DocLineComment => {
                    let annotation_node_id = self.tree.allocate(
                        Doc {
                            string: string_id,
                            style: AnnotationStyle::Line,
                            position: AnnotationPosition::Prefix,
                        },
                        merged_span,
                    );
                    self.tree.append_doc(target_node_id, annotation_node_id);
                }
                TokenType::BlockComment => {
                    let annotation_node_id = self.tree.allocate(
                        Comment {
                            string: string_id,
                            style: AnnotationStyle::Block,
                            position: AnnotationPosition::Prefix,
                        },
                        merged_span,
                    );
                    self.tree.append_comment(target_node_id, annotation_node_id);
                }
                TokenType::DocBlockComment => {
                    let annotation_node_id = self.tree.allocate(
                        Doc {
                            string: string_id,
                            style: AnnotationStyle::Block,
                            position: AnnotationPosition::Prefix,
                        },
                        merged_span,
                    );
                    self.tree.append_doc(target_node_id, annotation_node_id);
                }
                _ => panic!("unexpected token type: {group_type:?}"),
            }
        };

        // process tokens in groups
        let mut current_start: u32 = 0;
        let mut current_type: TokenType = tokens[0].token.r#type;
        for (i, token) in tokens.iter().enumerate() {
            if current_type != token.token.r#type {
                // skip one newline
                if token.token.r#type == TokenType::Newline
                    && let Some(prev_token) = tokens.get(i - 1)
                    && prev_token.token.r#type == current_type
                    && let Some(next_token) = tokens.get(i + 1)
                    && next_token.token.r#type == current_type
                {
                    continue;
                }
                // flush annotation group
                if ANNOTATION_TOKEN_TYPES.contains(&current_type) {
                    flush_annotation_group(current_start, i as u32, current_type);
                }
                // reset current group
                current_start = i as u32;
                current_type = token.token.r#type;
            }
        }
        if current_start != tokens.len() as u32 && ANNOTATION_TOKEN_TYPES.contains(&current_type) {
            flush_annotation_group(current_start, tokens.len() as u32, current_type);
        }
    }

    /// Clean an annotation token into its inner string (newlines are preserved).
    fn clean_annotation(&self, token: TokenSpan) -> Cow<'_, str> {
        let string = self.source.get_span_str(token.span);

        let annotation = match token.token.r#type {
            // line comments
            TokenType::LineComment => {
                // strip leading `//`
                string.strip_prefix("//").unwrap_or(string)
            }
            TokenType::DocLineComment => {
                // strip leading `///`
                string.strip_prefix("///").unwrap_or(string)
            }
            // block comments
            TokenType::BlockComment => {
                // strip leading `/*` and trailing `*/`
                string
                    .strip_prefix("/*")
                    .unwrap_or(string)
                    .strip_suffix("*/")
                    .unwrap_or(string)
            }
            TokenType::DocBlockComment => {
                // strip leading `/**` and trailing `*/`
                string
                    .strip_prefix("/**")
                    .unwrap_or(string)
                    .strip_suffix("*/")
                    .unwrap_or(string)
            }
            _ => panic!("unexpected token type: {:?}", token.token.r#type),
        };

        // strip leading space on every line
        if annotation.contains('\n') {
            // only allocate for multiline strings
            let cleaned: String = annotation
                .lines()
                .map(|line| line.strip_prefix(' ').unwrap_or(line))
                .collect::<Vec<_>>()
                .join("\n");
            Cow::Owned(cleaned)
        } else {
            Cow::Borrowed(annotation.strip_prefix(' ').unwrap_or(annotation))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::parse::tests::TestParser;
    use crate::{
        AnnotationPosition, AnnotationStyle, BlockFormat, Comment, Doc, Enum, EnumField,
        Expression, Statement, Struct, StructField, assert_node, assert_string,
    };

    #[test]
    fn test_attach_docs_to_struct() {
        let mut test = TestParser::new(
            r"
/// doc, floating

/// doc, struct
/// doc, struct continued
struct Floof {
    /// doc, struct field
    /// doc, struct field continued
    a: int32
}
        ",
        );
        let mut parser = test.parser();
        let statements = parser.eat_block_body(BlockFormat::Implicit).unwrap();
        parser.process_annotations();

        // struct Floof
        assert_eq!(statements.len(), 1);
        assert_node!(parser.tree, statements[0], Statement::Struct(node) => {
            let docs = parser.tree.get_docs_for(node.id);
            assert_eq!(docs.len(), 1);

            // doc, struct + doc, struct continued
            assert_node!(parser.tree, docs[0], Doc { string, style, position } => {
                let string = parser.get_string(*string);
                assert_eq!(string, "doc, struct\ndoc, struct continued");
                assert_eq!(*style, AnnotationStyle::Line);
                assert_eq!(*position, AnnotationPosition::Prefix);
            });

            // struct Floof
            assert_node!(parser.tree, *node, Struct { fields, .. } => {
                assert_eq!(fields.len(), 1);

                // a: int32
                assert_node!(parser.tree, fields[0], StructField { name, .. } => {
                    assert_eq!(parser.get_string(name.unwrap()), "a");

                    // doc, struct field + doc, struct field continued
                    let docs = parser.tree.get_docs_for(fields[0].id);
                    assert_eq!(docs.len(), 1);
                    // doc, struct field + doc, struct field continued
                    assert_node!(parser.tree, docs[0], Doc { string, style, position } => {
                        let string = parser.get_string(*string);
                        assert_eq!(string, "doc, struct field\ndoc, struct field continued");
                        assert_eq!(*style, AnnotationStyle::Line);
                        assert_eq!(*position, AnnotationPosition::Prefix);
                    });
                });
            });

        })
    }

    #[test]
    fn test_attach_docs_to_enum() {
        let mut test = TestParser::new(
            r"
enum Floof {
    /// A
    A
}
        ",
        );
        let mut parser = test.parser();
        parser.eat_newline().unwrap();
        let statements = parser.eat_block_body(BlockFormat::Implicit).unwrap();
        parser.process_annotations();

        // enum Floof
        assert_eq!(statements.len(), 1);
        assert_node!(parser.tree, statements[0], Statement::Enum(node) => {
            // enum Floof
            assert_node!(parser.tree, *node, Enum { fields, .. } => {
                assert_eq!(fields.len(), 1);
                // A
                assert_node!(parser.tree, fields[0], EnumField { name, .. } => {
                    assert_string!(parser.session, *name, "A");
                    let docs = parser.tree.get_docs_for(fields[0].id);
                    assert_eq!(docs.len(), 1);
                    // A + doc, enum field
                    assert_node!(parser.tree, docs[0], Doc { string, .. } => {
                        let string = parser.get_string(*string);
                        assert_eq!(string, "A");
                    });
                });
            });
        })
    }

    #[test]
    fn test_attach_docs_to_enum_with_values() {
        let mut test = TestParser::new(
            r"
/// doc, enum
/// doc, enum continued
enum Floof {
    /// A
    /// doc, enum field
    A = 1
    
    /// B
    /// doc, enum field
    B = 2

    C

    /// D
    D = 3
}
        ",
        );
        let mut parser = test.parser();
        let statements = parser.eat_block_body(BlockFormat::Implicit).unwrap();
        parser.process_annotations();

        // enum Floof
        assert_eq!(statements.len(), 1);
        assert_node!(parser.tree, statements[0], Statement::Enum(node) => {
            // enum Floof
            assert_node!(parser.tree, *node, Enum { fields, .. } => {
                assert_eq!(fields.len(), 4);

                // A
                assert_node!(parser.tree, fields[0], EnumField { name, .. } => {
                    assert_string!(parser.session, *name, "A");
                    let docs = parser.tree.get_docs_for(fields[0].id);
                    assert_eq!(docs.len(), 1);
                    // A + doc, enum field
                    assert_node!(parser.tree, docs[0], Doc { string, .. } => {
                        let string = parser.get_string(*string);
                        assert_eq!(string, "A\ndoc, enum field");
                    });
                });

                // B
                assert_node!(parser.tree, fields[1], EnumField { name, .. } => {
                    assert_string!(parser.session, *name, "B");
                    let docs = parser.tree.get_docs_for(fields[1].id);
                    assert_eq!(docs.len(), 1);
                    // B + doc, enum field
                    assert_node!(parser.tree, docs[0], Doc { string, .. } => {
                        let string = parser.get_string(*string);
                        assert_eq!(string, "B\ndoc, enum field");
                    });
                });

                // C
                assert_node!(parser.tree, fields[2], EnumField { name, .. } => {
                    assert_string!(parser.session, *name, "C");
                    // <nothing>
                    let docs = parser.tree.get_docs_for(fields[2].id);
                    assert_eq!(docs.len(), 0);
                });

                // D
                assert_node!(parser.tree, fields[3], EnumField { name, .. } => {
                    assert_string!(parser.session, *name, "D");
                    let docs = parser.tree.get_docs_for(fields[3].id);
                    assert_eq!(docs.len(), 1);
                    // D + doc, enum field
                    assert_node!(parser.tree, docs[0], Doc { string, .. } => {
                        let string = parser.get_string(*string);
                        assert_eq!(string, "D");
                    });
                });
            });
        })
    }

    #[test]
    fn test_attach_comments_to_let() {
        let mut test = TestParser::new(
            r"
// comment, floating

// comment 1
// comment 1.1
let x = 1 + 1
        ",
        );
        let mut parser = test.parser();
        let statements = parser.eat_block_body(BlockFormat::Implicit).unwrap();
        parser.process_annotations();

        // let x = 1 + 1 (let node)
        assert_eq!(statements.len(), 1);
        assert_node!(parser.tree, statements[0], Statement::Expression(expr_node) => {
            // let x = 1 + 1
            assert_node!(parser.tree, *expr_node, Expression::Let(let_node) => {
                let comments = parser.tree.get_comments_for(let_node.id);
                assert_eq!(comments.len(), 1);
                // comment 1 + comment 1.1
                assert_node!(parser.tree, comments[0], Comment { string, style, position } => {
                    let string = parser.get_string(*string);
                    assert_eq!(string, "comment 1\ncomment 1.1");
                    assert_eq!(*style, AnnotationStyle::Line);
                    assert_eq!(*position, AnnotationPosition::Prefix);
                });
            });

        });
    }

    #[test]
    fn test_attach_comments_to_call() {
        let mut test = TestParser::new(
            r"
// comment 2
func() /* comment 3, detached */
        ",
        );
        let mut parser = test.parser();
        let statements = parser.eat_block_body(BlockFormat::Implicit).unwrap();
        parser.process_annotations();

        // func() (expression node)
        assert_node!(parser.tree, statements[0], Statement::Expression(expr_node) => {
            // comments should be on the biggest, "lowest" next node
            let comments = parser.tree.get_comments_for(expr_node.id);
            assert_eq!(comments.len(), 1);

            // comment 2 (comment 3 should not be attached)
            assert_node!(parser.tree, comments[0], Comment { string, style, position } => {
                let string = parser.get_string(*string);
                assert_eq!(string, "comment 2");
                assert_eq!(*style, AnnotationStyle::Line);
                assert_eq!(*position, AnnotationPosition::Prefix);
            });
        });
    }
}
