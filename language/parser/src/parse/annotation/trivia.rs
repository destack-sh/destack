use crate::{ParseError, Parser};
use destack_ast::{
    ANNOTATION_NODE_TYPES, Annotation, AnnotationPosition, Argument, Blank, Comment, CommentStyle,
    Declaration, Doc, DocStyle, Expression, IfCondition, IfKind, LocalNodeId, NodeType, TokenSpan,
    TokenType,
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

#[allow(clippy::too_many_arguments)]
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
        self.previous_token_matching(
            token_index as usize,
            tokens,
            ignore_span,
            enclosing_span,
            |token| token.token.ty != TokenType::Whitespace,
        )
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
        self.next_token_matching(
            token_index as usize + group_len,
            tokens,
            ignore_span,
            enclosing_span,
            |token| token.token.ty != TokenType::Whitespace,
        )
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

    /// Return whether a line comment token is a leading formatter directive marker.
    fn is_leading_formatter_directive_comment(&self, token: TokenSpan) -> bool {
        if !matches!(
            token.token.ty,
            TokenType::LineComment | TokenType::DocLineComment
        ) {
            return false;
        }

        let raw = self.get_span_str(token.span);
        let marker = match token.token.ty {
            TokenType::LineComment => raw.strip_prefix("//").unwrap_or(raw),
            TokenType::DocLineComment => raw.strip_prefix("///").unwrap_or(raw),
            _ => raw,
        }
        .trim();
        matches!(
            marker,
            "format-ignore-start"
                | "format-ignore-end"
                | "prettier-ignore-start"
                | "prettier-ignore-end"
                | "fmt-ignore-start"
                | "fmt-ignore-end"
                | "biome-ignore-start"
                | "biome-ignore-end"
        )
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

    /// Scan backward for the nearest token that matches the given predicate.
    fn previous_token_matching<F>(
        &self,
        start_index: usize,
        tokens: &[TokenSpan],
        ignore_span: &MultiSpan,
        enclosing_span: Option<Span>,
        mut predicate: F,
    ) -> Option<(usize, TokenSpan)>
    where
        F: FnMut(TokenSpan) -> bool,
    {
        // stream head has no previous token
        if start_index == 0 {
            return None;
        }

        // scan backward until we find a token that passes span and predicate filters
        let mut index = start_index;
        while index > 0 {
            index -= 1;
            let token = *tokens.get(index)?;

            // skip ignored spans
            if ignore_span.contains(&token.span) {
                continue;
            }

            // skip tokens rejected by the caller predicate
            if !predicate(token) {
                continue;
            }

            // stop when we crossed out of the local enclosing scope
            if let Some(enclosing_span) = enclosing_span
                && !enclosing_span.intersects(token.span)
            {
                return None;
            }

            // found a matching token in range
            return Some((index, token));
        }

        // no matching token was found
        None
    }

    /// Scan forward for the nearest token that matches the given predicate.
    fn next_token_matching<F>(
        &self,
        start_index: usize,
        tokens: &[TokenSpan],
        ignore_span: &MultiSpan,
        enclosing_span: Option<Span>,
        mut predicate: F,
    ) -> Option<(usize, TokenSpan)>
    where
        F: FnMut(TokenSpan) -> bool,
    {
        // scan forward until we find a token that passes span and predicate filters
        let mut index = start_index;
        loop {
            let token = *tokens.get(index)?;

            // skip ignored spans
            if ignore_span.contains(&token.span) {
                index += 1;
                continue;
            }

            // skip tokens rejected by the caller predicate
            if !predicate(token) {
                index += 1;
                continue;
            }

            // stop when we crossed out of the local enclosing scope
            if let Some(enclosing_span) = enclosing_span
                && !enclosing_span.intersects(token.span)
            {
                return None;
            }

            // found a matching token in range
            return Some((index, token));
        }
    }

    /// Return the previous non-trivia token within the optional enclosing span.
    fn previous_non_trivia_token(
        &self,
        token_index: u32,
        tokens: &[TokenSpan],
        ignore_span: &MultiSpan,
        enclosing_span: Option<Span>,
    ) -> Option<(usize, TokenSpan)> {
        self.previous_token_matching(
            token_index as usize,
            tokens,
            ignore_span,
            enclosing_span,
            |token| !self.is_trivia_token(token),
        )
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
        self.next_token_matching(
            token_index as usize + group_len,
            tokens,
            ignore_span,
            enclosing_span,
            |token| !self.is_trivia_token(token),
        )
    }

    /// Find a right owner for prefix attachment at one token seam.
    fn find_prefix_owner_for_token(
        &self,
        token: TokenSpan,
        ignore_span: &MultiSpan,
    ) -> Option<u32> {
        // prefer exact start owners at the seam token
        if let Some(owner) =
            self.find_node_starting_at(&token.span, NodeSearchMode::BiggestOutermost)
        {
            return Some(owner.idx);
        }

        // otherwise use the smallest enclosing semantic owner
        if let Some(owner) = self.find_node_enclosing_at(
            &token.span,
            NodeSearchMode::SmallestOutermost,
            |candidate| {
                !ANNOTATION_NODE_TYPES.contains(&self.tree.get_node_type(candidate.idx))
                    && !ignore_span.contains(&candidate.span)
            },
        ) {
            return Some(owner.idx);
        }

        // final fallback: largest enclosing semantic owner
        self.find_node_enclosing_at(&token.span, NodeSearchMode::BiggestOutermost, |candidate| {
            !ANNOTATION_NODE_TYPES.contains(&self.tree.get_node_type(candidate.idx))
                && !ignore_span.contains(&candidate.span)
        })
        .map(|owner| owner.idx)
    }

    /// Find a call or new argument wrapper owner for comma-right prefix seams.
    fn find_call_or_new_argument_owner_for_token(&self, token: TokenSpan) -> Option<u32> {
        // find the narrowest call or new expression whose dynamic argument span covers this token
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

        // return the concrete argument owner that contains this token
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

    /// Find the nearest cast, satisfies, or as-const expression owner for `as` and `satisfies` seams.
    fn find_cast_or_satisfies_owner_for_separator(
        &self,
        start_span: Span,
        previous_token: TokenSpan,
        next_token: TokenSpan,
        separator_kind: TriviaSeparatorKind,
    ) -> Option<u32> {
        let matches_owner = |candidate_id: u32| {
            if self.tree.get_node_type(candidate_id) != NodeType::Expression {
                return false;
            }

            let expression_id = LocalNodeId::<Expression>::new(candidate_id);
            match self.tree.get(expression_id) {
                Expression::TypeBinary { operator, .. } => {
                    matches!(
                        (separator_kind, operator),
                        (
                            TriviaSeparatorKind::As,
                            destack_ast::TypeBinaryOperator::Cast
                        ) | (
                            TriviaSeparatorKind::Satisfies,
                            destack_ast::TypeBinaryOperator::Satisfies
                        )
                    )
                }
                Expression::TypeUnary { operator, .. } => {
                    separator_kind == TriviaSeparatorKind::As
                        && matches!(
                            operator,
                            destack_ast::TypeUnaryOperator::AsConst
                                | destack_ast::TypeUnaryOperator::AsComptime
                        )
                }
                _ => false,
            }
        };

        for span in [next_token.span, previous_token.span, start_span] {
            let Some(scope) = self.find_node_enclosing_at(
                &span,
                NodeSearchMode::SmallestInnermost,
                |candidate| matches_owner(candidate.idx),
            ) else {
                continue;
            };
            return Some(scope.idx);
        }

        None
    }

    /// Find the nearest enclosing label expression owner.
    fn find_enclosing_label_owner(&self, span: Span) -> Option<u32> {
        self.find_node_enclosing_at(&span, NodeSearchMode::SmallestOutermost, |candidate| {
            if self.tree.get_node_type(candidate.idx) != NodeType::Expression {
                return false;
            }

            let expression_id = LocalNodeId::<Expression>::new(candidate.idx);
            matches!(self.tree.get(expression_id), Expression::Labelled { .. })
        })
        .map(|scope| scope.idx)
    }

    /// Find mapped-type value owner for mapped `:` seams.
    fn find_mapped_type_value_owner(&self, span: Span) -> Option<u32> {
        let mapped_scope =
            self.find_node_enclosing_at(&span, NodeSearchMode::SmallestOutermost, |candidate| {
                if self.tree.get_node_type(candidate.idx) != NodeType::Expression {
                    return false;
                }

                let expression_id = LocalNodeId::<Expression>::new(candidate.idx);
                matches!(self.tree.get(expression_id), Expression::TypeMapped { .. })
            })?;

        let mapped_expression_id = LocalNodeId::<Expression>::new(mapped_scope.idx);
        let Expression::TypeMapped { value, .. } = self.tree.get(mapped_expression_id) else {
            return None;
        };

        Some(value.id)
    }

    /// Find ternary left owner for line comments after `?` or `:`.
    fn find_ternary_left_owner_for_separator(
        &self,
        separator_token: TokenSpan,
        next_token: TokenSpan,
        separator_kind: TriviaSeparatorKind,
    ) -> Option<u32> {
        let ternary_scope = self.find_node_enclosing_at(
            &separator_token.span,
            NodeSearchMode::SmallestOutermost,
            |candidate| {
                if self.tree.get_node_type(candidate.idx) != NodeType::Expression {
                    return false;
                }

                let expression_id = LocalNodeId::<Expression>::new(candidate.idx);
                matches!(
                    self.tree.get(expression_id),
                    Expression::If {
                        kind: IfKind::Ternary,
                        else_expression: Some(_),
                        ..
                    }
                )
            },
        )?;

        let ternary_expression_id = LocalNodeId::<Expression>::new(ternary_scope.idx);
        let Expression::If {
            condition,
            then_expression,
            else_expression: Some(else_expression),
            ..
        } = self.tree.get(ternary_expression_id)
        else {
            return None;
        };

        match separator_kind {
            TriviaSeparatorKind::Maybe => {
                let then_span = self.tree.get_span(*then_expression);
                if !(then_span.start <= next_token.span.start
                    && then_span.end >= next_token.span.end)
                {
                    return None;
                }

                match condition {
                    IfCondition::Expression { condition } => Some(condition.id),
                    IfCondition::Let { declarator, .. } => Some(declarator.id),
                }
            }
            TriviaSeparatorKind::Colon => {
                let else_span = self.tree.get_span(*else_expression);
                if !(else_span.start <= next_token.span.start
                    && else_span.end >= next_token.span.end)
                {
                    return None;
                }

                Some(then_expression.id)
            }
            _ => None,
        }
    }

    /// Return true when a trivia token is a line comment or doc line comment.
    fn is_line_comment_token(&self, token: TokenSpan) -> bool {
        matches!(
            token.token.ty,
            TokenType::LineComment | TokenType::DocLineComment
        )
    }

    /// Find the left owner immediately before a separator token.
    fn find_left_owner_before_separator(
        &self,
        separator_index: usize,
        tokens: &[TokenSpan],
        ignore_span: &MultiSpan,
        enclosing_span: Option<Span>,
    ) -> Option<(usize, u32)> {
        // locate the nearest non trivia token before the separator
        let (before_separator_index, before_separator_token) = self
            .previous_non_trivia_token(separator_index as u32, tokens, ignore_span, enclosing_span)
            .or_else(|| {
                self.previous_non_trivia_token(separator_index as u32, tokens, ignore_span, None)
            })
            .map(|(index, token)| (index as usize, token))?;

        // resolve postfix owner from that left seam token
        let left_owner_id = self.find_postfix_owner_for_token(before_separator_token)?;

        Some((before_separator_index, left_owner_id))
    }

    /// Return true when separator comments should stay on the left owner.
    fn separator_prefers_left_owner(&self, separator_kind: TriviaSeparatorKind) -> bool {
        matches!(
            separator_kind,
            TriviaSeparatorKind::Pipe | TriviaSeparatorKind::Ampersand
        )
    }

    /// Return true when comma seams should defer to generic line seam resolution.
    fn comma_defers_to_line_seam_resolution(&self, right_owner_id: u32) -> bool {
        !self.is_call_or_new_argument_owner(right_owner_id)
            && !self.is_class_heritage_expression_owner(right_owner_id)
    }

    /// Choose postfix position for comments bound to the owner before a separator.
    fn position_for_left_separator_owner(
        &self,
        separator_kind: TriviaSeparatorKind,
        before_separator_index: usize,
        group_start_index: usize,
        group_end_index: usize,
        next_index: usize,
        line_indices: &[u32],
    ) -> AnnotationPosition {
        let is_same_line_as_before_separator =
            line_indices[before_separator_index] == line_indices[group_start_index];
        if !is_same_line_as_before_separator {
            return AnnotationPosition::BlockPostfix;
        }

        if separator_kind == TriviaSeparatorKind::Comma {
            return AnnotationPosition::LinePostfixBoundary;
        }

        if line_indices[next_index] != line_indices[group_end_index] {
            AnnotationPosition::LinePostfixBoundary
        } else {
            AnnotationPosition::LinePostfix
        }
    }

    /// Choose postfix position for comments bound before a separator token.
    fn position_for_before_separator_owner(
        &self,
        separator_kind: TriviaSeparatorKind,
        previous_index: usize,
        next_index: usize,
        group_start_index: usize,
        group_end_index: usize,
        line_indices: &[u32],
    ) -> AnnotationPosition {
        if separator_kind == TriviaSeparatorKind::Maybe
            && line_indices[previous_index] == line_indices[group_start_index]
            && line_indices[next_index] != line_indices[group_start_index]
        {
            return AnnotationPosition::LinePostfixBoundary;
        }

        if line_indices[previous_index] == line_indices[group_start_index]
            && line_indices[next_index] != line_indices[group_end_index]
        {
            return AnnotationPosition::LinePostfixBoundary;
        }

        if line_indices[previous_index] == line_indices[group_start_index] {
            AnnotationPosition::LinePostfix
        } else {
            AnnotationPosition::BlockPostfix
        }
    }

    /// Resolve ownership for trivia immediately before a separator token.
    fn resolve_before_separator_target(
        &self,
        start_token: TokenSpan,
        previous_index: usize,
        previous_token: TokenSpan,
        next_index: usize,
        next_token: TokenSpan,
        line_indices: &[u32],
        group_start_index: usize,
        group_end_index: usize,
        statement_wrappers: &[Option<u32>],
        ignore_span: &MultiSpan,
    ) -> Option<(AnnotationPosition, u32)> {
        let separator_kind = self.separator_kind_for_token(next_token)?;
        let mut target_node_id = if self.is_closer_token(previous_token) {
            self.find_node_ending_at(&previous_token.span, NodeSearchMode::SmallestOutermost)
                .map(|span| span.idx)
                .or_else(|| self.find_postfix_owner_for_token(previous_token))
        } else {
            self.find_postfix_owner_for_token(previous_token)
        }?;

        if separator_kind == TriviaSeparatorKind::Maybe
            && self.is_line_comment_token(start_token)
            && line_indices[previous_index] == line_indices[group_start_index]
            && line_indices[next_index] != line_indices[group_start_index]
            && let Some(optional_chain_owner) = self.find_node_enclosing_at(
                &next_token.span,
                NodeSearchMode::BiggestOutermost,
                |candidate| {
                    self.tree.get_node_type(candidate.idx) == NodeType::Expression
                        && !ignore_span.contains(&candidate.span)
                },
            )
        {
            target_node_id = self.promote_statement_owner(
                start_token,
                optional_chain_owner.idx,
                statement_wrappers,
            );
        }

        if separator_kind == TriviaSeparatorKind::Colon
            && self.tree.get_node_type(target_node_id) == NodeType::Parameter
        {
            return Some((AnnotationPosition::BlockInfix, target_node_id));
        }

        let position = self.position_for_before_separator_owner(
            separator_kind,
            previous_index,
            next_index,
            group_start_index,
            group_end_index,
            line_indices,
        );
        Some((position, target_node_id))
    }

    /// Resolve ownership for trivia immediately after a separator token.
    #[allow(clippy::too_many_arguments)]
    fn resolve_after_separator_target(
        &self,
        start_token: TokenSpan,
        previous_index: usize,
        previous_token: TokenSpan,
        next_index: usize,
        next_token: TokenSpan,
        tokens: &[TokenSpan],
        line_indices: &[u32],
        group_start_index: usize,
        group_end_index: usize,
        is_one_line: bool,
        statement_wrappers: &[Option<u32>],
        ignore_span: &MultiSpan,
        enclosing_span: Option<Span>,
    ) -> Option<(AnnotationPosition, u32)> {
        // classify separator seam shape once for all downstream ownership rules
        let separator_kind = self.separator_kind_for_token(previous_token)?;
        let is_line_comment = self.is_line_comment_token(start_token);
        let is_split_line_seam = line_indices[previous_index] == line_indices[group_start_index]
            && line_indices[next_index] != line_indices[group_start_index];

        // label seams after `:` stay before the labelled statement
        if separator_kind == TriviaSeparatorKind::Colon
            && is_line_comment
            && let Some(label_owner_id) = self.find_enclosing_label_owner(start_token.span)
        {
            return Some((
                AnnotationPosition::BlockPrefix,
                self.promote_statement_owner(start_token, label_owner_id, statement_wrappers),
            ));
        }

        // ternary line-comment seams after `?` or `:` stay on the left branch boundary
        if matches!(
            separator_kind,
            TriviaSeparatorKind::Maybe | TriviaSeparatorKind::Colon
        ) && is_line_comment
            && is_split_line_seam
            && let Some(ternary_left_owner_id) = self.find_ternary_left_owner_for_separator(
                previous_token,
                next_token,
                separator_kind,
            )
        {
            return Some((
                AnnotationPosition::LinePostfixBoundary,
                self.promote_statement_owner(
                    start_token,
                    ternary_left_owner_id,
                    statement_wrappers,
                ),
            ));
        }

        // line seams after `as` and `satisfies` stay with the full operator expression
        if matches!(
            separator_kind,
            TriviaSeparatorKind::As | TriviaSeparatorKind::Satisfies
        ) && is_line_comment
            && is_split_line_seam
            && let Some(operator_owner_id) = self.find_cast_or_satisfies_owner_for_separator(
                start_token.span,
                previous_token,
                next_token,
                separator_kind,
            )
        {
            return Some((
                AnnotationPosition::LinePostfixBoundary,
                self.promote_statement_owner(start_token, operator_owner_id, statement_wrappers),
            ));
        }

        // mapped value `:` seams stay trailing on the mapped value expression
        if separator_kind == TriviaSeparatorKind::Colon
            && is_line_comment
            && is_split_line_seam
            && let Some(mapped_value_id) = self.find_mapped_type_value_owner(start_token.span)
        {
            return Some((
                AnnotationPosition::LinePostfixBoundary,
                self.promote_statement_owner(start_token, mapped_value_id, statement_wrappers),
            ));
        }

        // comma before a closer keeps trailing ownership on the left element
        if separator_kind == TriviaSeparatorKind::Comma
            && self.is_closer_token(next_token)
            && let Some((before_separator_index, left_owner_id)) = self
                .find_left_owner_before_separator(
                    previous_index,
                    tokens,
                    ignore_span,
                    enclosing_span,
                )
        {
            let position = self.position_for_left_separator_owner(
                separator_kind,
                before_separator_index,
                group_start_index,
                group_end_index,
                next_index,
                line_indices,
            );
            return Some((position, left_owner_id));
        }

        // comma line comments default to left ownership unless they are formatter directives
        if separator_kind == TriviaSeparatorKind::Comma
            && is_line_comment
            && !self.is_leading_formatter_directive_comment(start_token)
            && let Some((before_separator_index, left_owner_id)) = self
                .find_left_owner_before_separator(
                    previous_index,
                    tokens,
                    ignore_span,
                    enclosing_span,
                )
        {
            let position = self.position_for_left_separator_owner(
                separator_kind,
                before_separator_index,
                group_start_index,
                group_end_index,
                next_index,
                line_indices,
            );
            return Some((position, left_owner_id));
        }

        // resolve the right seam owner for non-left-preferring separator cases
        let right_owner_id = if separator_kind == TriviaSeparatorKind::Comma {
            self.find_call_or_new_argument_owner_for_token(next_token)
                .or_else(|| self.find_prefix_owner_for_token(next_token, ignore_span))
        } else {
            self.find_prefix_owner_for_token(next_token, ignore_span)
        }?;

        // comma seams outside call args and class heritage must defer to line seam resolution:
        // this intentionally falls through to resolve_line_prefix_postfix_target, which keeps
        // `a, /* b */ c` style seams on `c` while preserving trailing-left line-comment behavior
        if separator_kind == TriviaSeparatorKind::Comma
            && self.comma_defers_to_line_seam_resolution(right_owner_id)
        {
            return None;
        }

        // separators that are intrinsically left-preferring bind to the left owner
        if self.separator_prefers_left_owner(separator_kind)
            && let Some((before_separator_index, left_owner_id)) = self
                .find_left_owner_before_separator(
                    previous_index,
                    tokens,
                    ignore_span,
                    enclosing_span,
                )
        {
            let position = self.position_for_left_separator_owner(
                separator_kind,
                before_separator_index,
                group_start_index,
                group_end_index,
                next_index,
                line_indices,
            );
            return Some((position, left_owner_id));
        }

        // otherwise bind prefix style to the right owner and choose line or block position
        let position = self.position_for_after_separator(
            separator_kind,
            is_one_line,
            line_indices[group_end_index],
            line_indices[next_index],
            right_owner_id,
        );
        Some((
            position,
            self.promote_statement_owner(start_token, right_owner_id, statement_wrappers),
        ))
    }

    /// Resolve owner for leading type-arm separators after `=` in type declarations.
    fn resolve_leading_type_separator_target(
        &self,
        start_span: Span,
        separator_token: TokenSpan,
        next_index: usize,
        tokens: &[TokenSpan],
        ignore_span: &MultiSpan,
        enclosing_span: Option<Span>,
    ) -> Option<u32> {
        if !matches!(
            self.separator_kind_for_token(separator_token),
            Some(TriviaSeparatorKind::Pipe | TriviaSeparatorKind::Ampersand)
        ) {
            return None;
        }

        if let Some((_, after_separator_token)) = self.next_non_trivia_token_after_index(
            next_index + 1,
            tokens,
            ignore_span,
            enclosing_span,
        ) && let Some(binary_owner) = self.find_node_enclosing_at(
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
        ) {
            return Some(binary_owner.idx);
        }

        let declaration_scope = self.find_node_enclosing_at(
            &start_span,
            NodeSearchMode::SmallestOutermost,
            |candidate| self.tree.get_node_type(candidate.idx) == NodeType::Declaration,
        )?;
        let declaration_id = LocalNodeId::<Declaration>::new(declaration_scope.idx);
        if let Declaration::Type { value, .. } = self.tree.get(declaration_id)
            && let Expression::Binary { operator, .. } = self.tree.get(*value)
            && matches!(
                operator,
                destack_ast::BinaryOperator::ElementwiseOr
                    | destack_ast::BinaryOperator::ElementwiseAnd
            )
        {
            return Some(value.id);
        }

        None
    }

    /// Resolve seams before `else`, `catch`, and `finally` keywords.
    fn resolve_control_keyword_seam_target(
        &self,
        next_non_trivia_token: Option<(usize, TokenSpan)>,
        tokens: &[TokenSpan],
        ignore_span: &MultiSpan,
        enclosing_span: Option<Span>,
    ) -> Option<(AnnotationPosition, u32)> {
        // match only the control continuation keywords that own following branches
        let (next_index, next_token) = next_non_trivia_token?;
        if !(self.token_is_identifier_keyword(next_token, "else")
            || self.token_is_identifier_keyword(next_token, "catch")
            || self.token_is_identifier_keyword(next_token, "finally"))
        {
            return None;
        }

        // bind to the immediate statement or block after the keyword
        let (_, after_keyword_token) = self.next_non_trivia_token_after_index(
            next_index + 1,
            tokens,
            ignore_span,
            enclosing_span,
        )?;
        let target_node_id = self.find_prefix_owner_for_token(after_keyword_token, ignore_span)?;

        Some((AnnotationPosition::BlockPrefix, target_node_id))
    }

    /// Resolve line-comment seams after `if (...)` to then-branch ownership.
    fn resolve_if_head_seam_target(
        &self,
        start_token: TokenSpan,
        previous_non_trivia_token: Option<(usize, TokenSpan)>,
        next_non_trivia_token: Option<(usize, TokenSpan)>,
    ) -> Option<(AnnotationPosition, u32)> {
        // this seam rule only applies to line comments
        if !self.is_line_comment_token(start_token) {
            return None;
        }

        // require a seam directly after `)` and before a token inside the then branch
        let (Some((_, previous_token)), Some((_, next_token))) =
            (previous_non_trivia_token, next_non_trivia_token)
        else {
            return None;
        };
        if previous_token.token.ty != TokenType::CloseParenthesis {
            return None;
        }

        let if_scope = self.find_node_enclosing_at(
            &start_token.span,
            NodeSearchMode::SmallestOutermost,
            |candidate| {
                if self.tree.get_node_type(candidate.idx) != NodeType::Expression {
                    return false;
                }

                let expression_id = LocalNodeId::<Expression>::new(candidate.idx);
                matches!(self.tree.get(expression_id), Expression::If { .. })
            },
        )?;
        let if_expression_id = LocalNodeId::<Expression>::new(if_scope.idx);
        let Expression::If {
            then_expression, ..
        } = self.tree.get(if_expression_id)
        else {
            return None;
        };

        let then_span = self.tree.get_span(*then_expression);
        let next_span = next_token.span;
        if !(then_span.start <= next_span.start && then_span.end >= next_span.end) {
            return None;
        }

        // target the then branch owner, using the first statement for non-empty block branches
        let then_owner_id = if let Expression::Block(block_id) = self.tree.get(*then_expression) {
            let block = self.tree.get(*block_id);
            if block.expressions.is_empty() {
                then_expression.id
            } else {
                block.expressions[0].id
            }
        } else {
            then_expression.id
        };

        Some((AnnotationPosition::BlockPrefix, then_owner_id))
    }

    /// Resolve seams between lambda parameters and the wide arrow.
    fn resolve_lambda_arrow_seam_target(
        &self,
        start_token: TokenSpan,
        next_non_trivia_token: Option<(usize, TokenSpan)>,
    ) -> Option<(AnnotationPosition, u32)> {
        let (_, next_token) = next_non_trivia_token?;
        if next_token.token.ty != TokenType::ArrowWide || start_token.token.ty == TokenType::Newline
        {
            return None;
        }

        let target_node_id = self.find_enclosing_lambda_declaration_owner(start_token.span)?;
        Some((AnnotationPosition::BlockInfix, target_node_id))
    }

    /// Resolve callee-to-paren seams for call and new expressions.
    fn resolve_callee_paren_seam_target(
        &self,
        start_token: TokenSpan,
        previous_non_trivia_token: Option<(usize, TokenSpan)>,
        next_non_trivia_token: Option<(usize, TokenSpan)>,
        tokens: &[TokenSpan],
        line_indices: &[u32],
        group_end_index: usize,
        ignore_span: &MultiSpan,
        enclosing_span: Option<Span>,
    ) -> Option<(AnnotationPosition, u32)> {
        // require a seam between a callee and its opening parenthesis
        let (Some((_, previous_token)), Some((next_index, next_token))) =
            (previous_non_trivia_token, next_non_trivia_token)
        else {
            return None;
        };
        if next_token.token.ty != TokenType::OpenParenthesis || self.is_opener_token(previous_token)
        {
            return None;
        }

        // resolve the enclosing call or new expression and its argument list
        let call_or_new_scope = self.find_node_enclosing_at(
            &start_token.span,
            NodeSearchMode::SmallestOutermost,
            |candidate| {
                if self.tree.get_node_type(candidate.idx) != NodeType::Expression {
                    return false;
                }

                let expression_id = LocalNodeId::<Expression>::new(candidate.idx);
                matches!(
                    self.tree.get(expression_id),
                    Expression::Call { .. } | Expression::New { .. }
                )
            },
        )?;
        let expression_id = LocalNodeId::<Expression>::new(call_or_new_scope.idx);
        let (left_id, dynamic_arguments): (LocalNodeId<Expression>, &[LocalNodeId<Argument>]) =
            match self.tree.get(expression_id) {
                Expression::Call {
                    left,
                    dynamic_arguments,
                    ..
                }
                | Expression::New {
                    left,
                    dynamic_arguments,
                    ..
                } => (*left, dynamic_arguments.as_slice()),
                _ => unreachable!("callee-to-paren seams only target call/new expressions"),
            };

        let left_span = self.tree.get_span(left_id);
        if previous_token.span.end != left_span.end {
            return None;
        }

        // empty argument lists keep this seam as callee infix ownership
        if dynamic_arguments.is_empty() {
            return Some((AnnotationPosition::BlockInfix, call_or_new_scope.idx));
        }

        // if the list closes immediately, keep infix ownership on the call or new expression
        let next_after_open = self
            .next_non_trivia_token_after_index(next_index + 1, tokens, ignore_span, enclosing_span)
            .or_else(|| {
                self.next_non_trivia_token_after_index(next_index + 1, tokens, ignore_span, None)
            });
        if let Some((_after_open_index, after_open_token)) = next_after_open
            && self.is_closer_token(after_open_token)
        {
            return Some((AnnotationPosition::BlockInfix, call_or_new_scope.idx));
        }

        // otherwise bind prefix ownership to the first dynamic argument
        let first_argument_id = dynamic_arguments[0];
        let position = if let Some((after_open_index, _)) = next_after_open {
            if line_indices[group_end_index] == line_indices[after_open_index] {
                AnnotationPosition::LinePrefix
            } else {
                AnnotationPosition::BlockPrefix
            }
        } else {
            AnnotationPosition::LinePrefix
        };

        Some((position, first_argument_id.id))
    }

    /// Resolve parameter-type seams before `:`.
    fn resolve_parameter_type_seam_target(
        &self,
        start_token: TokenSpan,
        next_non_trivia_token: Option<(usize, TokenSpan)>,
    ) -> Option<(AnnotationPosition, u32)> {
        let (_, next_token) = next_non_trivia_token?;
        if next_token.token.ty != TokenType::Colon || start_token.token.ty == TokenType::Newline {
            return None;
        }

        let parameter_scope = self.find_node_enclosing_at(
            &start_token.span,
            NodeSearchMode::SmallestOutermost,
            |candidate| self.tree.get_node_type(candidate.idx) == NodeType::Parameter,
        )?;
        Some((AnnotationPosition::BlockInfix, parameter_scope.idx))
    }

    /// Resolve `as const` token-gap seams.
    fn resolve_as_const_token_gap_target(
        &self,
        start_token: TokenSpan,
        previous_non_trivia_token: Option<(usize, TokenSpan)>,
        next_non_trivia_token: Option<(usize, TokenSpan)>,
        is_block_comment: bool,
        is_block_comment_single_line: bool,
        tokens: &[TokenSpan],
        line_indices: &[u32],
        group_start_index: usize,
        statement_wrappers: &[Option<u32>],
        ignore_span: &MultiSpan,
        enclosing_span: Option<Span>,
    ) -> Option<(AnnotationPosition, u32)> {
        // require the exact `as` <trivia> `const` token gap seam
        let (Some((previous_separator_index, previous_token)), Some((_, next_token))) =
            (previous_non_trivia_token, next_non_trivia_token)
        else {
            return None;
        };
        if self.separator_kind_for_token(previous_token) != Some(TriviaSeparatorKind::As)
            || !self.token_is_identifier_keyword(next_token, "const")
        {
            return None;
        }

        // resolve the as-const expression owner once for both line and block paths
        let as_const_owner = self.find_cast_or_satisfies_owner_for_separator(
            start_token.span,
            previous_token,
            next_token,
            TriviaSeparatorKind::As,
        );

        // line and multiline block comments trail the as-const expression
        let is_line_comment = self.is_line_comment_token(start_token);
        if (is_line_comment || (is_block_comment && !is_block_comment_single_line))
            && let Some(as_const_owner_id) = as_const_owner
        {
            return Some((
                AnnotationPosition::LinePostfixBoundary,
                self.promote_statement_owner(start_token, as_const_owner_id, statement_wrappers),
            ));
        }

        // single line block comments stay on the left operand side
        if is_block_comment_single_line
            && let Some((before_separator_index, left_owner_id)) = self
                .find_left_owner_before_separator(
                    previous_separator_index,
                    tokens,
                    ignore_span,
                    enclosing_span,
                )
        {
            let position =
                if line_indices[before_separator_index] == line_indices[group_start_index] {
                    AnnotationPosition::LinePostfix
                } else {
                    AnnotationPosition::BlockPostfix
                };
            return Some((position, left_owner_id));
        }

        None
    }

    /// Return true when next token can receive inline prefix ownership.
    fn is_inline_prefix_candidate_token(&self, token: TokenSpan) -> bool {
        token.token.ty != TokenType::Newline
            && !matches!(
                token.token.ty,
                TokenType::CloseParenthesis
                    | TokenType::CloseBrace
                    | TokenType::CloseBracket
                    | TokenType::End
            )
    }

    /// Return true when a token is on the same seam line as the trivia group.
    fn is_same_line_prefix_seam(
        &self,
        next_index: usize,
        line_indices: &[u32],
        group_start_index: usize,
        group_end_index: usize,
    ) -> bool {
        line_indices[group_end_index] == line_indices[next_index]
            || line_indices[group_start_index] == line_indices[next_index]
    }

    /// Choose search mode for same-line postfix seams.
    fn line_postfix_search_mode(
        &self,
        previous_token: TokenSpan,
        start_token: TokenSpan,
        next_non_trivia_token: Option<(usize, TokenSpan)>,
        is_end_of_line: bool,
        is_full_line_trivia: bool,
    ) -> NodeSearchMode {
        let next_non_trivia_is_closer = next_non_trivia_token
            .is_some_and(|(_, next_non_trivia_token)| self.is_closer_token(next_non_trivia_token));
        let is_line_comment_token = self.is_line_comment_token(start_token);

        if matches!(
            previous_token.token.ty,
            TokenType::Comma | TokenType::Semicolon
        ) {
            return NodeSearchMode::BiggestOutermost;
        }

        if next_non_trivia_is_closer {
            if is_line_comment_token {
                return NodeSearchMode::SmallestOutermost;
            }

            if is_end_of_line {
                return NodeSearchMode::BiggestOutermost;
            }

            return NodeSearchMode::SmallestOutermost;
        }

        if is_line_comment_token
            && is_end_of_line
            && next_non_trivia_token
                .is_some_and(|(_, token)| token.token.ty == TokenType::Semicolon)
        {
            return NodeSearchMode::SmallestOutermost;
        }

        if (is_line_comment_token && is_end_of_line) || is_full_line_trivia {
            NodeSearchMode::BiggestOutermost
        } else {
            NodeSearchMode::SmallestOutermost
        }
    }

    /// Find a preceding owner when postfix seam follows a trailing separator.
    fn find_separator_preceding_owner_for_line_postfix(
        &self,
        previous_index: usize,
        previous_token: TokenSpan,
        tokens: &[TokenSpan],
        ignore_span: &MultiSpan,
        search_mode: NodeSearchMode,
    ) -> Option<u32> {
        if !matches!(
            previous_token.token.ty,
            TokenType::Comma | TokenType::Semicolon
        ) {
            return None;
        }

        let (_, before_separator_token) = self
            .previous_non_trivia_token(previous_index as u32, tokens, ignore_span, None)
            .map(|(index, token)| (index as usize, token))?;

        self.find_node_ending_at(&before_separator_token.span, search_mode)
            .map(|span| span.idx)
    }

    /// Resolve same-line postfix ownership for line and block comment seams.
    fn resolve_line_postfix_target(
        &self,
        start_token: TokenSpan,
        previous_token: Option<(usize, TokenSpan)>,
        next_token: Option<(usize, TokenSpan)>,
        next_non_trivia_token: Option<(usize, TokenSpan)>,
        tokens: &[TokenSpan],
        line_indices: &[u32],
        group_end_index: usize,
        is_full_line_trivia: bool,
        ignore_span: &MultiSpan,
    ) -> Option<(AnnotationPosition, u32)> {
        // require a same-line left seam token for postfix ownership
        let (previous_index, previous_token) = previous_token?;
        if line_indices[previous_index] != line_indices[group_end_index]
            || previous_token.token.ty == TokenType::Newline
            || self.is_opener_token(previous_token)
        {
            return None;
        }

        // classify end-of-line status and select owner search mode
        let is_end_of_line = next_token.is_none()
            || next_token.is_some_and(|(_, token)| {
                token.token.ty == TokenType::Newline || token.token.ty == TokenType::End
            });
        let search_mode = self.line_postfix_search_mode(
            previous_token,
            start_token,
            next_non_trivia_token,
            is_end_of_line,
            is_full_line_trivia,
        );

        // find the left owner at the seam token, with separator fallback at line end
        let target_node_id = self
            .find_node_ending_at(&previous_token.span, search_mode)
            .map(|span| span.idx)
            .or_else(|| {
                if !is_end_of_line {
                    return None;
                }

                self.find_separator_preceding_owner_for_line_postfix(
                    previous_index,
                    previous_token,
                    tokens,
                    ignore_span,
                    search_mode,
                )
            })?;

        // prefer structural owners at line boundaries for stable list and member seams
        let target_node_id = if is_end_of_line {
            self.find_structural_owner_ending_at(previous_token, ignore_span)
                .unwrap_or(target_node_id)
        } else {
            target_node_id
        };

        // choose boundary vs interior postfix position from right seam shape
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
        let position = if is_boundary {
            AnnotationPosition::LinePostfixBoundary
        } else {
            AnnotationPosition::LinePostfix
        };

        // return the resolved same-line postfix seam
        Some((position, target_node_id))
    }

    /// Find prefix owner for an inline same-line seam.
    fn find_inline_prefix_owner(
        &self,
        next_token: TokenSpan,
        start_search_mode: NodeSearchMode,
        ignore_span: &MultiSpan,
        include_outermost_fallback: bool,
    ) -> Option<u32> {
        let target_node_id = self
            .find_node_starting_at(&next_token.span, start_search_mode)
            .or_else(|| {
                self.find_node_enclosing_at(&next_token.span, start_search_mode, |candidate| {
                    !ANNOTATION_NODE_TYPES.contains(&self.tree.get_node_type(candidate.idx))
                        && !ignore_span.contains(&candidate.span)
                })
            })
            .or_else(|| {
                if !include_outermost_fallback {
                    return None;
                }

                self.find_node_enclosing_at(
                    &next_token.span,
                    NodeSearchMode::BiggestOutermost,
                    |candidate| {
                        !ANNOTATION_NODE_TYPES.contains(&self.tree.get_node_type(candidate.idx))
                            && !ignore_span.contains(&candidate.span)
                    },
                )
            })
            .map(|span| span.idx);

        target_node_id
    }

    /// Resolve one-line line-prefix and line-postfix ownership.
    fn resolve_line_prefix_postfix_target(
        &self,
        start_token: TokenSpan,
        previous_token: Option<(usize, TokenSpan)>,
        next_token: Option<(usize, TokenSpan)>,
        next_non_trivia_token: Option<(usize, TokenSpan)>,
        enclosing_scope_id: Option<u32>,
        enclosing_scope_span: Option<Span>,
        tokens: &[TokenSpan],
        line_indices: &[u32],
        group_start_index: usize,
        group_end_index: usize,
        is_block_comment_single_line: bool,
        statement_wrappers: &[Option<u32>],
        ignore_span: &MultiSpan,
    ) -> Option<(AnnotationPosition, u32)> {
        // classify whether the trivia has left or right same-line seam neighbors
        let has_left_same_line = previous_token.is_some_and(|(previous_index, previous_token)| {
            previous_token.token.ty != TokenType::Newline
                && line_indices[previous_index] == line_indices[group_start_index]
        });
        let has_right_same_line = next_token.is_some_and(|(next_index, next_token)| {
            next_token.token.ty != TokenType::Newline
                && line_indices[next_index] == line_indices[group_end_index]
        });
        let is_full_line_trivia = !has_left_same_line && !has_right_same_line;

        // keep same-line comment seams before dotted continuation on the parent path
        if let (Some((previous_index, _)), Some((next_index, next_token)), Some(enclosing_scope_id)) =
            (previous_token, next_token, enclosing_scope_id)
            && next_token.token.ty == TokenType::Dot
            && line_indices[group_end_index] == line_indices[next_index]
            && line_indices[previous_index] == line_indices[group_end_index]
            && self.tree.get_node_type(enclosing_scope_id) == NodeType::Expression
        {
            let enclosing_expression_id = LocalNodeId::<Expression>::new(enclosing_scope_id);
            if matches!(
                self.tree.get(enclosing_expression_id),
                Expression::Path { .. }
            ) {
                return Some((AnnotationPosition::BlockInfix, enclosing_scope_id));
            }
        }

        // try canonical same-line postfix resolution first
        if let Some((position, target_node_id)) = self.resolve_line_postfix_target(
            start_token,
            previous_token,
            next_token,
            next_non_trivia_token,
            tokens,
            line_indices,
            group_end_index,
            is_full_line_trivia,
            ignore_span,
        ) {
            return Some((position, target_node_id));
        }

        // inline block comment before a node on the same line is a prefix seam
        if is_block_comment_single_line
            && let Some((next_index, next_token)) = next_token
            && self.is_inline_prefix_candidate_token(next_token)
            && self.is_same_line_prefix_seam(
                next_index,
                line_indices,
                group_start_index,
                group_end_index,
            )
        {
            let start_search_mode = if has_left_same_line {
                NodeSearchMode::SmallestOutermost
            } else {
                NodeSearchMode::BiggestOutermost
            };
            if let Some(target_node_id) =
                self.find_inline_prefix_owner(next_token, start_search_mode, ignore_span, true)
            {
                return Some((
                    AnnotationPosition::LinePrefix,
                    self.promote_statement_owner(start_token, target_node_id, statement_wrappers),
                ));
            }
        }

        // inline comment between path segments belongs to the path seam owner
        if let Some((_, previous_token)) = previous_token
            && let Some(enclosing_scope_id) = enclosing_scope_id
            && let Some(enclosing_scope_span) = enclosing_scope_span
            && self.tree.get_node_type(enclosing_scope_id) == NodeType::Expression
        {
            let expression_id = LocalNodeId::<Expression>::new(enclosing_scope_id);
            if let Expression::Path { path, .. } = self.tree.get(expression_id)
                && path.segments.len() > 1
                && enclosing_scope_span.end > previous_token.span.end
            {
                return Some((
                    AnnotationPosition::LinePostfixBoundary,
                    self.promote_statement_owner(start_token, expression_id.id, statement_wrappers),
                ));
            }
        }

        // resolve same-line inline prefix seams when no postfix or path seam matched
        if let Some((next_index, next_token)) = next_token
            && self.is_inline_prefix_candidate_token(next_token)
            && self.is_same_line_prefix_seam(
                next_index,
                line_indices,
                group_start_index,
                group_end_index,
            )
            && let Some(target_node_id) = self.find_inline_prefix_owner(
                next_token,
                NodeSearchMode::SmallestOutermost,
                ignore_span,
                false,
            )
        {
            return Some((
                AnnotationPosition::LinePrefix,
                self.promote_statement_owner(start_token, target_node_id, statement_wrappers),
            ));
        }

        // no same-line prefix or postfix owner was found
        None
    }

    /// Resolve closer-adjacent seams before `)`, `]`, and `}`.
    #[allow(clippy::too_many_arguments)]
    fn resolve_closer_adjacent_target(
        &self,
        start_token: TokenSpan,
        previous_index: usize,
        previous_token: TokenSpan,
        next_index: usize,
        next_token: TokenSpan,
        tokens: &[TokenSpan],
        line_indices: &[u32],
        group_start_index: usize,
        group_end_index: usize,
        ignore_span: &MultiSpan,
        enclosing_span: Option<Span>,
    ) -> Option<(AnnotationPosition, u32)> {
        if !self.is_closer_token(next_token) {
            return None;
        }

        // closer-adjacent: before `)`, `]`, `}` binds left as boundary postfix
        if !self.is_opener_token(previous_token)
            && line_indices[previous_index] == line_indices[group_start_index]
            && line_indices[next_index] == line_indices[group_end_index]
            && let Some(target_node_id) =
                self.find_postfix_owner_for_token(previous_token)
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
        if previous_token.token.ty == TokenType::OpenParenthesis
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
        if previous_token.token.ty == TokenType::OpenParenthesis
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

        None
    }

    /// Resolve marker seams where a line comment appears before an explicit semicolon.
    fn resolve_line_comment_semicolon_marker_target(
        &self,
        start_token: TokenSpan,
        previous_non_trivia_token: Option<(usize, TokenSpan)>,
        next_non_trivia_token: Option<(usize, TokenSpan)>,
        statement_wrappers: &[Option<u32>],
        ignore_span: &MultiSpan,
    ) -> Option<(AnnotationPosition, u32)> {
        if !self.is_line_comment_token(start_token) {
            return None;
        }

        let (Some((_, previous_token)), Some((_, next_token))) =
            (previous_non_trivia_token, next_non_trivia_token)
        else {
            return None;
        };
        if next_token.token.ty != TokenType::Semicolon {
            return None;
        }

        let target_node_id = self
            .find_structural_owner_ending_at(previous_token, ignore_span)
            .or_else(|| self.find_postfix_owner_for_token(previous_token))?;
        Some((
            AnnotationPosition::LinePostfixBoundary,
            self.promote_statement_owner(start_token, target_node_id, statement_wrappers),
        ))
    }

    /// Resolve one-sided forward seams by scanning forward to the first valid owner token.
    fn resolve_one_sided_forward_target(
        &self,
        token_index: u32,
        group_len: usize,
        tokens: &[TokenSpan],
        start_token: TokenSpan,
        is_block_prefix_only: bool,
        statement_wrappers: &[Option<u32>],
        ignore_span: &MultiSpan,
    ) -> Option<(AnnotationPosition, u32)> {
        let mut next_index = token_index as usize + group_len;
        while let Some(next_token) = tokens.get(next_index) {
            // skip trivia and ignored spans while scanning for the forward owner seam
            if ignore_span.contains(&next_token.span)
                || TRIVIA_TOKEN_TYPES.contains(&next_token.token.ty)
                || next_token.token.ty == TokenType::Whitespace
            {
                next_index += 1;
                continue;
            }

            // semicolons are transparent for one-sided forward ownership
            if next_token.token.ty == TokenType::Semicolon {
                next_index += 1;
                continue;
            }

            // stop at hard statement or container boundaries
            if next_token.token.ty == TokenType::OpenBrace || self.is_closer_token(*next_token) {
                break;
            }

            // prefer owners that start exactly at the next token
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

            // attach as forward block-prefix when we found a stable owner
            if let Some(next_node) = next_node {
                return Some((
                    AnnotationPosition::BlockPrefix,
                    self.promote_statement_owner(start_token, next_node.idx, statement_wrappers),
                ));
            }

            // fallback for chained member seams where dot does not start a node
            if next_token.token.ty == TokenType::Dot
                && let Some(target_node_id) = self
                    .find_node_enclosing_at(
                        &next_token.span,
                        NodeSearchMode::SmallestOutermost,
                        |candidate| {
                            !ANNOTATION_NODE_TYPES.contains(&self.tree.get_node_type(candidate.idx))
                                && !ignore_span.contains(&candidate.span)
                        },
                    )
                    .map(|span| span.idx)
            {
                return Some((
                    AnnotationPosition::BlockPrefix,
                    self.promote_statement_owner(start_token, target_node_id, statement_wrappers),
                ));
            }

            // advance when no owner was found at this token
            next_index += 1;
        }

        None
    }

    /// Resolve one-sided backward seams by scanning backward to the first valid owner token.
    fn resolve_one_sided_backward_target(
        &self,
        token_index: u32,
        tokens: &[TokenSpan],
        start_token: TokenSpan,
        is_block_prefix_only: bool,
        statement_wrappers: &[Option<u32>],
        ignore_span: &MultiSpan,
        enclosing_span: Option<Span>,
    ) -> Option<(AnnotationPosition, u32)> {
        // block-prefix-only and stream-head seams cannot bind backward
        if is_block_prefix_only || token_index == 0 {
            return None;
        }

        // scan backward to the nearest non-trivia attachable token
        let mut previous_index = token_index as usize;
        while previous_index > 0 {
            previous_index -= 1;
            let Some(previous_token) = tokens.get(previous_index) else {
                break;
            };

            // skip trivia and ignored spans while scanning
            if ignore_span.contains(&previous_token.span)
                || TRIVIA_TOKEN_TYPES.contains(&previous_token.token.ty)
                || previous_token.token.ty == TokenType::Whitespace
            {
                continue;
            }

            // stop when we leave the enclosing scope seam
            if let Some(enclosing_span) = enclosing_span
                && !enclosing_span.intersects(previous_token.span)
            {
                break;
            }

            // bind backward to the closest postfix owner
            if let Some(previous_node) = (!self.is_opener_token(*previous_token))
                .then(|| {
                    self.find_node_ending_at(&previous_token.span, NodeSearchMode::BiggestOutermost)
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

        None
    }

    /// Resolve empty-brace seams in jsx-like argument containers to stub infix owners.
    fn resolve_empty_brace_stub_infix_target(
        &self,
        start_token: TokenSpan,
        previous_non_trivia_token: Option<(usize, TokenSpan)>,
        next_non_trivia_token: Option<(usize, TokenSpan)>,
        statement_wrappers: &[Option<u32>],
    ) -> Option<(AnnotationPosition, u32)> {
        let (Some((_, previous_non_trivia_token)), Some((_, next_non_trivia_token))) =
            (previous_non_trivia_token, next_non_trivia_token)
        else {
            return None;
        };
        if previous_non_trivia_token.token.ty != TokenType::OpenBrace
            || next_non_trivia_token.token.ty != TokenType::CloseBrace
        {
            return None;
        }

        let argument_scope = self.find_node_enclosing_at(
            &start_token.span,
            NodeSearchMode::SmallestOutermost,
            |candidate| self.tree.get_node_type(candidate.idx) == NodeType::Argument,
        )?;
        let argument_id = LocalNodeId::<Argument>::new(argument_scope.idx);
        let argument_value = match self.tree.get(argument_id) {
            Argument::Named { value, .. }
            | Argument::Labeled { value, .. }
            | Argument::Positional { value, .. }
            | Argument::Spread { value, .. } => *value,
        };
        if !matches!(self.tree.get(argument_value), Expression::Stub) {
            return None;
        }

        Some((
            AnnotationPosition::BlockInfix,
            self.promote_statement_owner(start_token, argument_value.id, statement_wrappers),
        ))
    }

    /// Resolve no-side seams to enclosing container infix ownership.
    fn resolve_no_side_container_target(
        &self,
        start_token: TokenSpan,
        enclosing_scope_id: Option<u32>,
        is_block_prefix_only: bool,
        statement_wrappers: &[Option<u32>],
    ) -> Option<(AnnotationPosition, u32)> {
        if is_block_prefix_only {
            return None;
        }

        let enclosing_scope_id = enclosing_scope_id?;
        Some((
            AnnotationPosition::BlockInfix,
            self.promote_statement_owner(start_token, enclosing_scope_id, statement_wrappers),
        ))
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
        // multiline trivia after separators always binds as block prefix
        if !is_one_line {
            return AnnotationPosition::BlockPrefix;
        }

        // ternary and type-colon seams use inline prefix ownership
        if matches!(
            separator,
            TriviaSeparatorKind::Colon | TriviaSeparatorKind::Maybe
        ) {
            return AnnotationPosition::LinePrefix;
        }

        // cast and satisfies seams only stay inline when the right owner starts on this line
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

        // call and new argument commas prefer inline right-prefix ownership
        if separator == TriviaSeparatorKind::Comma
            && self.is_call_or_new_argument_owner(right_owner_id)
        {
            return AnnotationPosition::LinePrefix;
        }

        // default: inline when right owner is on this line, otherwise block prefix
        if group_end_line_index == next_token_line_index {
            AnnotationPosition::LinePrefix
        } else {
            AnnotationPosition::BlockPrefix
        }
    }

    /// Resolve trivia owner and annotation position for one trivia group.
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

        // classify trivia shape from token kind and line relation
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

        // compute seam context tokens and enclosing scope once
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

        // resolve control keyword seams before generic separator logic
        if let Some(target) = self.resolve_control_keyword_seam_target(
            next_non_trivia_token,
            tokens,
            ignore_span,
            enclosing_span,
        ) {
            return Some(target);
        }

        // resolve if-head line seams before generic separator logic
        if let Some(target) = self.resolve_if_head_seam_target(
            start_token,
            previous_non_trivia_token,
            next_non_trivia_token,
        ) {
            return Some(target);
        }

        // resolve lambda arrow seams before generic separator logic
        if let Some(target) =
            self.resolve_lambda_arrow_seam_target(start_token, next_non_trivia_token)
        {
            return Some(target);
        }

        // resolve call and new callee-to-paren seams before generic separator logic
        if let Some(target) = self.resolve_callee_paren_seam_target(
            start_token,
            previous_non_trivia_token,
            next_non_trivia_token,
            tokens,
            line_indices,
            group_end_index,
            ignore_span,
            enclosing_span,
        ) {
            return Some(target);
        }

        // resolve parameter type seams before generic separator logic
        if let Some(target) =
            self.resolve_parameter_type_seam_target(start_token, next_non_trivia_token)
        {
            return Some(target);
        }

        // leading type arm separators after `=` belong to the full binary type owner
        if let (Some((_, previous_token)), Some((next_index, next_token))) =
            (previous_non_trivia_token, next_non_trivia_token)
            && previous_token.token.ty == TokenType::Assign
            && let Some(owner_id) = self.resolve_leading_type_separator_target(
                start_token.span,
                next_token,
                next_index,
                tokens,
                ignore_span,
                enclosing_span,
            )
        {
            return Some((AnnotationPosition::BlockPrefix, owner_id));
        }

        // resolve as-const token-gap exception seams
        if let Some(target) = self.resolve_as_const_token_gap_target(
            start_token,
            previous_non_trivia_token,
            next_non_trivia_token,
            is_block_comment,
            is_block_comment_single_line,
            tokens,
            line_indices,
            group_start_index,
            statement_wrappers,
            ignore_span,
            enclosing_span,
        ) {
            return Some(target);
        }

        // separator-adjacent: before-separator binds left, after-separator binds right
        if start_token.token.ty != TokenType::Newline
            && let (Some((previous_index, previous_token)), Some((next_index, next_token))) =
                (previous_non_trivia_token, next_non_trivia_token)
        {
            // resolve seams where trivia appears before separator tokens
            if let Some(target) = self.resolve_before_separator_target(
                start_token,
                previous_index,
                previous_token,
                next_index,
                next_token,
                line_indices,
                group_start_index,
                group_end_index,
                statement_wrappers,
                ignore_span,
            ) {
                return Some(target);
            }

            // resolve seams where trivia appears after separator tokens
            if let Some(target) = self.resolve_after_separator_target(
                start_token,
                previous_index,
                previous_token,
                next_index,
                next_token,
                tokens,
                line_indices,
                group_start_index,
                group_end_index,
                is_one_line,
                statement_wrappers,
                ignore_span,
                enclosing_span,
            ) {
                return Some(target);
            }

            // resolve seams directly before closer delimiters
            if let Some(target) = self.resolve_closer_adjacent_target(
                start_token,
                previous_index,
                previous_token,
                next_index,
                next_token,
                tokens,
                line_indices,
                group_start_index,
                group_end_index,
                ignore_span,
                enclosing_span,
            ) {
                return Some(target);
            }
        }

        // resolve one-line same-line prefix and postfix seams
        if !is_block_prefix_only && is_one_line && start_token.token.ty != TokenType::Newline {
            // resolve same-line prefix and postfix ownership in one pass
            if let Some(target) = self.resolve_line_prefix_postfix_target(
                start_token,
                previous_token,
                next_token,
                next_non_trivia_token,
                enclosing_scope.map(|scope| scope.idx),
                enclosing_scope.map(|scope| scope.span),
                tokens,
                line_indices,
                group_start_index,
                group_end_index,
                is_block_comment_single_line,
                statement_wrappers,
                ignore_span,
            ) {
                return Some(target);
            }
        }

        // resolve line comment marker seams before one-sided scans
        if let Some(target) = self.resolve_line_comment_semicolon_marker_target(
            start_token,
            previous_non_trivia_token,
            next_non_trivia_token,
            statement_wrappers,
            ignore_span,
        ) {
            return Some(target);
        }

        // resolve one-sided forward seams
        if let Some(target) = self.resolve_one_sided_forward_target(
            token_index,
            group_len,
            tokens,
            start_token,
            is_block_prefix_only,
            statement_wrappers,
            ignore_span,
        ) {
            return Some(target);
        }

        // resolve one-sided backward seams
        if let Some(target) = self.resolve_one_sided_backward_target(
            token_index,
            tokens,
            start_token,
            is_block_prefix_only,
            statement_wrappers,
            ignore_span,
            enclosing_span,
        ) {
            return Some(target);
        }

        // resolve empty brace stub seams for jsx container stubs
        if let Some(target) = self.resolve_empty_brace_stub_infix_target(
            start_token,
            previous_non_trivia_token,
            next_non_trivia_token,
            statement_wrappers,
        ) {
            return Some(target);
        }

        // resolve remaining no-side seams to enclosing container infix
        if let Some(target) = self.resolve_no_side_container_target(
            start_token,
            enclosing_scope.map(|scope| scope.idx),
            is_block_prefix_only,
            statement_wrappers,
        ) {
            return Some(target);
        }

        // no owner was found for this trivia group
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
