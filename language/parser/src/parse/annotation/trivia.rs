use crate::{ParseError, Parser};
use destack_ast::{
    ANNOTATION_NODE_TYPES, Annotation, AnnotationPosition, Argument, Blank, Comment, CommentStyle,
    Declaration, Doc, DocStyle, Expression, LocalNodeId, NodeType, TokenSpan, TokenType,
};
use destack_source::{MultiSpan, NodeSearchMode, Span};

const TRIVIA_TOKEN_TYPES: [TokenType; 5] = [
    TokenType::Newline,
    TokenType::LineComment,
    TokenType::DocLineComment,
    TokenType::BlockComment,
    TokenType::DocBlockComment,
];

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum TriviaSeparatorKind {
    Comma,
    Pipe,
    Ampersand,
    Colon,
    Maybe,
    As,
    Satisfies,
}

impl Parser {
    /// Attach comment, doc, and blank trivia to AST owners.
    pub(crate) fn attach_trivia_annotations(&mut self) {
        // skip when the stream has no trivia to bind
        if !self.token_stream.has_comment_trivia_tokens()
            && !self.token_stream.has_blank_trivia_tokens()
        {
            return;
        }

        // keep attach_trivia idempotent for callers that may call it more than once
        if self.has_attached_trivia_annotations() {
            return;
        }

        // materialize the full stream and index once before ownership lookups
        self.token_stream.lex_to_end();
        self.tree.build_position_index();

        // collect merged semantic and side tokens for trivia grouping
        let (tokens, line_indices) = self.collect_trivia_tokens();
        if tokens.is_empty() {
            return;
        }

        // precompute statement wrappers for statement promotion
        let statement_wrappers = self.collect_statement_wrappers();

        // ignore existing side annotation spans: mostly decorators and already-attached side nodes
        let ignore_span = self.compute_side_span();
        self.attach_side_trivia(&tokens, &line_indices, &ignore_span, &statement_wrappers);

        // keep owner-local annotation order stable by source span
        self.tree.sort_annotations();
    }

    /// Return true when comment or doc or blank annotations are already attached.
    fn has_attached_trivia_annotations(&self) -> bool {
        self.tree.iter_nodes::<Annotation>().any(|annotation_id| {
            matches!(
                self.tree.get(annotation_id),
                Annotation::Blank { .. } | Annotation::Doc { .. } | Annotation::Comment { .. }
            )
        })
    }

    /// Build statement wrapper ids keyed by expression id.
    fn collect_statement_wrappers(&self) -> Vec<Option<u32>> {
        let mut wrappers = vec![None; self.tree.next_id() as usize];

        // expression statement wrappers
        let mut node_id = 0u32;
        while node_id < self.tree.next_id() {
            if self.tree.get_node_type(node_id) == NodeType::Expression {
                let expression_id = LocalNodeId::<Expression>::new(node_id);
                if let Expression::Statement(inner_id) = self.tree.get(expression_id) {
                    wrappers[inner_id.id as usize] = Some(node_id);
                }
            }
            node_id += 1;
        }

        wrappers
    }

    /// Collect semantic and side tokens in source order with matching line indices.
    fn collect_trivia_tokens(&self) -> (Vec<TokenSpan>, Vec<u32>) {
        let mut tokens: Vec<TokenSpan> = Vec::new();
        let mut line_indices: Vec<u32> = Vec::new();

        // keep line index tracking branchless for the common push path
        let line_starts = self.file.line_start_offsets.as_deref();
        let mut line_index = 0usize;
        let mut next_line_start = line_starts
            .and_then(|starts| starts.get(1).copied())
            .unwrap_or(u32::MAX);
        let mut push_token = |token: TokenSpan| {
            // deduplicate identical tokens that can appear from speculative lexer rewinds
            if let Some(last_token) = tokens.last()
                && last_token.token.ty == token.token.ty
                && last_token.span == token.span
            {
                return;
            }

            if let Some(starts) = line_starts {
                while token.span.start >= next_line_start {
                    line_index += 1;
                    next_line_start = starts.get(line_index + 1).copied().unwrap_or(u32::MAX);
                }
                line_indices.push(line_index as u32);
            } else {
                line_indices.push(0);
            }

            tokens.push(token);
        };

        // fast path: no side comments means blanks come only from semantic newlines
        if !self.token_stream.has_comment_trivia_tokens() {
            for token in self.token_stream.tokens() {
                if token.token.ty != TokenType::Whitespace {
                    push_token(*token);
                }
            }
            return (tokens, line_indices);
        }

        // merge semantic and side streams by span start
        let mut semantic_index = 0usize;
        let mut side_index = 0usize;
        let semantic_tokens = self.token_stream.tokens();
        let side_tokens = self.token_stream.side_tokens();
        loop {
            while let Some(token) = semantic_tokens.get(semantic_index) {
                if token.token.ty != TokenType::Whitespace {
                    break;
                }
                semantic_index += 1;
            }

            while let Some(token) = side_tokens.get(side_index) {
                if token.token.ty != TokenType::Whitespace {
                    break;
                }
                side_index += 1;
            }

            let semantic_token = semantic_tokens.get(semantic_index).copied();
            let side_token = side_tokens.get(side_index).copied();
            match (semantic_token, side_token) {
                (Some(semantic_token), Some(side_token)) => {
                    if semantic_token.span.start <= side_token.span.start {
                        push_token(semantic_token);
                        semantic_index += 1;
                    } else {
                        push_token(side_token);
                        side_index += 1;
                    }
                }
                (Some(semantic_token), None) => {
                    push_token(semantic_token);
                    semantic_index += 1;
                }
                (None, Some(side_token)) => {
                    push_token(side_token);
                    side_index += 1;
                }
                (None, None) => break,
            }
        }

        (tokens, line_indices)
    }

    /// Attach all comment, doc, and blank trivia groups.
    fn attach_side_trivia(
        &mut self,
        tokens: &[TokenSpan],
        line_indices: &[u32],
        ignore_span: &MultiSpan,
        statement_wrappers: &[Option<u32>],
    ) {
        if tokens.is_empty() {
            return;
        }

        // group contiguous trivia with one-newline bridging for comment/doc clusters
        let mut current_token_type = tokens[0].token.ty;
        let mut group_start_index = 0usize;
        let mut group_end_index = 0usize;
        let mut group_len = 1usize;

        for (index, token) in tokens.iter().enumerate().skip(1) {
            if token.token.ty != current_token_type {
                let start_token = tokens[group_start_index];
                let previous_token = if index > 1 {
                    Some((index - 2, tokens[index - 2]))
                } else {
                    None
                };

                // preserve line postfix ownership for single token groups
                let is_line_postfix = group_len == 1
                    && previous_token.is_some_and(|(previous_index, previous_token)| {
                        previous_token.token.ty != TokenType::Newline
                            && line_indices[previous_index] == line_indices[group_start_index]
                    });

                // allow one newline between grouped same-kind comment/doc tokens
                if token.token.ty == TokenType::Newline
                    && current_token_type != TokenType::Newline
                    && tokens
                        .get(index + 1)
                        .is_some_and(|next_token| next_token.token.ty == current_token_type)
                    && !is_line_postfix
                {
                    continue;
                }

                // flush the previous group
                let newline_blank_lines = if current_token_type == TokenType::Newline {
                    self.blank_lines_for_newline_group(
                        group_start_index,
                        group_end_index,
                        group_len,
                        tokens,
                        line_indices,
                    )
                } else {
                    None
                };
                if TRIVIA_TOKEN_TYPES.contains(&current_token_type)
                    && (current_token_type != TokenType::Newline || newline_blank_lines.is_some())
                    && !ignore_span.contains(&start_token.span)
                    && !ignore_span.contains(&tokens[group_end_index].span)
                {
                    self.attach_side_trivia_group(
                        (index - group_len) as u32,
                        current_token_type,
                        tokens,
                        line_indices,
                        group_start_index,
                        group_end_index,
                        group_len,
                        newline_blank_lines,
                        statement_wrappers,
                        ignore_span,
                    );
                }

                current_token_type = token.token.ty;
                group_start_index = index;
                group_end_index = index;
                group_len = 0;
            }

            if token.token.ty == current_token_type {
                if group_len == 0 {
                    group_start_index = index;
                }
                group_end_index = index;
                group_len += 1;
            }
        }

        // flush trailing group
        let newline_blank_lines = if current_token_type == TokenType::Newline {
            self.blank_lines_for_newline_group(
                group_start_index,
                group_end_index,
                group_len,
                tokens,
                line_indices,
            )
        } else {
            None
        };
        if TRIVIA_TOKEN_TYPES.contains(&current_token_type)
            && (current_token_type != TokenType::Newline || newline_blank_lines.is_some())
        {
            self.attach_side_trivia_group(
                (tokens.len() - group_len) as u32,
                current_token_type,
                tokens,
                line_indices,
                group_start_index,
                group_end_index,
                group_len,
                newline_blank_lines,
                statement_wrappers,
                ignore_span,
            );
        }
    }

