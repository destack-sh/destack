//! Annotation parsing.

use std::borrow::Cow;

use dyst_language_source::Span;
use dyst_language_token::{TokenSpan, TokenType};

use crate::{
    Annotation, AnnotationPosition, Blank, Comment, CommentStyle, Doc, DocStyle, NodeId,
    NodeSearch, Parser,
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
    pub(crate) fn attach_annotations(&mut self) {
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
                let start_token = current_token_group[0];
                let prev_token = if i > 1 { Some(tokens[i - 2]) } else { None };
                let is_line_postfix = current_token_group.len() == 1
                    && self.is_same_line(start_token.span, start_token.span)
                    && prev_token.is_some()
                    && self.is_same_line(start_token.span, prev_token.unwrap().span)
                    && prev_token.unwrap().token.r#type != TokenType::Newline;

                // skip up to one newline in-between non-blank annotations
                //  (unless the current token is a suffix comment)
                if token.token.r#type == TokenType::Newline
                    && current_token_type != TokenType::Newline
                    && let Some(next_token) = tokens.get(i + 1)
                    && next_token.token.r#type == current_token_type
                    && !is_line_postfix
                {
                    continue;
                }

                // flush group before different annotation type
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

        // flush group at the end
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

    /// Find the annotation position for a given token index and group.
    fn find_annotation_position(
        &self,
        token_idx: u32,
        tokens: &[TokenSpan],
        group: &[TokenSpan],
    ) -> Option<(AnnotationPosition, u32)> {
        debug_assert!(!group.is_empty());

        let start_token = group[0];
        let end_token = group[group.len() - 1];
        let prev_token = if token_idx > 0 {
            Some(tokens[token_idx as usize - 1])
        } else {
            None
        };
        let next_token = tokens.get(token_idx as usize + group.len());
        let is_one_line = self.is_same_line(start_token.span, end_token.span);
        let is_line_comment = start_token.token.r#type == TokenType::LineComment
            || start_token.token.r#type == TokenType::DocLineComment;

        // line prefix or postfix (or block infix if we have nothing)
        // entire span must be on one line together with the previous/next token
        if is_one_line
            && let Some(prev_token) = prev_token
            && prev_token.token.r#type != TokenType::Newline
            && self.is_same_line(prev_token.span, end_token.span)
        {
            // line postfix has directly preceding node that ends at the start token
            if let Some(target_node_id) = self
                .find_node_ending_at(
                    &prev_token.span,
                    if is_line_comment {
                        NodeSearch::BiggestOuter
                    } else {
                        NodeSearch::SmallestInner
                    },
                )
                .map(|span| span.idx)
            {
                // line postfix boundary if next token is newline (or end)
                if next_token.is_none()
                    || next_token.unwrap().token.r#type == TokenType::Newline
                    || next_token.unwrap().token.r#type == TokenType::End
                {
                    return Some((AnnotationPosition::LinePostfixBoundary, target_node_id));
                }
                // otherwise regular line postfix
                else {
                    return Some((AnnotationPosition::LinePostfix, target_node_id));
                }
            }
            // line prefix has directly following node that starts at the end token
            else if let Some(next_token) = next_token
                && next_token.token.r#type != TokenType::Newline
                && self.is_same_line(end_token.span, next_token.span)
                && let Some(target_node_id) = self
                    .find_node_starting_at(&next_token.span, NodeSearch::SmallestInner)
                    .map(|span| span.idx)
            {
                return Some((AnnotationPosition::LinePrefix, target_node_id));
            }
            // block infix has no directly preceding node, but have enclosing node
            else if let Some(target_node_id) = self
                .find_node_enclosing(&prev_token.span, NodeSearch::SmallestInner)
                .map(|span| span.idx)
            {
                return Some((AnnotationPosition::BlockInfix, target_node_id));
            }
            // no directly preceding node, no enclosing node, floating
            else {
                return None;
            }
        }

        // find the following targetable token (block postfix)
        let next_targetable_token = {
            let mut next_token_idx = token_idx as usize + group.len();
            loop {
                let Some(next_token) = tokens.get(next_token_idx) else {
                    break None;
                };
                if !ANNOTATION_TOKEN_TYPES.contains(&next_token.token.r#type) {
                    break Some(next_token);
                } else {
                    next_token_idx += 1;
                }
            }
        };
        if let Some(next_targetable_token) = next_targetable_token
            && let Some(next_node) =
                self.find_node_starting_at(&next_targetable_token.span, NodeSearch::BiggestOuter)
        {
            return Some((AnnotationPosition::BlockPrefix, next_node.idx));
        }

        // find the preceding targetable token (block prefix)
        let prev_targetable_token = {
            let mut prev_token_idx = token_idx as usize - 1;
            loop {
                let Some(prev_token) = tokens.get(prev_token_idx) else {
                    break None;
                };
                if !ANNOTATION_TOKEN_TYPES.contains(&prev_token.token.r#type) {
                    break Some(prev_token);
                } else {
                    prev_token_idx -= 1;
                }
            }
        };
        if let Some(prev_targetable_token) = prev_targetable_token
            && let Some(prev_node) =
                self.find_node_ending_at(&prev_targetable_token.span, NodeSearch::BiggestOuter)
        {
            return Some((AnnotationPosition::BlockPostfix, prev_node.idx));
        }

        // find inner enclosing node (block infix)
        if let Some(enclosing_node) =
            self.find_node_enclosing(&start_token.span, NodeSearch::SmallestInner)
        {
            return Some((AnnotationPosition::BlockInfix, enclosing_node.idx));
        }

        // nothing to attach to
        None
    }

    /// Make and attach an annotation group.
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

        // find the node to attach to
        let Some((position, target_node_id)) =
            self.find_annotation_position(token_idx, tokens, group)
        else {
            return None; // could not find a position
        };

        // create the annotation
        let annotation_id = match token_type {
            TokenType::Newline => {
                let lines = group.len() as u32 - 1;
                let blank = self.tree.allocate(Blank { lines }, span);
                self.tree.allocate(
                    Annotation::Blank {
                        node: blank,
                        position,
                    },
                    span,
                )
            }
            TokenType::LineComment => {
                let string = self.intern_string(self.clean_annotation_string_group(group));
                let comment = self.tree.allocate(
                    Comment {
                        string,
                        style: CommentStyle::Line,
                    },
                    span,
                );
                self.tree.allocate(
                    Annotation::Comment {
                        node: comment,
                        position,
                    },
                    span,
                )
            }
            TokenType::BlockComment => {
                let string = self.intern_string(self.clean_annotation_string_group(group));
                let comment = self.tree.allocate(
                    Comment {
                        string,
                        style: CommentStyle::Block,
                    },
                    span,
                );
                self.tree.allocate(
                    Annotation::Comment {
                        node: comment,
                        position,
                    },
                    span,
                )
            }
            TokenType::DocLineComment => {
                let string = self.intern_string(self.clean_annotation_string_group(group));
                let doc = self.tree.allocate(
                    Doc {
                        string,
                        style: DocStyle::Line,
                    },
                    span,
                );
                self.tree.allocate(
                    Annotation::Doc {
                        node: doc,
                        position,
                    },
                    span,
                )
            }
            TokenType::DocBlockComment => {
                let string = self.intern_string(self.clean_annotation_string_group(group));
                let doc = self.tree.allocate(
                    Doc {
                        string,
                        style: DocStyle::Block,
                    },
                    span,
                );
                self.tree.allocate(
                    Annotation::Doc {
                        node: doc,
                        position,
                    },
                    span,
                )
            }
            _ => panic!("unexpected token type: {token_type:?}"),
        };

        // attach the annotation to the node
        self.tree.append_annotation(target_node_id, annotation_id);

        Some(annotation_id)
    }

    /// Clean a group of annotation tokens into their inner string.
    #[inline]
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
                    .trim_matches(' ')
            }
            TokenType::DocBlockComment => {
                // strip leading `/**` and trailing `*/`
                string
                    .strip_prefix("/**")
                    .unwrap_or(string)
                    .strip_suffix("*/")
                    .unwrap_or(string)
                    .trim_matches(' ')
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
        Annotation, AnnotationPosition, BinaryOperator, Blank, BlockFormat, Comment, CommentStyle,
        Doc, DocStyle, Expression, Function, Let, Statement, Struct, StructField, assert_node,
        assert_path, assert_string,
    };

    /// Line suffix is attached to the previous node on the same line.
    #[test]
    fn test_attach_line_postfix_to_statement() {
        let mut test = TestParser::new("let A = 1 // line comment");
        let mut parser = test.parser();
        let statements = parser.eat_block_body(BlockFormat::Implicit).unwrap();
        parser.finalize();

        // let A = 1
        assert_eq!(statements.len(), 1);
        // line comment, suffix
        let annotations = parser.tree.get_annotations_for(statements[0].id);
        assert_eq!(annotations.len(), 1);
        assert_node!(parser.tree, annotations[0], Annotation::Comment { node, position } => {
            assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
            assert_node!(parser.tree, *node, Comment { string, style } => {
                assert_eq!(parser.get_string(*string), "line comment");
                assert_eq!(*style, CommentStyle::Line);
            });
        });
    }

    /// Multi-line suffix is attached to the previous node on the same line as a block postfix.
    #[test]
    fn test_attach_multi_line_postfix_to_statement() {
        let mut test = TestParser::new(
            "let A = 1 /* line comment
over multiple lines with trailing space    */",
        );
        let mut parser = test.parser();
        let statements = parser.eat_block_body(BlockFormat::Implicit).unwrap();
        parser.finalize();

        // let A = 1
        assert_eq!(statements.len(), 1);
        // block comment, postfix
        let annotations = parser.tree.get_annotations_for(statements[0].id);
        assert_eq!(annotations.len(), 1);
        assert_node!(parser.tree, annotations[0], Annotation::Comment { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPostfix);
            assert_node!(parser.tree, *node, Comment { string, style } => {
                assert_eq!(parser.get_string(*string), "line comment\nover multiple lines with trailing space");
                assert_eq!(*style, CommentStyle::Block);
            });
        });
    }

    /// Inline prefix, infix and suffix comments should be attached to closest inner node on the same line.
    #[test]
    fn test_attach_line_prefix_infix_postfix_to_statement() {
        let mut test =
            TestParser::new("let X = /* Pre-A comment */ A /* A comment */ && B /* B comment */");
        let mut parser = test.parser();
        let statements = parser.eat_block_body(BlockFormat::Implicit).unwrap();
        parser.finalize();

        assert_eq!(statements.len(), 1);

        // A && B
        assert_node!(parser.tree, statements[0], Statement::Expression(node) => {
            assert_node!(parser.tree, *node, Expression::Let(node) => {
                assert_node!(parser.tree, *node, Let { value, .. } => {
                    assert_node!(parser.tree, value.unwrap(), Expression::Binary { left, right, operator } => {
                        assert_eq!(*operator, BinaryOperator::And);
                        // A
                        assert_node!(parser.tree, *left, Expression::Path(path) => {
                            assert_path!(parser.session, *path, "A");
                        });
                        let annotations = parser.tree.get_annotations_for(left.id);
                        // line prefix, pre-A comment
                        assert_eq!(annotations.len(), 2);
                        assert_node!(parser.tree, annotations[0], Annotation::Comment { node, position } => {
                            assert_eq!(*position, AnnotationPosition::LinePrefix);
                            assert_node!(parser.tree, *node, Comment { string, style } => {
                                assert_eq!(parser.get_string(*string), "Pre-A comment");
                                assert_eq!(*style, CommentStyle::Block);
                            });
                        });
                        // line postfix, A comment
                        assert_node!(parser.tree, annotations[1], Annotation::Comment { node, position } => {
                            assert_eq!(*position, AnnotationPosition::LinePostfix);
                            assert_node!(parser.tree, *node, Comment { string, style } => {
                                assert_eq!(parser.get_string(*string), "A comment");
                                assert_eq!(*style, CommentStyle::Block);
                            });
                        });

                        // B
                        assert_node!(parser.tree, *right, Expression::Path(path) => {
                            assert_path!(parser.session, *path, "B");
                        });
                        // line postfix boundary, B comment
                        let annotations = parser.tree.get_annotations_for(right.id);
                        assert_eq!(annotations.len(), 1);
                        assert_node!(parser.tree, annotations[0], Annotation::Comment { node, position } => {
                            assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
                            assert_node!(parser.tree, *node, Comment { string, style } => {
                                assert_eq!(parser.get_string(*string), "B comment");
                                assert_eq!(*style, CommentStyle::Block);
                            });
                        });
                    });
                });
            });
        });
    }

    /// Line suffix is attached separatelyfrom other surrounding comments.
    #[test]
    fn test_attach_line_postfix_to_statement_with_surrounding_comments() {
        let mut test = TestParser::new(
            r"
// block prefix comment
let A = 1 // line suffix comment
// block postfix comment
",
        );
        let mut parser = test.parser();
        parser.eat_newline().unwrap();
        let statements = parser.eat_block_body(BlockFormat::Implicit).unwrap();
        parser.finalize();

        // let A = 1
        assert_eq!(statements.len(), 1);
        let annotations = parser.tree.get_annotations_for(statements[0].id);
        assert_eq!(annotations.len(), 3);

        // block prefix comment
        assert_node!(parser.tree, annotations[0], Annotation::Comment { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPrefix);
            assert_node!(parser.tree, *node, Comment { string, style } => {
                assert_eq!(parser.get_string(*string), "block prefix comment");
                assert_eq!(*style, CommentStyle::Line);
            });
        });
        // line postfix boundary comment
        assert_node!(parser.tree, annotations[1], Annotation::Comment { node, position } => {
            assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
            assert_node!(parser.tree, *node, Comment { string, style } => {
                assert_eq!(parser.get_string(*string), "line suffix comment");
                assert_eq!(*style, CommentStyle::Line);
            });
        });
        // block postfix comment
        assert_node!(parser.tree, annotations[2], Annotation::Comment { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPostfix);
            assert_node!(parser.tree, *node, Comment { string, style } => {
                assert_eq!(parser.get_string(*string), "block postfix comment");
                assert_eq!(*style, CommentStyle::Line);
            });
        });
    }

    /// Blanks (>2 successive newlines) are just annotations and should be attached to the next node.
    /// Like any other annotation, if no next or containing node is found, attach to previous node as suffix.
    #[test]
    fn test_attach_blanks_to_statements() {
        let mut test = TestParser::new("\n\nlet A = 1\n\nlet B = 2\n\n");
        let mut parser = test.parser();
        let statements = parser.eat_block_body(BlockFormat::Implicit).unwrap();
        parser.finalize();
        assert_eq!(statements.len(), 2);

        // A has one prefix block blank
        let a_annotations = parser.tree.get_annotations_for(statements[0].id);
        assert_eq!(a_annotations.len(), 1);
        assert_node!(parser.tree, a_annotations[0], Annotation::Blank { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPrefix);
            assert_node!(parser.tree, *node, Blank { lines } => {
                assert_eq!(*lines, 1);
            });
        });

        // B has one prefix block blank and one postfix block blank
        // (the postfix blank after B because there is nothing else to attach to)
        let b_annotations = parser.tree.get_annotations_for(statements[1].id);
        assert_eq!(b_annotations.len(), 2);
        assert_node!(parser.tree, b_annotations[0], Annotation::Blank { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPrefix);
            assert_node!(parser.tree, *node, Blank { lines } => {
                assert_eq!(*lines, 1);
            });
        });
        assert_node!(parser.tree, b_annotations[1], Annotation::Blank { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPostfix);
            assert_node!(parser.tree, *node, Blank { lines } => {
                assert_eq!(*lines, 1);
            });
        });
    }

    /// Annotations inside an empty node should be treated as infix within the containing node.
    #[test]
    fn test_attach_comments_infix_in_block() {
        let mut test = TestParser::new(
            r"
function main() {
    // block comment, infix
}",
        );
        let mut parser = test.parser();
        parser.eat_newline().unwrap();

        let function = parser.eat_function(None).unwrap();
        parser.finalize();

        assert_node!(parser.tree, function, Function { body, .. } => {
            // doc block infix
            // comment, infix
            let annotations = parser.tree.get_annotations_for(body.unwrap().id);
            assert_eq!(annotations.len(), 1);
            assert_node!(parser.tree, annotations[0], Annotation::Comment { node, position } => {
                assert_eq!(*position, AnnotationPosition::BlockInfix);
                assert_node!(parser.tree, *node, Comment { string, style } => {
                    assert_eq!(parser.get_string(*string), "block comment, infix");
                    assert_eq!(*style, CommentStyle::Line);
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

    // random comment
}",
        );
        let mut parser = test.parser();
        parser.eat_newline().unwrap();
        let statements = parser.eat_block_body(BlockFormat::Implicit).unwrap();
        parser.finalize();

        // struct Floof
        assert_eq!(statements.len(), 1);
        let annotations = parser.tree.get_annotations_for(statements[0].id);
        assert_eq!(annotations.len(), 3);
        // doc block prefix, floating
        assert_node!(parser.tree, annotations[0], Annotation::Doc { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPrefix);
            assert_node!(parser.tree, *node, Doc { string, style } => {
                assert_eq!(parser.get_string(*string), "doc, floating");
                assert_eq!(*style, DocStyle::Line);
            });
        });
        // blank block prefix
        assert_node!(parser.tree, annotations[1], Annotation::Blank { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPrefix);
            assert_node!(parser.tree, *node, Blank { lines } => {
                assert_eq!(*lines, 1);
            });
        });
        // doc block prefix
        // struct\ndoc, struct continued
        assert_node!(parser.tree, annotations[2], Annotation::Doc { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPrefix);
            assert_node!(parser.tree, *node, Doc { string, style } => {
                assert_eq!(parser.get_string(*string), "doc, struct\ndoc, struct continued");
                assert_eq!(*style, DocStyle::Line);
            });
        });

        // struct Floof
        assert_node!(parser.tree, statements[0], Statement::Struct(node) => {
            assert_node!(parser.tree, *node, Struct { fields, .. } => {
                // a: int32
                assert_eq!(fields.len(), 1);
                assert_node!(parser.tree, fields[0], StructField { name, .. } => {
                    assert_string!(parser.session, name.unwrap(), "a");
                    let annotations = parser.tree.get_annotations_for(fields[0].id);
                    assert_eq!(annotations.len(), 4);

                    // doc block prefix
                    // struct field\ndoc, struct field continued
                    assert_node!(parser.tree, annotations[0], Annotation::Doc { node, position } => {
                        assert_eq!(*position, AnnotationPosition::BlockPrefix);
                        assert_node!(parser.tree, *node, Doc { string, style } => {
                            assert_eq!(parser.get_string(*string), "doc, struct field\ndoc, struct field continued");
                            assert_eq!(*style, DocStyle::Line);
                        });
                    });

                    // doc line postfix boundary
                    // doc, struct field infix
                    assert_node!(parser.tree, annotations[1], Annotation::Comment { node, position } => {
                        assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
                        assert_node!(parser.tree, *node, Comment { string, style } => {
                            assert_eq!(parser.get_string(*string), "doc, struct field infix");
                            assert_eq!(*style, CommentStyle::Line);
                        });
                    });

                    // blank block postfix
                    assert_node!(parser.tree, annotations[2], Annotation::Blank { node, position } => {
                        assert_eq!(*position, AnnotationPosition::BlockPostfix);
                        assert_node!(parser.tree, *node, Blank { lines } => {
                            assert_eq!(*lines, 1);
                        });
                    });

                    // doc block postfix
                    assert_node!(parser.tree, annotations[3], Annotation::Comment { node, position } => {
                        assert_eq!(*position, AnnotationPosition::BlockPostfix);
                        assert_node!(parser.tree, *node, Comment { string, style } => {
                            assert_eq!(parser.get_string(*string), "random comment");
                            assert_eq!(*style, CommentStyle::Line);
                        });
                    });
                });

            });
        })
    }
}
