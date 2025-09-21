//! Annotation parsing.

use std::borrow::Cow;

use dyst_language_source::Span;
use dyst_language_token::{TokenSpan, TokenType};

use crate::{Annotation, AnnotationPosition, Blank, Comment, CommentStyle, Doc, DocStyle, Parser};

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
    ) {
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
        let is_line_suffix = self.is_same_line(start_token.span, end_token.span)
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
        let Some(target_node) = target_node else {
            return;
        };

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

        // nocheckin
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
        AnnotationPosition, Blank, BlockFormat, Comment, CommentStyle, Doc, DocStyle, Enum,
        EnumField, Expression, Statement, Struct, StructField, assert_node, assert_string,
    };

    #[test]
    fn test_attach_blanks_between_statements() {
        let mut test = TestParser::new(
            r"


let A = 1

let B = 2

        ",
        );
        let mut parser = test.parser();
        let statements = parser.eat_block_body(BlockFormat::Implicit).unwrap();
        parser.process_annotations();

        assert_eq!(statements.len(), 2);

        // A has two prefix block blanks
        let a_blanks = parser.tree.get_blanks_for(statements[0].id);
        assert_eq!(a_blanks.len(), 2);
        assert_node!(parser.tree, a_blanks[0], Blank { position, lines } => {
            assert_eq!(*position, AnnotationPosition::BlockPrefix);
            assert_eq!(*lines, 1);
        });
        assert_node!(parser.tree, a_blanks[1], Blank { position, lines } => {
            assert_eq!(*position, AnnotationPosition::BlockPrefix);
            assert_eq!(*lines, 1);
        });

        // B has one prefix block blank and one postfix block blank
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

    #[test]
    fn test_attach_docs_to_struct_mixed_block_prefix() {
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
            assert_node!(parser.tree, docs[0], Doc { string, position, style } => {
                let string = parser.get_string(*string);
                assert_eq!(string, "doc, struct\ndoc, struct continued");
                assert_eq!(*position, AnnotationPosition::BlockPrefix);
                assert_eq!(*style, DocStyle::Line);
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
                    assert_node!(parser.tree, docs[0], Doc { string, position, style } => {
                        let string = parser.get_string(*string);
                        assert_eq!(string, "doc, struct field\ndoc, struct field continued");
                        assert_eq!(*position, AnnotationPosition::BlockPrefix);
                        assert_eq!(*style, DocStyle::Line);
                    });
                });
            });

        })
    }

    #[test]
    fn test_attach_docs_to_enum_mixed_block_prefix() {
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
                    assert_node!(parser.tree, docs[0], Doc { string, position, style } => {
                        let string = parser.get_string(*string);
                        assert_eq!(string, "A");
                        assert_eq!(*position, AnnotationPosition::BlockPrefix);
                        assert_eq!(*style, DocStyle::Line);
                    });
                });
            });
        })
    }

    #[test]
    fn test_attach_docs_to_enum_with_values_mixed_block_prefix() {
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
                    assert_node!(parser.tree, docs[0], Doc { string, position, style } => {
                        let string = parser.get_string(*string);
                        assert_eq!(string, "A\ndoc, enum field");
                        assert_eq!(*position, AnnotationPosition::BlockPrefix);
                        assert_eq!(*style, DocStyle::Line);
                    });
                });

                // B
                assert_node!(parser.tree, fields[1], EnumField { name, .. } => {
                    assert_string!(parser.session, *name, "B");
                    let docs = parser.tree.get_docs_for(fields[1].id);
                    assert_eq!(docs.len(), 1);
                    // B + doc, enum field
                    assert_node!(parser.tree, docs[0], Doc { string, position, style } => {
                        let string = parser.get_string(*string);
                        assert_eq!(string, "B\ndoc, enum field");
                        assert_eq!(*position, AnnotationPosition::BlockPrefix);
                        assert_eq!(*style, DocStyle::Line);
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
                    assert_node!(parser.tree, docs[0], Doc { string, position, style } => {
                        let string = parser.get_string(*string);
                        assert_eq!(string, "D");
                        assert_eq!(*position, AnnotationPosition::BlockPrefix);
                        assert_eq!(*style, DocStyle::Line);
                    });
                });
            });
        })
    }

    #[test]
    fn test_attach_comments_mixed_block_prefix() {
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
                assert_node!(parser.tree, comments[0], Comment { string, position, style } => {
                    let string = parser.get_string(*string);
                    assert_eq!(string, "comment 1\ncomment 1.1");
                    assert_eq!(*position, AnnotationPosition::BlockPrefix);
                    assert_eq!(*style, CommentStyle::Line);
                });
            });

        });
    }

    #[test]
    fn test_attach_comments_mixed_block_prefix_and_line_suffix() {
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
            assert_node!(parser.tree, comments[0], Comment { string, position, style } => {
                let string = parser.get_string(*string);
                assert_eq!(string, "comment 2");
                assert_eq!(*position, AnnotationPosition::BlockPrefix);
                assert_eq!(*style, CommentStyle::Line);
            });
        });
    }

    #[test]
    fn test_attach_postfix_comment_same_line() {
        let mut test = TestParser::new(
            r"
let value = 42 // trailing comment
        ",
        );
        let mut parser = test.parser();
        let statements = parser.eat_block_body(BlockFormat::Implicit).unwrap();
        parser.process_annotations();

        assert_eq!(statements.len(), 1);
        assert_node!(parser.tree, statements[0], Statement::Expression(expr_node) => {
            assert_node!(parser.tree, *expr_node, Expression::Let(let_node) => {
                let comments = parser.tree.get_comments_for(let_node.id);
                assert_eq!(comments.len(), 1);
                assert_node!(parser.tree, comments[0], Comment { string, position, style } => {
                    let string = parser.get_string(*string);
                    assert_eq!(string, "trailing comment");
                    assert_eq!(*position, AnnotationPosition::LineSuffix);
                    assert_eq!(*style, CommentStyle::Line);
                });
            });
        });
    }

    #[test]
    fn test_collect_blank_annotations_between_statements() {
        let mut test = TestParser::new(
            r"

let a = 1

let b = 2
        ",
        );
        let mut parser = test.parser();
        let statements = parser.eat_block_body(BlockFormat::Implicit).unwrap();
        parser.process_annotations();

        assert_eq!(statements.len(), 2);

        assert_node!(parser.tree, statements[0], Statement::Expression(expr_node) => {
            assert_node!(parser.tree, *expr_node, Expression::Let(let_node) => {
                let blanks = parser.tree.get_blanks_for(let_node.id);
                assert!(blanks.is_empty());
            });
        });

        assert_node!(parser.tree, statements[1], Statement::Expression(expr_node) => {
            assert_node!(parser.tree, *expr_node, Expression::Let(let_node) => {
                let blanks = parser.tree.get_blanks_for(let_node.id);
                assert_eq!(blanks.len(), 1);
                assert_node!(parser.tree, blanks[0], Blank { position, lines } => {
                    assert_eq!(*position, AnnotationPosition::BlockPrefix);
                    assert_eq!(*lines, 1);
                });
            });
        });
    }
}