    /// Return blank-line count for a newline group when it should become a blank annotation.
    fn blank_lines_for_newline_group(
        &self,
        group_start_index: usize,
        group_end_index: usize,
        group_len: usize,
        tokens: &[TokenSpan],
        line_indices: &[u32],
    ) -> Option<u32> {
        if group_len > 1 {
            return Some(group_len as u32 - 1);
        }

        let Some(previous_index) = group_start_index.checked_sub(1) else {
            return None;
        };
        let next_index = group_end_index + 1;
        let Some(next_line_index) = line_indices.get(next_index).copied() else {
            return None;
        };
        let Some(previous_line_index) = line_indices.get(previous_index).copied() else {
            return None;
        };
        let Some(previous_token) = tokens.get(previous_index) else {
            return None;
        };
        let Some(next_token) = tokens.get(next_index) else {
            return None;
        };

        if !matches!(
            previous_token.token.ty,
            TokenType::LineComment | TokenType::DocLineComment
        ) {
            return None;
        }

        if next_token.span.start <= previous_token.span.end {
            return None;
        }

        let line_delta = next_line_index.saturating_sub(previous_line_index);
        let blank_lines = line_delta.saturating_sub(1);
        if blank_lines == 0 {
            None
        } else {
            Some(blank_lines)
        }
    }

    /// Attach one trivia group after finding a stable owner and position.
    #[allow(clippy::too_many_arguments)]
    fn attach_side_trivia_group(
        &mut self,
        token_index: u32,
        token_type: TokenType,
        tokens: &[TokenSpan],
        line_indices: &[u32],
        group_start_index: usize,
        group_end_index: usize,
        group_len: usize,
        newline_blank_lines: Option<u32>,
        statement_wrappers: &[Option<u32>],
        ignore_span: &MultiSpan,
    ) {
        debug_assert!(TRIVIA_TOKEN_TYPES.contains(&token_type));
        debug_assert!(group_len > 0);

        let start_token = tokens[group_start_index];
        let end_token = tokens[group_end_index];
        let span = Span::new(
            start_token.span.file,
            start_token.span.start,
            end_token.span.end,
        );

        // line comments have stronger postfix preference than block comments
        let is_line_comment = start_token.token.ty == TokenType::LineComment
            || start_token.token.ty == TokenType::DocLineComment;

        let Some((position, target_node_id)) = self.find_trivia_target(
            token_index,
            tokens,
            line_indices,
            group_start_index,
            group_end_index,
            group_len,
            is_line_comment,
            false,
            statement_wrappers,
            ignore_span,
        ) else {
            let node_type = match token_type {
                TokenType::Newline => NodeType::Blank,
                TokenType::LineComment | TokenType::BlockComment => NodeType::Comment,
                TokenType::DocLineComment | TokenType::DocBlockComment => NodeType::Doc,
                _ => unreachable!("unexpected trivia token type: {token_type:?}"),
            };
            let error = ParseError::unexpected_for(span, node_type);
            self.error(&error);
            return;
        };

        // materialize payload and annotation node
        let annotation_id = match token_type {
            TokenType::Newline => {
                let lines = newline_blank_lines.unwrap_or(group_len as u32 - 1);
                let blank_id = self.tree.insert(Blank { lines }, span);
                self.tree.insert(
                    Annotation::Blank {
                        node: blank_id,
                        position,
                    },
                    span,
                )
            }
            TokenType::LineComment => {
                let string = self.clean_trivia_string(
                    token_type,
                    tokens,
                    group_start_index,
                    group_end_index,
                    group_len,
                );
                let string = self.strings.intern(string);
                let comment_id = self.tree.insert(
                    Comment {
                        string,
                        style: CommentStyle::Slash,
                    },
                    span,
                );
                self.tree.insert(
                    Annotation::Comment {
                        node: comment_id,
                        position,
                    },
                    span,
                )
            }
            TokenType::BlockComment => {
                let string = self.clean_trivia_string(
                    token_type,
                    tokens,
                    group_start_index,
                    group_end_index,
                    group_len,
                );
                let string = self.strings.intern(string);
                let comment_id = self.tree.insert(
                    Comment {
                        string,
                        style: CommentStyle::Star,
                    },
                    span,
                );
                self.tree.insert(
                    Annotation::Comment {
                        node: comment_id,
                        position,
                    },
                    span,
                )
            }
            TokenType::DocLineComment => {
                let string = self.clean_trivia_string(
                    token_type,
                    tokens,
                    group_start_index,
                    group_end_index,
                    group_len,
                );
                let string = self.strings.intern(string);
                let doc_id = self.tree.insert(
                    Doc {
                        string,
                        style: DocStyle::Slash,
                    },
                    span,
                );
                self.tree.insert(
                    Annotation::Doc {
                        node: doc_id,
                        position,
                    },
                    span,
                )
            }
            TokenType::DocBlockComment => {
                let string = self.clean_trivia_string(
                    token_type,
                    tokens,
                    group_start_index,
                    group_end_index,
                    group_len,
                );
                let string = self.strings.intern(string);
                let doc_id = self.tree.insert(
                    Doc {
                        string,
                        style: DocStyle::Star,
                    },
                    span,
                );
                self.tree.insert(
                    Annotation::Doc {
                        node: doc_id,
                        position,
                    },
                    span,
                )
            }
            _ => unreachable!("unexpected trivia token type: {token_type:?}"),
        };

        self.tree.append_annotation(target_node_id, annotation_id);
    }

    /// Promote expression owners to statement wrappers when line trivia targets a statement expression.
    fn promote_statement_owner(
        &self,
        start_token: TokenSpan,
        target_node_id: u32,
        statement_wrappers: &[Option<u32>],
    ) -> u32 {
        if start_token.token.ty == TokenType::Newline
            && self.tree.get_node_type(target_node_id) == NodeType::Expression
            && let Some(statement_id) = statement_wrappers
                .get(target_node_id as usize)
                .and_then(|entry| *entry)
        {
            return statement_id;
        }

        target_node_id
    }

    /// Return true when a block comment contains no line terminators.
    fn is_single_line_block_comment(&self, start_token: TokenSpan) -> bool {
        let raw = self.get_span_str(start_token.span);
        !raw.as_bytes()
            .iter()
            .any(|byte| *byte == b'\n' || *byte == b'\r')
    }

