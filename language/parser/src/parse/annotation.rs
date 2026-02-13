use crate::parse::timing::tags;
use crate::{ParseResult, Parser};
use destack_ast::{
    Annotation, AnnotationPosition, Blank, Comment, CommentStyle, Decorator, Doc, DocStyle,
    LocalNodeId, TokenSpan, TokenType,
};
use destack_source::Span;

const ANNOTATION_TOKEN_TYPES: [TokenType; 5] = [
    TokenType::Newline,
    TokenType::LineComment,
    TokenType::DocLineComment,
    TokenType::BlockComment,
    TokenType::DocBlockComment,
];
const DECORATOR_EXPRESSION_PRECEDENCE: u16 = u16::MAX;

/// Side-token view for inline annotation grouping.
#[derive(Debug, Copy, Clone)]
struct InlineAnnotationToken {
    /// Semantic token index that owns this side token range.
    token_index: usize,
    /// Index in the global side-token stream.
    side_index: usize,
    /// Side-token payload.
    token: TokenSpan,
}

impl Parser {
    /// Eat any leading decorators and return collected decorator ids.
    pub(crate) fn eat_decorators_prefix_collect_maybe(
        &mut self,
    ) -> ParseResult<Vec<LocalNodeId<Decorator>>> {
        let mut decorators = Vec::new();
        while self.peek_is(TokenType::At) {
            let start = self.mark_span();
            let decorator = self.with_recovery(
                &start,
                |parser| parser.eat_decorator().map(Some),
                None,
                TokenType::Newline,
            );
            if let Some(decorator) = decorator {
                decorators.push(decorator);
            }

            // consume trailing newlines between decorator entries
            self.eat_newlines_maybe()?;
        }

        Ok(decorators)
    }

    /// Eat one decorator expression.
    fn eat_decorator(&mut self) -> ParseResult<LocalNodeId<Decorator>> {
        let start = self.mark_span();

        // eat @ marker
        self.eat_token(TokenType::At)?;

        // decorators always parse as value expressions
        let mut decorator_options = self
            .options
            .not_in_position()
            .in_left_precedence(DECORATOR_EXPRESSION_PRECEDENCE)
            .not_in_sequence_expression()
            .in_decorator();
        decorator_options.in_type = false;
        decorator_options.in_static = false;
        decorator_options.in_super_type = false;
        decorator_options.in_before_type = false;
        decorator_options.in_type_conditional_right = false;
        decorator_options.in_type_mapped_constraint = false;

        // parse decorator target expression
        let expression = self.eat_expression(decorator_options)?;

        // store decorator side node
        let decorator = self
            .tree
            .insert(Decorator { expression }, self.get_span_from(&start));
        let main_span = self
            .tree
            .get_main_span(expression)
            .unwrap_or_else(|| self.tree.get_span(expression));
        self.tree.set_main_span(decorator, main_span);
        Ok(decorator)
    }

    /// Attach decorator nodes to a parsed target.
    pub(crate) fn attach_decorators_to_target(
        &mut self,
        decorators: Vec<LocalNodeId<Decorator>>,
        target_node_id: u32,
    ) {
        for decorator_id in decorators {
            let span = self.tree.get_span(decorator_id);
            let annotation_id = self.tree.insert(
                Annotation::Decorator {
                    node: decorator_id,
                    position: AnnotationPosition::BlockPrefix,
                },
                span,
            );
            self.tree.append_annotation(target_node_id, annotation_id);
        }
    }

    /// Attach leading annotations for one semantic token in statement and declaration contexts.
    #[inline(always)]
    pub(crate) fn attach_inline_leading_annotations_for_token(
        &mut self,
        token_index: usize,
        skipped_newline_count: usize,
        target_node_id: u32,
    ) {
        self.attach_inline_leading_annotations_for_token_with_mode(
            token_index,
            skipped_newline_count,
            target_node_id,
            AnnotationPosition::BlockPrefix,
            true,
        );
    }

    /// Attach leading annotations for one semantic token in expression contexts.
    #[inline(always)]
    pub(crate) fn attach_inline_expression_leading_annotations_for_token(
        &mut self,
        token_index: usize,
        skipped_newline_count: usize,
        target_node_id: u32,
    ) {
        self.attach_inline_leading_annotations_for_token_with_mode(
            token_index,
            skipped_newline_count,
            target_node_id,
            AnnotationPosition::LinePrefix,
            false,
        );
    }

    /// Attach infix annotations for one semantic token in container contexts.
    #[inline(always)]
    pub(crate) fn attach_inline_infix_annotations_for_token(
        &mut self,
        token_index: usize,
        skipped_newline_count: usize,
        target_node_id: u32,
    ) {
        self.attach_inline_blank_from_skipped_newlines_with_position(
            token_index,
            skipped_newline_count,
            target_node_id,
            AnnotationPosition::BlockInfix,
        );

        let token_window_start =
            self.annotation_leading_window_start(token_index, skipped_newline_count);
        let annotation_tokens = self.collect_unclaimed_side_annotation_tokens_for_window(
            token_window_start,
            token_index + 1,
        );
        if annotation_tokens.is_empty() {
            return;
        }

        let mut group_start_index = 0usize;
        while group_start_index < annotation_tokens.len() {
            let token_type = annotation_tokens[group_start_index].token.token.ty;
            let group_end_index =
                self.next_same_type_group_end(&annotation_tokens, group_start_index, true);

            let group_len = group_end_index - group_start_index + 1;
            if self.is_comment_annotation_token_type(token_type) {
                self.attach_inline_annotation_group(
                    token_type,
                    &annotation_tokens,
                    group_start_index,
                    group_end_index,
                    group_len,
                    target_node_id,
                    AnnotationPosition::BlockInfix,
                );
            }

            group_start_index = group_end_index + 1;
        }
    }

    /// Attach postfix blank lines from skipped semantic newline tokens.
    #[inline(always)]
    pub(crate) fn attach_inline_postfix_blank_from_skipped_newlines(
        &mut self,
        token_index: usize,
        skipped_newline_count: usize,
        target_node_id: u32,
    ) {
        self.attach_inline_blank_from_skipped_newlines_with_position(
            token_index,
            skipped_newline_count,
            target_node_id,
            AnnotationPosition::BlockPostfix,
        );
    }

    /// Attach prefix blank lines from skipped semantic newline tokens.
    #[inline(always)]
    pub(crate) fn attach_inline_prefix_blank_from_skipped_newlines(
        &mut self,
        token_index: usize,
        skipped_newline_count: usize,
        target_node_id: u32,
    ) {
        self.attach_inline_blank_from_skipped_newlines_with_position(
            token_index,
            skipped_newline_count,
            target_node_id,
            AnnotationPosition::BlockPrefix,
        );
    }

    /// Attach leading annotations for a synthetic stub target.
    pub(crate) fn attach_inline_stub_annotations_for_token(
        &mut self,
        token_index: usize,
        skipped_newline_count: usize,
        target_node_id: u32,
    ) {
        // collect side tokens around the synthetic stub boundary
        let token_window_start =
            self.annotation_leading_window_start(token_index, skipped_newline_count);
        let token_window_start = token_window_start.saturating_sub(1);
        let annotation_tokens = self.collect_unclaimed_side_annotation_tokens_for_window(
            token_window_start,
            token_index + 1,
        );
        if annotation_tokens.is_empty() {
            return;
        }

        // attach only comment-like groups as infix content on the stub
        let mut group_start_index = 0usize;
        while group_start_index < annotation_tokens.len() {
            let token_type = annotation_tokens[group_start_index].token.token.ty;
            let group_end_index =
                self.next_same_type_group_end(&annotation_tokens, group_start_index, true);

            let group_len = group_end_index - group_start_index + 1;
            if self.is_comment_annotation_token_type(token_type) {
                self.attach_inline_annotation_group(
                    token_type,
                    &annotation_tokens,
                    group_start_index,
                    group_end_index,
                    group_len,
                    target_node_id,
                    AnnotationPosition::BlockInfix,
                );
            }

            group_start_index = group_end_index + 1;
        }
    }

