//! Annotation parsing.

use crate::parse::prelude::*;
use crate::{ParseResult, Parser};
use dyst_ast::{
    ANNOTATION_NODE_TYPES, Annotation, AnnotationPosition, Blank, Comment, CommentStyle, Decorator,
    Doc, DocStyle, NodeId, NodeType, Tag, TokenSpan, TokenType,
};
use dyst_source::{MultiSpan, NodeSearch, Span};

const ANNOTATION_TOKEN_TYPES: [TokenType; 5] = [
    TokenType::Newline,
    TokenType::LineComment,
    TokenType::DocLineComment,
    TokenType::BlockComment,
    TokenType::DocBlockComment,
];

const STATIC_BLOCK_KEYWORDS_STR: [&str; 3] = ["loop", "for", "while"];

impl<'a> Parser<'a> {
    /// Eat all side annotations (like Tags).
    /// NOTE: One full pass, consuming the entire Parser.
    pub(crate) fn eat_side_annotations(&mut self) {
        // parse out all the side annotations
        while let Ok(token) = self.peek() {
            // #
            // tags are line and block scoped
            if token.token.ty == TokenType::Tag && self.language.compatibility.is_none() {
                let _ = self.with_recovery(
                    self.mark(),
                    |parser| parser.eat_tag().map(Some),
                    None,
                    TokenType::Newline,
                );
            }
            // @
            // decorators are block scoped only
            else if token.token.ty == TokenType::At
                // must be block scoped
                && (self.prev().is_none()
                    || self.prev().unwrap().token.ty == TokenType::Newline)
                && let Ok(next) = self.peek_next()
                && let next_span_str = self.get_span_str(next.span)
                && !STATIC_BLOCK_KEYWORDS_STR.contains(&next_span_str)
            {
                // speculatively parse the decorator
                let speculative_start = (self.mark(), self.tree.next_id());
                let decorator = self.with_recovery(
                    self.mark(),
                    |parser| parser.eat_decorator().map(Some),
                    None,
                    TokenType::Newline,
                );

                // allow `@if(...)` if it's unambiguously an if without a following block
                if decorator.is_some() && next_span_str == "if" && self.peek_block().is_ok() {
                    // otherwise ignore this decorator, it's just a static if
                    self.restore(speculative_start.0, speculative_start.1);
                    self.bump();
                    continue;
                }
            }
            // keep going
            else {
                self.bump();
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
    fn eat_tag(&mut self) -> ParseResult<NodeId<Tag>> {
        let start = self.mark();

        // #
        self.eat_token(TokenType::Tag)?;

        // receiver
        let receiver = self.eat_path().for_node_type(NodeType::Tag)?;

        // arguments
        let arguments = if self.peek_token(TokenType::OpenParenthesis).is_ok() {
            self.bump(); // eat open parenthesis
            self.eat_newlines_maybe()?;
            // empty parentheses
            if self.peek_token(TokenType::CloseParenthesis).is_ok() {
                self.bump(); // eat close parenthesis
                None
            }
            // non-empty parentheses
            else {
                let arguments = self
                    .eat_arguments_body(TokenType::CloseParenthesis)
                    .for_node_type(NodeType::Tag)?;
                self.eat_token(TokenType::CloseParenthesis)
                    .for_node_type(NodeType::Tag)?;
                Some(arguments)
            }
        } else {
            None
        };

        // tag
        let tag = self.tree.insert(
            Tag {
                left: receiver,
                arguments,
            },
            self.get_span_from(start),
        );
        Ok(tag)
    }

    /// Eat a decorator.
    ///
    /// Examples:
    /// ```
    /// @foo
    /// @foo(1, 2, 3)
    /// ```
    fn eat_decorator(&mut self) -> ParseResult<NodeId<Decorator>> {
        let start = self.mark();

        // @
        self.eat_token(TokenType::At)?;

        // receiver
        let left = self.eat_path().for_node_type(NodeType::Decorator)?;

        // arguments
        let arguments = if self.peek_token(TokenType::OpenParenthesis).is_ok() {
            self.bump(); // eat open parenthesis
            self.eat_newlines_maybe()?;
            // empty parentheses
            if self.peek_token(TokenType::CloseParenthesis).is_ok() {
                self.bump(); // eat close parenthesis
                None
            }
            // non-empty parentheses
            else {
                let arguments = self
                    .eat_arguments_body(TokenType::CloseParenthesis)
                    .for_node_type(NodeType::Decorator)?;
                self.eat_token(TokenType::CloseParenthesis)
                    .for_node_type(NodeType::Decorator)?;
                Some(arguments)
            }
        } else {
            None
        };

        // decorator
        let decorator = self
            .tree
            .insert(Decorator { left, arguments }, self.get_span_from(start));
        Ok(decorator)
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
                .filter(|token| token.token.ty != TokenType::Whitespace),
        );
        tokens.sort_by_key(|token| token.span.start);
        if tokens.is_empty() {
            return;
        }
        let tokens = tokens;

        // attach annotations
        let side_span = self.compute_side_span();
        self.attach_side_annotations(&tokens, &side_span);
        self.attach_main_annotations(&tokens, &side_span);

        // sort annotations per node
        self.tree.sort_annotations();
    }

    /// Attach comment and doc annotations to the tokens.
    fn attach_side_annotations(&mut self, tokens: &[TokenSpan], ignore_span: &MultiSpan) {
        // build annotation groups
        let mut current_token_type: TokenType = tokens[0].token.ty;
        let mut current_token_group: Vec<TokenSpan> = Vec::new();
        for (i, token) in tokens.iter().enumerate() {
            if token.token.ty != current_token_type {
                let start_token = current_token_group[0];
                let prev_token = if i > 1 { Some(tokens[i - 2]) } else { None };
                let is_line_postfix = current_token_group.len() == 1
                    && self.is_same_line(start_token.span, start_token.span)
                    && prev_token.is_some()
                    && self.is_same_line(start_token.span, prev_token.unwrap().span)
                    && prev_token.unwrap().token.ty != TokenType::Newline;

                // skip up to one newline in-between non-blank annotations
                //  (unless the current token is a suffix comment)
                if token.token.ty == TokenType::Newline
                    && current_token_type != TokenType::Newline
                    && let Some(next_token) = tokens.get(i + 1)
                    && next_token.token.ty == current_token_type
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
                current_token_type = token.token.ty;
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

    /// Attach the side Tag annotations to relevant nodes.
    fn attach_main_annotations(&mut self, tokens: &[TokenSpan], ignore_span: &MultiSpan) {
        // attach Tags
        for tag_id in self.tree.get_nodes::<Tag>() {
            let span = self.tree.get_span(tag_id);

            // find the annotation position
            let Some((position, target_node_id)) =
                self.find_main_annotation_target(tokens, ignore_span, false, span)
            else {
                // error if no position found
                let error = ParseError::unexpected_for(span, NodeType::Tag);
                self.handle_error(&error);
                continue;
            };

            // attach the annotation
            let annotation_id = self.tree.insert(
                Annotation::Tag {
                    node: tag_id,
                    position,
                },
                span,
            );
            self.tree.append_annotation(target_node_id, annotation_id);
        }

        // attach Decorators
        for decorator_id in self.tree.get_nodes::<Decorator>() {
            let span = self.tree.get_span(decorator_id);

            // find the annotation position
            let Some((position, target_node_id)) =
                self.find_main_annotation_target(tokens, ignore_span, true, span)
            else {
                // error if no position found
                let error = ParseError::unexpected_for(span, NodeType::Decorator);
                self.handle_error(&error);
                continue;
            };

            // attach the annotation
            let annotation_id = self.tree.insert(
                Annotation::Decorator {
                    node: decorator_id,
                    position,
                },
                span,
            );
            self.tree.append_annotation(target_node_id, annotation_id);
        }
    }

    fn find_main_annotation_target(
        &self,
        tokens: &[TokenSpan],
        ignore_span: &MultiSpan,
        is_block_prefix_only: bool,
        span: Span,
    ) -> Option<(AnnotationPosition, u32)> {
        #[cfg(debug_assertions)]
        let _span_str = self.get_span_str(span);

        // find corresponding token group
        let token_idx = tokens
            .iter()
            .position(|token| token.span.start == span.start)
            .unwrap_or_else(|| unreachable!("annotation span not found in tokens: {span:?}"));
        let token_group = tokens
            .iter()
            .skip(token_idx)
            .take_while(|token| token.span.end <= span.end)
            .copied()
            .collect::<Vec<_>>();

        // find annotation position
        self.find_annotation_target(
            token_idx as u32,
            tokens,
            &token_group,
            true,
            is_block_prefix_only,
            ignore_span,
        )
    }

    /// Find the annotation position for a given token index and group.
    fn find_annotation_target(
        &self,
        token_idx: u32,
        tokens: &[TokenSpan],
        group: &[TokenSpan],
        is_full_line: bool,
        is_block_prefix_only: bool,
        ignore_span: &MultiSpan,
    ) -> Option<(AnnotationPosition, u32)> {
        debug_assert!(!group.is_empty());

        let start_token = group[0];
        let end_token = group[group.len() - 1];
        let is_one_line = self.is_same_line(start_token.span, end_token.span);
        let enclosing_scope =
            self.find_node_enclosing_at(&start_token.span, NodeSearch::SmallestInnermost, |span| {
                !ANNOTATION_NODE_TYPES.contains(&self.tree.get_type(span.idx))
                    && !ignore_span.contains(&span.span)
            });
        let enclosing_span = enclosing_scope.map(|scope| scope.span);

        #[cfg(debug_assertions)]
        let _start_token_str = self.get_span_str(start_token.span);
        #[cfg(debug_assertions)]
        let _end_token_str = self.get_span_str(end_token.span);
        #[cfg(debug_assertions)]
        let _enclosing_span_str = enclosing_span.map(|span| self.get_span_str(span));

        // line prefix or postfix
        if !is_block_prefix_only && is_one_line {
            // find previous token not in ignore span
            let prev_token = if token_idx > 0 {
                let mut prev_token_idx = token_idx as usize;
                loop {
                    if prev_token_idx == 0 {
                        break None; // stop at the start of the tokens
                    }
                    prev_token_idx -= 1;
                    let Some(prev_token) = tokens.get(prev_token_idx) else {
                        break None; // stop at the start of the tokens
                    };
                    #[cfg(debug_assertions)]
                    let _prev_token_str = self.get_span_str(prev_token.span);
                    if ignore_span.contains(&prev_token.span)
                        || prev_token.token.ty == TokenType::Whitespace
                    {
                        continue; // ignore non-targetable tokens
                    }
                    if let Some(enclosing_span) = enclosing_span
                        && !enclosing_span.intersects(prev_token.span)
                    {
                        break None; // stop outside the enclosing scope
                    }
                    break Some(prev_token);
                }
            } else {
                None
            };

            // find next token not in ignore span
            let next_token = {
                let mut next_token_idx = token_idx as usize + group.len();
                loop {
                    let Some(next_token) = tokens.get(next_token_idx) else {
                        break None; // stop at the end of the tokens
                    };
                    #[cfg(debug_assertions)]
                    let _next_token_str = self.get_span_str(next_token.span);
                    if ignore_span.contains(&next_token.span)
                        || next_token.token.ty == TokenType::Whitespace
                    {
                        next_token_idx += 1;
                        continue; // ignore non-targetable tokens
                    }
                    if let Some(enclosing_span) = enclosing_span
                        && !enclosing_span.intersects(next_token.span)
                    {
                        break None; // stop outside the enclosing scope
                    }
                    break Some(next_token);
                }
            };

            // line postfix: check for directly preceding node that ends at the start token
            if let Some(prev_token) = prev_token
                && self.is_same_line(prev_token.span, end_token.span)
                && prev_token.token.ty != TokenType::Newline
                && let Some(target_node_id) = self
                    .find_node_ending_at(
                        &prev_token.span,
                        if is_full_line {
                            NodeSearch::BiggestOutermost
                        } else {
                            NodeSearch::SmallestOutermost
                        },
                    )
                    .map(|span| span.idx)
            {
                // line postfix boundary if next token is newline (or end)
                if next_token.is_none()
                    || next_token.unwrap().token.ty == TokenType::Newline
                    || next_token.unwrap().token.ty == TokenType::End
                {
                    return Some((AnnotationPosition::LinePostfixBoundary, target_node_id));
                }
                // otherwise regular line postfix
                else {
                    return Some((AnnotationPosition::LinePostfix, target_node_id));
                }
            }
            // line prefix: check for directly following node that starts at the end token
            else if let Some(next_token) = next_token
                && next_token.token.ty != TokenType::Newline
                && self.is_same_line(end_token.span, next_token.span)
                && let Some(target_node_id) = self
                    .find_node_starting_at(&next_token.span, NodeSearch::SmallestOutermost)
                    .map(|span| span.idx)
            {
                return Some((AnnotationPosition::LinePrefix, target_node_id));
            }
        }

        // block prefix: find the following targetable node
        let mut next_token_idx = token_idx as usize + group.len();
        while let Some(next_token) = tokens.get(next_token_idx) {
            #[cfg(debug_assertions)]
            let _next_token_str = self.get_span_str(next_token.span);
            if ignore_span.contains(&next_token.span)
                || ANNOTATION_TOKEN_TYPES.contains(&next_token.token.ty)
                || next_token.token.ty == TokenType::Whitespace
            {
                next_token_idx += 1;
                continue; // ignore non-targetable tokens
            }
            if let Some(enclosing_span) = enclosing_span
                && !enclosing_span.intersects(next_token.span)
            {
                break; // stop outside the enclosing scope
            }
            if let Some(next_node) =
                self.find_node_starting_at(&next_token.span, NodeSearch::BiggestOutermost)
            {
                return Some((AnnotationPosition::BlockPrefix, next_node.idx));
            }
            next_token_idx += 1;
        }

        // block postfix: find the preceding targetable node
        if !is_block_prefix_only && token_idx > 0 {
            let mut prev_token_idx = token_idx as usize;
            while prev_token_idx > 0 {
                prev_token_idx -= 1;
                let Some(prev_token) = tokens.get(prev_token_idx) else {
                    break;
                };
                #[cfg(debug_assertions)]
                let _prev_token_str = self.get_span_str(prev_token.span);
                if ignore_span.contains(&prev_token.span)
                    || ANNOTATION_TOKEN_TYPES.contains(&prev_token.token.ty)
                    || prev_token.token.ty == TokenType::Whitespace
                {
                    continue; // ignore non-targetable tokens
                }
                if let Some(enclosing_span) = enclosing_span
                    && !enclosing_span.intersects(prev_token.span)
                {
                    break; // stop outside the enclosing scope
                }
                if let Some(prev_node) =
                    self.find_node_ending_at(&prev_token.span, NodeSearch::BiggestOutermost)
                {
                    return Some((AnnotationPosition::BlockPostfix, prev_node.idx));
                }
            }
        }

        // find inner enclosing node (block infix)
        if !is_block_prefix_only && let Some(enclosing_node) = enclosing_scope {
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
    ) {
        debug_assert!(ANNOTATION_TOKEN_TYPES.contains(&token_type));
        debug_assert!(!group.is_empty());

        let start_token = group[0];
        let end_token = group[group.len() - 1];
        let span = Span::new(
            start_token.span.file,
            start_token.span.start,
            end_token.span.end,
        );
        #[cfg(debug_assertions)]
        let _span_str = self.get_span_str(span);

        // find the node to attach to
        let is_line_comment = start_token.token.ty == TokenType::LineComment
            || start_token.token.ty == TokenType::DocLineComment;
        let Some((position, target_node_id)) = self.find_annotation_target(
            token_idx,
            tokens,
            group,
            is_line_comment,
            false,
            ignore_span,
        ) else {
            let node_type = match token_type {
                TokenType::Newline => NodeType::Blank,
                TokenType::LineComment => NodeType::Comment,
                TokenType::BlockComment => NodeType::Comment,
                TokenType::DocLineComment => NodeType::Doc,
                TokenType::DocBlockComment => NodeType::Doc,
                _ => unreachable!("unexpected token type: {token_type:?}"),
            };
            let error = ParseError::unexpected_for(span, node_type);
            self.handle_error(&error);
            return; // could not find a position
        };

        // create the annotation
        let annotation_id = match token_type {
            TokenType::Newline => {
                let lines = group.len() as u32 - 1;
                let blank = self.tree.insert(Blank { lines }, span);
                self.tree.insert(
                    Annotation::Blank {
                        node: blank,
                        position,
                    },
                    span,
                )
            }
            TokenType::LineComment => {
                let string = self.clean_annotation_string(token_type, group);
                let string = self.strings.intern(string);
                let comment = self.tree.insert(
                    Comment {
                        string,
                        style: CommentStyle::Slash,
                    },
                    span,
                );
                self.tree.insert(
                    Annotation::Comment {
                        node: comment,
                        position,
                    },
                    span,
                )
            }
            TokenType::BlockComment => {
                let string = self.clean_annotation_string(token_type, group);
                let string = self.strings.intern(string);
                let comment = self.tree.insert(
                    Comment {
                        string,
                        style: CommentStyle::Star,
                    },
                    span,
                );
                self.tree.insert(
                    Annotation::Comment {
                        node: comment,
                        position,
                    },
                    span,
                )
            }
            TokenType::DocLineComment => {
                let string = self.clean_annotation_string(token_type, group);
                let string = self.strings.intern(string);
                let doc = self.tree.insert(
                    Doc {
                        string,
                        style: DocStyle::Slash,
                    },
                    span,
                );
                self.tree.insert(
                    Annotation::Doc {
                        node: doc,
                        position,
                    },
                    span,
                )
            }
            TokenType::DocBlockComment => {
                let string = self.clean_annotation_string(token_type, group);
                let string = self.strings.intern(string);
                let doc = self.tree.insert(
                    Doc {
                        string,
                        style: DocStyle::Star,
                    },
                    span,
                );
                self.tree.insert(
                    Annotation::Doc {
                        node: doc,
                        position,
                    },
                    span,
                )
            }
            _ => unreachable!("unexpected token type: {token_type:?}"),
        };

        // attach the annotation to the node
        self.tree.append_annotation(target_node_id, annotation_id);
    }

    /// Clean annotation tokens into their inner string, preserving intentional spacing.
    fn clean_annotation_string(&self, token_type: TokenType, group: &[TokenSpan]) -> String {
        debug_assert!(!group.is_empty());

        let mut cleaned_tokens = Vec::with_capacity(group.len());

        for token in group {
            // strip comment prefixes and suffixes
            let raw_str = self.file.get_span_str(token.span).unwrap_or_default();
            let mut inner_str = match token_type {
                TokenType::LineComment => raw_str.strip_prefix("//").unwrap_or(raw_str),
                TokenType::DocLineComment => raw_str.strip_prefix("///").unwrap_or(raw_str),
                TokenType::BlockComment => raw_str
                    .strip_prefix("/*")
                    .unwrap_or(raw_str)
                    .strip_suffix("*/")
                    .unwrap_or(raw_str),
                TokenType::DocBlockComment => raw_str
                    .strip_prefix("/**")
                    .unwrap_or(raw_str)
                    .strip_suffix("*/")
                    .unwrap_or(raw_str),
                _ => panic!("unexpected token type: {token_type:?}"),
            };

            if matches!(
                token_type,
                TokenType::BlockComment | TokenType::DocBlockComment
            ) {
                if inner_str.starts_with(' ') {
                    inner_str = &inner_str[1..];
                }
                while inner_str.ends_with(' ') {
                    inner_str = inner_str.strip_suffix(' ').unwrap();
                }
            }

            // clean up whitespace and formatting characters
            let cleaned = match token_type {
                TokenType::LineComment | TokenType::DocLineComment => {
                    if inner_str.contains('\n') {
                        // strip leading space from each line
                        inner_str
                            .lines()
                            .map(|line| line.strip_prefix(' ').unwrap_or(line).to_owned())
                            .collect::<Vec<_>>()
                            .join("\n")
                    } else {
                        // strip leading space
                        inner_str.strip_prefix(' ').unwrap_or(inner_str).to_owned()
                    }
                }
                TokenType::BlockComment | TokenType::DocBlockComment => {
                    if inner_str.trim().is_empty() {
                        String::new()
                    } else {
                        // strip block comment formatting with optional asterisk prefixes
                        // (also strip first/last space before/after the asterisk)
                        inner_str
                            .split('\n')
                            .map(|line| {
                                let mut cleaned_line = if let Some((idx, ch)) =
                                    line.char_indices().find(|&(_, ch)| ch != ' ')
                                    && ch == '*'
                                {
                                    // strip asterisk prefix and following space
                                    let mut line_str = &line[idx + ch.len_utf8()..];
                                    if line_str.starts_with(' ') {
                                        line_str = &line_str[1..];
                                    }
                                    line_str.to_owned()
                                } else {
                                    // strip leading space
                                    line.strip_prefix(' ').unwrap_or(line).to_owned()
                                };

                                if cleaned_line.chars().all(|ch| ch == ' ') {
                                    cleaned_line.clear();
                                } else {
                                    cleaned_line = cleaned_line.trim_end_matches(' ').to_owned();
                                }

                                cleaned_line
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
    use dyst_ast::{
        Annotation, AnnotationPosition, Argument, BinaryOperator, Blank, Block, BlockFormat,
        Comment, CommentStyle, DeclarationDescriptor, Decorator, Definition, Doc, DocStyle,
        Expression, Key, Name, Property, ScalarLiteral, Tag,
    };

    use crate::parse::tests::TestParser;
    use crate::{assert_node, assert_path, assert_string};

    /// Block comments should retain all their newlines (including leading and trailing newlines).
    #[test]
    fn test_attach_block_comment_retain_newlines() {
        let mut test: TestParser = TestParser::new(
            r#"
/* 
 * Comment 1
 */
let x;
/*
 * Comment 2.1
 * Comment 2.2
 * Comment 2.3
 */
let y;
"#,
        );
        let mut parser = test.prepare();
        let expressions = parser.eat_block_body(BlockFormat::Implicit).unwrap();
        parser.finish();

        assert_eq!(expressions.len(), 2);

        // let x;
        let x_annotations = parser.tree.get_annotations(expressions[0].id);
        assert_eq!(x_annotations.len(), 1);
        assert_node!(parser.tree, x_annotations[0], Annotation::Comment { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPrefix);
            assert_node!(parser.tree, *node, Comment { string, style } => {
                assert_string!(parser, *string, "\nComment 1\n");
                assert_eq!(*style, CommentStyle::Star);
            });
        });
        // let y;
        let y_annotations = parser.tree.get_annotations(expressions[1].id);
        assert_eq!(y_annotations.len(), 1);
        assert_node!(parser.tree, y_annotations[0], Annotation::Comment { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPrefix);
            assert_node!(parser.tree, *node, Comment { string, style } => {
                assert_string!(parser, *string, "\nComment 2.1\nComment 2.2\nComment 2.3\n");
                assert_eq!(*style, CommentStyle::Star);
            });
        });
    }

    /// Tag annotations should be parsed around a struct.
    #[test]
    fn test_attach_tag_annotations_surrounding_struct() {
        let mut test = TestParser::new(
            r#"
/// Test doc
#dyst.BeginGroup("MyGroup", length: 1)
struct Test {}
#dyst.EndGroup
"#,
        );
        let mut parser = test.prepare();
        let expressions = parser.parse();

        assert_eq!(expressions.len(), 1);
        let annotations = parser.tree.get_annotations(expressions[0].id);
        assert_eq!(annotations.len(), 3);
        // doc block prefix
        // /// Test doc
        assert_node!(parser.tree, annotations[0], Annotation::Doc { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPrefix);
            assert_node!(parser.tree, *node, Doc { string, style } => {
                assert_string!(parser, *string, "Test doc");
                assert_eq!(*style, DocStyle::Slash);
            });
        });
        // tag block prefix
        // #BeginGroup("MyGroup", 1)
        assert_node!(parser.tree, annotations[1], Annotation::Tag { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPrefix);
            assert_node!(parser.tree, *node, Tag { left, arguments } => {
                assert_path!(parser, *left, "dyst.BeginGroup");
                assert!(arguments.is_some());
                assert_eq!(arguments.as_ref().unwrap().len(), 2);
                // "MyGroup"
                assert_node!(parser.tree, arguments.as_ref().unwrap()[0], Argument::Positional { modifiers: _, value } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(string_id)) => {
                        assert_string!(parser, *string_id, "MyGroup");
                    });
                });
                // 1
                assert_node!(parser.tree, arguments.as_ref().unwrap()[1], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                    assert_string!(parser, *name, "length");
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
                });
            });
        });
        // tag block postfix
        // #EndGroup
        assert_node!(parser.tree, annotations[2], Annotation::Tag { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPostfix);
            assert_node!(parser.tree, *node, Tag { left, arguments } => {
                assert_path!(parser, *left, "dyst.EndGroup");
                assert!(arguments.is_none());
            });
        });
    }

    /// Decorator annotations should be parsed around any block.
    #[test]
    fn test_attach_decorator_to_function() {
        let mut test = TestParser::new("@foo\nfunction foo() { }");
        let mut parser = test.prepare();
        let expressions = parser.parse();

        assert_eq!(expressions.len(), 1);
        let annotations = parser.tree.get_annotations(expressions[0].id);
        assert_eq!(annotations.len(), 1);
        // @foo
        assert_node!(parser.tree, annotations[0], Annotation::Decorator { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPrefix);
            assert_node!(parser.tree, *node, Decorator { left, arguments } => {
                assert_path!(parser, *left, "foo");
                assert!(arguments.is_none());
            });
        });
    }

    /// Annotations should default to infix within source if no other position is found.
    #[test]
    fn test_attach_annotations_fallback_to_infix() {
        // C and D are not in "valid" positions and should fall back to infix
        let mut test = TestParser::new("#A\n#B struct #C Test #D { #E } #F\n#G");
        let mut parser = test.prepare();
        let expressions = parser.parse();

        assert_eq!(expressions.len(), 1);
        let annotations = parser.tree.get_annotations(expressions[0].id);
        assert_eq!(annotations.len(), 4);
        // A: block prefix
        assert_node!(parser.tree, annotations[0], Annotation::Tag { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPrefix);
            assert_node!(parser.tree, *node, Tag { left, arguments } => {
                assert_path!(parser, *left, "A");
                assert!(arguments.is_none());
            });
        });
        // B: line prefix
        assert_node!(parser.tree, annotations[1], Annotation::Tag { node, position } => {
            assert_eq!(*position, AnnotationPosition::LinePrefix);
            assert_node!(parser.tree, *node, Tag { left, arguments } => {
                assert_path!(parser, *left, "B");
                assert!(arguments.is_none());
            });
        });
        // F: line postfix boundary
        assert_node!(parser.tree, annotations[2], Annotation::Tag { node, position } => {
            assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
            assert_node!(parser.tree, *node, Tag { left, arguments } => {
                assert_path!(parser, *left, "F");
                assert!(arguments.is_none());
            });
        });
        // G: block postfix
        assert_node!(parser.tree, annotations[3], Annotation::Tag { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPostfix);
            assert_node!(parser.tree, *node, Tag { left, arguments } => {
                assert_path!(parser, *left, "G");
                assert!(arguments.is_none());
            });
        });

        // block infix to innermost node (Definition::Struct)
        assert_node!(parser.tree, expressions[0], Expression::Definition(node) => {
            let annotations = parser.tree.get_annotations(node.id);
            assert_eq!(annotations.len(), 3);
            // C: block infix
            assert_node!(parser.tree, annotations[0], Annotation::Tag { node, position } => {
                assert_eq!(*position, AnnotationPosition::BlockInfix);
                assert_node!(parser.tree, *node, Tag { left, arguments } => {
                    assert_path!(parser, *left, "C");
                    assert!(arguments.is_none());
                });
            });
            // D: block infix
            assert_node!(parser.tree, annotations[1], Annotation::Tag { node, position } => {
                assert_eq!(*position, AnnotationPosition::BlockInfix);
                assert_node!(parser.tree, *node, Tag { left, arguments } => {
                    assert_path!(parser, *left, "D");
                    assert!(arguments.is_none());
                });
            });
            // E: block infix
            assert_node!(parser.tree, annotations[2], Annotation::Tag { node, position } => {
                assert_eq!(*position, AnnotationPosition::BlockInfix);
                assert_node!(parser.tree, *node, Tag { left, arguments } => {
                    assert_path!(parser, *left, "E");
                    assert!(arguments.is_none());
                });
            });
        });
    }

    /// Line suffix is attached to the previous node on the same line.
    #[test]
    fn test_attach_line_postfix_to_expression() {
        let mut test = TestParser::new("let A = 1 // line comment");
        let mut parser = test.prepare();
        let expressions = parser.parse();

        // let A = 1
        assert_eq!(expressions.len(), 1);
        // line comment, suffix
        let annotations = parser.tree.get_annotations(expressions[0].id);
        assert_eq!(annotations.len(), 1);
        assert_node!(parser.tree, annotations[0], Annotation::Comment { node, position } => {
            assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
            assert_node!(parser.tree, *node, Comment { string, style } => {
                assert_string!(parser, *string, "line comment");
                assert_eq!(*style, CommentStyle::Slash);
            });
        });
    }

    /// Multi-line suffix is attached to the previous node on the same line as a block postfix.
    #[test]
    fn test_attach_multi_line_postfix_to_expression() {
        let mut test = TestParser::new(
            "let A = 1 /* line comment
over multiple lines with trailing space    */",
        );
        let mut parser = test.prepare();
        let expressions = parser.parse();

        // let A = 1
        assert_eq!(expressions.len(), 1);
        // block comment, postfix
        let annotations = parser.tree.get_annotations(expressions[0].id);
        assert_eq!(annotations.len(), 1);
        assert_node!(parser.tree, annotations[0], Annotation::Comment { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPostfix);
            assert_node!(parser.tree, *node, Comment { string, style } => {
                assert_string!(parser, *string, "line comment\nover multiple lines with trailing space");
                assert_eq!(*style, CommentStyle::Star);
            });
        });
    }