    /// Return the previous targetable token within the optional enclosing span.
    fn previous_targetable_token(
        &self,
        token_index: u32,
        tokens: &[TokenSpan],
        ignore_span: &MultiSpan,
        enclosing_span: Option<Span>,
    ) -> Option<(usize, TokenSpan)> {
        if token_index == 0 {
            return None;
        }

        let mut previous_index = token_index as usize;
        while previous_index > 0 {
            previous_index -= 1;
            let previous_token = tokens.get(previous_index)?;

            if ignore_span.contains(&previous_token.span)
                || previous_token.token.ty == TokenType::Whitespace
            {
                continue;
            }

            if let Some(enclosing_span) = enclosing_span
                && !enclosing_span.intersects(previous_token.span)
            {
                return None;
            }

            return Some((previous_index, *previous_token));
        }

        None
    }

    /// Return the next targetable token within the optional enclosing span.
    fn next_targetable_token(
        &self,
        token_index: u32,
        group_len: usize,
        tokens: &[TokenSpan],
        ignore_span: &MultiSpan,
        enclosing_span: Option<Span>,
    ) -> Option<(usize, TokenSpan)> {
        let mut next_index = token_index as usize + group_len;
        loop {
            let next_token = tokens.get(next_index)?;

            if ignore_span.contains(&next_token.span)
                || next_token.token.ty == TokenType::Whitespace
            {
                next_index += 1;
                continue;
            }

            if let Some(enclosing_span) = enclosing_span
                && !enclosing_span.intersects(next_token.span)
            {
                return None;
            }

            return Some((next_index, *next_token));
        }
    }