    /// Attach trailing annotations before the current parser token to an expression.
    #[inline(always)]
    pub(crate) fn attach_inline_trailing_annotations_for_current_token(
        &mut self,
        target_node_id: u32,
    ) {
        // bail when there are no semantic tokens
        let token_len = self.tokens().len();
        if token_len == 0 {
            return;
        }

        // clamp the parser position to a valid semantic token index
        let mut token_index = self.pos_index();
        if token_index >= token_len {
            token_index = token_len - 1;
        }

        // skip windows with no non whitespace side trivia
        let token_window_start = self.annotation_leading_window_start(token_index, 0);
        if !self.token_window_has_non_whitespace_side(token_window_start, token_index + 1) {
            return;
        }

        // collect candidate side tokens for trailing attachment
        let annotation_tokens = self.collect_unclaimed_side_annotation_tokens_for_window(
            token_window_start,
            token_index + 1,
        );
        if annotation_tokens.is_empty() {
            return;
        }

        let current_token_type = self.token_type_at(token_index);
        let is_boundary = self.is_inline_trailing_boundary_token(current_token_type);

        // process each mergeable token group and attach it with boundary-aware positioning
        let mut group_start_index = 0usize;
        while group_start_index < annotation_tokens.len() {
            let token_type = annotation_tokens[group_start_index].token.token.ty;
            let mut group_end_index = group_start_index;
            while group_end_index + 1 < annotation_tokens.len()
                && self.annotation_tokens_can_merge(
                    &annotation_tokens[group_end_index],
                    &annotation_tokens[group_end_index + 1],
                )
            {
                group_end_index += 1;
            }

            let group_len = group_end_index - group_start_index + 1;
            if ANNOTATION_TOKEN_TYPES.contains(&token_type) && token_type != TokenType::Newline {
                let is_group_on_current_token = annotation_tokens
                    [group_start_index..=group_end_index]
                    .iter()
                    .all(|token| token.token_index == token_index);
                let has_line_break_before_boundary = self
                    .annotation_group_has_line_break_before_boundary(
                        &annotation_tokens,
                        group_end_index,
                        token_index,
                    );
                let is_line_boundary_attachment =
                    is_group_on_current_token && !has_line_break_before_boundary;
                let position = match token_type {
                    TokenType::LineComment | TokenType::DocLineComment => {
                        if is_boundary && is_line_boundary_attachment {
                            AnnotationPosition::LinePostfixBoundary
                        } else if is_boundary {
                            AnnotationPosition::BlockPostfix
                        } else {
                            AnnotationPosition::LinePostfix
                        }
                    }
                    TokenType::BlockComment | TokenType::DocBlockComment => {
                        let is_single_line = self.annotation_group_is_single_line(
                            &annotation_tokens,
                            group_start_index,
                            group_end_index,
                        );
                        if is_single_line {
                            if is_boundary && is_line_boundary_attachment {
                                AnnotationPosition::LinePostfixBoundary
                            } else if is_boundary {
                                AnnotationPosition::BlockPostfix
                            } else {
                                AnnotationPosition::LinePostfix
                            }
                        } else {
                            AnnotationPosition::BlockPostfix
                        }
                    }
                    _ => continue,
                };

                // attach the grouped annotation to the target and claim side tokens
                self.attach_inline_annotation_group(
                    token_type,
                    &annotation_tokens,
                    group_start_index,
                    group_end_index,
                    group_len,
                    target_node_id,
                    position,
                );
            }

            group_start_index = group_end_index + 1;
        }
    }

    /// Return true when the current token is a trailing annotation boundary.
    fn is_inline_trailing_boundary_token(&self, token_type: TokenType) -> bool {
        matches!(
            token_type,
            TokenType::Newline
                | TokenType::CloseBrace
                | TokenType::CloseBracket
                | TokenType::CloseParenthesis
                | TokenType::End
        )
    }

    /// Attach boundary annotations before a dot token to the left expression.
    #[inline(always)]
    pub(crate) fn attach_inline_dot_boundary_annotations_for_token(
        &mut self,
        token_index: usize,
        skipped_newline_count: usize,
        target_node_id: u32,
    ) {
        // collect trivia around the dot boundary
        let token_window_start =
            self.annotation_leading_window_start(token_index, skipped_newline_count);
        let token_window_start = token_window_start.saturating_sub(1);
        let annotation_tokens = self.collect_unclaimed_side_annotation_tokens_for_window(
            token_window_start,
            token_index + 1,
        );
        if annotation_tokens.is_empty() {
            return;
        }

        // choose boundary position from line-break context
        let has_line_break_before = self.line_terminator_before_index(token_index);
        let position = if has_line_break_before {
            AnnotationPosition::LinePostfixBoundary
        } else {
            AnnotationPosition::BlockInfix
        };

        let mut group_start_index = 0usize;
        while group_start_index < annotation_tokens.len() {
            let token_type = annotation_tokens[group_start_index].token.token.ty;
            let group_end_index =
                self.next_same_type_group_end(&annotation_tokens, group_start_index, false);

            let group_len = group_end_index - group_start_index + 1;
            if self.is_comment_annotation_token_type(token_type) {
                // line comments after a line break are prefix candidates for the right side
                if has_line_break_before
                    && matches!(
                        token_type,
                        TokenType::LineComment | TokenType::DocLineComment
                    )
                {
                    group_start_index = group_end_index + 1;
                    continue;
                }

                // attach eligible comment group at the boundary position
                self.attach_inline_annotation_group(
                    token_type,
                    &annotation_tokens,
                    group_start_index,
                    group_end_index,
                    group_len,
                    target_node_id,
                    position,
                );
            }

            group_start_index = group_end_index + 1;
        }
    }

    /// Attach chained line comment prefixes to the following member target.
    #[inline(always)]
    pub(crate) fn attach_inline_dot_prefix_annotations_for_token(
        &mut self,
        token_index: usize,
        skipped_newline_count: usize,
        target_node_id: u32,
    ) {
        // prefix attachment only applies across line breaks
        if !self.line_terminator_before_index(token_index) {
            return;
        }

        // collect nearby trivia that could prefix the next chain segment
        let token_window_start =
            self.annotation_leading_window_start(token_index, skipped_newline_count);
        let token_window_start = token_window_start.saturating_sub(1);
        let annotation_tokens = self.collect_unclaimed_side_annotation_tokens_for_window(
            token_window_start,
            token_index + 1,
        );
        if annotation_tokens.is_empty() {
            return;
        }

        // only line comment groups attach as dot-prefix annotations
        let mut group_start_index = 0usize;
        while group_start_index < annotation_tokens.len() {
            let token_type = annotation_tokens[group_start_index].token.token.ty;
            let group_end_index =
                self.next_same_type_group_end(&annotation_tokens, group_start_index, false);

            let group_len = group_end_index - group_start_index + 1;
            if matches!(
                token_type,
                TokenType::LineComment | TokenType::DocLineComment
            ) {
                self.attach_inline_annotation_group(
                    token_type,
                    &annotation_tokens,
                    group_start_index,
                    group_end_index,
                    group_len,
                    target_node_id,
                    AnnotationPosition::BlockPrefix,
                );
            }

            group_start_index = group_end_index + 1;
        }
    }

    /// Attach grouped leading annotations for one semantic token.
    #[inline(always)]
    fn attach_inline_leading_annotations_for_token_with_mode(
        &mut self,
        token_index: usize,
        skipped_newline_count: usize,
        target_node_id: u32,
        inline_position: AnnotationPosition,
        require_line_break_before: bool,
    ) {
        // enforce line-break-only modes early
        let has_line_break_before = self.line_terminator_before_index(token_index);
        if require_line_break_before && !has_line_break_before {
            return;
        }

        // collect leading side tokens from skipped newline tokens and the target token
        let mut token_window_start =
            self.annotation_leading_window_start(token_index, skipped_newline_count);
        if !require_line_break_before {
            token_window_start = token_window_start.saturating_sub(1);
        }

        let has_non_whitespace_side =
            self.token_window_has_non_whitespace_side(token_window_start, token_index + 1);
        if !has_non_whitespace_side && skipped_newline_count <= 1 {
            let materialized_newline_count = self.materialized_leading_newline_count(token_index);
            if materialized_newline_count <= 1 {
                return;
            }
        }

        // attach blank lines from skipped semantic newlines
        self.attach_inline_blank_from_skipped_newlines_with_position(
            token_index,
            skipped_newline_count,
            target_node_id,
            AnnotationPosition::BlockPrefix,
        );

        if !has_non_whitespace_side {
            return;
        }

        // collect concrete side annotation tokens for this leading window
        let annotation_tokens = self.collect_unclaimed_side_annotation_tokens_for_window(
            token_window_start,
            token_index + 1,
        );
        if annotation_tokens.is_empty() {
            return;
        }

        // process contiguous token groups
        let mut group_start_index = 0usize;
        while group_start_index < annotation_tokens.len() {
            let token_type = annotation_tokens[group_start_index].token.token.ty;
            let group_end_index =
                self.next_same_type_group_end(&annotation_tokens, group_start_index, true);

            let group_len = group_end_index - group_start_index + 1;
            if ANNOTATION_TOKEN_TYPES.contains(&token_type)
                && (token_type != TokenType::Newline || group_len > 1)
            {
                let position = if token_type == TokenType::Newline {
                    AnnotationPosition::BlockPrefix
                } else if has_line_break_before {
                    AnnotationPosition::BlockPrefix
                } else {
                    inline_position
                };

                self.attach_inline_annotation_group(
                    token_type,
                    &annotation_tokens,
                    group_start_index,
                    group_end_index,
                    group_len,
                    target_node_id,
                    position,
                );
            }

            group_start_index = group_end_index + 1;
        }
    }

