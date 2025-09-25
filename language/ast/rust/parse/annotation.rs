//! Annotation parsing.

use dyst_language_source::{MultiSpan, Span};
use dyst_language_token::{TokenSpan, TokenType};

use crate::parse::prelude::*;
use crate::{
    Annotation, AnnotationPosition, Blank, Comment, CommentStyle, Doc, DocStyle, NodeId,
    NodeSearch, NodeType, ParseResult, Parser, Tag,
};

const ANNOTATION_TOKEN_TYPES: [TokenType; 5] = [
    TokenType::Newline,
    TokenType::LineComment,
    TokenType::DocLineComment,
    TokenType::BlockComment,
    TokenType::DocBlockComment,
];

impl Annotation {
    pub fn position(&self) -> AnnotationPosition {
        match self {
            Annotation::Blank { position, .. } => *position,
            Annotation::Doc { position, .. } => *position,
            Annotation::Comment { position, .. } => *position,
            Annotation::Tag { position, .. } => *position,
        }
    }
}

impl<'a> Parser<'a> {
    /// Pre-parse unattached annotations (like Tags).
    /// We do one full pass and consume the entire Parser.
    pub(crate) fn prepare_annotations(&mut self) {
        // parse out all the tags
        while let Ok(token) = self.peek() {
            // #
            if token.token.r#type == TokenType::Tag {
                let _ = self.with_recovery(
                    self.mark(),
                    |parser| parser.eat_tag().map(Some),
                    None,
                    TokenType::Newline,
                );
            } else {
                self.bump(); // keep going
            }
        }
    }

    /// Eat a tag.
    /// Tags are basically just free-floating typed (struct/tuple) annotations.
    ///
    /// Examples:
    /// ```
    /// #Foo
    /// #Foo(x: 1)
    /// ```
    pub(crate) fn eat_tag(&mut self) -> ParseResult<NodeId<Tag>> {
        let start = self.mark();

        // #
        self.eat_token(TokenType::Tag)?;

        // name
        let name = self.eat_identifier()?;

        // arguments
        let arguments = if self.peek_token(TokenType::OpenParenthesis).is_ok() {
            self.bump(); // eat open parenthesis
            self.eat_newlines_maybe()?;
            let arguments = self.eat_arguments_body().for_node_type(NodeType::Tag)?;
            self.eat_token(TokenType::CloseParenthesis)
                .for_node_type(NodeType::Tag)?;
            Some(arguments)
        } else {
            None
        };

        // tag
        let tag = self
            .tree
            .allocate(Tag { name, arguments }, self.get_span_from(start));
        Ok(tag)
    }

    /// Attach all annotations to respective AST nodes.
    /// Must be called *after* primary parsing.
    pub(crate) fn attach_annotations(&mut self) {
        // combine tokens
        let mut tokens = Vec::with_capacity(self.tokens.len());
        tokens.extend(self.tokens.clone());
        tokens.extend(
            self.side_tokens
                .iter()
                .filter(|token| token.token.r#type != TokenType::Whitespace),
        );
        tokens.sort_by_key(|token| token.span.start);
        if tokens.is_empty() {
            return;
        }
        let tokens = tokens;

        // attach annotations
        let ignore_span = MultiSpan::new(self.tree.get_spans_for(NodeType::Tag));
        self.attach_side_annotations(&tokens, &ignore_span);
        self.attach_tag_annotations(&tokens, &ignore_span);

        // sort annotations per node
        self.tree.sort_annotations();
    }

    /// Attach comment and doc annotations to the tokens.
    fn attach_side_annotations(&mut self, tokens: &[TokenSpan], ignore_span: &MultiSpan) {
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
                    && !ignore_span.contains(&current_token_group[0].span)
                    && !ignore_span
                        .contains(&current_token_group[current_token_group.len() - 1].span)
                {
                    self.attach_side_annotation(
                        (i - current_token_group.len()) as u32,
                        current_token_type,
                        tokens,
                        &current_token_group,
                        ignore_span,
                    );
                }

                // begin new group
                current_token_type = token.token.r#type;
                current_token_group.clear();
            }
            current_token_group.push(*token);
        }

        // flush group at the end
        if ANNOTATION_TOKEN_TYPES.contains(&current_token_type)
            && (current_token_type != TokenType::Newline || current_token_group.len() > 1)
        {
            self.attach_side_annotation(
                (tokens.len() - current_token_group.len()) as u32,
                current_token_type,
                tokens,
                &current_token_group,
                ignore_span,
            );
        }
    }

    /// Attach tag annotations to the tokens.
    fn attach_tag_annotations(&mut self, tokens: &[TokenSpan], ignore_span: &MultiSpan) {
        for tag_id in self.tree.get_nodes_for::<Tag>() {
            let span = self.tree.get_span(tag_id);

            // find corresponding token group
            let token_idx = tokens
                .iter()
                .position(|token| token.span.start == span.start)
                .unwrap_or_else(|| panic!("tag span not found in tokens: {span:?}"));
            let token_group = tokens
                .iter()
                .skip(token_idx)
                .take_while(|token| token.span.end <= span.end)
                .copied()
                .collect::<Vec<_>>();

            // find annotation position
            let Some((position, target_node_id)) = self.find_annotation_position(
                token_idx as u32,
                tokens,
                &token_group,
                true,
                ignore_span,
            ) else {
                continue;
            };

            // attach the annotation
            let annotation_id = self.tree.allocate(
                Annotation::Tag {
                    node: tag_id,
                    position,
                },
                span,
            );
            self.tree.append_annotation(target_node_id, annotation_id);
        }
    }

    /// Find the annotation position for a given token index and group.
    fn find_annotation_position(
        &self,
        token_idx: u32,
        tokens: &[TokenSpan],
        group: &[TokenSpan],
        is_full_line: bool,
        ignore_span: &MultiSpan,
    ) -> Option<(AnnotationPosition, u32)> {
        debug_assert!(!group.is_empty());

        let start_token = group[0];
        let end_token = group[group.len() - 1];
        let is_one_line = self.is_same_line(start_token.span, end_token.span);

        // nocheckin: only attach annotations forward/backward within innermost / same parent node?

        // line prefix or postfix (or block infix if we have nothing)
        // entire span must be on one line together with the previous/next token
        if is_one_line {
            // find previous token not in ignore span
            let prev_token = if token_idx > 0 {
                let mut prev_token_idx = token_idx as usize - 1;
                loop {
                    let Some(prev_token) = tokens.get(prev_token_idx) else {
                        break None;
                    };
                    if !ignore_span.contains(&prev_token.span) {
                        break Some(prev_token);
                    } else {
                        prev_token_idx -= 1;
                    }
                }
            } else {
                None
            };

            // find next token not in ignore span
            let next_token = {
                let mut next_token_idx = token_idx as usize + group.len();
                loop {
                    let Some(next_token) = tokens.get(next_token_idx) else {
                        break None;
                    };
                    if !ignore_span.contains(&next_token.span) {
                        break Some(next_token);
                    } else {
                        next_token_idx += 1;
                    }
                }
            };

            // line postfix has directly preceding node that ends at the start token
            if let Some(prev_token) = prev_token
                && self.is_same_line(prev_token.span, end_token.span)
                && prev_token.token.r#type != TokenType::Newline
                && let Some(target_node_id) = self
                    .find_node_ending_at(
                        &prev_token.span,
                        if is_full_line {
                            NodeSearch::Outer
                        } else {
                            NodeSearch::Inner
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
                    .find_node_starting_at(&next_token.span, NodeSearch::Inner)
                    .map(|span| span.idx)
            {
                return Some((AnnotationPosition::LinePrefix, target_node_id));
            }
        }

        // find the following targetable token (block postfix)
        let next_targetable_token = {
            let mut next_token_idx = token_idx as usize + group.len();
            loop {
                let Some(next_token) = tokens.get(next_token_idx) else {
                    break None;
                };
                if !ANNOTATION_TOKEN_TYPES.contains(&next_token.token.r#type)
                    && !ignore_span.contains(&next_token.span)
                {
                    break Some(next_token);
                } else {
                    next_token_idx += 1;
                }
            }
        };
        if let Some(next_targetable_token) = next_targetable_token
            && let Some(next_node) =
                self.find_node_starting_at(&next_targetable_token.span, NodeSearch::Outer)
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
                if !ANNOTATION_TOKEN_TYPES.contains(&prev_token.token.r#type)
                    && !ignore_span.contains(&prev_token.span)
                {
                    break Some(prev_token);
                } else {
                    prev_token_idx -= 1;
                }
            }
        };
        if let Some(prev_targetable_token) = prev_targetable_token
            && let Some(prev_node) =
                self.find_node_ending_at(&prev_targetable_token.span, NodeSearch::Outer)
        {
            return Some((AnnotationPosition::BlockPostfix, prev_node.idx));
        }

        // find inner enclosing node (block infix)
        if let Some(enclosing_node) = self.find_node_enclosing(&start_token.span, NodeSearch::Inner)
        {
            return Some((AnnotationPosition::BlockInfix, enclosing_node.idx));
        }

        // nothing to attach to
        None
    }

    /// Make and attach an annotation group.
    fn attach_side_annotation(
        &mut self,
        token_idx: u32,
        token_type: TokenType,
        tokens: &[TokenSpan],
        group: &[TokenSpan],
        ignore_span: &MultiSpan,
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
        let is_line_comment = start_token.token.r#type == TokenType::LineComment
            || start_token.token.r#type == TokenType::DocLineComment;
        let Some((position, target_node_id)) =
            self.find_annotation_position(token_idx, tokens, group, is_line_comment, ignore_span)
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
                let string = self.intern_string(self.clean_annotation_string(token_type, group));
                let comment = self.tree.allocate(
                    Comment {
                        string,
                        style: CommentStyle::Slash,
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
                let string = self.intern_string(self.clean_annotation_string(token_type, group));
                let comment = self.tree.allocate(
                    Comment {
                        string,
                        style: CommentStyle::Star,
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
                let string = self.intern_string(self.clean_annotation_string(token_type, group));
                let doc = self.tree.allocate(
                    Doc {
                        string,
                        style: DocStyle::Slash,
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
                let string = self.intern_string(self.clean_annotation_string(token_type, group));
                let doc = self.tree.allocate(
                    Doc {
                        string,
                        style: DocStyle::Star,
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

    /// Clean annotation tokens into their inner string, preserving intentional spacing.
    fn clean_annotation_string(&self, token_type: TokenType, group: &[TokenSpan]) -> String {
        debug_assert!(!group.is_empty());

        let mut cleaned_tokens = Vec::with_capacity(group.len());

        for token in group {
            let raw = self.source.get_span_str(token.span);

            // strip comment prefixes and suffixes
            let inner = match token_type {
                TokenType::LineComment => raw.strip_prefix("//").unwrap_or(raw),
                TokenType::DocLineComment => raw.strip_prefix("///").unwrap_or(raw),
                TokenType::BlockComment => raw
                    .strip_prefix("/*")
                    .unwrap_or(raw)
                    .strip_suffix("*/")
                    .unwrap_or(raw),
                TokenType::DocBlockComment => raw
                    .strip_prefix("/**")
                    .unwrap_or(raw)
                    .strip_suffix("*/")
                    .unwrap_or(raw),
                _ => panic!("unexpected token type: {token_type:?}"),
            };

            // clean up whitespace and formatting characters
            let cleaned = match token_type {
                TokenType::LineComment | TokenType::DocLineComment => {
                    if inner.contains('\n') {
                        // handle multiline comments by stripping leading space from each line
                        inner
                            .lines()
                            .map(|line| line.strip_prefix(' ').unwrap_or(line).to_owned())
                            .collect::<Vec<_>>()
                            .join("\n")
                    } else {
                        // single line, just strip leading space
                        inner.strip_prefix(' ').unwrap_or(inner).to_owned()
                    }
                }
                TokenType::BlockComment | TokenType::DocBlockComment => {
                    let trimmed = inner.trim();
                    if trimmed.is_empty() {
                        String::new()
                    } else {
                        // handle block comment formatting with optional asterisk prefixes
                        trimmed
                            .lines()
                            .map(|line| {
                                let trimmed_star = line.trim_start_matches(' ');
                                if let Some(after_star) = trimmed_star.strip_prefix('*') {
                                    // line has asterisk prefix, strip it and following space
                                    after_star
                                        .strip_prefix(' ')
                                        .unwrap_or(after_star)
                                        .to_owned()
                                } else {
                                    // no asterisk, just strip leading space
                                    line.strip_prefix(' ').unwrap_or(line).to_owned()
                                }
                            })
                            .collect::<Vec<_>>()
                            .join("\n")
                    }
                }
                _ => unreachable!(),
            };

            cleaned_tokens.push(cleaned);
        }

        cleaned_tokens.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use crate::parse::tests::TestParser;
    use crate::{
        Annotation, AnnotationPosition, Argument, BinaryOperator, Blank, Block, BlockFormat,
        Comment, CommentStyle, Doc, DocStyle, Expression, Function, Let, Statement, Struct,
        StructField, Tag, assert_lit_int, assert_lit_string, assert_node, assert_path,
        assert_string,
    };

    /// Tag annotations should be parsed in a separate pass.
    #[test]
    fn test_attach_tag_annotations() {
        let mut test = TestParser::new(
            r#"
/// Test doc
#BeginGroup("MyGroup", length: 1)
struct Test {}
#EndGroup
"#,
        );
        let mut parser = test.prepare();
        let statements = parser.eat_block_body(BlockFormat::Implicit).unwrap();
        parser.finalize();

        assert_eq!(statements.len(), 1);
        let annotations = parser.tree.get_annotations_for(statements[0].id);
        assert_eq!(annotations.len(), 3);
        // doc block prefix
        // /// Test doc
        assert_node!(parser.tree, annotations[0], Annotation::Doc { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPrefix);
            assert_node!(parser.tree, *node, Doc { string, style } => {
                assert_string!(parser.session, *string, "Test doc");
                assert_eq!(*style, DocStyle::Slash);
            });
        });
        // tag block prefix
        // #BeginGroup("MyGroup", 1)
        assert_node!(parser.tree, annotations[1], Annotation::Tag { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPrefix);
            assert_node!(parser.tree, *node, Tag { name, arguments } => {
                assert_string!(parser.session, *name, "BeginGroup");
                assert!(arguments.is_some());
                assert_eq!(arguments.as_ref().unwrap().len(), 2);
                // "MyGroup"
                assert_node!(parser.tree, arguments.as_ref().unwrap()[0], Argument::Positional { value } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(literal) => {
                        assert_lit_string!(parser.session, parser.tree.get(*literal), "MyGroup");
                    });
                });
                // 1
                assert_node!(parser.tree, arguments.as_ref().unwrap()[1], Argument::Named { name, value } => {
                    assert_string!(parser.session, *name, "length");
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(literal) => {
                        assert_lit_int!(parser.session, parser.tree.get(*literal), 1);
                    });
                });
            });
        });
        // tag block postfix
        // #EndGroup
        assert_node!(parser.tree, annotations[2], Annotation::Tag { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPostfix);
            assert_node!(parser.tree, *node, Tag { name, arguments } => {
                assert_string!(parser.session, *name, "EndGroup");
                assert!(arguments.is_none());
            });
        });
    }

    /// Line suffix is attached to the previous node on the same line.
    #[test]
    fn test_attach_line_postfix_to_statement() {
        let mut test = TestParser::new("let A = 1 // line comment");
        let mut parser = test.prepare();
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
                assert_eq!(*style, CommentStyle::Slash);
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
        let mut parser = test.prepare();
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
                assert_eq!(*style, CommentStyle::Star);
            });
        });
    }

    /// Multiline block comments are cleaned up properly.
    #[test]
    fn test_clean_multiline_block_comment() {
        let mut test = TestParser::new(
            "{
    /** some multiline
     * block comment
     * over multiple lines */
    let X = 1
}",
        );
        let mut parser = test.prepare();
        let block = parser.eat_block().unwrap();
        parser.finalize();

        assert_node!(parser.tree, block, Block { statements, .. } => {
            assert_eq!(statements.len(), 1);
            let annotations = parser.tree.get_annotations_for(statements[0].id);
            assert_eq!(annotations.len(), 1);
            assert_node!(parser.tree, annotations[0], Annotation::Doc { node, position } => {
                assert_eq!(*position, AnnotationPosition::BlockPrefix);
                assert_node!(parser.tree, *node, Doc { string, style } => {
                    assert_eq!(parser.get_string(*string), "some multiline\nblock comment\nover multiple lines");
                    assert_eq!(*style, DocStyle::Star);
                });
            });
        });
    }

    /// Inline prefix, infix and suffix comments should be attached to closest inner node on the same line.
    #[test]
    fn test_attach_line_prefix_infix_postfix_to_statement() {
        let mut test =
            TestParser::new("let X = /* Pre-A comment */ A /* A comment */ && B /* B comment */");
        let mut parser = test.prepare();
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
                                assert_eq!(*style, CommentStyle::Star);
                            });
                        });
                        // line postfix, A comment
                        assert_node!(parser.tree, annotations[1], Annotation::Comment { node, position } => {
                            assert_eq!(*position, AnnotationPosition::LinePostfix);
                            assert_node!(parser.tree, *node, Comment { string, style } => {
                                assert_eq!(parser.get_string(*string), "A comment");
                                assert_eq!(*style, CommentStyle::Star);
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
                                assert_eq!(*style, CommentStyle::Star);
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
        let mut parser = test.prepare();
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
                assert_eq!(*style, CommentStyle::Slash);
            });
        });
        // line postfix boundary comment
        assert_node!(parser.tree, annotations[1], Annotation::Comment { node, position } => {
            assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
            assert_node!(parser.tree, *node, Comment { string, style } => {
                assert_eq!(parser.get_string(*string), "line suffix comment");
                assert_eq!(*style, CommentStyle::Slash);
            });
        });
        // block postfix comment
        assert_node!(parser.tree, annotations[2], Annotation::Comment { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPostfix);
            assert_node!(parser.tree, *node, Comment { string, style } => {
                assert_eq!(parser.get_string(*string), "block postfix comment");
                assert_eq!(*style, CommentStyle::Slash);
            });
        });
    }

    /// Blanks (>2 successive newlines) are just annotations and should be attached to the next node.
    /// Like any other annotation, if no next or containing node is found, attach to previous node as suffix.
    #[test]
    fn test_attach_blanks_to_statements() {
        let mut test = TestParser::new("\n\nlet A = 1\n\nlet B = 2\n\n");
        let mut parser = test.prepare();
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
        let mut parser = test.prepare();
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
                    assert_eq!(*style, CommentStyle::Slash);
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
        let mut parser = test.prepare();
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
                assert_eq!(*style, DocStyle::Slash);
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
                assert_eq!(*style, DocStyle::Slash);
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
                            assert_eq!(*style, DocStyle::Slash);
                        });
                    });

                    // doc line postfix boundary
                    // doc, struct field infix
                    assert_node!(parser.tree, annotations[1], Annotation::Comment { node, position } => {
                        assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
                        assert_node!(parser.tree, *node, Comment { string, style } => {
                            assert_eq!(parser.get_string(*string), "doc, struct field infix");
                            assert_eq!(*style, CommentStyle::Slash);
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
                            assert_eq!(*style, CommentStyle::Slash);
                        });
                    });
                });

            });
        })
    }
}