    /// Multiline block doc comments are cleaned up properly.
    #[test]
    fn test_clean_multiline_block_doc() {
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
        parser.finish();

        assert_node!(parser.tree, block, Block { expressions, .. } => {
            assert_eq!(expressions.len(), 1);
            let annotations = parser.tree.get_annotations(expressions[0].id);
            assert_eq!(annotations.len(), 1);
            assert_node!(parser.tree, annotations[0], Annotation::Doc { node, position } => {
                assert_eq!(*position, AnnotationPosition::BlockPrefix);
                assert_node!(parser.tree, *node, Doc { string, style } => {
                    assert_string!(parser, *string, "some multiline\nblock comment\nover multiple lines");
                    assert_eq!(*style, DocStyle::Star);
                });
            });
        });
    }

    /// Block doc comments in definition blocks should attach to the following function.
    #[test]
    fn test_attach_doc_block_prefix_to_next_function() {
        let mut test = TestParser::new(
            r"interface X {
    /** Doc A */
    a(): A
    /** Doc B */
    b(): B
}",
        );
        let mut parser = test.prepare();
        let interface_id = parser
            .eat_interface(DeclarationDescriptor::default())
            .unwrap();
        parser.finish();

        // interface X
        assert_node!(parser.tree, interface_id, Definition::Interface { properties, .. } => {
            assert_eq!(properties.len(), 2);

            // a(): A
            let annotations = parser.tree.get_annotations(properties[0].id);
            assert_eq!(annotations.len(), 1);
            assert_node!(parser.tree, annotations[0], Annotation::Doc { node, position } => {
                assert_eq!(*position, AnnotationPosition::BlockPrefix);
                assert_node!(parser.tree, *node, Doc { string, style } => {
                    assert_eq!(*style, DocStyle::Star);
                    assert_string!(parser, *string, "Doc A");
                });
            });

            // b(): B
            let annotations = parser.tree.get_annotations(properties[1].id);
            assert_eq!(annotations.len(), 1);
            assert_node!(parser.tree, annotations[0], Annotation::Doc { node, position } => {
                assert_eq!(*position, AnnotationPosition::BlockPrefix);
                assert_node!(parser.tree, *node, Doc { string, style } => {
                    assert_eq!(*style, DocStyle::Star);
                    assert_string!(parser, *string, "Doc B");
                });
            });
        });
    }

    /// Block doc comments should attach to the following function in a nested function.
    #[test]
    fn test_attach_doc_block_prefix_to_nested_function() {
        let mut test = TestParser::new(
            r"function foo() {
    /** Doc A */
    function a(): A
    /** Doc B */
    function b(): B
    /** Doc C */
    function c(): C {
        remove(hey.so)
    }
}",
        );
        let mut parser = test.prepare();
        let function = parser
            .eat_function(DeclarationDescriptor::default(), false, false)
            .unwrap();
        parser.finish();

        // function foo()
        assert_node!(parser.tree, function, Definition::Function { body: body_id, .. } => {
            assert_node!(parser.tree, body_id.unwrap(), Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { expressions, .. } => {
                    assert_eq!(expressions.len(), 3);

                    // function a(): A
                    assert_node!(parser.tree, expressions[0], Expression::Definition(node) => {
                        assert_node!(parser.tree, *node, Definition::Function { descriptor, .. } => {
                            assert_string!(parser, descriptor.name.unwrap().string(), "a");
                        });
                    });
                    let annotations = parser.tree.get_annotations(expressions[0].id);
                    assert_eq!(annotations.len(), 1);
                    assert_node!(parser.tree, annotations[0], Annotation::Doc { node, position } => {
                        assert_eq!(*position, AnnotationPosition::BlockPrefix);
                        assert_node!(parser.tree, *node, Doc { string, style } => {
                            assert_eq!(*style, DocStyle::Star);
                            assert_string!(parser, *string, "Doc A");
                        });
                    });

                    // function b(): B
                    assert_node!(parser.tree, expressions[1], Expression::Definition(node) => {
                        assert_node!(parser.tree, *node, Definition::Function { descriptor, .. } => {
                            assert_string!(parser, descriptor.name.unwrap().string(), "b");
                        });
                    });
                    let annotations = parser.tree.get_annotations(expressions[1].id);
                    assert_eq!(annotations.len(), 1);
                    assert_node!(parser.tree, annotations[0], Annotation::Doc { node, position } => {
                        assert_eq!(*position, AnnotationPosition::BlockPrefix);
                        assert_node!(parser.tree, *node, Doc { string, style } => {
                            assert_eq!(*style, DocStyle::Star);
                            assert_string!(parser, *string, "Doc B");
                        });
                    });

                    // function c() C { .. }
                    assert_node!(parser.tree, expressions[2], Expression::Definition(node) => {
                        assert_node!(parser.tree, *node, Definition::Function { descriptor, .. } => {
                            assert_string!(parser, descriptor.name.unwrap().string(), "c");
                        });
                    });
                    let annotations = parser.tree.get_annotations(expressions[2].id);
                    assert_eq!(annotations.len(), 1);
                    assert_node!(parser.tree, annotations[0], Annotation::Doc { node, position } => {
                        assert_eq!(*position, AnnotationPosition::BlockPrefix);
                        assert_node!(parser.tree, *node, Doc { string, style } => {
                            assert_eq!(*style, DocStyle::Star);
                            assert_string!(parser, *string, "Doc C");
                        });
                    });
                });
            });
        });
    }

    /// Inline prefix, infix and suffix comments should be attached to closest inner node on the same line.
    #[test]
    fn test_attach_line_prefix_infix_postfix_to_expression() {
        let mut test =
            TestParser::new("let X = /* Pre-A comment */ A /* A comment */ && B /* B comment */");
        let mut parser = test.prepare();
        let expressions = parser.eat_block_body(BlockFormat::Implicit).unwrap();
        parser.finish();

        assert_eq!(expressions.len(), 1);

        // A && B
        assert_node!(parser.tree, expressions[0], Expression::Let { value, .. } => {
            assert_node!(parser.tree, value.unwrap(), Expression::Binary { left, right, operator } => {
                assert_eq!(*operator, BinaryOperator::And);
                // A
                assert_node!(parser.tree, *left, Expression::Path { path, static_arguments: _ } => {
                    assert_path!(parser, *path, "A");
                });
                let annotations = parser.tree.get_annotations(left.id);
                // line prefix, pre-A comment
                assert_eq!(annotations.len(), 2);
                assert_node!(parser.tree, annotations[0], Annotation::Comment { node, position } => {
                    assert_eq!(*position, AnnotationPosition::LinePrefix);
                    assert_node!(parser.tree, *node, Comment { string, style } => {
                        assert_string!(parser, *string, "Pre-A comment");
                        assert_eq!(*style, CommentStyle::Star);
                    });
                });
                // line postfix, A comment
                assert_node!(parser.tree, annotations[1], Annotation::Comment { node, position } => {
                    assert_eq!(*position, AnnotationPosition::LinePostfix);
                    assert_node!(parser.tree, *node, Comment { string, style } => {
                        assert_string!(parser, *string, "A comment");
                        assert_eq!(*style, CommentStyle::Star);
                    });
                });

                // B
                assert_node!(parser.tree, *right, Expression::Path { path, static_arguments: _ } => {
                    assert_path!(parser, *path, "B");
                });
                // line postfix boundary, B comment
                let annotations = parser.tree.get_annotations(right.id);
                assert_eq!(annotations.len(), 1);
                assert_node!(parser.tree, annotations[0], Annotation::Comment { node, position } => {
                    assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
                    assert_node!(parser.tree, *node, Comment { string, style } => {
                        assert_string!(parser, *string, "B comment");
                        assert_eq!(*style, CommentStyle::Star);
                    });
                });
            });
        });
    }

    /// Line suffix is attached separatelyfrom other surrounding comments.
    #[test]
    fn test_attach_line_postfix_to_expression_with_surrounding_comments() {
        let mut test = TestParser::new(
            r"
// block prefix comment
let A = 1 // line suffix comment
// block postfix comment
",
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();
        let expressions = parser.eat_block_body(BlockFormat::Implicit).unwrap();
        parser.finish();

        // let A = 1
        assert_eq!(expressions.len(), 1);
        let annotations = parser.tree.get_annotations(expressions[0].id);
        assert_eq!(annotations.len(), 3);

        // block prefix comment
        assert_node!(parser.tree, annotations[0], Annotation::Comment { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPrefix);
            assert_node!(parser.tree, *node, Comment { string, style } => {
                assert_string!(parser, *string, "block prefix comment");
                assert_eq!(*style, CommentStyle::Slash);
            });
        });
        // line postfix boundary comment
        assert_node!(parser.tree, annotations[1], Annotation::Comment { node, position } => {
            assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
            assert_node!(parser.tree, *node, Comment { string, style } => {
                assert_string!(parser, *string, "line suffix comment");
                assert_eq!(*style, CommentStyle::Slash);
            });
        });
        // block postfix comment
        assert_node!(parser.tree, annotations[2], Annotation::Comment { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPostfix);
            assert_node!(parser.tree, *node, Comment { string, style } => {
                assert_string!(parser, *string, "block postfix comment");
                assert_eq!(*style, CommentStyle::Slash);
            });
        });
    }

    /// Blanks (>2 successive newlines) are just annotations and should be attached to the next node.
    /// Like any other annotation, if no next or containing node is found, attach to previous node as suffix.
    #[test]
    fn test_attach_blanks_to_expressions() {
        let mut test = TestParser::new("\n\nlet A = 1\n\nlet B = 2\n\n");
        let mut parser = test.prepare();
        let expressions = parser.eat_block_body(BlockFormat::Implicit).unwrap();
        parser.finish();
        assert_eq!(expressions.len(), 2);

        // A has one prefix block blank
        let a_annotations = parser.tree.get_annotations(expressions[0].id);
        assert_eq!(a_annotations.len(), 1);
        assert_node!(parser.tree, a_annotations[0], Annotation::Blank { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPrefix);
            assert_node!(parser.tree, *node, Blank { lines } => {
                assert_eq!(*lines, 1);
            });
        });

        // B has one prefix block blank and one postfix block blank
        // (the postfix blank after B because there is nothing else to attach to)
        let b_annotations = parser.tree.get_annotations(expressions[1].id);
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

    /// Annotations inside an empty node should be treated as infix within the innermost containing node.
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

        let function = parser
            .eat_function(DeclarationDescriptor::default(), false, false)
            .unwrap();
        parser.finish();

        // (annotation should be infix to innermost node, i.e. the block)
        assert_node!(parser.tree, function, Definition::Function { body: body_id, .. } => {
            assert_node!(parser.tree, body_id.unwrap(), Expression::Block(block_id) => {
                // block comment, infix
                let annotations = parser.tree.get_annotations(block_id.id);
                assert_eq!(annotations.len(), 1);
                assert_node!(parser.tree, annotations[0], Annotation::Comment { node, position } => {
                    assert_eq!(*position, AnnotationPosition::BlockInfix);
                    assert_node!(parser.tree, *node, Comment { string, style } => {
                        assert_string!(parser, *string, "block comment, infix");
                        assert_eq!(*style, CommentStyle::Slash);
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

    // random comment
}",
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();
        let expressions = parser.eat_block_body(BlockFormat::Implicit).unwrap();
        parser.finish();

        // struct Floof
        assert_eq!(expressions.len(), 1);
        let annotations = parser.tree.get_annotations(expressions[0].id);
        assert_eq!(annotations.len(), 3);
        // doc block prefix, floating
        assert_node!(parser.tree, annotations[0], Annotation::Doc { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPrefix);
            assert_node!(parser.tree, *node, Doc { string, style } => {
                assert_string!(parser, *string, "doc, floating");
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
                assert_string!(parser, *string, "doc, struct\ndoc, struct continued");
                assert_eq!(*style, DocStyle::Slash);
            });
        });

        // struct Floof
        assert_node!(parser.tree, expressions[0], Expression::Definition(node) => {
            assert_node!(parser.tree, *node, Definition::Struct { properties, .. } => {
                // a: int32
                assert_eq!(properties.len(), 1);
                assert_node!(parser.tree, properties[0], Property::Field { key: Some(Key::Name(Name::Identifier(name))), .. } => {
                    assert_string!(parser, *name, "a");
                    let annotations = parser.tree.get_annotations(properties[0].id);
                    assert_eq!(annotations.len(), 4);

                    // doc block prefix
                    // struct field\ndoc, struct field continued
                    assert_node!(parser.tree, annotations[0], Annotation::Doc { node, position } => {
                        assert_eq!(*position, AnnotationPosition::BlockPrefix);
                        assert_node!(parser.tree, *node, Doc { string, style } => {
                            assert_string!(parser, *string, "doc, struct field\ndoc, struct field continued");
                            assert_eq!(*style, DocStyle::Slash);
                        });
                    });

                    // doc line postfix boundary
                    // doc, struct field infix
                    assert_node!(parser.tree, annotations[1], Annotation::Comment { node, position } => {
                        assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
                        assert_node!(parser.tree, *node, Comment { string, style } => {
                            assert_string!(parser, *string, "doc, struct field infix");
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
                            assert_string!(parser, *string, "random comment");
                            assert_eq!(*style, CommentStyle::Slash);
                        });
                    });
                });
                });
        })
    }

    #[test]
    fn test_attach_annotations_in_mixed_nested_declaration() {
        let mut test = TestParser::new(
            r"
// Outer comment
export namespace Outer {
    // Middle comment
    export namespace Middle {
        // Inner comment
        export type Inner = { }
    }
}",
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();
        let expressions = parser.eat_block_body(BlockFormat::Implicit).unwrap();
        parser.finish();

        // Outer
        assert_node!(parser.tree, expressions[0], Expression::Definition(node) => {
            // Outer comment
            let annotations = parser.tree.get_annotations(expressions[0].id);
            assert_eq!(annotations.len(), 1);
            assert_node!(parser.tree, annotations[0], Annotation::Comment { node, position } => {
                assert_eq!(*position, AnnotationPosition::BlockPrefix);
                assert_node!(parser.tree, *node, Comment { string, style } => {
                    assert_string!(parser, *string, "Outer comment");
                    assert_eq!(*style, CommentStyle::Slash);
                });
            });

            // Outer
            assert_node!(parser.tree, *node, Definition::Namespace { descriptor, expressions, .. } => {
                assert_string!(parser, descriptor.name.unwrap().string(), "Outer");

                // Middle comment
                assert_eq!(expressions.len(), 1);
                let annotations = parser.tree.get_annotations(expressions[0].id);
                assert_eq!(annotations.len(), 1);
                assert_node!(parser.tree, annotations[0], Annotation::Comment { node, position } => {
                    assert_eq!(*position, AnnotationPosition::BlockPrefix);
                    assert_node!(parser.tree, *node, Comment { string, style } => {
                        assert_string!(parser, *string, "Middle comment");
                        assert_eq!(*style, CommentStyle::Slash);
                    });
                });

                // Middle
                assert_node!(parser.tree, expressions[0], Expression::Definition(node) => {
                    assert_node!(parser.tree, *node, Definition::Namespace { descriptor, expressions, .. } => {
                        assert_string!(parser, descriptor.name.unwrap().string(), "Middle");

                        // Inner comment
                        assert_eq!(expressions.len(), 1);
                        let annotations = parser.tree.get_annotations(expressions[0].id);
                        assert_eq!(annotations.len(), 1);
                        assert_node!(parser.tree, annotations[0], Annotation::Comment { node, position } => {
                            assert_eq!(*position, AnnotationPosition::BlockPrefix);
                            assert_node!(parser.tree, *node, Comment { string, style } => {
                                assert_string!(parser, *string, "Inner comment");
                                assert_eq!(*style, CommentStyle::Slash);
                            });
                        });
                    });
                });
            });

        });
    }

    #[test]
    fn test_attach_multiple_comments_around_expression_in_successive_blocks() {
        let mut test = TestParser::new(
            r"root: {
    // comment part 0
    a: {
        // comment part 1
        // comment part 2
        const A = 1
        // comment part 3
        // comment part 4
    }
    // comment part 5
    // comment part 6
    b: {
        // comment part 7
        // comment part 8
        const B = 2
        // comment part 9
        // comment part 10
    }
    // comment part 11
}",
        );
        let mut parser = test.prepare();
        let block = parser.eat_block().unwrap();
        parser.finish();

        assert_node!(parser.tree, block, Block { expressions, .. } => {
            assert_eq!(expressions.len(), 2);

            // a
            let a = expressions[0];
            let a_annotations = parser.tree.get_annotations(a.id);
            assert_eq!(a_annotations.len(), 1); // (0 as block prefix)

            // comment part 0
            assert_node!(parser.tree, a_annotations[0], Annotation::Comment { node, position } => {
                assert_eq!(*position, AnnotationPosition::BlockPrefix);
                assert_node!(parser.tree, *node, Comment { string, style } => {
                    assert_string!(parser, *string, "comment part 0");
                    assert_eq!(*style, CommentStyle::Slash);
                });
            });

            // a: { .. }
            assert_node!(parser.tree, a, Expression::Block (block_id) => {
                assert_node!(parser.tree, *block_id, Block { expressions, .. } => {
                    assert_eq!(expressions.len(), 1);
                    let annotations = parser.tree.get_annotations(expressions[0].id);
                    assert_eq!(annotations.len(), 2);
                    // comment part 1\ncomment part 2
                    assert_node!(parser.tree, annotations[0], Annotation::Comment { node, position } => {
                        assert_eq!(*position, AnnotationPosition::BlockPrefix);
                        assert_node!(parser.tree, *node, Comment { string, style } => {
                            assert_string!(parser, *string, "comment part 1\ncomment part 2");
                            assert_eq!(*style, CommentStyle::Slash);
                        });
                    });
                    // comment part 3\ncomment part 4
                    assert_node!(parser.tree, annotations[1], Annotation::Comment { node, position } => {
                        assert_eq!(*position, AnnotationPosition::BlockPostfix);
                        assert_node!(parser.tree, *node, Comment { string, style } => {
                            assert_string!(parser, *string, "comment part 3\ncomment part 4");
                            assert_eq!(*style, CommentStyle::Slash);
                        });
                    });
                });
            });

            // b
            let b = expressions[1];
            let b_annotations = parser.tree.get_annotations(b.id);
            assert_eq!(b_annotations.len(), 2); // (5+6 as block prefix, 11 as block postfix)

            // comment part 5\ncomment part 6
            assert_node!(parser.tree, b_annotations[0], Annotation::Comment { node, position } => {
                assert_eq!(*position, AnnotationPosition::BlockPrefix);
                assert_node!(parser.tree, *node, Comment { string, style } => {
                    assert_string!(parser, *string, "comment part 5\ncomment part 6");
                    assert_eq!(*style, CommentStyle::Slash);
                });
            });

            // b: { .. }
            assert_node!(parser.tree, b, Expression::Block (block_id) => {
                assert_node!(parser.tree, *block_id, Block { expressions, .. } => {
                    assert_eq!(expressions.len(), 1);
                    let annotations = parser.tree.get_annotations(expressions[0].id);
                    assert_eq!(annotations.len(), 2);
                    // comment part 7\ncomment part 8
                    assert_node!(parser.tree, annotations[0], Annotation::Comment { node, position } => {
                        assert_eq!(*position, AnnotationPosition::BlockPrefix);
                        assert_node!(parser.tree, *node, Comment { string, style } => {
                            assert_string!(parser, *string, "comment part 7\ncomment part 8");
                            assert_eq!(*style, CommentStyle::Slash);
                        });
                    });
                    // comment part 9\ncomment part 10
                    assert_node!(parser.tree, annotations[1], Annotation::Comment { node, position } => {
                        assert_eq!(*position, AnnotationPosition::BlockPostfix);
                        assert_node!(parser.tree, *node, Comment { string, style } => {
                            assert_string!(parser, *string, "comment part 9\ncomment part 10");
                            assert_eq!(*style, CommentStyle::Slash);
                        });
                    });
                });
            });

            // comment part 11
            assert_node!(parser.tree, b_annotations[1], Annotation::Comment { node, position } => {
                assert_eq!(*position, AnnotationPosition::BlockPostfix);
                assert_node!(parser.tree, *node, Comment { string, style } => {
                    assert_string!(parser, *string, "comment part 11");
                    assert_eq!(*style, CommentStyle::Slash);
                });
            });
        });
    }
}