    /// Attach blank annotations from skipped semantic newline tokens.
    #[inline(always)]
    fn attach_inline_blank_from_skipped_newlines_with_position(
        &mut self,
        token_index: usize,
        skipped_newline_count: usize,
        target_node_id: u32,
        position: AnnotationPosition,
    ) {
        // normalize newline count with materialized fallback in lazy mode
        let mut skipped_newline_count = skipped_newline_count;
        if skipped_newline_count <= 1 {
            let materialized_newline_count = self.materialized_leading_newline_count(token_index);
            skipped_newline_count = skipped_newline_count.max(materialized_newline_count);
        }

        // reject degenerate ranges with no attachable blank content
        if skipped_newline_count <= 1 || token_index == 0 || token_index < skipped_newline_count {
            return;
        }

        let start_newline_index = token_index - skipped_newline_count;
        let end_newline_index = token_index - 1;
        let token_len = self.tokens().len();
        if start_newline_index >= token_len || end_newline_index >= token_len {
            return;
        }

        // count blank newlines and keep first and last blank token indexes
        let mut blank_lines = 0u32;
        let mut first_blank_newline_index: Option<usize> = None;
        let mut last_blank_newline_index: Option<usize> = None;
        for newline_index in (start_newline_index + 1)..=end_newline_index {
            if self.skipped_newline_token_has_comment(newline_index) {
                continue;
            }

            blank_lines += 1;
            if first_blank_newline_index.is_none() {
                first_blank_newline_index = Some(newline_index);
            }
            last_blank_newline_index = Some(newline_index);
        }
        if blank_lines == 0 {
            return;
        }

        let (Some(first_blank_newline_index), Some(last_blank_newline_index)) =
            (first_blank_newline_index, last_blank_newline_index)
        else {
            return;
        };

        // build span from first to last blank newline token
        let tokens = self.tokens();
        let start_token = tokens[first_blank_newline_index];
        let end_token = tokens[last_blank_newline_index];
        let span = Span::new(
            start_token.span.file,
            start_token.span.start,
            end_token.span.end,
        );

        let blank = self.tree.insert(Blank { lines: blank_lines }, span);

        // attach blank annotation at the requested position
        let annotation_id = self.tree.insert(
            Annotation::Blank {
                node: blank,
                position,
            },
            span,
        );
        self.tree.append_annotation(target_node_id, annotation_id);
    }

    /// Return true when a skipped newline token has comment or doc side trivia.
    fn skipped_newline_token_has_comment(&mut self, token_index: usize) -> bool {
        let (side_start, side_end) = self.side_range_for_token(token_index);
        if side_start >= side_end {
            return false;
        }

        for side_index in side_start..side_end {
            let side_token = self.token_stream.side_tokens()[side_index];
            if matches!(
                side_token.token.ty,
                TokenType::LineComment
                    | TokenType::DocLineComment
                    | TokenType::BlockComment
                    | TokenType::DocBlockComment
            ) {
                return true;
            }
        }

        false
    }

    /// Return the side-token range for one semantic token index.
    fn side_range_for_token(&mut self, token_index: usize) -> (usize, usize) {
        if self.tokens_prelexed && !self.has_active_split() {
            return self.token_stream.leading_side_range_prelexed(token_index);
        }

        self.token_stream.leading_side_range(token_index)
    }

    /// Return the semantic token window start used for leading annotation scans.
    #[inline(always)]
    fn annotation_leading_window_start(
        &mut self,
        token_index: usize,
        skipped_newline_count: usize,
    ) -> usize {
        if skipped_newline_count > 0 {
            return token_index.saturating_sub(skipped_newline_count);
        }

        if !self.line_terminator_before_index(token_index) {
            return token_index;
        }

        let materialized_newline_count = self.materialized_leading_newline_count(token_index);
        token_index.saturating_sub(materialized_newline_count)
    }

    /// Count contiguous semantic newline tokens directly before the token index.
    #[inline(always)]
    fn materialized_leading_newline_count(&mut self, token_index: usize) -> usize {
        if token_index == 0 {
            return 0;
        }

        // ensure trivia before this boundary is materialized in lazy lex mode
        let _ = self.token_at(token_index.saturating_sub(1));

        let tokens = self.tokens();
        let mut cursor = token_index.min(tokens.len());
        let mut newline_count = 0usize;

        while cursor > 0 && tokens[cursor - 1].token.ty == TokenType::Newline {
            newline_count += 1;
            cursor -= 1;
        }

        newline_count
    }

    /// Collect unclaimed side annotation tokens for a semantic token window.
    #[inline(always)]
    fn collect_unclaimed_side_annotation_tokens_for_window(
        &mut self,
        token_window_start: usize,
        token_window_end_exclusive: usize,
    ) -> Vec<InlineAnnotationToken> {
        // clamp token window to current semantic token length
        let token_len = self.tokens().len();
        let token_window_start = token_window_start.min(token_len);
        let token_window_end_exclusive = token_window_end_exclusive.min(token_len);
        if token_window_start >= token_window_end_exclusive {
            return Vec::new();
        }

        let has_non_whitespace_side = self
            .token_window_has_non_whitespace_side(token_window_start, token_window_end_exclusive);
        if !has_non_whitespace_side {
            return Vec::new();
        }

        // derive side token window that corresponds to this semantic token window
        self.ensure_claimed_side_tokens_capacity();
        let (side_window_start, _) = self.side_range_for_token(token_window_start);
        let (_, side_window_end) = self.side_range_for_token(token_window_end_exclusive - 1);
        if side_window_start >= side_window_end {
            return Vec::new();
        }

        // collect unclaimed non-whitespace side tokens owned by this token window
        let mut tokens = Vec::new();
        for side_index in side_window_start..side_window_end {
            if self.annotation_claimed_side_tokens[side_index] {
                continue;
            }

            let token = self.token_stream.side_tokens()[side_index];
            if token.token.ty == TokenType::Whitespace {
                continue;
            }

            let token_index = self
                .token_stream
                .side_owner_token_index(side_index)
                .unwrap_or(token_window_end_exclusive - 1);
            if token_index < token_window_start || token_index >= token_window_end_exclusive {
                continue;
            }

            tokens.push(InlineAnnotationToken {
                token_index,
                side_index,
                token,
            });
        }

        tokens
    }

    /// Return true when a semantic token window has non-whitespace side trivia.
    #[inline(always)]
    fn token_window_has_non_whitespace_side(
        &mut self,
        token_window_start: usize,
        token_window_end_exclusive: usize,
    ) -> bool {
        if self.tokens_prelexed && !self.has_active_split() {
            self.token_stream
                .has_non_whitespace_side_in_window_prelexed(
                    token_window_start,
                    token_window_end_exclusive,
                )
        } else {
            self.token_stream
                .has_non_whitespace_side_in_window(token_window_start, token_window_end_exclusive)
        }
    }

    /// Ensure claimed-side-token flags cover the current side stream length.
    fn ensure_claimed_side_tokens_capacity(&mut self) {
        let side_len = self.token_stream.side_tokens().len();
        if self.annotation_claimed_side_tokens.len() < side_len {
            self.annotation_claimed_side_tokens.resize(side_len, false);
        }
    }

    /// Return true when the token type is comment or doc trivia.
    #[inline(always)]
    fn is_comment_annotation_token_type(&self, token_type: TokenType) -> bool {
        matches!(
            token_type,
            TokenType::LineComment
                | TokenType::DocLineComment
                | TokenType::BlockComment
                | TokenType::DocBlockComment
        )
    }

    /// Find the end index of a same-type token group.
    #[inline(always)]
    fn next_same_type_group_end(
        &self,
        tokens: &[InlineAnnotationToken],
        group_start_index: usize,
        requires_adjacent_owner_index: bool,
    ) -> usize {
        let token_type = tokens[group_start_index].token.token.ty;
        let mut group_end_index = group_start_index;

        while group_end_index + 1 < tokens.len() {
            let current = tokens[group_end_index];
            let next = tokens[group_end_index + 1];
            if next.token.token.ty != token_type {
                break;
            }

            if requires_adjacent_owner_index
                && next.token_index > current.token_index.saturating_add(1)
            {
                break;
            }

            group_end_index += 1;
        }

        group_end_index
    }

    /// Return true when a grouped block-style annotation stays on one line.
    fn annotation_group_is_single_line(
        &self,
        tokens: &[InlineAnnotationToken],
        group_start_idx: usize,
        group_end_idx: usize,
    ) -> bool {
        for token in &tokens[group_start_idx..=group_end_idx] {
            let raw = self.get_span_str(token.token.span);
            for byte in raw.as_bytes() {
                if *byte == b'\n' || *byte == b'\r' {
                    return false;
                }
            }
        }

        true
    }

    /// Return true when text between annotation group end and boundary token contains a newline.
    fn annotation_group_has_line_break_before_boundary(
        &self,
        tokens: &[InlineAnnotationToken],
        group_end_idx: usize,
        boundary_token_index: usize,
    ) -> bool {
        let Some(boundary_token) = self.tokens().get(boundary_token_index) else {
            return false;
        };
        let group_end_token = tokens[group_end_idx].token;

        if group_end_token.span.file != boundary_token.span.file {
            return false;
        }

        // annotations that start after the boundary token are on later lines
        if group_end_token.span.start >= boundary_token.span.start {
            return true;
        }

        // overlapping spans are treated as same line boundary attachments
        if group_end_token.span.end >= boundary_token.span.start {
            return false;
        }

        let between_span = Span::new(
            group_end_token.span.file,
            group_end_token.span.end,
            boundary_token.span.start,
        );
        let between_text = self.get_span_str(between_span);
        between_text
            .as_bytes()
            .iter()
            .any(|byte| *byte == b'\n' || *byte == b'\r')
    }

