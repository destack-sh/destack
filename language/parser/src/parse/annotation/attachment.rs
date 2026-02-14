use crate::parse::timing::tags;
use crate::{ParseResult, Parser};
use destack_ast::{
    Annotation, AnnotationPosition, Blank, Comment, CommentStyle, Decorator, Doc, DocStyle, Key,
    LocalNodeId, Member, NodeType, Property, TokenSpan, TokenType,
};
use destack_source::Span;

use super::{
    AnnotationBoundaryKind, BlankBoundaryKind, DotBoundaryKind, LeadingAnnotationKind,
    TrailingAnnotationKind,
};

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
    /// Semantic token index that owns or equals this annotation token.
    token_index: usize,
    /// Optional index in the global side-token stream.
    side_index: Option<usize>,
    /// Annotation token payload.
    token: TokenSpan,
}

/// Boundary context used for inline trailing annotation positioning.
#[derive(Debug, Copy, Clone)]
struct InlineTrailingBoundaryContext {
    /// The semantic token type at the parser cursor.
    current_token_type: TokenType,
    /// Whether there is a line terminator before the current token.
    has_line_break_before_current: bool,
    /// Whether this token boundary prefers boundary style postfix attachment.
    is_boundary: bool,
    /// The previous semantic token type, skipping newline tokens.
    previous_non_newline_token_type: Option<TokenType>,
}

/// Remap metadata for computed key expressions whose outer prefix belongs to the owner node.
#[derive(Debug, Copy, Clone)]
struct ComputedKeyOwnerCandidate {
    /// Property or member node id that owns the computed key wrapper.
    owner_node_id: u32,
    /// Span of the full computed key wrapper, including brackets.
    key_wrapper_span: Span,
}

impl Parser {
    /// Build computed key owner metadata keyed by key expression id.
    fn collect_computed_key_owner_candidates(&self) -> Vec<Vec<ComputedKeyOwnerCandidate>> {
        // allocate one slot per global node id
        let total_nodes = self.tree.next_id() as usize;
        let mut owners = vec![Vec::new(); total_nodes];

        // collect property and member computed key owners
        let mut node_id = 0usize;
        while node_id < total_nodes {
            let global_id = node_id as u32;
            match self.tree.get_node_type(global_id) {
                NodeType::Property => {
                    let property_id = LocalNodeId::<Property>::new(global_id);
                    let property = self.tree.get(property_id);
                    let key_expression_id = match property {
                        Property::Field {
                            key: Some(Key::Expression(key_expression_id)),
                            ..
                        } => Some(*key_expression_id),
                        _ => None,
                    };
                    if let Some(key_expression_id) = key_expression_id {
                        let key_wrapper_span = self
                            .tree
                            .get_main_span(property_id)
                            .unwrap_or_else(|| self.tree.get_span(property_id));
                        owners[key_expression_id.id as usize].push(ComputedKeyOwnerCandidate {
                            owner_node_id: global_id,
                            key_wrapper_span,
                        });
                    }
                }
                NodeType::Member => {
                    let member_id = LocalNodeId::<Member>::new(global_id);
                    let member = self.tree.get(member_id);
                    let key_expression_id = match member {
                        Member::Field {
                            key: Some(Key::Expression(key_expression_id)),
                            ..
                        }
                        | Member::Method {
                            key: Some(Key::Expression(key_expression_id)),
                            ..
                        } => Some(*key_expression_id),
                        _ => None,
                    };
                    if let Some(key_expression_id) = key_expression_id {
                        let key_wrapper_span = self
                            .tree
                            .get_main_span(member_id)
                            .unwrap_or_else(|| self.tree.get_span(member_id));
                        owners[key_expression_id.id as usize].push(ComputedKeyOwnerCandidate {
                            owner_node_id: global_id,
                            key_wrapper_span,
                        });
                    }
                }
                _ => {}
            }

            node_id += 1;
        }

        owners
    }

