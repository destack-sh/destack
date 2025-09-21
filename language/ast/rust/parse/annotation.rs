//! Annotation parsing.

use std::borrow::Cow;

use dyst_language_source::Span;
use dyst_language_token::{TokenSpan, TokenType};

use crate::{
    Annotation, AnnotationPosition, Blank, Comment, CommentStyle, Doc, DocStyle, NodeId, Parser,
};

const ANNOTATION_TOKEN_TYPES: [TokenType; 5] = [
    TokenType::Newline,
    TokenType::LineComment,
    TokenType::DocLineComment,
    TokenType::BlockComment,
    TokenType::DocBlockComment,
];

impl<'a> Parser<'a> {
    /// Attach all annotations to respective AST nodes.
    /// Must be called *after* primary parsing.
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

        // build annotation groups
        let mut current_token_type: TokenType = tokens[0].token.r#type;
        let mut current_token_group: Vec<TokenSpan> = Vec::new();
        for (i, token) in tokens.iter().enumerate() {
            if token.token.r#type != current_token_type {
                if ANNOTATION_TOKEN_TYPES.contains(&current_token_type)
                    && (current_token_type != TokenType::Newline || current_token_group.len() > 1)
                {
                    self.attach_annotation_group(
                        (i - current_token_group.len()) as u32,
                        current_token_type,
                        &tokens,
                        &current_token_group,
                    );
                }
                current_token_type = token.token.r#type;
                current_token_group.clear();
            }
            current_token_group.push(*token);
        }
        if ANNOTATION_TOKEN_TYPES.contains(&current_token_type)
            && (current_token_type != TokenType::Newline || current_token_group.len() > 1)
        {
            self.attach_annotation_group(
                (tokens.len() - current_token_group.len()) as u32,
                current_token_type,
                &tokens,
                &current_token_group,
            );
        }
    }

    /// Attach an annotation group to the AST.
    fn attach_annotation_group(
        &mut self,
        token_idx: u32,
        token_type: TokenType,
        tokens: &[TokenSpan],
        group: &[TokenSpan],
    ) -> Option<NodeId<Annotation>> {
        debug_assert!(ANNOTATION_TOKEN_TYPES.contains(&token_type));
        debug_assert!(!group.is_empty());

        let start_token = group[0];
        let end_token = group[group.len() - 1];
        let span = Span::new(
            start_token.span.source,
            start_token.span.start,
            end_token.span.end,
        );

        // find the target to attach to
        let prev_token = if token_idx > 0 {
            tokens.get(token_idx as usize - 1)
        } else {
            None
        };
        let is_line_suffix = self.is_span_same_line(start_token.span, end_token.span)
            && prev_token.is_some()
            && prev_token.unwrap().token.r#type != TokenType::Newline;
        let (target_token, target_node) = {
            if is_line_suffix {
                let target_token = prev_token.unwrap();
                let target_node = self.find_node_ending_at(target_token);
                (target_token, target_node)
            } else {
                let target_token = tokens.get(token_idx as usize + group.len()).unwrap();
                let target_node = self.find_node_starting_at(target_token);
                (target_token, target_node)
            }
        };
        let target_node = target_node?;

        // create the annotation
        let annotation_id = match token_type {
            TokenType::Newline => {
                let lines = group.len() as u32 - 1;
                let blank = self.tree.allocate(
                    Blank {
                        position: AnnotationPosition::BlockPrefix,
                        lines,
                    },
                    span,
                );
                self.tree.allocate(Annotation::Blank(blank), span)
            }
            TokenType::LineComment => {
                let string = self.intern_string(self.clean_annotation_string_group(group));
                let comment = self.tree.allocate(
                    Comment {
                        string,
                        position: AnnotationPosition::BlockPrefix,
                        style: CommentStyle::Line,
                    },
                    span,
                );
                self.tree.allocate(Annotation::Comment(comment), span)
            }
            TokenType::BlockComment => {
                let string = self.intern_string(self.clean_annotation_string_group(group));
                let comment = self.tree.allocate(
                    Comment {
                        string,
                        position: AnnotationPosition::BlockPrefix,
                        style: CommentStyle::Block,
                    },
                    span,
                );
                self.tree.allocate(Annotation::Comment(comment), span)
            }
            TokenType::DocLineComment => {
                let string = self.intern_string(self.clean_annotation_string_group(group));
                let doc = self.tree.allocate(
                    Doc {
                        string,
                        position: AnnotationPosition::BlockPrefix,
                        style: DocStyle::Line,
                    },
                    span,
                );
                self.tree.allocate(Annotation::Doc(doc), span)
            }
            TokenType::DocBlockComment => {
                let string = self.intern_string(self.clean_annotation_string_group(group));
                let doc = self.tree.allocate(
                    Doc {
                        string,
                        position: AnnotationPosition::BlockPrefix,
                        style: DocStyle::Block,
                    },
                    span,
                );
                self.tree.allocate(Annotation::Doc(doc), span)
            }
            _ => panic!("unexpected token type: {token_type:?}"),
        };

        // attach the annotation to the target node
        self.tree.append_annotation(target_node.idx, annotation_id);

        Some(annotation_id)
    }

    /// Clean a group of annotation tokens into their inner string.
    fn clean_annotation_string_group(&self, group: &[TokenSpan]) -> String {
        group
            .iter()
            .map(|token| self.clean_annotation_string(*token))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Clean an annotation token into its inner string
    /// Newlines are preserved, first leading space is stripped.
    fn clean_annotation_string(&self, token: TokenSpan) -> Cow<'_, str> {
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
        Annotation, AnnotationPosition, Blank, Block, BlockFormat, Comment, CommentStyle, Doc,
        DocStyle, Function, Statement, Struct, StructField, Type, assert_node, assert_string,
    };

    /// Blanks (>2 successive newlines) are annotations and should be attached to the next node.
    /// Like any other annotation, if no next or containing node is found, attach to previous node as suffix.
    #[test]
    fn test_attach_blanks_to_lets() {
        let mut test = TestParser::new("\n\nlet A = 1\nlet B = 2\n\n");
        let mut parser = test.parser();
        let statements = parser.eat_block_body(BlockFormat::Implicit).unwrap();
        parser.process_annotations();

        assert_eq!(statements.len(), 2);

        // A has one prefix block blank
        let a_blanks = parser.tree.get_blanks_for(statements[0].id);
        assert_eq!(a_blanks.len(), 1);
        assert_node!(parser.tree, a_blanks[0], Blank { position, lines } => {
            assert_eq!(*position, AnnotationPosition::BlockPrefix);
            assert_eq!(*lines, 2);
        });

        // B has one prefix block blank and one postfix block blank
        // (the postfix blank after B because there is nothing else to attach to)
        let b_blanks = parser.tree.get_blanks_for(statements[1].id);
        assert_eq!(b_blanks.len(), 2);
        assert_node!(parser.tree, b_blanks[0], Blank { position, lines } => {
            assert_eq!(*position, AnnotationPosition::BlockPrefix);
            assert_eq!(*lines, 1);
        });
        let b_blanks = parser.tree.get_blanks_for(statements[1].id);
        assert_eq!(b_blanks.len(), 1);
        assert_node!(parser.tree, b_blanks[0], Blank { position, lines } => {
            assert_eq!(*position, AnnotationPosition::BlockPostfix);
            assert_eq!(*lines, 1);
        });
    }

    /// Annotations inside an empty node should be treated as infix within the containing node.
    #[test]
    fn test_attach_comments_infix_in_block() {
        let mut test = TestParser::new("function main() {\n\t// block comment, infix\n}");
        let mut parser = test.parser();

        let function = parser.eat_function(None).unwrap();
        parser.process_annotations();

        // function main
        assert_node!(parser.tree, function, Function { body, .. } => {
            assert_node!(parser.tree, body.unwrap(), Block { statements, .. } => {
                assert_eq!(statements.len(), 0);
                // doc block infix
                // comment, infix
                let annotations = parser.tree.get_annotations_for(function.id);
                assert_eq!(annotations.len(), 1);
                assert_node!(parser.tree, annotations[0], Annotation::Comment(node) => {
                    assert_node!(parser.tree, *node, Comment { string, position, style } => {
                        assert_eq!(parser.get_string(*string), "block comment, infix");
                        assert_eq!(*position, AnnotationPosition::BlockInfix);
                        assert_eq!(*style, CommentStyle::Line);
                    });
                });
            });
        });
    }

    /// Mixed annotations should also be attached to the node they are attached to.
    /// Successive annotations of the same type should be merged as relevant.
    /// Within a containing node (like the struct), the comment at the end should be treated as infix
    ///  since we don't have a following node to attach to (but do have a containing node).
    #[test]
    fn test_attach_mixed_annotations_to_struct() {
        let mut test = TestParser::new(
            r"
/// doc, floating

/// doc, struct
/// doc, struct continued
struct Floof {
    /// doc, struct field
    /// doc, struct field continued
    a: int32 // doc, struct field infix

    /// random doc
}
        ",
        );
        let mut parser = test.parser();
        let statements = parser.eat_block_body(BlockFormat::Implicit).unwrap();
        parser.process_annotations();

        // struct Floof
        assert_eq!(statements.len(), 1);
        assert_node!(parser.tree, statements[0], Statement::Struct(node) => {
            assert_node!(parser.tree, *node, Struct { fields, .. } => {
                let annotations = parser.tree.get_annotations_for(node.id);
                assert_eq!(annotations.len(), 4);
                // doc block prefix, floating
                assert_node!(parser.tree, annotations[0], Annotation::Doc(node) => {
                    assert_node!(parser.tree, *node, Doc { string, position, style } => {
                        assert_eq!(parser.get_string(*string), "doc, floating");
                        assert_eq!(*position, AnnotationPosition::BlockPrefix);
                        assert_eq!(*style, DocStyle::Line);
                    });
                });
                // blank block prefix
                assert_node!(parser.tree, annotations[1], Annotation::Blank(node) => {
                    assert_node!(parser.tree, *node, Blank { position, lines } => {
                        assert_eq!(*position, AnnotationPosition::BlockPrefix);
                        assert_eq!(*lines, 1);
                    });
                });
                // doc block prefix
                // struct\ndoc, struct continued
                assert_node!(parser.tree, annotations[2], Annotation::Doc(node) => {
                    assert_node!(parser.tree, *node, Doc { string, position, style } => {
                        assert_eq!(parser.get_string(*string), "doc, struct");
                        assert_eq!(*position, AnnotationPosition::BlockPrefix);
                        assert_eq!(*style, DocStyle::Line);
                    });
                });

                // a: int32
                assert_eq!(fields.len(), 1);
                assert_node!(parser.tree, fields[0], StructField { name, .. } => {
                    assert_string!(parser.session, name.unwrap(), "a");
                    let annotations = parser.tree.get_annotations_for(fields[0].id);
                    assert_eq!(annotations.len(), 2);

                    // doc block prefix
                    // struct field\ndoc, struct field continued
                    assert_node!(parser.tree, annotations[0], Annotation::Doc(node) => {
                        assert_node!(parser.tree, *node, Doc { string, position, style } => {
                            assert_eq!(parser.get_string(*string), "doc, struct field\ndoc, struct field continued");
                            assert_eq!(*position, AnnotationPosition::BlockPrefix);
                            assert_eq!(*style, DocStyle::Line);
                        });
                    });

                    // doc line suffix
                    // doc, struct field infix
                    assert_node!(parser.tree, annotations[1], Annotation::Doc(node) => {
                        assert_node!(parser.tree, *node, Doc { string, position, style } => {
                            assert_eq!(parser.get_string(*string), "doc, struct field infix");
                            assert_eq!(*position, AnnotationPosition::LineSuffix);
                            assert_eq!(*style, DocStyle::Line);
                        });
                    });
                });

                // doc block infix
                assert_node!(parser.tree, annotations[3], Annotation::Doc(node) => {
                    assert_node!(parser.tree, *node, Doc { string, position, style } => {
                        assert_eq!(parser.get_string(*string), "random doc");
                        assert_eq!(*position, AnnotationPosition::BlockInfix);
                        assert_eq!(*style, DocStyle::Line);
                    });
                });
            });
        })
    }
}