    /// Return true when two inline annotation tokens belong to the same merged group.
    fn annotation_tokens_can_merge(
        &self,
        left: &InlineAnnotationToken,
        right: &InlineAnnotationToken,
    ) -> bool {
        if left.token.token.ty != right.token.token.ty {
            return false;
        }

        if right.token_index > left.token_index.saturating_add(1) {
            return false;
        }

        if left.token.span.file != right.token.span.file {
            return false;
        }

        if right.token.span.start <= left.token.span.end {
            return true;
        }

        let between_span = Span::new(
            left.token.span.file,
            left.token.span.end,
            right.token.span.start,
        );
        self.count_line_breaks_in_span(between_span) <= 1
    }

    /// Count logical line breaks in a span.
    fn count_line_breaks_in_span(&self, span: Span) -> usize {
        let text = self.get_span_str(span);
        let bytes = text.as_bytes();
        let mut index = 0usize;
        let mut line_breaks = 0usize;

        while index < bytes.len() {
            let byte = bytes[index];
            if byte == b'\r' {
                line_breaks += 1;

                if index + 1 < bytes.len() && bytes[index + 1] == b'\n' {
                    index += 1;
                }
            } else if byte == b'\n' {
                line_breaks += 1;
            }

            index += 1;
        }

        line_breaks
    }