    /// Return true when an annotation should be promoted from a computed key expression to its owner.
    #[inline(always)]
    fn annotation_needs_computed_key_owner(
        annotation: &Annotation,
        annotation_span: Span,
        key_wrapper_span: Span,
    ) -> bool {
        // never rewrite decorators here
        if matches!(annotation, Annotation::Decorator { .. }) {
            return false;
        }

        // comments before `[` belong to the property or member owner
        annotation_span.file == key_wrapper_span.file
            && annotation_span.end <= key_wrapper_span.start
    }

    /// Normalize annotation owners for computed key wrappers.
    fn normalize_computed_key_annotation_owners(&mut self) {
        // collect computed key owner mappings once
        let owner_candidates = self.collect_computed_key_owner_candidates();
        if !owner_candidates
            .iter()
            .any(|candidates| !candidates.is_empty())
        {
            return;
        }

        // skip remap when no attached annotation actually crosses a computed key wrapper boundary
        let has_computed_key_owner_remap =
            self.tree
                .get_all_annotations()
                .iter()
                .any(|(target_node_id, annotation_ids)| {
                    let Some(candidates) = owner_candidates.get(*target_node_id as usize) else {
                        return false;
                    };
                    if candidates.is_empty() {
                        return false;
                    }

                    annotation_ids.iter().any(|annotation_id| {
                        let annotation = self.tree.get::<Annotation>(*annotation_id);
                        let annotation_span = self.tree.get_span(*annotation_id);
                        candidates.iter().any(|candidate| {
                            Self::annotation_needs_computed_key_owner(
                                annotation,
                                annotation_span,
                                candidate.key_wrapper_span,
                            )
                        })
                    })
                });
        if !has_computed_key_owner_remap {
            return;
        }

        // remap annotations before `[` from key expressions to owner nodes
        self.tree
            .remap_annotation_targets(|target_node_id, _, annotation, annotation_span| {
                let Some(candidates) = owner_candidates.get(target_node_id as usize) else {
                    return target_node_id;
                };
                if candidates.is_empty() {
                    return target_node_id;
                }

                let mut selected_owner_node_id = target_node_id;
                let mut selected_owner_span_len = u32::MAX;
                for candidate in candidates {
                    if !Self::annotation_needs_computed_key_owner(
                        annotation,
                        annotation_span,
                        candidate.key_wrapper_span,
                    ) {
                        continue;
                    }

                    let owner_span_len = candidate
                        .key_wrapper_span
                        .end
                        .saturating_sub(candidate.key_wrapper_span.start);
                    if owner_span_len < selected_owner_span_len {
                        selected_owner_span_len = owner_span_len;
                        selected_owner_node_id = candidate.owner_node_id;
                    }
                }

                selected_owner_node_id
            });
    }

    /// Eat any leading decorators and return collected decorator ids.
    pub(crate) fn eat_decorators_prefix_collect_maybe(
        &mut self,
    ) -> ParseResult<Vec<LocalNodeId<Decorator>>> {
        let decorators_with_cursors = self.eat_decorators_prefix_collect_with_cursors_maybe()?;
        let decorators = decorators_with_cursors
            .into_iter()
            .map(|(decorator_id, _, _)| decorator_id)
            .collect::<Vec<_>>();
        Ok(decorators)
    }