    /// Return whether a token is one of the fixed separator kinds.
    fn separator_kind_for_token(&self, token: TokenSpan) -> Option<TriviaSeparatorKind> {
        match token.token.ty {
            TokenType::Comma => Some(TriviaSeparatorKind::Comma),
            TokenType::ElementwiseOr => Some(TriviaSeparatorKind::Pipe),
            TokenType::ElementwiseAnd => Some(TriviaSeparatorKind::Ampersand),
            TokenType::Colon => Some(TriviaSeparatorKind::Colon),
            TokenType::Maybe => Some(TriviaSeparatorKind::Maybe),
            TokenType::Identifier => {
                let text = self.get_span_str(token.span);
                if text == "as" {
                    Some(TriviaSeparatorKind::As)
                } else if text == "satisfies" {
                    Some(TriviaSeparatorKind::Satisfies)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Return whether a token is a closer delimiter.
    fn is_closer_token(&self, token: TokenSpan) -> bool {
        matches!(
            token.token.ty,
            TokenType::CloseParenthesis | TokenType::CloseBrace | TokenType::CloseBracket
        )
    }

    /// Return whether a token is an opener delimiter.
    fn is_opener_token(&self, token: TokenSpan) -> bool {
        matches!(
            token.token.ty,
            TokenType::OpenParenthesis | TokenType::OpenBrace | TokenType::OpenBracket
        )
    }

    /// Return true when a token text is the given identifier keyword.
    fn token_is_identifier_keyword(&self, token: TokenSpan, keyword: &str) -> bool {
        token.token.ty == TokenType::Identifier && self.get_span_str(token.span) == keyword
    }

    /// Return true when a token is trivia.
    fn is_trivia_token(&self, token: TokenSpan) -> bool {
        matches!(
            token.token.ty,
            TokenType::Whitespace
                | TokenType::Newline
                | TokenType::LineComment
                | TokenType::DocLineComment
                | TokenType::BlockComment
                | TokenType::DocBlockComment
        )
    }

    /// Return the previous non-trivia token within the optional enclosing span.
    fn previous_non_trivia_token(
        &self,
        token_index: u32,
        tokens: &[TokenSpan],
        ignore_span: &MultiSpan,
        enclosing_span: Option<Span>,
    ) -> Option<(usize, TokenSpan)> {
        if token_index == 0 {
            return None;
        }

        let mut previous_index = token_index as usize;
        while previous_index > 0 {
            previous_index -= 1;
            let previous_token = tokens.get(previous_index)?;

            if ignore_span.contains(&previous_token.span) || self.is_trivia_token(*previous_token) {
                continue;
            }

            if let Some(enclosing_span) = enclosing_span
                && !enclosing_span.intersects(previous_token.span)
            {
                return None;
            }

            return Some((previous_index, *previous_token));
        }

        None
    }

    /// Return the next non-trivia token within the optional enclosing span.
    fn next_non_trivia_token(
        &self,
        token_index: u32,
        group_len: usize,
        tokens: &[TokenSpan],
        ignore_span: &MultiSpan,
        enclosing_span: Option<Span>,
    ) -> Option<(usize, TokenSpan)> {
        let mut next_index = token_index as usize + group_len;
        loop {
            let next_token = tokens.get(next_index)?;

            if ignore_span.contains(&next_token.span) || self.is_trivia_token(*next_token) {
                next_index += 1;
                continue;
            }

            if let Some(enclosing_span) = enclosing_span
                && !enclosing_span.intersects(next_token.span)
            {
                return None;
            }

            return Some((next_index, *next_token));
        }
    }

    /// Find a right owner for prefix attachment at one token seam.
    fn find_prefix_owner_for_token(
        &self,
        token: TokenSpan,
        ignore_span: &MultiSpan,
    ) -> Option<u32> {
        self.find_node_starting_at(&token.span, NodeSearchMode::BiggestOutermost)
            .or_else(|| {
                self.find_node_enclosing_at(
                    &token.span,
                    NodeSearchMode::SmallestOutermost,
                    |candidate| {
                        !ANNOTATION_NODE_TYPES.contains(&self.tree.get_node_type(candidate.idx))
                            && !ignore_span.contains(&candidate.span)
                    },
                )
            })
            .or_else(|| {
                self.find_node_enclosing_at(
                    &token.span,
                    NodeSearchMode::BiggestOutermost,
                    |candidate| {
                        !ANNOTATION_NODE_TYPES.contains(&self.tree.get_node_type(candidate.idx))
                            && !ignore_span.contains(&candidate.span)
                    },
                )
            })
            .map(|span| span.idx)
    }

    /// Find a call or new argument wrapper owner for comma-right prefix seams.
    fn find_call_or_new_argument_owner_for_token(&self, token: TokenSpan) -> Option<u32> {
        let call_or_new_scope = self.find_node_enclosing_at(
            &token.span,
            NodeSearchMode::SmallestOutermost,
            |candidate| {
                if self.tree.get_node_type(candidate.idx) != NodeType::Expression {
                    return false;
                }

                let expression_id = LocalNodeId::<Expression>::new(candidate.idx);
                let argument_ids = match self.tree.get(expression_id) {
                    Expression::Call {
                        dynamic_arguments, ..
                    }
                    | Expression::New {
                        dynamic_arguments, ..
                    } => dynamic_arguments,
                    _ => return false,
                };

                argument_ids.iter().any(|argument_id| {
                    let argument_span = self.tree.get_span(*argument_id);
                    argument_span.start <= token.span.start && argument_span.end >= token.span.end
                })
            },
        )?;

        let expression_id = LocalNodeId::<Expression>::new(call_or_new_scope.idx);
        let argument_ids = match self.tree.get(expression_id) {
            Expression::Call {
                dynamic_arguments, ..
            }
            | Expression::New {
                dynamic_arguments, ..
            } => dynamic_arguments,
            _ => return None,
        };

        argument_ids
            .iter()
            .find(|argument_id| {
                let argument_span = self.tree.get_span(**argument_id);
                argument_span.start <= token.span.start && argument_span.end >= token.span.end
            })
            .map(|argument_id| argument_id.id)
    }

    /// Find a left owner for postfix attachment at one token seam.
    fn find_postfix_owner_for_token(&self, token: TokenSpan) -> Option<u32> {
        self.find_node_ending_at(&token.span, NodeSearchMode::BiggestOutermost)
            .map(|span| span.idx)
    }

    /// Find a structural member or property owner that ends at the given token.
    fn find_structural_owner_ending_at(
        &self,
        token: TokenSpan,
        ignore_span: &MultiSpan,
    ) -> Option<u32> {
        self.find_node_enclosing_at(
            &token.span,
            NodeSearchMode::SmallestOutermost,
            |candidate| {
                matches!(
                    self.tree.get_node_type(candidate.idx),
                    NodeType::Member | NodeType::Property
                ) && candidate.span.end == token.span.end
                    && !ignore_span.contains(&candidate.span)
            },
        )
        .map(|span| span.idx)
    }

    /// Find the first non-trivia token after a given index.
    fn next_non_trivia_token_after_index(
        &self,
        index: usize,
        tokens: &[TokenSpan],
        ignore_span: &MultiSpan,
        enclosing_span: Option<Span>,
    ) -> Option<(usize, TokenSpan)> {
        self.next_non_trivia_token(index as u32, 0, tokens, ignore_span, enclosing_span)
            .or_else(|| self.next_non_trivia_token(index as u32, 0, tokens, ignore_span, None))
    }

    /// Return true when an expression owner is part of a class heritage list.
    fn is_class_heritage_expression_owner(&self, owner_id: u32) -> bool {
        if self.tree.get_node_type(owner_id) != NodeType::Expression {
            return false;
        }

        let owner_expression_id = LocalNodeId::<Expression>::new(owner_id);
        let owner_span = self.tree.get_span(owner_expression_id);
        let Some(declaration_span) = self.find_node_enclosing_at(
            &owner_span,
            NodeSearchMode::BiggestOutermost,
            |candidate| self.tree.get_node_type(candidate.idx) == NodeType::Declaration,
        ) else {
            return false;
        };

        let declaration_id = LocalNodeId::<Declaration>::new(declaration_span.idx);
        let Declaration::Class { heritage, .. } = self.tree.get(declaration_id) else {
            return false;
        };

        heritage
            .extends_types
            .as_ref()
            .is_some_and(|types| types.contains(&owner_expression_id))
            || heritage
                .implements_types
                .as_ref()
                .is_some_and(|types| types.contains(&owner_expression_id))
    }

    /// Return true when an argument owner belongs to call or new dynamic arguments.
    fn is_call_or_new_argument_owner(&self, owner_id: u32) -> bool {
        if self.tree.get_node_type(owner_id) != NodeType::Argument {
            return false;
        }

        let owner_argument_id = LocalNodeId::<Argument>::new(owner_id);
        let owner_span = self.tree.get_span(owner_argument_id);
        let expression_scope = self.find_node_enclosing_at(
            &owner_span,
            NodeSearchMode::SmallestOutermost,
            |candidate| {
                if self.tree.get_node_type(candidate.idx) != NodeType::Expression {
                    return false;
                }
                let expression_id = LocalNodeId::<Expression>::new(candidate.idx);
                match self.tree.get(expression_id) {
                    Expression::Call {
                        dynamic_arguments, ..
                    }
                    | Expression::New {
                        dynamic_arguments, ..
                    } => dynamic_arguments.contains(&owner_argument_id),
                    _ => false,
                }
            },
        );

        expression_scope.is_some()
    }

    /// Find the enclosing lambda declaration owner for a trivia span.
    fn find_enclosing_lambda_declaration_owner(&self, span: Span) -> Option<u32> {
        self.find_node_enclosing_at(
            &span,
            NodeSearchMode::SmallestOutermost,
            |candidate| {
                if self.tree.get_node_type(candidate.idx) != NodeType::Declaration {
                    return false;
                }

                let declaration_id = LocalNodeId::<Declaration>::new(candidate.idx);
                matches!(
                    self.tree.get(declaration_id),
                    Declaration::Function { signature, .. } if signature.kind == destack_ast::FunctionKind::Lambda
                )
            },
        )
        .map(|span| span.idx)
    }

    /// Choose prefix position for trivia right of a separator.
    fn position_for_after_separator(
        &self,
        separator: TriviaSeparatorKind,
        is_one_line: bool,
        group_end_line_index: u32,
        next_token_line_index: u32,
        right_owner_id: u32,
    ) -> AnnotationPosition {
        if !is_one_line {
            return AnnotationPosition::BlockPrefix;
        }

        if matches!(
            separator,
            TriviaSeparatorKind::Colon | TriviaSeparatorKind::Maybe
        ) {
            return AnnotationPosition::LinePrefix;
        }

        if matches!(
            separator,
            TriviaSeparatorKind::As | TriviaSeparatorKind::Satisfies
        ) {
            return if group_end_line_index == next_token_line_index {
                AnnotationPosition::LinePrefix
            } else {
                AnnotationPosition::BlockPrefix
            };
        }

        if separator == TriviaSeparatorKind::Comma
            && self.is_call_or_new_argument_owner(right_owner_id)
        {
            return AnnotationPosition::LinePrefix;
        }

        if group_end_line_index == next_token_line_index {
            AnnotationPosition::LinePrefix
        } else {
            AnnotationPosition::BlockPrefix
        }
    }

    /// Resolve trivia owner and annotation position for one trivia group.
    #[allow(clippy::too_many_arguments)]
    fn find_trivia_target(
        &self,
        token_index: u32,
        tokens: &[TokenSpan],
        line_indices: &[u32],
        group_start_index: usize,
        group_end_index: usize,
        group_len: usize,
        _is_full_line_hint: bool,
        is_block_prefix_only: bool,
        statement_wrappers: &[Option<u32>],
        ignore_span: &MultiSpan,
    ) -> Option<(AnnotationPosition, u32)> {
        debug_assert!(group_len > 0);

        let start_token = tokens[group_start_index];
        let is_block_comment = matches!(
            start_token.token.ty,
            TokenType::BlockComment | TokenType::DocBlockComment
        );
        let is_block_comment_single_line =
            is_block_comment && self.is_single_line_block_comment(start_token);
        let is_one_line = if is_block_comment && !is_block_comment_single_line {
            false
        } else {
            line_indices[group_start_index] == line_indices[group_end_index]
        };

        let enclosing_scope = self.find_node_enclosing_at(
            &start_token.span,
            NodeSearchMode::SmallestInnermost,
            |candidate| {
                !ANNOTATION_NODE_TYPES.contains(&self.tree.get_node_type(candidate.idx))
                    && !ignore_span.contains(&candidate.span)
            },
        );
        let enclosing_span = enclosing_scope.map(|scope| scope.span);
        let previous_token =
            self.previous_targetable_token(token_index, tokens, ignore_span, enclosing_span);
        let next_token =
            self.next_targetable_token(token_index, group_len, tokens, ignore_span, enclosing_span);
        let previous_non_trivia_token = self
            .previous_non_trivia_token(token_index, tokens, ignore_span, enclosing_span)
            .or_else(|| self.previous_non_trivia_token(token_index, tokens, ignore_span, None));
        let next_non_trivia_token = self
            .next_non_trivia_token(token_index, group_len, tokens, ignore_span, enclosing_span)
            .or_else(|| {
                self.next_non_trivia_token(token_index, group_len, tokens, ignore_span, None)
            });

        // declaration head seams before body or generics belong to the declaration owner
        // skip lambda `=> { ... }` seams: those comments belong to the body side
        if let Some((_, next_token)) = next_non_trivia_token
            && ((next_token.token.ty == TokenType::OpenBrace && is_block_comment)
                || next_token.token.ty == TokenType::LessThan)
            && !previous_non_trivia_token
                .is_some_and(|(_, previous_token)| previous_token.token.ty == TokenType::ArrowWide)
            && let Some(declaration_scope) = self.find_node_enclosing_at(
                &start_token.span,
                NodeSearchMode::SmallestInnermost,
                |candidate| self.tree.get_node_type(candidate.idx) == NodeType::Declaration,
            )
        {
            return Some((AnnotationPosition::BlockInfix, declaration_scope.idx));
        }

        // export head seams keep a trailing boundary on the declaration owner
        if let (Some((_, previous_token)), Some((_, next_token))) =
            (previous_non_trivia_token, next_non_trivia_token)
            && self.token_is_identifier_keyword(previous_token, "export")
            && let Some(declaration_scope) = self.find_node_enclosing_at(
                &next_token.span,
                NodeSearchMode::SmallestOutermost,
                |candidate| self.tree.get_node_type(candidate.idx) == NodeType::Declaration,
            )
        {
            return Some((
                AnnotationPosition::LinePostfixBoundary,
                declaration_scope.idx,
            ));
        }

        // control keyword seams: comments before else, catch, and finally belong to the following owner
        if let Some((next_index, next_token)) = next_non_trivia_token
            && (self.token_is_identifier_keyword(next_token, "else")
                || self.token_is_identifier_keyword(next_token, "catch")
                || self.token_is_identifier_keyword(next_token, "finally"))
            && let Some((_, after_keyword_token)) = self.next_non_trivia_token_after_index(
                next_index + 1,
                tokens,
                ignore_span,
                enclosing_span,
            )
            && let Some(target_node_id) =
                self.find_prefix_owner_for_token(after_keyword_token, ignore_span)
        {
            return Some((AnnotationPosition::BlockPrefix, target_node_id));
        }

        // lambda arrow seams: comments between parameters and `=>` belong to the lambda owner
        if let Some((_, next_token)) = next_non_trivia_token
            && next_token.token.ty == TokenType::ArrowWide
            && start_token.token.ty != TokenType::Newline
            && let Some(target_node_id) =
                self.find_enclosing_lambda_declaration_owner(start_token.span)
        {
            return Some((AnnotationPosition::BlockInfix, target_node_id));
        }

        // parameter type seams: comments before `:` belong to the parameter owner
        if let Some((_, next_token)) = next_non_trivia_token
            && next_token.token.ty == TokenType::Colon
            && start_token.token.ty != TokenType::Newline
            && let Some(parameter_scope) = self.find_node_enclosing_at(
                &start_token.span,
                NodeSearchMode::SmallestOutermost,
                |candidate| self.tree.get_node_type(candidate.idx) == NodeType::Parameter,
            )
        {
            return Some((AnnotationPosition::BlockInfix, parameter_scope.idx));
        }

        // leading type arm separators: comments before leading `|` or `&` bind to the full type binary owner
        if let (Some((_, previous_token)), Some((next_index, next_token))) =
            (previous_non_trivia_token, next_non_trivia_token)
            && previous_token.token.ty == TokenType::Assign
            && matches!(
                self.separator_kind_for_token(next_token),
                Some(TriviaSeparatorKind::Pipe | TriviaSeparatorKind::Ampersand)
            )
            && let Some((_, after_separator_token)) = self.next_non_trivia_token_after_index(
                next_index + 1,
                tokens,
                ignore_span,
                enclosing_span,
            )
            && let Some(binary_owner) = self.find_node_enclosing_at(
                &after_separator_token.span,
                NodeSearchMode::BiggestOutermost,
                |candidate| {
                    if self.tree.get_node_type(candidate.idx) != NodeType::Expression {
                        return false;
                    }
                    let expression_id = LocalNodeId::<Expression>::new(candidate.idx);
                    matches!(
                        self.tree.get(expression_id),
                        Expression::Binary {
                            operator: destack_ast::BinaryOperator::ElementwiseOr
                                | destack_ast::BinaryOperator::ElementwiseAnd,
                            ..
                        }
                    )
                },
            )
        {
            return Some((AnnotationPosition::BlockPrefix, binary_owner.idx));
        }

        // fallback for type declarations with leading `|` or `&`: bind to declaration value binary
        if let (Some((_, previous_token)), Some((_, next_token))) =
            (previous_non_trivia_token, next_non_trivia_token)
            && previous_token.token.ty == TokenType::Assign
            && matches!(
                self.separator_kind_for_token(next_token),
                Some(TriviaSeparatorKind::Pipe | TriviaSeparatorKind::Ampersand)
            )
            && let Some(declaration_scope) = self.find_node_enclosing_at(
                &start_token.span,
                NodeSearchMode::SmallestOutermost,
                |candidate| self.tree.get_node_type(candidate.idx) == NodeType::Declaration,
            )
        {
            let declaration_id = LocalNodeId::<Declaration>::new(declaration_scope.idx);
            if let Declaration::Type { value, .. } = self.tree.get(declaration_id)
                && let Expression::Binary { operator, .. } = self.tree.get(*value)
                && matches!(
                    operator,
                    destack_ast::BinaryOperator::ElementwiseOr
                        | destack_ast::BinaryOperator::ElementwiseAnd
                )
            {
                return Some((AnnotationPosition::BlockPrefix, value.id));
            }
        }

        // as-const exception: comment between `as` and `const` belongs to the as-const node
        if let (Some((_, previous_token)), Some((_, next_token))) =
            (previous_non_trivia_token, next_non_trivia_token)
            && self.separator_kind_for_token(previous_token) == Some(TriviaSeparatorKind::As)
            && self.token_is_identifier_keyword(next_token, "const")
            && let Some(target) = self
                .find_node_enclosing_at(
                    &next_token.span,
                    NodeSearchMode::SmallestInnermost,
                    |candidate| {
                        if self.tree.get_node_type(candidate.idx) != NodeType::Expression {
                            return false;
                        }
                        let expression_id = LocalNodeId::<Expression>::new(candidate.idx);
                        matches!(
                            self.tree.get(expression_id),
                            Expression::TypeUnary {
                                operator: destack_ast::TypeUnaryOperator::AsConst,
                                ..
                            }
                        )
                    },
                )
                .or_else(|| {
                    self.find_node_enclosing_at(
                        &previous_token.span,
                        NodeSearchMode::SmallestInnermost,
                        |candidate| {
                            if self.tree.get_node_type(candidate.idx) != NodeType::Expression {
                                return false;
                            }
                            let expression_id = LocalNodeId::<Expression>::new(candidate.idx);
                            matches!(
                                self.tree.get(expression_id),
                                Expression::TypeUnary {
                                    operator: destack_ast::TypeUnaryOperator::AsConst,
                                    ..
                                }
                            )
                        },
                    )
                })
                .or_else(|| {
                    self.find_node_enclosing_at(
                        &start_token.span,
                        NodeSearchMode::SmallestInnermost,
                        |candidate| {
                            if self.tree.get_node_type(candidate.idx) != NodeType::Expression {
                                return false;
                            }
                            let expression_id = LocalNodeId::<Expression>::new(candidate.idx);
                            matches!(
                                self.tree.get(expression_id),
                                Expression::TypeUnary {
                                    operator: destack_ast::TypeUnaryOperator::AsConst,
                                    ..
                                }
                            )
                        },
                    )
                })
        {
            return Some((AnnotationPosition::BlockInfix, target.idx));
        }

        // separator-adjacent: before-separator binds left, after-separator binds right
        if start_token.token.ty != TokenType::Newline
            && let (Some((previous_index, previous_token)), Some((next_index, next_token))) =
                (previous_non_trivia_token, next_non_trivia_token)
        {
            if self.separator_kind_for_token(next_token).is_some()
                && let Some(target_node_id) = if self.is_closer_token(previous_token) {
                    self.find_node_ending_at(
                        &previous_token.span,
                        NodeSearchMode::SmallestOutermost,
                    )
                    .map(|span| span.idx)
                    .or_else(|| self.find_postfix_owner_for_token(previous_token))
                } else {
                    self.find_postfix_owner_for_token(previous_token)
                }
            {
                let separator_kind = self
                    .separator_kind_for_token(next_token)
                    .expect("separator kind must exist");
                if separator_kind == TriviaSeparatorKind::Colon
                    && self.tree.get_node_type(target_node_id) == NodeType::Parameter
                {
                    return Some((AnnotationPosition::BlockInfix, target_node_id));
                }
                let position = if separator_kind == TriviaSeparatorKind::Maybe
                    && line_indices[previous_index] == line_indices[group_start_index]
                    && line_indices[next_index] != line_indices[group_start_index]
                {
                    AnnotationPosition::LinePostfixBoundary
                } else if line_indices[previous_index] == line_indices[group_start_index]
                    && line_indices[next_index] != line_indices[group_end_index]
                {
                    AnnotationPosition::LinePostfixBoundary
                } else if line_indices[previous_index] == line_indices[group_start_index] {
                    AnnotationPosition::LinePostfix
                } else {
                    AnnotationPosition::BlockPostfix
                };
                return Some((position, target_node_id));
            }

            if let Some(separator_kind) = self.separator_kind_for_token(previous_token)
                && let Some(target_node_id) = (if separator_kind == TriviaSeparatorKind::Comma {
                    self.find_call_or_new_argument_owner_for_token(next_token)
                        .or_else(|| self.find_prefix_owner_for_token(next_token, ignore_span))
                } else {
                    self.find_prefix_owner_for_token(next_token, ignore_span)
                })
            {
                // comma comments default to trailing-left ownership except call args and class heritage
                if separator_kind == TriviaSeparatorKind::Comma
                    && !self.is_call_or_new_argument_owner(target_node_id)
                    && !self.is_class_heritage_expression_owner(target_node_id)
                {
                    // let line postfix/block postfix resolution decide ownership
                } else {
                    let position = self.position_for_after_separator(
                        separator_kind,
                        is_one_line,
                        line_indices[group_end_index],
                        line_indices[next_index],
                        target_node_id,
                    );
                    return Some((
                        position,
                        self.promote_statement_owner(
                            start_token,
                            target_node_id,
                            statement_wrappers,
                        ),
                    ));
                }
            }

            // closer-adjacent: before `)`, `]`, `}` binds left as boundary postfix
            if self.is_closer_token(next_token)
                && !self.is_opener_token(previous_token)
                && line_indices[previous_index] == line_indices[group_start_index]
                && line_indices[next_index] == line_indices[group_end_index]
                && let Some(target_node_id) = self
                    .find_postfix_owner_for_token(previous_token)
                    .or_else(|| {
                        if self.is_opener_token(previous_token) {
                            return None;
                        }

                        self.find_node_enclosing_at(
                            &previous_token.span,
                            NodeSearchMode::SmallestOutermost,
                            |candidate| {
                                !ANNOTATION_NODE_TYPES
                                    .contains(&self.tree.get_node_type(candidate.idx))
                                    && !ignore_span.contains(&candidate.span)
                            },
                        )
                        .map(|span| span.idx)
                    })
            {
                return Some((AnnotationPosition::LinePostfixBoundary, target_node_id));
            }

            // inline opener boundary: `callee(/* comment */)` in empty call/new argument lists
            if self.is_closer_token(next_token)
                && previous_token.token.ty == TokenType::OpenParenthesis
                && let Some(call_or_new_owner) = self.find_node_enclosing_at(
                    &start_token.span,
                    NodeSearchMode::SmallestOutermost,
                    |candidate| {
                        if self.tree.get_node_type(candidate.idx) != NodeType::Expression {
                            return false;
                        }

                        let expression_id = LocalNodeId::<Expression>::new(candidate.idx);
                        match self.tree.get(expression_id) {
                            Expression::Call {
                                dynamic_arguments, ..
                            }
                            | Expression::New {
                                dynamic_arguments, ..
                            } => dynamic_arguments.is_empty(),
                            _ => false,
                        }
                    },
                )
            {
                return Some((AnnotationPosition::BlockInfix, call_or_new_owner.idx));
            }

            // inline opener boundary fallback: keep comment on the left owner
            if self.is_closer_token(next_token)
                && previous_token.token.ty == TokenType::OpenParenthesis
                && let Some((_, before_opener_token)) = self.previous_non_trivia_token(
                    previous_index as u32,
                    tokens,
                    ignore_span,
                    enclosing_span,
                )
                && let Some(target_node_id) = self.find_postfix_owner_for_token(before_opener_token)
            {
                return Some((AnnotationPosition::BlockPostfix, target_node_id));
            }
        }

        // line prefix and postfix resolution
        if !is_block_prefix_only && is_one_line && start_token.token.ty != TokenType::Newline {
            let has_left_same_line =
                previous_token.is_some_and(|(previous_index, previous_token)| {
                    previous_token.token.ty != TokenType::Newline
                        && line_indices[previous_index] == line_indices[group_start_index]
                });
            let has_right_same_line = next_token.is_some_and(|(next_index, next_token)| {
                next_token.token.ty != TokenType::Newline
                    && line_indices[next_index] == line_indices[group_end_index]
            });
            let is_full_line_trivia = !has_left_same_line && !has_right_same_line;

            // keep same-line comment seams before dotted continuation on the parent path
            if let (
                Some((previous_index, _)),
                Some((next_index, next_token)),
                Some(enclosing_scope),
            ) = (previous_token, next_token, enclosing_scope)
                && next_token.token.ty == TokenType::Dot
                && line_indices[group_end_index] == line_indices[next_index]
                && line_indices[previous_index] == line_indices[group_end_index]
                && self.tree.get_node_type(enclosing_scope.idx) == NodeType::Expression
            {
                let enclosing_expression_id = LocalNodeId::<Expression>::new(enclosing_scope.idx);
                if matches!(
                    self.tree.get(enclosing_expression_id),
                    Expression::Path { .. }
                ) {
                    return Some((AnnotationPosition::BlockInfix, enclosing_scope.idx));
                }
            }

            let line_postfix_target = if let Some((previous_index, previous_token)) = previous_token
                && line_indices[previous_index] == line_indices[group_end_index]
                && previous_token.token.ty != TokenType::Newline
                && !self.is_opener_token(previous_token)
            {
                let next_non_trivia_is_closer =
                    next_non_trivia_token.is_some_and(|(_, next_non_trivia_token)| {
                        self.is_closer_token(next_non_trivia_token)
                    });
                let is_end_of_line = next_token.is_none()
                    || next_token.is_some_and(|(_, token)| {
                        token.token.ty == TokenType::Newline || token.token.ty == TokenType::End
                    });
                let is_line_comment_token = matches!(
                    start_token.token.ty,
                    TokenType::LineComment | TokenType::DocLineComment
                );
                let search_mode = if matches!(
                    previous_token.token.ty,
                    TokenType::Comma | TokenType::Semicolon
                ) {
                    NodeSearchMode::BiggestOutermost
                } else if next_non_trivia_is_closer {
                    if is_line_comment_token {
                        NodeSearchMode::SmallestOutermost
                    } else if is_end_of_line {
                        NodeSearchMode::BiggestOutermost
                    } else {
                        NodeSearchMode::SmallestOutermost
                    }
                } else if is_line_comment_token
                    && is_end_of_line
                    && next_non_trivia_token
                        .is_some_and(|(_, token)| token.token.ty == TokenType::Semicolon)
                {
                    NodeSearchMode::SmallestOutermost
                } else if (is_line_comment_token && is_end_of_line) || is_full_line_trivia {
                    NodeSearchMode::BiggestOutermost
                } else {
                    NodeSearchMode::SmallestOutermost
                };
                self.find_node_ending_at(&previous_token.span, search_mode)
                    .or_else(|| {
                        // keep trailing separator comments on the preceding element
                        if is_end_of_line
                            && matches!(
                                previous_token.token.ty,
                                TokenType::Comma | TokenType::Semicolon
                            )
                            && token_index > 1
                        {
                            let mut before_separator_index = token_index as usize - 1;
                            while before_separator_index > 0 {
                                before_separator_index -= 1;
                                let before_separator_token = tokens.get(before_separator_index)?;
                                if ignore_span.contains(&before_separator_token.span)
                                    || before_separator_token.token.ty == TokenType::Whitespace
                                {
                                    continue;
                                }

                                return self.find_node_ending_at(
                                    &before_separator_token.span,
                                    search_mode,
                                );
                            }
                        }

                        None
                    })
                    .map(|span| {
                        (
                            span.idx,
                            is_line_comment_token,
                            is_end_of_line,
                            previous_token,
                        )
                    })
            } else {
                None
            };

            if let Some((target_node_id, is_line_comment_token, is_end_of_line, previous_token)) =
                line_postfix_target
            {
                let target_node_id = if is_line_comment_token
                    && is_end_of_line
                    && next_non_trivia_token
                        .is_some_and(|(_, token)| token.token.ty == TokenType::CloseBrace)
                {
                    self.find_structural_owner_ending_at(previous_token, ignore_span)
                        .unwrap_or(target_node_id)
                } else {
                    target_node_id
                };

                let has_next_same_line = next_token.is_some_and(|(next_index, next_token)| {
                    next_token.token.ty != TokenType::Newline
                        && line_indices[next_index] == line_indices[group_end_index]
                });
                let is_boundary = !has_next_same_line
                    || next_token.is_none()
                    || next_token.is_some_and(|(_, next_token)| {
                        self.is_closer_token(next_token)
                            || matches!(next_token.token.ty, TokenType::Semicolon | TokenType::End)
                    });

                if is_boundary {
                    return Some((AnnotationPosition::LinePostfixBoundary, target_node_id));
                }

                return Some((AnnotationPosition::LinePostfix, target_node_id));
            }

            // inline block comment before a node on the same line is a prefix seam
            if is_block_comment_single_line
                && let Some((next_index, next_token)) = next_token
                && next_token.token.ty != TokenType::Newline
                && !matches!(
                    next_token.token.ty,
                    TokenType::CloseParenthesis
                        | TokenType::CloseBrace
                        | TokenType::CloseBracket
                        | TokenType::End
                )
                && (line_indices[group_end_index] == line_indices[next_index]
                    || line_indices[group_start_index] == line_indices[next_index])
            {
                let start_search_mode = if has_left_same_line {
                    NodeSearchMode::SmallestOutermost
                } else {
                    NodeSearchMode::BiggestOutermost
                };
                let target_node_id = self
                    .find_node_starting_at(&next_token.span, start_search_mode)
                    .or_else(|| {
                        self.find_node_enclosing_at(
                            &next_token.span,
                            start_search_mode,
                            |candidate| {
                                !ANNOTATION_NODE_TYPES
                                    .contains(&self.tree.get_node_type(candidate.idx))
                                    && !ignore_span.contains(&candidate.span)
                            },
                        )
                    })
                    .or_else(|| {
                        self.find_node_enclosing_at(
                            &next_token.span,
                            NodeSearchMode::BiggestOutermost,
                            |candidate| {
                                !ANNOTATION_NODE_TYPES
                                    .contains(&self.tree.get_node_type(candidate.idx))
                                    && !ignore_span.contains(&candidate.span)
                            },
                        )
                    })
                    .map(|span| span.idx);

                if let Some(target_node_id) = target_node_id {
                    return Some((
                        AnnotationPosition::LinePrefix,
                        self.promote_statement_owner(
                            start_token,
                            target_node_id,
                            statement_wrappers,
                        ),
                    ));
                }
            }

            // inline comment between path segments belongs to the path seam owner
            if let Some((_, previous_token)) = previous_token
                && let Some(enclosing_scope) = enclosing_scope
                && self.tree.get_node_type(enclosing_scope.idx) == NodeType::Expression
            {
                let expression_id = LocalNodeId::<Expression>::new(enclosing_scope.idx);
                if let Expression::Path { path, .. } = self.tree.get(expression_id)
                    && path.segments.len() > 1
                    && enclosing_scope.span.end > previous_token.span.end
                {
                    return Some((
                        AnnotationPosition::LinePostfixBoundary,
                        self.promote_statement_owner(
                            start_token,
                            expression_id.id,
                            statement_wrappers,
                        ),
                    ));
                }
            }

            // line prefix fallback: target the following node on the same line seam
            if let Some((next_index, next_token)) = next_token
                && next_token.token.ty != TokenType::Newline
                && !matches!(
                    next_token.token.ty,
                    TokenType::CloseParenthesis
                        | TokenType::CloseBrace
                        | TokenType::CloseBracket
                        | TokenType::End
                )
                && (line_indices[group_end_index] == line_indices[next_index]
                    || line_indices[group_start_index] == line_indices[next_index])
            {
                let target_node_id = self
                    .find_node_starting_at(&next_token.span, NodeSearchMode::SmallestOutermost)
                    .or_else(|| {
                        self.find_node_enclosing_at(
                            &next_token.span,
                            NodeSearchMode::SmallestOutermost,
                            |candidate| {
                                !ANNOTATION_NODE_TYPES
                                    .contains(&self.tree.get_node_type(candidate.idx))
                                    && !ignore_span.contains(&candidate.span)
                            },
                        )
                    })
                    .map(|span| span.idx);

                if let Some(target_node_id) = target_node_id {
                    return Some((
                        AnnotationPosition::LinePrefix,
                        self.promote_statement_owner(
                            start_token,
                            target_node_id,
                            statement_wrappers,
                        ),
                    ));
                }
            }
        }

        // block prefix: first valid target after the trivia group
        let mut next_index = token_index as usize + group_len;
        while let Some(next_token) = tokens.get(next_index) {
            if ignore_span.contains(&next_token.span)
                || TRIVIA_TOKEN_TYPES.contains(&next_token.token.ty)
                || next_token.token.ty == TokenType::Whitespace
            {
                next_index += 1;
                continue;
            }

            // stop before delimiter boundaries: these resolve via postfix or infix fallbacks
            if next_token.token.ty == TokenType::OpenBrace
                || next_token.token.ty == TokenType::Semicolon
                || self.is_closer_token(*next_token)
            {
                break;
            }

            if let Some(enclosing_span) = enclosing_span
                && !enclosing_span.intersects(next_token.span)
                && start_token.token.ty != TokenType::Newline
            {
                break;
            }

            let next_node = if is_block_prefix_only {
                self.find_node_starting_at(&next_token.span, NodeSearchMode::BiggestOutermost)
                    .or_else(|| {
                        self.find_node_enclosing_at(
                            &next_token.span,
                            NodeSearchMode::SmallestOutermost,
                            |candidate| {
                                !ANNOTATION_NODE_TYPES
                                    .contains(&self.tree.get_node_type(candidate.idx))
                                    && !ignore_span.contains(&candidate.span)
                            },
                        )
                    })
            } else {
                self.find_node_starting_at(&next_token.span, NodeSearchMode::BiggestOutermost)
            };

            if let Some(next_node) = next_node {
                return Some((
                    AnnotationPosition::BlockPrefix,
                    self.promote_statement_owner(start_token, next_node.idx, statement_wrappers),
                ));
            }

            if next_token.token.ty == TokenType::Dot {
                let target_node_id = self
                    .find_node_enclosing_at(
                        &next_token.span,
                        NodeSearchMode::SmallestOutermost,
                        |candidate| {
                            !ANNOTATION_NODE_TYPES.contains(&self.tree.get_node_type(candidate.idx))
                                && !ignore_span.contains(&candidate.span)
                        },
                    )
                    .map(|span| span.idx);
                if let Some(target_node_id) = target_node_id {
                    return Some((
                        AnnotationPosition::BlockPrefix,
                        self.promote_statement_owner(
                            start_token,
                            target_node_id,
                            statement_wrappers,
                        ),
                    ));
                }
            }

            next_index += 1;
        }

        // block postfix: first valid target before the trivia group
        if !is_block_prefix_only && token_index > 0 {
            let mut previous_index = token_index as usize;
            while previous_index > 0 {
                previous_index -= 1;
                let Some(previous_token) = tokens.get(previous_index) else {
                    break;
                };

                if ignore_span.contains(&previous_token.span)
                    || TRIVIA_TOKEN_TYPES.contains(&previous_token.token.ty)
                    || previous_token.token.ty == TokenType::Whitespace
                {
                    continue;
                }

                if let Some(enclosing_span) = enclosing_span
                    && !enclosing_span.intersects(previous_token.span)
                {
                    break;
                }

                if let Some(previous_node) = (!self.is_opener_token(*previous_token))
                    .then(|| {
                        self.find_node_ending_at(
                            &previous_token.span,
                            NodeSearchMode::BiggestOutermost,
                        )
                    })
                    .flatten()
                {
                    return Some((
                        AnnotationPosition::BlockPostfix,
                        self.promote_statement_owner(
                            start_token,
                            previous_node.idx,
                            statement_wrappers,
                        ),
                    ));
                }
            }
        }

        // empty brace seam in jsx-like containers belongs to the stub expression owner
        if let (Some((_, previous_non_trivia_token)), Some((_, next_non_trivia_token))) =
            (previous_non_trivia_token, next_non_trivia_token)
            && previous_non_trivia_token.token.ty == TokenType::OpenBrace
            && next_non_trivia_token.token.ty == TokenType::CloseBrace
            && let Some(argument_scope) = self.find_node_enclosing_at(
                &start_token.span,
                NodeSearchMode::SmallestOutermost,
                |candidate| self.tree.get_node_type(candidate.idx) == NodeType::Argument,
            )
        {
            let argument_id = LocalNodeId::<Argument>::new(argument_scope.idx);
            let argument_value = match self.tree.get(argument_id) {
                Argument::Named { value, .. }
                | Argument::Labeled { value, .. }
                | Argument::Positional { value, .. }
                | Argument::Spread { value, .. } => *value,
            };
            if matches!(self.tree.get(argument_value), Expression::Stub) {
                return Some((
                    AnnotationPosition::BlockInfix,
                    self.promote_statement_owner(
                        start_token,
                        argument_value.id,
                        statement_wrappers,
                    ),
                ));
            }
        }

        // block infix: last fallback is the smallest enclosing non-annotation node
        if !is_block_prefix_only && let Some(enclosing_node) = enclosing_scope {
            return Some((
                AnnotationPosition::BlockInfix,
                self.promote_statement_owner(start_token, enclosing_node.idx, statement_wrappers),
            ));
        }

        None
    }

    /// Normalize raw comment and doc payload text.
    fn clean_trivia_string(
        &self,
        token_type: TokenType,
        tokens: &[TokenSpan],
        group_start_index: usize,
        group_end_index: usize,
        group_len: usize,
    ) -> String {
        debug_assert!(group_len > 0);

        let mut cleaned_tokens = Vec::with_capacity(group_len);
        for token in tokens[group_start_index..=group_end_index].iter() {
            if token.token.ty != token_type {
                continue;
            }

            let raw_string = self.file.get_span_str(token.span).unwrap_or_default();
            let mut inner_string = match token_type {
                TokenType::LineComment => raw_string.strip_prefix("//").unwrap_or(raw_string),
                TokenType::DocLineComment => raw_string.strip_prefix("///").unwrap_or(raw_string),
                TokenType::BlockComment => raw_string
                    .strip_prefix("/*")
                    .unwrap_or(raw_string)
                    .strip_suffix("*/")
                    .unwrap_or(raw_string),
                TokenType::DocBlockComment => raw_string
                    .strip_prefix("/**")
                    .unwrap_or(raw_string)
                    .strip_suffix("*/")
                    .unwrap_or(raw_string),
                _ => unreachable!("unexpected trivia token type: {token_type:?}"),
            };

            if matches!(
                token_type,
                TokenType::BlockComment | TokenType::DocBlockComment
            ) {
                if inner_string.starts_with(' ') {
                    inner_string = &inner_string[1..];
                }
                while inner_string.ends_with(' ') {
                    inner_string = inner_string
                        .strip_suffix(' ')
                        .expect("suffix strip should succeed");
                }
            }

            let cleaned = match token_type {
                TokenType::LineComment | TokenType::DocLineComment => {
                    if inner_string.contains('\n') {
                        let mut cleaned = String::with_capacity(inner_string.len());
                        for (index, line) in inner_string.lines().enumerate() {
                            if index > 0 {
                                cleaned.push('\n');
                            }
                            let line = line.strip_prefix(' ').unwrap_or(line);
                            cleaned.push_str(line);
                        }
                        cleaned
                    } else {
                        inner_string
                            .strip_prefix(' ')
                            .unwrap_or(inner_string)
                            .to_owned()
                    }
                }
                TokenType::BlockComment | TokenType::DocBlockComment => {
                    if inner_string.trim().is_empty() {
                        String::new()
                    } else {
                        let mut cleaned = String::with_capacity(inner_string.len());
                        for (index, line) in inner_string.split('\n').enumerate() {
                            if index > 0 {
                                cleaned.push('\n');
                            }

                            let mut line = if let Some((position, ch)) =
                                line.char_indices().find(|(_, ch)| *ch != ' ')
                                && ch == '*'
                            {
                                let mut line_string = &line[position + ch.len_utf8()..];
                                if line_string.starts_with(' ') {
                                    line_string = &line_string[1..];
                                }
                                line_string
                            } else {
                                line.strip_prefix(' ').unwrap_or(line)
                            };

                            if line.chars().all(|ch| ch == ' ') {
                                line = "";
                            } else {
                                line = line.trim_end_matches(' ');
                            }

                            cleaned.push_str(line);
                        }
                        cleaned
                    }
                }
                _ => unreachable!("unexpected trivia token type: {token_type:?}"),
            };

            cleaned_tokens.push(cleaned);
        }

        cleaned_tokens.join("\n")
    }
}