    /// Attach one grouped inline annotation to a target node.
    fn attach_inline_annotation_group(
        &mut self,
        token_type: TokenType,
        tokens: &[InlineAnnotationToken],
        group_start_idx: usize,
        group_end_idx: usize,
        group_len: usize,
        target_node_id: u32,
        position: AnnotationPosition,
    ) {
        let start_token = tokens[group_start_idx].token;
        let end_token = tokens[group_end_idx].token;
        let span = Span::new(
            start_token.span.file,
            start_token.span.start,
            end_token.span.end,
        );

        let annotation_id = match token_type {
            TokenType::Newline => {
                let blank = self.tree.insert(
                    Blank {
                        lines: group_len as u32 - 1,
                    },
                    span,
                );
                self.tree.insert(
                    Annotation::Blank {
                        node: blank,
                        position,
                    },
                    span,
                )
            }
            TokenType::LineComment => {
                let string = self.clean_annotation_string(
                    token_type,
                    tokens,
                    group_start_idx,
                    group_end_idx,
                    group_len,
                );
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
                let string = self.clean_annotation_string(
                    token_type,
                    tokens,
                    group_start_idx,
                    group_end_idx,
                    group_len,
                );
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
                let string = self.clean_annotation_string(
                    token_type,
                    tokens,
                    group_start_idx,
                    group_end_idx,
                    group_len,
                );
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
                let string = self.clean_annotation_string(
                    token_type,
                    tokens,
                    group_start_idx,
                    group_end_idx,
                    group_len,
                );
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
            _ => return,
        };

        // mark side tokens as claimed after attachment
        self.ensure_claimed_side_tokens_capacity();
        for token in &tokens[group_start_idx..=group_end_idx] {
            self.annotation_claimed_side_tokens[token.side_index] = true;
        }

        // attach annotation to parsed target
        self.tree.append_annotation(target_node_id, annotation_id);
    }

    /// Finalize annotations after parsing.
    pub(crate) fn attach_annotations(&mut self) {
        if !self.should_attach_annotations() {
            return;
        }

        // inline attachment runs during parse: only keep stable ordering here
        let _timing = self.timing_scope(tags::PARSE_ANNOTATIONS_MAIN);
        self.tree.sort_annotations();
    }

    /// Return true when there are annotations worth attaching.
    pub(crate) fn should_attach_annotations(&mut self) -> bool {
        self.token_stream.lex_to_end();
        self.has_comment_annotation_tokens() || self.has_blank_annotation_tokens()
    }

    /// Return true when there are comment or doc tokens.
    pub(crate) fn has_comment_annotation_tokens(&self) -> bool {
        self.token_stream.has_comment_annotation_tokens()
    }

    /// Return true when there are blank line tokens.
    pub(crate) fn has_blank_annotation_tokens(&self) -> bool {
        self.token_stream.has_blank_annotation_tokens()
    }

    /// Clean annotation tokens into their inner string, preserving intentional spacing.
    fn clean_annotation_string(
        &self,
        token_type: TokenType,
        tokens: &[InlineAnnotationToken],
        group_start_idx: usize,
        group_end_idx: usize,
        group_len: usize,
    ) -> String {
        debug_assert!(group_len > 0);

        // fast path for single-token groups
        if group_len == 1 {
            let token = tokens[group_start_idx];
            if token.token.token.ty == token_type {
                return self.clean_annotation_token_string(token_type, token.token.span);
            }
        }

        // clean each token in the group and join with explicit line breaks
        let mut cleaned_tokens = Vec::with_capacity(group_len);

        for token in &tokens[group_start_idx..=group_end_idx] {
            if token.token.token.ty != token_type {
                continue;
            }

            cleaned_tokens.push(self.clean_annotation_token_string(token_type, token.token.span));
        }

        cleaned_tokens.join("\n")
    }

    /// Clean one annotation token into its normalized text.
    fn clean_annotation_token_string(&self, token_type: TokenType, token_span: Span) -> String {
        // strip comment delimiters and capture the raw inner text
        let raw_str = self.file.get_span_str(token_span).unwrap_or_default();
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

        // trim block-style edge spaces before line normalization
        if matches!(
            token_type,
            TokenType::BlockComment | TokenType::DocBlockComment
        ) {
            if inner_str.starts_with(' ') {
                inner_str = &inner_str[1..];
            }

            while let Some(stripped) = inner_str.strip_suffix(' ') {
                inner_str = stripped;
            }
        }

        // normalize lines based on comment kind rules
        match token_type {
            TokenType::LineComment | TokenType::DocLineComment => {
                if inner_str.contains('\n') {
                    let mut cleaned = String::with_capacity(inner_str.len());
                    for (index, line) in inner_str.lines().enumerate() {
                        if index > 0 {
                            cleaned.push('\n');
                        }
                        let line = line.strip_prefix(' ').unwrap_or(line);
                        cleaned.push_str(line);
                    }
                    cleaned
                } else {
                    inner_str.strip_prefix(' ').unwrap_or(inner_str).to_owned()
                }
            }
            TokenType::BlockComment | TokenType::DocBlockComment => {
                if inner_str.trim().is_empty() {
                    String::new()
                } else {
                    let mut cleaned = String::with_capacity(inner_str.len());
                    for (index, line) in inner_str.split('\n').enumerate() {
                        if index > 0 {
                            cleaned.push('\n');
                        }

                        // strip optional asterisk prefixes and leading spaces
                        let mut line = if let Some((pos, ch)) =
                            line.char_indices().find(|&(_, ch)| ch != ' ')
                            && ch == '*'
                        {
                            let mut line_str = &line[pos + ch.len_utf8()..];
                            if line_str.starts_with(' ') {
                                line_str = &line_str[1..];
                            }
                            line_str
                        } else {
                            line.strip_prefix(' ').unwrap_or(line)
                        };

                        // drop lines with only spaces, otherwise trim trailing spaces
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
            _ => unreachable!(),
        }
    }
}
#[cfg(test)]
mod tests {
    use destack_ast::{
        Annotation, AnnotationPosition, Argument, BinaryOperator, Blank, Block, BlockFormat,
        Comment, CommentStyle, Declaration, DeclarationAbstraction, DeclarationDescriptor,
        Declarator, Decorator, Doc, DocStyle, Expression, FunctionKind, FunctionMode, Key, Member,
        Name, Parameter, TypeKind, TypeLiteral,
    };
    use destack_source::LanguageType;

    use crate::{TestParser, assert_expression_path, assert_node, assert_path, assert_string};

    /// Empty file with only a line comment should produce a Stub with the comment attached.
    #[test]
    fn test_attach_comment_to_stub_in_empty_file() {
        let mut test = TestParser::new("// just a comment");
        let mut parser = test.prepare();
        let expressions = parser.parse();

        // should have one Stub expression
        assert_eq!(expressions.len(), 1);
        assert_node!(parser.tree, expressions[0], Expression::Stub => {});

        // the comment should be attached to the Stub as infix (inside the "empty" file)
        let annotations = parser.tree.get_annotations(expressions[0].id);
        assert_eq!(annotations.len(), 1);
        assert_node!(parser.tree, annotations[0], Annotation::Comment { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockInfix);
            assert_node!(parser.tree, *node, Comment { string, style } => {
                assert_string!(parser, *string, "just a comment");
                assert_eq!(*style, CommentStyle::Slash);
            });
        });
    }

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
        parser.finish_annotations();

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

    /// Decorator annotations should be parsed around any block.
    #[test]
    fn test_attach_decorator_to_function() {
        let mut test = TestParser::new(
            r"@foo
function foo() { }",
        );
        let mut parser = test.prepare();
        let expressions = parser.parse();

        // function foo() { }
        let decorators = parser.tree.get_nodes::<Decorator>();
        assert_eq!(decorators.len(), 1, "decorators: {decorators:?}");
        assert_eq!(expressions.len(), 1);
        let annotations = parser.tree.get_annotations(expressions[0].id);
        assert_eq!(annotations.len(), 1, "annotations: {annotations:?}",);
        // @foo
        assert_node!(parser.tree, annotations[0], Annotation::Decorator { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPrefix);
            assert_node!(parser.tree, *node, Decorator { expression } => {
                assert_expression_path!(parser, parser.tree.get(*expression), "foo");
            });
        });
    }

    /// Decorators on runtime parameters should be attached to the parameter node.
    #[test]
    fn test_attach_decorator_to_parameter() {
        let mut test = TestParser::new("function demo(@if(true) value: number): void { }");
        let mut parser = test.prepare();
        let expressions = parser.parse();

        // function demo
        assert_eq!(expressions.len(), 1);
        let expression_id = parser.unwrap_statement_expression(expressions[0]);
        assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, .. } => {
                // value: number
                assert_eq!(signature.dynamic_parameters.len(), 1);
                let parameter_id = signature.dynamic_parameters[0];
                let annotations = parser.tree.get_annotations(parameter_id.id);
                assert_eq!(annotations.len(), 1);

                // @if(true)
                assert_node!(parser.tree, annotations[0], Annotation::Decorator { node, position } => {
                    assert_eq!(*position, AnnotationPosition::BlockPrefix);
                    assert_node!(parser.tree, *node, Decorator { expression } => {
                        assert_node!(parser.tree, *expression, Expression::Call { left, dynamic_arguments, .. } => {
                            assert_expression_path!(parser, parser.tree.get(*left), "if");
                            assert_eq!(dynamic_arguments.len(), 1);
                        });
                    });
                });
            });
        });
    }

    /// Decorators on call arguments should be attached to the argument node.
    #[test]
    fn test_attach_decorator_to_call_argument() {
        let mut test = TestParser::new(
            r"function call(value: number): void { }
call(@if(true) 1);",
        );
        let mut parser = test.prepare();
        let expressions = parser.parse();

        // call(@if(true) 1)
        assert_eq!(expressions.len(), 2);
        let call_expression_id = parser.unwrap_statement_expression(expressions[1]);
        assert_node!(parser.tree, call_expression_id, Expression::Call { dynamic_arguments, .. } => {
            // @if(true) 1
            assert_eq!(dynamic_arguments.len(), 1);
            let argument_id = dynamic_arguments[0];
            let annotations = parser.tree.get_annotations(argument_id.id);
            assert_eq!(annotations.len(), 1);

            // @if(true)
            assert_node!(parser.tree, annotations[0], Annotation::Decorator { node, position } => {
                assert_eq!(*position, AnnotationPosition::BlockPrefix);
                assert_node!(parser.tree, *node, Decorator { expression } => {
                    assert_node!(parser.tree, *expression, Expression::Call { left, dynamic_arguments, .. } => {
                        assert_expression_path!(parser, parser.tree.get(*left), "if");
                        assert_eq!(dynamic_arguments.len(), 1);
                    });
                });
            });
        });
    }

    /// Decorators after export modifiers should attach to the exported declaration expression.
    #[test]
    fn test_attach_decorator_after_export_modifier() {
        let mut test = TestParser::new_with_options(
            "export default @after abstract class Foo { }",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        let expressions = parser.parse();

        // export default @after abstract class Foo { }
        assert_eq!(expressions.len(), 1);
        let expression_id = parser.unwrap_statement_expression(expressions[0]);
        assert_node!(parser.tree, expression_id, Expression::Export { items, .. } => {
            assert_eq!(items.len(), 1);

            // default value
            let value_id = parser
                .tree
                .get(items[0])
                .value
                .expect("expected export default value");
            assert_node!(parser.tree, value_id, Expression::Declaration(_) => {});

            // @after
            let annotations = parser.tree.get_annotations(value_id.id);
            assert_eq!(annotations.len(), 1);
            assert_node!(parser.tree, annotations[0], Annotation::Decorator { node, position } => {
                assert_eq!(*position, AnnotationPosition::BlockPrefix);
                assert_node!(parser.tree, *node, Decorator { expression } => {
                    assert_expression_path!(parser, parser.tree.get(*expression), "after");
                });
            });
        });
    }

    /// Top level export decorators should attach to export or class targets as expected.
    #[test]
    fn test_parse_javascript_decorator_export_top_level_sequences() {
        let mut test = TestParser::new_with_options(
            r"@decorator
export class Foo { }
@first.field @second @(() => decorator)()
export class Bar {}
@before
export @after class Foo { }
@before
export abstract class Foo { }
@before
export @after abstract class Foo { }",
            LanguageType::JavaScript,
        );
        let mut parser = test.prepare();
        let expressions = parser.parse();

        // @decorator ... export class ...
        assert!(
            parser.errors.is_empty(),
            "unexpected parser errors: {:?}",
            parser.errors
        );
        assert_eq!(expressions.len(), 5);

        // expected declaration metadata
        let expected_names = ["Foo", "Bar", "Foo", "Foo", "Foo"];
        let expected_abstractions = [
            DeclarationAbstraction::Concrete,
            DeclarationAbstraction::Concrete,
            DeclarationAbstraction::Concrete,
            DeclarationAbstraction::Abstract,
            DeclarationAbstraction::Abstract,
        ];
        let expected_decorators = [
            vec!["decorator"],
            vec!["first.field", "second", "<<call>>"],
            vec!["before", "after"],
            vec!["before"],
            vec!["before", "after"],
        ];

        // per declaration assertions
        for index in 0..expressions.len() {
            // class declaration
            let expression_id = parser.unwrap_statement_expression(expressions[index]);
            assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
                assert_node!(parser.tree, *declaration_id, Declaration::Class { descriptor, .. } => {
                    assert!(descriptor.export.is_some());
                    assert_eq!(descriptor.abstraction, expected_abstractions[index]);
                    assert_string!(parser, descriptor.name.unwrap().string(), expected_names[index]);
                });
            });

            // decorator list
            let annotations = parser.tree.get_annotations(expression_id.id);
            assert_eq!(annotations.len(), expected_decorators[index].len());
            for annotation_index in 0..annotations.len() {
                let expected_decorator = expected_decorators[index][annotation_index];
                assert_node!(parser.tree, annotations[annotation_index], Annotation::Decorator { node, .. } => {
                    assert_node!(parser.tree, *node, Decorator { expression } => {
                        if expected_decorator == "<<call>>" {
                            assert_node!(parser.tree, *expression, Expression::Call { .. } => {});
                        } else {
                            assert_expression_path!(
                                parser,
                                parser.tree.get(*expression),
                                expected_decorator
                            );
                        }
                    });
                });
            }
        }
    }

    /// Decorator static arguments should preserve generic lambda details.
    #[test]
    fn test_attach_decorator_with_static_arguments() {
        let mut test = TestParser::new(
            r"@foo<<T>(value: T) => T>()
function foo() { }",
        );
        let mut parser = test.prepare();
        let expressions = parser.parse();

        // function foo() { }
        assert_eq!(expressions.len(), 1);
        let annotations = parser.tree.get_annotations(expressions[0].id);
        assert_eq!(annotations.len(), 1);

        // @foo<<T>(value: T) => T>()
        assert_node!(parser.tree, annotations[0], Annotation::Decorator { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPrefix);
            assert_node!(parser.tree, *node, Decorator { expression } => {
                assert_node!(parser.tree, *expression, Expression::Call { left, static_arguments, dynamic_arguments, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "foo");
                    let static_arguments = static_arguments.as_ref().expect("expected static arguments");
                    assert_eq!(static_arguments.len(), 1);
                    assert_node!(parser.tree, static_arguments[0], Argument::Positional { value, .. } => {
                        assert_node!(parser.tree, *value, Expression::Declaration(declaration_id) => {
                            assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, body, .. } => {
                                assert_eq!(signature.kind, FunctionKind::Lambda);
                                let generics = signature.generics.as_ref().expect("expected generics");
                                let static_parameters = generics
                                    .static_parameters
                                    .as_ref()
                                    .expect("expected static parameters");
                                assert_eq!(static_parameters.len(), 1);
                                assert_node!(parser.tree, static_parameters[0], Parameter::Named { name, ty, .. } => {
                                    assert_string!(parser, *name, "T");
                                    assert!(ty.is_none());
                                });
                                assert_eq!(signature.dynamic_parameters.len(), 1);
                                assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty, .. } => {
                                    assert_string!(parser, *name, "value");
                                    assert_expression_path!(parser, parser.tree.get(ty.unwrap()), "T");
                                });

                                // match return type or body for the arrow target
                                if let Some(return_type) = signature.return_type {
                                    assert_expression_path!(parser, parser.tree.get(return_type), "T");
                                } else {
                                    let body = body.expect("expected body expression");
                                    assert_expression_path!(parser, parser.tree.get(body), "T");
                                }
                            });
                        });
                    });
                    assert!(dynamic_arguments.is_empty());
                });
            });
        });
    }

    /// Decorator call chains should be parsed as a call chain.
    #[test]
    fn test_attach_decorator_with_call_chain() {
        let mut test = TestParser::new(
            r"@joiful.string().guid().required()
function foo() { }",
        );
        let mut parser = test.prepare();
        let expressions = parser.parse();

        // function foo() { }
        assert_eq!(expressions.len(), 1);
        let annotations = parser.tree.get_annotations(expressions[0].id);
        assert_eq!(annotations.len(), 1);

        // @joiful.string().guid().required()
        assert_node!(parser.tree, annotations[0], Annotation::Decorator { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPrefix);
            assert_node!(parser.tree, *node, Decorator { expression } => {
                assert_node!(parser.tree, *expression, Expression::Call { left, dynamic_arguments, .. } => {
                    assert!(dynamic_arguments.is_empty());
                    assert_node!(parser.tree, *left, Expression::Member { left: required_left, name, static_arguments } => {
                        assert_string!(parser, *name, "required");
                        assert!(static_arguments.is_none());
                        assert_node!(parser.tree, *required_left, Expression::Call { left, dynamic_arguments, .. } => {
                            assert!(dynamic_arguments.is_empty());
                            assert_node!(parser.tree, *left, Expression::Member { left: guid_left, name, static_arguments } => {
                                assert_string!(parser, *name, "guid");
                                assert!(static_arguments.is_none());
                                assert_node!(parser.tree, *guid_left, Expression::Call { left, dynamic_arguments, .. } => {
                                    assert!(dynamic_arguments.is_empty());
                                    assert_expression_path!(parser, parser.tree.get(*left), "joiful.string");
                                });
                            });
                        });
                    });
                });
            });
        });
    }

    /// Parse a decorator with static arguments.
    #[test]
    fn test_attach_decorator_with_static_arguments_simple() {
        let mut test = TestParser::new_with_options(
            r"@foo<T>()
function foo() { }",
            LanguageType::Destack,
        );
        let mut parser = test.prepare();
        let expressions = parser.parse();

        // function foo() { }
        assert_eq!(expressions.len(), 1);
        let annotations = parser.tree.get_annotations(expressions[0].id);
        assert_eq!(annotations.len(), 1);

        // @foo<T>()
        assert_node!(parser.tree, annotations[0], Annotation::Decorator { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPrefix);
            assert_node!(parser.tree, *node, Decorator { expression } => {
                assert_node!(parser.tree, *expression, Expression::Call { left, static_arguments, dynamic_arguments, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "foo");
                    let static_arguments = static_arguments.as_ref().expect("expected static arguments");
                    assert_eq!(static_arguments.len(), 1);
                    assert_node!(parser.tree, static_arguments[0], Argument::Positional { value, .. } => {
                        assert_expression_path!(parser, parser.tree.get(*value), "T");
                    });
                    assert!(dynamic_arguments.is_empty());
                });
            });
        });
    }

    /// Parse a decorator with a generic function type argument.
    #[test]
    fn test_attach_decorator_with_shift_left_static_arguments() {
        let mut test = TestParser::new_with_options(
            r"@f<<T>(v: T) => void>()
class Foo {}",
            LanguageType::Destack,
        );
        let mut parser = test.prepare();
        let expressions = parser.parse();

        // @f<<T>(v: T) => void>()
        let decorators = parser.tree.get_nodes::<Decorator>();
        assert_eq!(decorators.len(), 1, "decorators: {decorators:?}");
        let decorator_span = parser.tree.get_span(decorators[0]);
        assert_eq!(
            parser.file.span_str(decorator_span),
            "@f<<T>(v: T) => void>()",
        );

        // class Foo {}
        assert_eq!(expressions.len(), 1);
        let annotations = parser.tree.get_annotations(expressions[0].id);
        assert_eq!(annotations.len(), 1, "annotations: {annotations:?}");

        // <<T>(v: T) => void
        assert_node!(parser.tree, annotations[0], Annotation::Decorator { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPrefix);
            assert_node!(parser.tree, *node, Decorator { expression } => {
                assert_node!(parser.tree, *expression, Expression::Call { left, static_arguments, dynamic_arguments, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "f");
                    let static_arguments = static_arguments.as_ref().expect("expected static arguments");
                    assert_eq!(static_arguments.len(), 1);
                    assert_node!(parser.tree, static_arguments[0], Argument::Positional { value, .. } => {
                        assert_node!(parser.tree, *value, Expression::Declaration(declaration_id) => {
                            assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, body, .. } => {
                                assert_eq!(signature.kind, FunctionKind::Lambda);
                                assert!(body.is_none());
                                assert_node!(parser.tree, signature.return_type.expect("expected return type"), Expression::TypeLiteral(TypeLiteral::Void));
                                let generics = signature.generics.as_ref().expect("expected generics");
                                let static_parameters = generics.static_parameters.as_ref().expect("expected static parameters");
                                assert_eq!(static_parameters.len(), 1);
                                assert_node!(parser.tree, static_parameters[0], Parameter::Named { name, .. } => {
                                    assert_string!(parser, *name, "T");
                                });
                            });
                        });
                    });
                    assert!(dynamic_arguments.is_empty());
                });
            });
        });
    }

    /// Decorator annotations on accessors should attach to the member node.
    #[test]
    fn test_attach_decorator_to_accessor_member() {
        let mut test = TestParser::new(
            r#"
class Box {
    @if(true)
    get value(): int32 {
        return 1;
    }

    @if(true)
    set value(next: int32) {
        let _ = next;
    }
}
"#,
        );
        let mut parser = test.prepare();
        let expressions = parser.parse();

        // class Box
        assert_eq!(expressions.len(), 1);
        let expression_id = parser.unwrap_statement_expression(expressions[0]);
        assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Class { members, .. } => {
                assert_eq!(members.len(), 2);

                // get value(): int32
                assert_node!(parser.tree, members[0], Member::Method { signature, .. } => {
                    assert_eq!(signature.mode, Some(FunctionMode::Getter));
                });
                let getter_annotations = parser.tree.get_annotations(members[0].id);
                assert_eq!(getter_annotations.len(), 1);
                assert_node!(parser.tree, getter_annotations[0], Annotation::Decorator { node, position } => {
                    assert_eq!(*position, AnnotationPosition::BlockPrefix);
                    assert_node!(parser.tree, *node, Decorator { expression } => {
                        assert_node!(parser.tree, *expression, Expression::Call { left, dynamic_arguments, .. } => {
                            assert_expression_path!(parser, parser.tree.get(*left), "if");
                            assert_eq!(dynamic_arguments.len(), 1);
                        });
                    });
                });

                // set value(next: int32)
                assert_node!(parser.tree, members[1], Member::Method { signature, .. } => {
                    assert_eq!(signature.mode, Some(FunctionMode::Setter));
                });
                let setter_annotations = parser.tree.get_annotations(members[1].id);
                assert_eq!(setter_annotations.len(), 1);
                assert_node!(parser.tree, setter_annotations[0], Annotation::Decorator { node, position } => {
                    assert_eq!(*position, AnnotationPosition::BlockPrefix);
                    assert_node!(parser.tree, *node, Decorator { expression } => {
                        assert_node!(parser.tree, *expression, Expression::Call { left, dynamic_arguments, .. } => {
                            assert_expression_path!(parser, parser.tree.get(*left), "if");
                            assert_eq!(dynamic_arguments.len(), 1);
                        });
                    });
                });
            });
        });
    }

    /// Decorator annotations on enum and its variants should be attached correctly.
    #[test]
    fn test_attach_decorators_to_enum_and_variants() {
        let mut test = TestParser::new(
            r#"
@description("The status of an event.")
export enum EventStatus {
    @default
    @description("The event is a draft.")
    Draft,

    @description("The event is upcoming.")
    Upcoming,

    @description("The event is cancelled.")
    Cancelled,
}"#,
        );
        let mut parser = test.prepare();
        let expressions = parser.parse();

        // @description("The status of an event.")
        assert_eq!(expressions.len(), 1);
        let enum_annotations = parser.tree.get_annotations(expressions[0].id);
        assert_eq!(enum_annotations.len(), 1);
        assert_node!(parser.tree, enum_annotations[0], Annotation::Decorator { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPrefix);
            assert_node!(parser.tree, *node, Decorator { expression } => {
                assert_node!(parser.tree, *expression, Expression::Call { left, dynamic_arguments, static_arguments, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "description");
                    assert!(static_arguments.is_none());
                    assert_eq!(dynamic_arguments.len(), 1);
                });
            });
        });

        // EventStatus
        assert_node!(parser.tree, expressions[0], Expression::Declaration(decl) => {
            assert_node!(parser.tree, *decl, Declaration::Enum { fields, .. } => {
                assert_eq!(fields.len(), 3);

                // Draft: @default, @description
                let draft_annotations = parser.tree.get_annotations(fields[0].id);
                assert_eq!(draft_annotations.len(), 2);
                assert_node!(parser.tree, draft_annotations[0], Annotation::Decorator { node, .. } => {
                    assert_node!(parser.tree, *node, Decorator { expression } => {
                        assert_expression_path!(parser, parser.tree.get(*expression), "default");
                    });
                });
                assert_node!(parser.tree, draft_annotations[1], Annotation::Decorator { node, .. } => {
                    assert_node!(parser.tree, *node, Decorator { expression } => {
                        assert_node!(parser.tree, *expression, Expression::Call { left, dynamic_arguments, .. } => {
                            assert_expression_path!(parser, parser.tree.get(*left), "description");
                            assert_eq!(dynamic_arguments.len(), 1);
                        });
                    });
                });

                // Upcoming: blank + @description
                let upcoming_annotations = parser.tree.get_annotations(fields[1].id);
                assert_eq!(upcoming_annotations.len(), 2);
                assert_node!(parser.tree, upcoming_annotations[0], Annotation::Blank { .. } => {});
                assert_node!(parser.tree, upcoming_annotations[1], Annotation::Decorator { node, .. } => {
                    assert_node!(parser.tree, *node, Decorator { expression } => {
                        assert_node!(parser.tree, *expression, Expression::Call { left, dynamic_arguments, .. } => {
                            assert_expression_path!(parser, parser.tree.get(*left), "description");
                            assert_eq!(dynamic_arguments.len(), 1);
                        });
                    });
                });

                // Cancelled: blank + @description
                let cancelled_annotations = parser.tree.get_annotations(fields[2].id);
                assert_eq!(cancelled_annotations.len(), 2);
                assert_node!(parser.tree, cancelled_annotations[0], Annotation::Blank { .. } => {});
                assert_node!(parser.tree, cancelled_annotations[1], Annotation::Decorator { node, .. } => {
                    assert_node!(parser.tree, *node, Decorator { expression } => {
                        assert_node!(parser.tree, *expression, Expression::Call { left, dynamic_arguments, .. } => {
                            assert_expression_path!(parser, parser.tree.get(*left), "description");
                            assert_eq!(dynamic_arguments.len(), 1);
                        });
                    });
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

    /// Trailing comments on chained paths attach to the path expression.
    #[test]
    fn test_attach_trailing_comment_to_chain_path() {
        let mut test = TestParser::new(
            r"foo
  .getParameters /* trailing comment */
  ?.();",
        );
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression(parser.options).unwrap();
        parser.finish_annotations();

        // foo.getParameters?.()
        let mut current = expr_id;
        let mut path_id = None;
        loop {
            match parser.tree.get(current) {
                Expression::Path { .. } => {
                    path_id = Some(current);
                    break;
                }
                Expression::Call { left, .. }
                | Expression::Index { left, .. }
                | Expression::Maybe { left, .. }
                | Expression::Must { left, .. } => current = *left,
                _ => break,
            }
        }

        // foo /* trailing comment */
        let path_id = path_id.expect("expected a chained path expression");
        let annotations = parser.tree.get_annotations(path_id.id);
        assert_eq!(annotations.len(), 1);
        assert_node!(
            parser.tree,
            annotations[0],
            Annotation::Comment { node, position } => {
                assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
                assert_node!(parser.tree, *node, Comment { string, style } => {
                    assert_string!(parser, *string, "trailing comment");
                    assert_eq!(*style, CommentStyle::Star);
                });
            }
        );
    }

    /// Line comments between chain segments attach to the following member.
    #[test]
    fn test_attach_line_comment_before_chain_member() {
        let mut test = TestParser::new(
            r"Promise.all(writeIconFiles)
  // TO DO -- END
  .then(() => writeRegistry())",
        );
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression(parser.options).unwrap();
        parser.finish_annotations();

        // Promise.all(...).then(...)
        let mut current = expr_id;
        let mut member_id = None;
        loop {
            match parser.tree.get(current) {
                Expression::Member { name, left, .. } => {
                    let name_str = parser.strings.get(*name);
                    if name_str == "then" {
                        member_id = Some(current);
                        break;
                    }
                    current = *left;
                }
                Expression::Call { left, .. }
                | Expression::Index { left, .. }
                | Expression::Maybe { left, .. }
                | Expression::Must { left, .. } => current = *left,
                _ => break,
            }
        }

        // // TO DO -- END
        let member_id = member_id.expect("expected .then member");
        let annotations = parser.tree.get_annotations(member_id.id);
        assert_eq!(annotations.len(), 1);
        assert_node!(
            parser.tree,
            annotations[0],
            Annotation::Comment { node, position } => {
                assert_eq!(*position, AnnotationPosition::BlockPrefix);
                assert_node!(parser.tree, *node, Comment { string, style } => {
                    assert_string!(parser, *string, "TO DO -- END");
                    assert_eq!(*style, CommentStyle::Slash);
                });
            }
        );
    }

    /// Blank lines after a chained statement attach to the next statement.
    #[test]
    fn test_attach_blank_after_chained_statement() {
        let mut test = TestParser::new(
            r"Promise.all(writeIconFiles)
  // TO DO -- END
  .then(() => writeRegistry())

Promise.all(writeIconFiles)",
        );
        let mut parser = test.prepare();
        let expressions = parser.parse();

        // Promise.all(...); Promise.all(...)
        assert_eq!(expressions.len(), 2);

        // blank between top level calls
        let mut blank_parent = None;
        for (node_id, annotations) in parser.tree.get_all_annotations() {
            let has_blank = annotations.iter().any(|annotation_id| {
                matches!(
                    parser.tree.get::<Annotation>(*annotation_id),
                    Annotation::Blank { .. }
                )
            });
            if has_blank {
                blank_parent = Some(*node_id);
                break;
            }
        }

        // second Promise.all(...)
        let blank_parent_id = blank_parent.expect("expected blank annotation");
        assert_eq!(blank_parent_id, expressions[1].id);
    }

    /// Inline comments between path segments attach as line postfix boundaries.
    #[test]
    fn test_attach_inline_comment_between_path_segments() {
        let mut test = TestParser::new(
            r"wow /* inline comment */
  .omg!",
        );
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression(parser.options).unwrap();
        parser.finish_annotations();

        // wow.omg!
        let mut current = expr_id;
        let mut path_id = None;
        loop {
            match parser.tree.get(current) {
                Expression::Path { .. } => {
                    path_id = Some(current);
                    break;
                }
                Expression::Call { left, .. }
                | Expression::Index { left, .. }
                | Expression::Maybe { left, .. }
                | Expression::Must { left, .. } => current = *left,
                _ => break,
            }
        }

        // wow /* inline comment */
        let path_id = path_id.expect("expected a chained path expression");
        let annotations = parser.tree.get_annotations(path_id.id);
        assert_eq!(annotations.len(), 1);
        assert_node!(
            parser.tree,
            annotations[0],
            Annotation::Comment { node, position } => {
                assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
                assert_node!(parser.tree, *node, Comment { string, style } => {
                    assert_string!(parser, *string, "inline comment");
                    assert_eq!(*style, CommentStyle::Star);
                });
            }
        );
    }

    /// Inline comments before a dot path segment attach as block infix annotations.
    #[test]
    fn test_attach_inline_comment_before_dot_member() {
        let mut test = TestParser::new("wow /* inline comment */ .omg");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression(parser.options).unwrap();
        parser.finish_annotations();

        // wow .omg
        let mut current = expr_id;
        loop {
            match parser.tree.get(current) {
                Expression::Path { .. } => break,
                Expression::Call { left, .. }
                | Expression::Index { left, .. }
                | Expression::Maybe { left, .. }
                | Expression::Must { left, .. } => current = *left,
                _ => panic!("expected a path expression"),
            }
        }

        // wow /* inline comment */
        let annotations = parser.tree.get_annotations(current.id);
        assert_eq!(annotations.len(), 1);
        assert_node!(
            parser.tree,
            annotations[0],
            Annotation::Comment { node, position } => {
                assert_eq!(*position, AnnotationPosition::BlockInfix);
                assert_node!(parser.tree, *node, Comment { string, style } => {
                    assert_string!(parser, *string, "inline comment");
                    assert_eq!(*style, CommentStyle::Star);
                });
            }
        );
    }

    /// Inline comments after operators attach to the following operand as line prefixes.
    #[test]
    fn test_attach_inline_comment_between_binary_operands() {
        let mut test = TestParser::new("a && /* keep */ b");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression(parser.options).unwrap();
        parser.finish_annotations();

        // a && b
        assert_node!(
            parser.tree,
            expr_id,
            Expression::Binary { right, .. } => {
                let annotations = parser.tree.get_annotations(right.id);
                assert_eq!(annotations.len(), 1);
                assert_node!(
                    parser.tree,
                    annotations[0],
                    Annotation::Comment { node, position } => {
                        assert_eq!(*position, AnnotationPosition::LinePrefix);
                        assert_node!(parser.tree, *node, Comment { string, style } => {
                            assert_string!(parser, *string, "keep");
                            assert_eq!(*style, CommentStyle::Star);
                        });
                    }
                );
            }
        );
    }

    /// Inline comments before call arguments attach as line prefixes.
    #[test]
    fn test_attach_inline_comment_before_call_argument() {
        let mut test = TestParser::new("foo(/* first */ a)");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression(parser.options).unwrap();
        parser.finish_annotations();

        // foo(a)
        assert_node!(parser.tree, expr_id, Expression::Call { dynamic_arguments, .. } => {
            assert_eq!(dynamic_arguments.len(), 1);
            let argument_id = dynamic_arguments[0];
            let annotations = parser.tree.get_annotations(argument_id.id);

            // /* first */ a
            let annotation_owner_id = if annotations.is_empty() {
                let argument = parser.tree.get(argument_id);
                let value_id = match argument {
                    Argument::Named { value, .. }
                    | Argument::Labeled { value, .. }
                    | Argument::Positional { value, .. }
                    | Argument::Spread { value, .. } => *value,
                };
                value_id.id
            } else {
                argument_id.id
            };

            // /* first */
            let annotations = parser.tree.get_annotations(annotation_owner_id);
            assert_eq!(annotations.len(), 1);
            assert_node!(
                parser.tree,
                annotations[0],
                Annotation::Comment { node, position } => {
                    assert_eq!(*position, AnnotationPosition::LinePrefix);
                    assert_node!(parser.tree, *node, Comment { string, style } => {
                        assert_string!(parser, *string, "first");
                        assert_eq!(*style, CommentStyle::Star);
                    });
                }
            );
        });
    }

    /// Multi-line suffix is attached to the previous node on the same line as a block postfix.
    #[test]
    fn test_attach_multi_line_postfix_to_expression() {
        let mut test = TestParser::new(
            r"let A = 1 /* line comment
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
            r"{
    /** some multiline
     * block comment
     * over multiple lines */
    let X = 1
}",
        );
        let mut parser = test.prepare();
        let block = parser.eat_block().unwrap();
        parser.finish_annotations();

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

    /// Block doc comments in declaration blocks should attach to the following function.
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
        let start = parser.mark();
        let interface_id = parser
            .eat_interface(
                &start,
                DeclarationDescriptor::default(),
                TypeKind::Structural,
            )
            .unwrap();
        parser.finish_annotations();

        // interface X
        assert_node!(parser.tree, interface_id, Declaration::Interface { members, .. } => {
            assert_eq!(members.len(), 2);

            // a(): A
            let annotations = parser.tree.get_annotations(members[0].id);
            assert_eq!(annotations.len(), 1);
            assert_node!(parser.tree, annotations[0], Annotation::Doc { node, position } => {
                assert_eq!(*position, AnnotationPosition::BlockPrefix);
                assert_node!(parser.tree, *node, Doc { string, style } => {
                    assert_eq!(*style, DocStyle::Star);
                    assert_string!(parser, *string, "Doc A");
                });
            });

            // b(): B
            let annotations = parser.tree.get_annotations(members[1].id);
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
        let start = parser.mark();
        let function = parser
            .eat_function(&start, DeclarationDescriptor::default(), false, false)
            .unwrap();
        parser.finish_annotations();

        // function foo()
        assert_node!(parser.tree, function, Declaration::Function { body: body_id, .. } => {
            assert_node!(parser.tree, body_id.unwrap(), Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { expressions, .. } => {
                    assert_eq!(expressions.len(), 3);

                    // function a(): A
                    assert_node!(parser.tree, expressions[0], Expression::Declaration(node) => {
                        assert_node!(parser.tree, *node, Declaration::Function { descriptor, .. } => {
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
                    assert_node!(parser.tree, expressions[1], Expression::Declaration(node) => {
                        assert_node!(parser.tree, *node, Declaration::Function { descriptor, .. } => {
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
                    assert_node!(parser.tree, expressions[2], Expression::Declaration(node) => {
                        assert_node!(parser.tree, *node, Declaration::Function { descriptor, .. } => {
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
        parser.finish_annotations();

        // let X = A && B
        assert_eq!(expressions.len(), 1);

        // A && B
        assert_node!(parser.tree, expressions[0], Expression::Statement(statement_id) => {
            assert_node!(parser.tree, *statement_id, Expression::Let { declarators, .. } => {
                assert_eq!(declarators.len(), 1);
                assert_node!(parser.tree, declarators[0], Declarator { value, .. } => {
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
        parser.finish_annotations();

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
        let mut test = TestParser::new(
            r"

let A = 1

let B = 2

",
        );
        let mut parser = test.prepare();
        let expressions = parser.eat_block_body(BlockFormat::Implicit).unwrap();
        parser.finish_annotations();

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

        let start = parser.mark();
        let function = parser
            .eat_function(&start, DeclarationDescriptor::default(), false, false)
            .unwrap();
        parser.finish_annotations();

        // (annotation should be infix to innermost node, i.e. the block)
        assert_node!(parser.tree, function, Declaration::Function { body: body_id, .. } => {
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
        parser.finish_annotations();

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
        assert_node!(parser.tree, expressions[0], Expression::Declaration(node) => {
            assert_node!(parser.tree, *node, Declaration::Struct { members, .. } => {
                // a: int32
                assert_eq!(members.len(), 1);
                assert_node!(parser.tree, members[0], Member::Field { key: Some(Key::Name(Name::Identifier(name))), .. } => {
                    assert_string!(parser, *name, "a");
                    let annotations = parser.tree.get_annotations(members[0].id);
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

    /// Nested namespace declarations should keep prefix comments on each level.
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
        parser.finish_annotations();

        // Outer namespace
        assert_eq!(expressions.len(), 1);
        assert_node!(parser.tree, expressions[0], Expression::Declaration(node) => {
            // Outer comment
            let outer_annotations = parser.tree.get_annotations(expressions[0].id);
            assert_eq!(outer_annotations.len(), 1);
            assert_node!(parser.tree, outer_annotations[0], Annotation::Comment { node, position } => {
                assert_eq!(*position, AnnotationPosition::BlockPrefix);
                assert_node!(parser.tree, *node, Comment { string, style } => {
                    assert_string!(parser, *string, "Outer comment");
                    assert_eq!(*style, CommentStyle::Slash);
                });
            });

            // Outer
            assert_node!(parser.tree, *node, Declaration::Namespace { descriptor, expressions: outer_expressions, .. } => {
                assert_string!(parser, descriptor.name.unwrap().string(), "Outer");
                assert_eq!(outer_expressions.len(), 1);

                // Middle comment
                let middle_expression_id = outer_expressions[0];
                let middle_annotations = parser.tree.get_annotations(middle_expression_id.id);
                assert_eq!(middle_annotations.len(), 1);
                assert_node!(parser.tree, middle_annotations[0], Annotation::Comment { node, position } => {
                    assert_eq!(*position, AnnotationPosition::BlockPrefix);
                    assert_node!(parser.tree, *node, Comment { string, style } => {
                        assert_string!(parser, *string, "Middle comment");
                        assert_eq!(*style, CommentStyle::Slash);
                    });
                });

                // Middle
                assert_node!(parser.tree, middle_expression_id, Expression::Declaration(node) => {
                    assert_node!(parser.tree, *node, Declaration::Namespace { descriptor, expressions: middle_expressions, .. } => {
                        assert_string!(parser, descriptor.name.unwrap().string(), "Middle");
                        assert_eq!(middle_expressions.len(), 1);

                        // Inner comment
                        let inner_expression_id = middle_expressions[0];
                        let inner_annotations = parser.tree.get_annotations(inner_expression_id.id);
                        assert_eq!(inner_annotations.len(), 1);
                        assert_node!(parser.tree, inner_annotations[0], Annotation::Comment { node, position } => {
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

    /// Successive labelled blocks should keep surrounding comments attached to the right nodes.
    #[test]
    fn test_attach_multiple_comments_around_expression_in_successive_blocks() {
        let mut test = TestParser::new(
            r"{
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
        parser.finish_annotations();

        // { a: { .. } b: { .. } }
        assert_node!(parser.tree, block, Block { expressions, .. } => {
            assert_eq!(expressions.len(), 2);

            // a (labelled block)
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

            // a: { .. } - now Expression::Labelled
            assert_node!(parser.tree, a, Expression::Labelled { label: _, body } => {
                assert_node!(parser.tree, *body, Expression::Block (block_id) => {
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
            });

            // b (labelled block)
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

            // b: { .. } - now Expression::Labelled
            assert_node!(parser.tree, b, Expression::Labelled { label: _, body } => {
                assert_node!(parser.tree, *body, Expression::Block (block_id) => {
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

    /// Blank lines between array elements should be attached as block prefix annotations.
    #[test]
    fn test_attach_blanks_in_array_elements() {
        let mut test = TestParser::new(
            r"[
    1,

    2,
]",
        );
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression(parser.options).unwrap();
        parser.finish_annotations();

        assert_node!(parser.tree, expr_id, Expression::ArrayExpression { elements } => {
            assert_eq!(elements.len(), 2);

            // first element has no annotations
            let first_annotations = parser.tree.get_annotations(elements[0].id);
            assert!(first_annotations.is_empty(), "first element should have no annotations");

            // second element has blank prefix annotation
            let second_annotations = parser.tree.get_annotations(elements[1].id);
            assert_eq!(second_annotations.len(), 1, "second element should have one blank annotation");
            assert_node!(parser.tree, second_annotations[0], Annotation::Blank { node, position } => {
                assert_eq!(*position, AnnotationPosition::BlockPrefix);
                assert_node!(parser.tree, *node, Blank { lines } => {
                    assert_eq!(*lines, 1);
                });
            });
        });
    }
}