    /// Eat any leading decorators and return decorator ids with scanner cursors.
    pub(crate) fn eat_decorators_prefix_collect_with_cursors_maybe(
        &mut self,
    ) -> ParseResult<Vec<(LocalNodeId<Decorator>, usize, usize)>> {
        let mut decorators_with_cursors = Vec::new();
        while self.peek_is(TokenType::At) {
            let decorator_cursor = self.peek_cursor();
            let start = self.mark_span();
            let decorator = self.with_recovery(
                &start,
                |parser| parser.eat_decorator().map(Some),
                None,
                TokenType::Newline,
            );
            if let Some(decorator) = decorator {
                decorators_with_cursors.push((
                    decorator,
                    decorator_cursor.index,
                    decorator_cursor.skipped_newline_count,
                ));
            }

            // consume trailing newlines between decorator entries
            self.eat_newlines_maybe()?;
        }

        Ok(decorators_with_cursors)
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
            self.attach_decorator_to_target(decorator_id, target_node_id);
        }
    }

    /// Attach one decorator node to a parsed target.
    pub(crate) fn attach_decorator_to_target(
        &mut self,
        decorator_id: LocalNodeId<Decorator>,
        target_node_id: u32,
    ) {
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

    /// Attach annotations for one explicit token boundary.
    #[inline(always)]
    pub(crate) fn attach_boundary(
        &mut self,
        token_index: usize,
        skipped_newline_count: usize,
        target_node_id: u32,
        kind: AnnotationBoundaryKind,
    ) {
        match kind {
            AnnotationBoundaryKind::Leading(mode) => {
                self.attach_leading_annotations_for_token(
                    token_index,
                    skipped_newline_count,
                    target_node_id,
                    mode,
                );
            }
            AnnotationBoundaryKind::Infix => {
                self.attach_infix_annotations_for_token(
                    token_index,
                    skipped_newline_count,
                    target_node_id,
                );
            }
            AnnotationBoundaryKind::Stub => {
                self.attach_stub_annotations_for_token(
                    token_index,
                    skipped_newline_count,
                    target_node_id,
                );
            }
            AnnotationBoundaryKind::Trailing(mode) => {
                let preserve_line_postfix =
                    matches!(mode, TrailingAnnotationKind::PreserveLinePostfix);
                self.attach_trailing_annotations_for_token(
                    token_index,
                    target_node_id,
                    preserve_line_postfix,
                );
            }
            AnnotationBoundaryKind::TrailingLineBoundary => {
                self.attach_trailing_line_boundary_comments_for_token(token_index, target_node_id);
            }
            AnnotationBoundaryKind::DotBoundary(mode) => {
                let keep_line_comments_on_line_break =
                    matches!(mode, DotBoundaryKind::OptionalCall);
                self.attach_dot_boundary_annotations_for_token(
                    token_index,
                    skipped_newline_count,
                    target_node_id,
                    keep_line_comments_on_line_break,
                );
            }
            AnnotationBoundaryKind::DotPrefix => {
                self.attach_dot_prefix_annotations_for_token(
                    token_index,
                    skipped_newline_count,
                    target_node_id,
                );
            }
            AnnotationBoundaryKind::Blank(mode) => {
                let position = match mode {
                    BlankBoundaryKind::Prefix => AnnotationPosition::BlockPrefix,
                    BlankBoundaryKind::Postfix => AnnotationPosition::BlockPostfix,
                };
                self.attach_inline_blank_from_skipped_newlines_with_position(
                    token_index,
                    skipped_newline_count,
                    target_node_id,
                    position,
                );
            }
        }
    }

    /// Attach annotations for the current parser token boundary.
    #[inline(always)]
    pub(crate) fn attach_current_boundary(
        &mut self,
        target_node_id: u32,
        kind: AnnotationBoundaryKind,
    ) {
        let Some(token_index) = self.current_annotation_token_index() else {
            return;
        };

        self.attach_boundary(token_index, 0, target_node_id, kind);
    }

    /// Attach leading annotations for one semantic token.
    #[inline(always)]
    fn attach_leading_annotations_for_token(
        &mut self,
        token_index: usize,
        skipped_newline_count: usize,
        target_node_id: u32,
        kind: LeadingAnnotationKind,
    ) {
        let (inline_position, require_line_break_before, prefer_block_prefix_on_line_break) =
            match kind {
                LeadingAnnotationKind::Statement => (AnnotationPosition::BlockPrefix, true, true),
                LeadingAnnotationKind::Expression => (AnnotationPosition::LinePrefix, false, true),
                LeadingAnnotationKind::Wrapper => (AnnotationPosition::LinePrefix, false, false),
            };

        self.attach_inline_leading_annotations_for_token_with_mode(
            token_index,
            skipped_newline_count,
            target_node_id,
            inline_position,
            require_line_break_before,
            prefer_block_prefix_on_line_break,
        );
    }

    /// Attach infix annotations for one semantic token in container contexts.
    fn attach_infix_annotations_for_token(
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
        let annotation_tokens = self
            .collect_unclaimed_annotation_tokens_for_window(token_window_start, token_index + 1);
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

    /// Attach leading annotations for a synthetic stub target.
    fn attach_stub_annotations_for_token(
        &mut self,
        token_index: usize,
        skipped_newline_count: usize,
        target_node_id: u32,
    ) {
        // collect side tokens around the synthetic stub boundary
        let token_window_start =
            self.annotation_leading_window_start(token_index, skipped_newline_count);
        let token_window_start = token_window_start.saturating_sub(1);
        let annotation_tokens = self
            .collect_unclaimed_annotation_tokens_for_window(token_window_start, token_index + 1);
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

    /// Attach trailing annotations before one token index.
    #[inline(always)]
    fn attach_trailing_annotations_for_token(
        &mut self,
        token_index: usize,
        target_node_id: u32,
        preserve_line_postfix_on_newline_boundary: bool,
    ) {
        // collect candidate annotation tokens for this trailing window
        let token_window_start = self.annotation_leading_window_start(token_index, 0);
        let annotation_tokens = self
            .collect_unclaimed_annotation_tokens_for_window(token_window_start, token_index + 1);
        if annotation_tokens.is_empty() {
            return;
        }

        // compute boundary metadata once per token
        let boundary_context = self.inline_trailing_boundary_context(token_index);

        // process each mergeable token group and attach it with boundary-aware positioning
        let mut group_start_index = 0usize;
        while group_start_index < annotation_tokens.len() {
            let token_type = annotation_tokens[group_start_index].token.token.ty;
            let group_end_index =
                self.next_mergeable_annotation_group_end(&annotation_tokens, group_start_index);

            let group_len = group_end_index - group_start_index + 1;
            if !self.is_attachable_trailing_annotation_token_type(token_type) {
                group_start_index = group_end_index + 1;
                continue;
            }

            // compute the trailing position for this group in context
            let Some(position) = self.inline_trailing_annotation_position(
                &annotation_tokens,
                group_start_index,
                group_end_index,
                token_index,
                boundary_context,
                preserve_line_postfix_on_newline_boundary,
            ) else {
                group_start_index = group_end_index + 1;
                continue;
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

            group_start_index = group_end_index + 1;
        }
    }

    /// Return the current semantic token index used for inline annotation attachment.
    #[inline(always)]
    fn current_annotation_token_index(&mut self) -> Option<usize> {
        // bail when there are no semantic tokens
        let mut token_len = self.tokens().len();
        if token_len == 0 {
            return None;
        }

        // materialize the current scanner position in lazy lex mode
        let token_index = self.pos_index();
        if token_index >= token_len {
            let _ = self.token_at(token_index);
            token_len = self.tokens().len();
            if token_len == 0 {
                return None;
            }
        }

        // clamp parser position to the materialized semantic token range
        if token_index >= token_len {
            return Some(token_len - 1);
        }

        Some(token_index)
    }

    /// Build inline trailing boundary context for one token index.
    #[inline(always)]
    fn inline_trailing_boundary_context(
        &mut self,
        token_index: usize,
    ) -> InlineTrailingBoundaryContext {
        // read token and line-break context at this boundary
        let current_token_type = self.token_type_at(token_index);
        let has_line_break_before_current = self.line_terminator_before_index(token_index);
        let is_boundary = self.is_inline_trailing_boundary_token(current_token_type)
            || has_line_break_before_current;

        // keep previous token context for empty parenthesis special cases
        let previous_non_newline_token_type =
            self.previous_non_newline_token_type_before(token_index);

        InlineTrailingBoundaryContext {
            current_token_type,
            has_line_break_before_current,
            is_boundary,
            previous_non_newline_token_type,
        }
    }

    /// Return the end index for one mergeable inline annotation group.
    #[inline(always)]
    fn next_mergeable_annotation_group_end(
        &self,
        annotation_tokens: &[InlineAnnotationToken],
        group_start_index: usize,
    ) -> usize {
        // walk forward while neighbor tokens belong to one merged group
        let mut group_end_index = group_start_index;
        while group_end_index + 1 < annotation_tokens.len()
            && self.annotation_tokens_can_merge(
                &annotation_tokens[group_end_index],
                &annotation_tokens[group_end_index + 1],
            )
        {
            group_end_index += 1;
        }

        group_end_index
    }

    /// Return true when the token type can attach as a trailing inline annotation.
    #[inline(always)]
    fn is_attachable_trailing_annotation_token_type(&self, token_type: TokenType) -> bool {
        matches!(
            token_type,
            TokenType::LineComment
                | TokenType::DocLineComment
                | TokenType::BlockComment
                | TokenType::DocBlockComment
        )
    }

    /// Compute trailing inline annotation position for one grouped token range.
    #[inline(always)]
    fn inline_trailing_annotation_position(
        &mut self,
        annotation_tokens: &[InlineAnnotationToken],
        group_start_index: usize,
        group_end_index: usize,
        token_index: usize,
        boundary_context: InlineTrailingBoundaryContext,
        prefer_line_postfix_on_newline_boundary: bool,
    ) -> Option<AnnotationPosition> {
        let token_type = annotation_tokens[group_start_index].token.token.ty;

        // compute whether this group can stay on the current token boundary
        let is_group_on_current_token = annotation_tokens[group_start_index..=group_end_index]
            .iter()
            .all(|token| token.token_index == token_index);
        let has_line_break_before_boundary = self.annotation_group_has_line_break_before_boundary(
            annotation_tokens,
            group_end_index,
            token_index,
        );
        let is_line_boundary_attachment =
            is_group_on_current_token && !has_line_break_before_boundary;
        let is_inline_group_attachment =
            !self.annotation_group_starts_on_own_line(annotation_tokens, group_start_index);

        // place line comments directly as line postfix when possible
        if matches!(
            token_type,
            TokenType::LineComment | TokenType::DocLineComment
        ) {
            if prefer_line_postfix_on_newline_boundary
                && is_group_on_current_token
                && is_inline_group_attachment
                && boundary_context.current_token_type == TokenType::Newline
                && boundary_context.previous_non_newline_token_type
                    == Some(TokenType::CloseParenthesis)
            {
                return Some(AnnotationPosition::LinePostfix);
            }
            return Some(if boundary_context.is_boundary {
                if is_inline_group_attachment {
                    AnnotationPosition::LinePostfixBoundary
                } else {
                    AnnotationPosition::BlockPostfix
                }
            } else {
                AnnotationPosition::LinePostfix
            });
        }

        // single line block comments can behave like line comments at boundaries
        if matches!(
            token_type,
            TokenType::BlockComment | TokenType::DocBlockComment
        ) {
            let is_single_line = self.annotation_group_is_single_line(
                annotation_tokens,
                group_start_index,
                group_end_index,
            );
            if !is_single_line {
                return Some(AnnotationPosition::BlockPostfix);
            }

            let is_empty_parenthesis_boundary = boundary_context.current_token_type
                == TokenType::CloseParenthesis
                && boundary_context.previous_non_newline_token_type
                    == Some(TokenType::OpenParenthesis)
                && !boundary_context.has_line_break_before_current;
            if prefer_line_postfix_on_newline_boundary
                && is_line_boundary_attachment
                && boundary_context.current_token_type == TokenType::Newline
            {
                return Some(AnnotationPosition::LinePostfix);
            }
            return Some(
                if boundary_context.is_boundary
                    && (is_line_boundary_attachment || is_inline_group_attachment)
                {
                    if is_empty_parenthesis_boundary {
                        AnnotationPosition::BlockPostfix
                    } else {
                        AnnotationPosition::LinePostfixBoundary
                    }
                } else if boundary_context.is_boundary {
                    AnnotationPosition::BlockPostfix
                } else {
                    AnnotationPosition::LinePostfix
                },
            );
        }

        None
    }

    /// Return whether a grouped annotation starts on an otherwise empty source line.
    fn annotation_group_starts_on_own_line(
        &self,
        tokens: &[InlineAnnotationToken],
        group_start_idx: usize,
    ) -> bool {
        let group_start_span = tokens[group_start_idx].token.span;
        let head_span = Span::new(group_start_span.file, 0, group_start_span.start);
        let head_source = self.get_span_str(head_span);
        let line_start = head_source.rfind('\n').map_or(0usize, |index| index + 1);
        head_source[line_start..].trim().is_empty()
    }

    /// Attach trailing line comments before a token as line boundary postfix.
    #[inline(always)]
    fn attach_trailing_line_boundary_comments_for_token(
        &mut self,
        token_index: usize,
        target_node_id: u32,
    ) {
        // collect candidate comment tokens before the current token
        let token_window_start = self.annotation_leading_window_start(token_index, 0);
        let annotation_tokens = self
            .collect_unclaimed_annotation_tokens_for_window(token_window_start, token_index + 1);
        if annotation_tokens.is_empty() {
            return;
        }

        // keep full-line prefix docs/comments on the following node
        let target_span = self.tree.get_span_by_id(target_node_id);
        let target_end = target_span.end;

        // process only line-comment groups for statement style boundary ownership
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

            if matches!(
                token_type,
                TokenType::LineComment | TokenType::DocLineComment
            ) {
                let start_span = annotation_tokens[group_start_index].token.span;
                if start_span.file == target_span.file
                    && start_span.start >= target_end
                    && self.count_line_breaks_in_span(Span::new(
                        target_span.file,
                        target_end,
                        start_span.start,
                    )) == 0
                {
                    let group_len = group_end_index - group_start_index + 1;
                    self.attach_inline_annotation_group(
                        token_type,
                        &annotation_tokens,
                        group_start_index,
                        group_end_index,
                        group_len,
                        target_node_id,
                        AnnotationPosition::LinePostfixBoundary,
                    );
                }
            }

            group_start_index = group_end_index + 1;
        }
    }

    /// Return true when the current token is a trailing annotation boundary.
    fn is_inline_trailing_boundary_token(&self, token_type: TokenType) -> bool {
        matches!(
            token_type,
            TokenType::OpenBrace
                | TokenType::Newline
                | TokenType::Semicolon
                | TokenType::Comma
                | TokenType::CloseBrace
                | TokenType::CloseBracket
                | TokenType::CloseParenthesis
                | TokenType::End
        )
    }

    /// Attach boundary annotations before a dot style boundary token.
    fn attach_dot_boundary_annotations_for_token(
        &mut self,
        token_index: usize,
        skipped_newline_count: usize,
        target_node_id: u32,
        keep_line_comments_on_line_break: bool,
    ) {
        // collect trivia around the dot boundary
        let token_window_start =
            self.annotation_leading_window_start(token_index, skipped_newline_count);
        let token_window_start = token_window_start.saturating_sub(1);
        let mut annotation_tokens = self
            .collect_unclaimed_annotation_tokens_for_window(token_window_start, token_index + 1);
        if annotation_tokens.is_empty() {
            // side trivia near member boundaries can be owned by the next semantic token in lazy lex mode
            let extended_window_end = token_index.saturating_add(2);
            annotation_tokens = self.collect_unclaimed_annotation_tokens_for_window(
                token_window_start,
                extended_window_end,
            );

            let boundary_start = self
                .token_at(token_index)
                .map(|token| token.span.start)
                .unwrap_or(u32::MAX);
            annotation_tokens.retain(|token| token.token.span.start <= boundary_start);
        }
        if annotation_tokens.is_empty() {
            return;
        }

        // choose boundary position from line-break context
        let has_line_break_before = self.line_terminator_before_index(token_index);

        let mut group_start_index = 0usize;
        while group_start_index < annotation_tokens.len() {
            let token_type = annotation_tokens[group_start_index].token.token.ty;
            let group_end_index =
                self.next_same_type_group_end(&annotation_tokens, group_start_index, false);

            let group_len = group_end_index - group_start_index + 1;
            if self.is_comment_annotation_token_type(token_type) {
                // keep full-line chain comments for dot-prefix attachment unless explicitly requested
                let is_line_comment_group = matches!(
                    token_type,
                    TokenType::LineComment | TokenType::DocLineComment
                );
                if has_line_break_before
                    && is_line_comment_group
                    && !keep_line_comments_on_line_break
                {
                    group_start_index = group_end_index + 1;
                    continue;
                }

                let position = if has_line_break_before {
                    AnnotationPosition::LinePostfixBoundary
                } else {
                    AnnotationPosition::LinePostfix
                };

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
    fn attach_dot_prefix_annotations_for_token(
        &mut self,
        token_index: usize,
        skipped_newline_count: usize,
        target_node_id: u32,
    ) {
        // prefix attachment only applies across line breaks
        if !self.line_terminator_before_index(token_index) {
            return;
        }

        // preserve blank lines between chain segments around prefix comments
        self.attach_inline_blank_from_skipped_newlines_with_position(
            token_index,
            skipped_newline_count,
            target_node_id,
            AnnotationPosition::BlockPrefix,
        );

        // collect nearby trivia that could prefix the next chain segment
        let token_window_start =
            self.annotation_leading_window_start(token_index, skipped_newline_count);
        let token_window_start = token_window_start.saturating_sub(1);
        let annotation_tokens = self
            .collect_unclaimed_annotation_tokens_for_window(token_window_start, token_index + 1);
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
        prefer_block_prefix_on_line_break: bool,
    ) {
        // enforce line-break-only modes early
        let has_line_break_before = self.line_terminator_before_index(token_index);
        if require_line_break_before && !has_line_break_before {
            return;
        }

        // collect leading side tokens from skipped newline tokens and the target token
        let mut token_window_start =
            self.annotation_leading_window_start(token_index, skipped_newline_count);

        // include one token of left boundary context when there are no skipped newlines
        let previous_token_type = self.previous_non_newline_token_type_before(token_index);
        let has_left_boundary_context = matches!(
            previous_token_type,
            Some(
                TokenType::Assign
                    | TokenType::Comma
                    | TokenType::Colon
                    | TokenType::Maybe
                    | TokenType::Arrow
                    | TokenType::ArrowWide
                    | TokenType::LessThan
                    | TokenType::ShiftLeft
                    | TokenType::OpenParenthesis
                    | TokenType::OpenBracket
                    | TokenType::OpenBrace
            )
        );
        if skipped_newline_count == 0
            && token_window_start == token_index
            && token_index > 0
            && !self.line_terminator_before_index(token_index)
            && has_left_boundary_context
        {
            token_window_start = token_index - 1;
        }

        // attach blank lines from skipped semantic newlines
        self.attach_inline_blank_from_skipped_newlines_with_position(
            token_index,
            skipped_newline_count,
            target_node_id,
            AnnotationPosition::BlockPrefix,
        );

        // collect concrete annotation tokens for this leading window
        let annotation_tokens = self
            .collect_unclaimed_annotation_tokens_for_window(token_window_start, token_index + 1);
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
                } else if has_line_break_before && prefer_block_prefix_on_line_break {
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

    /// Return the previous non-newline semantic token type before an index.
    fn previous_non_newline_token_type_before(&mut self, token_index: usize) -> Option<TokenType> {
        if token_index == 0 {
            return None;
        }

        let mut cursor = token_index;
        while cursor > 0 {
            cursor -= 1;
            let token_type = self.token_type_at(cursor);
            if token_type == TokenType::Newline {
                continue;
            }

            return Some(token_type);
        }

        None
    }

    /// Collect unclaimed annotation tokens for a semantic token window.
    #[inline(always)]
    fn collect_unclaimed_annotation_tokens_for_window(
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

        // collect unclaimed non-whitespace side tokens owned by this token window
        self.ensure_claimed_side_tokens_capacity();
        let mut tokens = Vec::new();
        if self.token_window_has_non_whitespace_side(token_window_start, token_window_end_exclusive)
        {
            let (side_window_start, _) = self.side_range_for_token(token_window_start);
            let (_, side_window_end) = self.side_range_for_token(token_window_end_exclusive - 1);
            if side_window_start < side_window_end {
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
                    if token_index < token_window_start || token_index >= token_window_end_exclusive
                    {
                        continue;
                    }

                    tokens.push(InlineAnnotationToken {
                        token_index,
                        side_index: Some(side_index),
                        token,
                    });
                }
            }
        }

        // collect unclaimed semantic annotation tokens in the same window
        self.ensure_claimed_annotation_tokens_capacity();
        let semantic_tokens = self.tokens();
        for token_index in token_window_start..token_window_end_exclusive {
            if self.annotation_claimed_tokens[token_index] {
                continue;
            }

            let token = semantic_tokens[token_index];
            if !self.is_comment_annotation_token_type(token.token.ty) {
                continue;
            }

            tokens.push(InlineAnnotationToken {
                token_index,
                side_index: None,
                token,
            });
        }

        // keep attachment order deterministic by source location
        tokens.sort_by_key(|token| {
            (
                token.token.span.start,
                token.token.span.end,
                token.token_index,
                token.side_index.unwrap_or(usize::MAX),
            )
        });
        tokens.dedup_by(|left, right| {
            left.token.token.ty == right.token.token.ty
                && left.token.span.start == right.token.span.start
                && left.token.span.end == right.token.span.end
        });

        tokens
    }

    /// Return true when a semantic token window has non-whitespace side trivia.
    #[inline(always)]
    fn token_window_has_non_whitespace_side(
        &mut self,
        token_window_start: usize,
        token_window_end_exclusive: usize,
    ) -> bool {
        self.token_stream
            .has_non_whitespace_side_in_window(token_window_start, token_window_end_exclusive)
    }

    /// Ensure claimed-side-token flags cover the current side stream length.
    fn ensure_claimed_side_tokens_capacity(&mut self) {
        let side_len = self.token_stream.side_tokens().len();
        if self.annotation_claimed_side_tokens.len() < side_len {
            self.annotation_claimed_side_tokens.resize(side_len, false);
        }
    }

    /// Ensure claimed semantic-token flags cover the current semantic stream length.
    fn ensure_claimed_annotation_tokens_capacity(&mut self) {
        let token_len = self.tokens().len();
        if self.annotation_claimed_tokens.len() < token_len {
            self.annotation_claimed_tokens.resize(token_len, false);
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

            if !self.annotation_tokens_can_merge(&current, &next) {
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
        if right.token.span.start <= left.token.span.end {
            return true;
        }

        let between_span = Span::new(
            left.token.span.file,
            left.token.span.end,
            right.token.span.start,
        );
        let between_text = self.get_span_str(between_span);
        if !between_text.chars().all(char::is_whitespace) {
            return false;
        }

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

        // mark annotation tokens as claimed after attachment
        self.ensure_claimed_side_tokens_capacity();
        self.ensure_claimed_annotation_tokens_capacity();
        for token in &tokens[group_start_idx..=group_end_idx] {
            if let Some(side_index) = token.side_index {
                if !self.annotation_claimed_side_tokens[side_index] {
                    self.annotation_claimed_side_tokens[side_index] = true;
                    self.annotation_claimed_side_token_log.push(side_index);
                }
            } else {
                if !self.annotation_claimed_tokens[token.token_index] {
                    self.annotation_claimed_tokens[token.token_index] = true;
                    self.annotation_claimed_token_log.push(token.token_index);
                }
            }
        }

        // attach annotation to parsed target
        self.tree.append_annotation(target_node_id, annotation_id);
    }

    /// Finalize annotations after parsing.
    pub(crate) fn attach_annotations(&mut self) {
        if !self.should_attach_annotations() {
            return;
        }

        // normalize computed key owner ids, statement wrappers are resolved inline
        let _timing = self.timing_scope(tags::PARSE_ANNOTATIONS_MAIN);
        self.normalize_computed_key_annotation_owners();

        // keep annotation ordering stable for deterministic formatter behavior
        self.tree.sort_annotations();
    }

    /// Return true when there are annotations worth attaching.
    pub(crate) fn should_attach_annotations(&mut self) -> bool {
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
