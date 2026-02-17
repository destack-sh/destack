use crate::Parser;
use destack_ast::{
    ANNOTATION_NODE_TYPES, Annotation, AnnotationPosition, Blank, BlankTrivia, Comment,
    CommentDirective, CommentStyle, CommentTrivia, Doc, DocStyle, Expression, LocalNodeId,
    NodeType, StringId, TokenSpan, TokenType, TriviaBoundary, TriviaNewlineFlags,
    normalize_comment_payload,
};
use destack_source::{EnclosingSpan, NodeSearchMode, Span};

const NO_TOKEN_INDEX: u32 = u32::MAX;

#[derive(Debug)]
struct TokenNeighborIndex {
    previous_attachable: Vec<Option<usize>>,
    next_attachable: Vec<Option<usize>>,
    last_attachable: Option<usize>,
}

#[derive(Debug, Copy, Clone)]
struct PendingDocumentationAttachment {
    target_id: u32,
    position: AnnotationPosition,
    string: StringId,
    style: DocStyle,
    span: Span,
}

#[derive(Debug, Copy, Clone)]
struct PendingCommentTriviaRecord {
    span: Span,
    boundary: TriviaBoundary,
    directive: CommentDirective,
    string: Option<StringId>,
    style: CommentStyle,
}

#[derive(Debug, Copy, Clone)]
struct PendingBlankTriviaRecord {
    span: Span,
    boundary: TriviaBoundary,
    lines: u32,
}

impl Parser {
    /// Emit semantic documentation and seam trivia records.
    pub(crate) fn attach_trivia_annotations(&mut self) {
        {
            // materialize stream once so trivia flags and token indexes are stable
            let _lex_to_end_timing =
                self.timing_scope(crate::parse::timing::tags::PARSE_ANNOTATIONS_LEX_TO_END);
            self.token_stream.lex_to_end();
        }

        // quick skip when no trivia was observed during lexing
        if !self.token_stream.has_comment_trivia_tokens()
            && !self.token_stream.has_blank_trivia_tokens()
        {
            return;
        }

        // keep attach_trivia idempotent for repeated parser entrypoints
        {
            let _attached_check_timing =
                self.timing_scope(crate::parse::timing::tags::PARSE_ANNOTATIONS_ATTACHED_CHECK);
            if self.has_attached_trivia_annotations() {
                return;
            }
        }

        // collect snapshots for one sweep emission
        let (semantic_tokens, side_tokens, side_owner_token_indexes) = {
            let _collect_tokens_timing =
                self.timing_scope(crate::parse::timing::tags::PARSE_ANNOTATIONS_COLLECT_TOKENS);
            let semantic_tokens = self.token_stream.tokens().to_vec();
            let side_tokens = self.token_stream.side_tokens().to_vec();
            let side_owner_token_indexes = self.token_stream.side_owner_token_indexes().to_vec();
            if semantic_tokens.is_empty() && side_tokens.is_empty() {
                return;
            }

            (semantic_tokens, side_tokens, side_owner_token_indexes)
        };

        // build lightweight seam indexes
        let neighbor_index = {
            let _collect_wrappers_timing =
                self.timing_scope(crate::parse::timing::tags::PARSE_ANNOTATIONS_COLLECT_WRAPPERS);
            Self::build_token_neighbor_index(&semantic_tokens)
        };

        // emit side token comments and semantic docs
        let inserted_docs = {
            let _attach_side_timing =
                self.timing_scope(crate::parse::timing::tags::PARSE_ANNOTATIONS_ATTACH_SIDE);
            self.emit_comment_and_documentation_trivia(
                &semantic_tokens,
                &side_tokens,
                &side_owner_token_indexes,
                &neighbor_index,
            )
        };

        // emit blank runs
        {
            let _group_loop_timing = self
                .timing_scope(crate::parse::timing::tags::PARSE_ANNOTATIONS_ATTACH_SIDE_GROUP_LOOP);
            self.emit_blank_trivia(&semantic_tokens, &neighbor_index);
        }

        // keep semantic annotation order stable after doc inserts
        if inserted_docs {
            let _sort_timing =
                self.timing_scope(crate::parse::timing::tags::PARSE_ANNOTATIONS_SORT);
            self.tree.sort_annotations();
        }
    }

    /// Return true when trivia output already exists.
    fn has_attached_trivia_annotations(&self) -> bool {
        if !self.tree.comment_trivia().is_empty() || !self.tree.blank_trivia().is_empty() {
            return true;
        }

        self.tree
            .iter_nodes::<Annotation>()
            .any(|annotation_id| matches!(self.tree.get(annotation_id), Annotation::Doc { .. }))
    }

    /// Emit comment trivia records and semantic documentation attachments.
    fn emit_comment_and_documentation_trivia(
        &mut self,
        semantic_tokens: &[TokenSpan],
        side_tokens: &[TokenSpan],
        side_owner_token_indexes: &[u32],
        neighbor_index: &TokenNeighborIndex,
    ) -> bool {
        let mut pending_documentation =
            Vec::<PendingDocumentationAttachment>::with_capacity(side_tokens.len() / 16);
        let mut pending_comment_trivia =
            Vec::<PendingCommentTriviaRecord>::with_capacity(side_tokens.len());

        for (side_index, token) in side_tokens.iter().copied().enumerate() {
            // only comment-like side tokens participate in trivia output
            if !Self::is_comment_like_token(token.token.ty) {
                continue;
            }

            // normalize token seams from lexer side-owner indexes
            let side_owner = side_owner_token_indexes
                .get(side_index)
                .copied()
                .unwrap_or(NO_TOKEN_INDEX);
            let boundary_index = if side_owner == NO_TOKEN_INDEX {
                semantic_tokens.len()
            } else {
                side_owner as usize
            };
            let token_after = if boundary_index < semantic_tokens.len() {
                let boundary_token = semantic_tokens[boundary_index];
                if Self::is_attachable_semantic_token(boundary_token.token.ty) {
                    Some(boundary_index)
                } else {
                    neighbor_index.next_attachable[boundary_index]
                }
            } else {
                None
            };
            let token_before = if boundary_index < semantic_tokens.len() {
                neighbor_index.previous_attachable[boundary_index]
            } else {
                neighbor_index.last_attachable
            };

            // compute newline flags from source seams around the comment span
            let seam_before = token_before
                .map(|index| semantic_tokens[index].span.end)
                .unwrap_or(0);
            let seam_after = token_after
                .map(|index| semantic_tokens[index].span.start)
                .unwrap_or(self.file.len);
            let has_leading_newline =
                self.has_line_terminator_between(seam_before, token.span.start);
            let has_trailing_newline = self.has_line_terminator_between(token.span.end, seam_after);

            let boundary = TriviaBoundary {
                token_before: Self::encode_token_index(token_before),
                token_after: Self::encode_token_index(token_after),
                newlines: TriviaNewlineFlags::from_bools(has_leading_newline, has_trailing_newline),
                is_leading_candidate: token_after.is_some(),
            };

            // normalize payload text once for semantic docs and comment trivia
            let raw_text = self.file.span_str(token.span);
            let cleaned_text = normalize_comment_payload(raw_text);
            let directive = Self::classify_comment_directive(raw_text, cleaned_text.as_ref());
            let is_semantic_doc_token = Self::is_documentation_token(token.token.ty)
                && Self::documentation_token_should_stay_semantic(token.token.ty, raw_text);

            // documentation tokens stay semantic and preserve doc marker style
            if is_semantic_doc_token {
                let doc_style = if token.token.ty == TokenType::DocLineComment {
                    DocStyle::Slash
                } else {
                    DocStyle::Star
                };
                if let Some(target_id) =
                    self.find_documentation_target(token, token_after, semantic_tokens)
                {
                    let string = self.strings.intern(cleaned_text.as_ref());
                    let position = if has_leading_newline {
                        AnnotationPosition::BlockPrefix
                    } else {
                        AnnotationPosition::LinePrefix
                    };
                    pending_documentation.push(PendingDocumentationAttachment {
                        target_id,
                        position,
                        string,
                        style: doc_style,
                        span: token.span,
                    });
                    continue;
                }

                // fallback: keep unowned doc tokens as regular comments
            }

            // non-semantic comments are seam facts only
            let style = if matches!(
                token.token.ty,
                TokenType::LineComment | TokenType::DocLineComment
            ) {
                CommentStyle::Slash
            } else {
                CommentStyle::Star
            };
            pending_comment_trivia.push(PendingCommentTriviaRecord {
                span: token.span,
                boundary,
                directive,
                string: if cfg!(debug_assertions) {
                    Some(self.strings.intern_no_dedupe(cleaned_text.as_ref()))
                } else {
                    None
                },
                style,
            });
        }

        let inserted_docs = !pending_documentation.is_empty();

        // emit semantic documentation after lookup phase to keep source-map index stable
        for pending in pending_documentation {
            let doc_id = self.tree.insert(
                Doc {
                    string: pending.string,
                    style: pending.style,
                },
                pending.span,
            );
            self.tree
                .append_documentation(pending.target_id, doc_id, pending.position);
        }

        // emit comment trivia after lookup phase to keep source-map index stable
        for pending in pending_comment_trivia {
            let comment_id = self.tree.insert(
                Comment {
                    string: pending.string,
                    style: pending.style,
                },
                pending.span,
            );
            self.tree.push_comment_trivia(CommentTrivia {
                comment: comment_id,
                span: pending.span,
                boundary: pending.boundary,
                directive: pending.directive,
                target_node: None,
                position: AnnotationPosition::BlockInfix,
            });
        }

        inserted_docs
    }

    /// Emit blank trivia records from newline runs in semantic tokens.
    fn emit_blank_trivia(
        &mut self,
        semantic_tokens: &[TokenSpan],
        neighbor_index: &TokenNeighborIndex,
    ) {
        let mut pending_blank_trivia = Vec::<PendingBlankTriviaRecord>::new();
        let mut index = 0usize;
        while index < semantic_tokens.len() {
            let token = semantic_tokens[index];
            if token.token.ty != TokenType::Newline {
                index += 1;
                continue;
            }

            // collect one contiguous newline run
            let run_start = index;
            let mut run_end = index;
            while run_end + 1 < semantic_tokens.len()
                && semantic_tokens[run_end + 1].token.ty == TokenType::Newline
                && self.is_whitespace_only_gap(
                    semantic_tokens[run_end].span.end,
                    semantic_tokens[run_end + 1].span.start,
                )
            {
                run_end += 1;
            }

            // one newline is formatting whitespace, two or more create blank lines
            let run_length = run_end - run_start + 1;
            let blank_lines = run_length.saturating_sub(1) as u32;
            if blank_lines > 0 {
                let span = Span::new(
                    token.span.file,
                    semantic_tokens[run_start].span.start,
                    semantic_tokens[run_end].span.end,
                );
                let token_before = neighbor_index.previous_attachable[run_start];
                let token_after = neighbor_index.next_attachable[run_end];
                let boundary = TriviaBoundary {
                    token_before: Self::encode_token_index(token_before),
                    token_after: Self::encode_token_index(token_after),
                    newlines: TriviaNewlineFlags::from_bools(true, true),
                    is_leading_candidate: token_after.is_some(),
                };

                pending_blank_trivia.push(PendingBlankTriviaRecord {
                    span,
                    boundary,
                    lines: blank_lines,
                });
            }

            index = run_end + 1;
        }

        // emit blank trivia after lookup phase to keep source-map index stable
        for pending in pending_blank_trivia {
            let blank_id = self.tree.insert(
                Blank {
                    lines: pending.lines,
                },
                pending.span,
            );
            self.tree.push_blank_trivia(BlankTrivia {
                blank: blank_id,
                span: pending.span,
                boundary: pending.boundary,
                target_node: None,
                position: AnnotationPosition::BlockInfix,
            });
        }
    }

    /// Build previous and next attachable semantic token indexes.
    fn build_token_neighbor_index(semantic_tokens: &[TokenSpan]) -> TokenNeighborIndex {
        let mut previous_attachable = vec![None; semantic_tokens.len()];
        let mut next_attachable = vec![None; semantic_tokens.len()];

        // previous attachable token before each semantic token
        let mut previous = None;
        for (index, token) in semantic_tokens.iter().enumerate() {
            previous_attachable[index] = previous;
            if Self::is_attachable_semantic_token(token.token.ty) {
                previous = Some(index);
            }
        }

        // next attachable token after each semantic token
        let mut next = None;
        for index in (0..semantic_tokens.len()).rev() {
            next_attachable[index] = next;
            if Self::is_attachable_semantic_token(semantic_tokens[index].token.ty) {
                next = Some(index);
            }
        }

        TokenNeighborIndex {
            previous_attachable,
            next_attachable,
            last_attachable: previous,
        }
    }

    /// Return whether the source gap between two offsets contains only whitespace.
    fn is_whitespace_only_gap(&self, start: u32, end: u32) -> bool {
        if start >= end {
            return true;
        }

        let gap_span = Span::new(self.file_id, start, end);
        let gap_source = self.get_span_str(gap_span);
        gap_source
            .chars()
            .all(|character| character.is_whitespace())
    }

    /// Resolve the semantic documentation target for one doc token.
    fn find_documentation_target(
        &self,
        token: TokenSpan,
        token_after: Option<usize>,
        semantic_tokens: &[TokenSpan],
    ) -> Option<u32> {
        let token_after = self.normalize_documentation_token_after(token_after, semantic_tokens);

        // direct seam owner: use the smallest owner that starts at the following token
        if let Some(token_after) = token_after {
            let owner_token = semantic_tokens[token_after];
            if let Some(owner_id) = self.find_preferred_owner_starting_at(&owner_token.span) {
                let owner_id =
                    self.normalize_documentation_target(owner_id, token_after, semantic_tokens);
                return Some(owner_id);
            }
        }

        // fallback seam owner: resolve from the following token span
        if let Some(token_after) = token_after {
            let owner_token = semantic_tokens[token_after];
            if let Some(owner) = self.fallback_prefix_owner_for_token(owner_token) {
                return Some(owner);
            }
        }

        // side token owner fallback: resolve from the doc token span itself
        if let Some(owner) = self.fallback_prefix_owner_for_token(token) {
            return Some(owner);
        }

        // trivia-only fallback: attach to stable anchor expression
        self.find_trivia_anchor_owner()
    }

    /// Skip forward separators so docs bind to the real expression owner token.
    fn normalize_documentation_token_after(
        &self,
        token_after: Option<usize>,
        semantic_tokens: &[TokenSpan],
    ) -> Option<usize> {
        let mut token_index = token_after?;

        loop {
            let token = semantic_tokens.get(token_index).copied()?;

            // skip newline seams while searching for the first targetable token
            if token.token.ty == TokenType::Newline {
                token_index += 1;
                continue;
            }

            // skip leading type separators like `|` and `&`
            if Self::is_documentation_forward_separator(token.token.ty) {
                token_index += 1;
                continue;
            }

            if Self::is_attachable_semantic_token(token.token.ty) {
                return Some(token_index);
            }

            token_index += 1;
        }
    }

    /// Normalize one documentation owner id to a semantic expression target.
    fn normalize_documentation_target(
        &self,
        owner_id: u32,
        token_after_index: usize,
        semantic_tokens: &[TokenSpan],
    ) -> u32 {
        let Some(token_after) = semantic_tokens.get(token_after_index).copied() else {
            return owner_id;
        };
        if token_after.token.ty != TokenType::OpenParenthesis {
            return owner_id;
        }

        let previous_token = semantic_tokens[..token_after_index]
            .iter()
            .rev()
            .copied()
            .find(|token| token.token.ty != TokenType::Newline);
        let is_extends_seam = previous_token.is_some_and(|token| {
            token.token.ty == TokenType::Identifier && self.get_span_str(token.span) == "extends"
        });
        if !is_extends_seam {
            return owner_id;
        }

        let mut current_id = owner_id;

        loop {
            if self.tree.get_node_type(current_id) != NodeType::Expression {
                return current_id;
            }

            let expression_id = LocalNodeId::<Expression>::new(current_id);
            let Some(next_id) = (match self.tree.get(expression_id) {
                Expression::Parenthesized { expression } => Some(expression.id),
                Expression::Statement(expression) => Some(expression.id),
                _ => None,
            }) else {
                return current_id;
            };

            current_id = next_id;
        }
    }

    /// Return a stable trivia anchor owner for trivia-only files.
    fn find_trivia_anchor_owner(&self) -> Option<u32> {
        let mut first_expression_id = None;
        for expression_id in self.tree.iter_nodes::<Expression>() {
            if first_expression_id.is_none() {
                first_expression_id = Some(expression_id.id);
            }

            if matches!(self.tree.get(expression_id), Expression::Stub) {
                return Some(expression_id.id);
            }
        }

        first_expression_id
    }

    /// Find the best non-annotation owner that starts at one token span.
    fn find_preferred_owner_starting_at(&self, span: &Span) -> Option<u32> {
        let mut best_owner: Option<EnclosingSpan> = None;
        self.tree.source_map.visit_enclosing_spans(
            span.start,
            span.end.saturating_sub(1),
            |candidate| {
                if candidate.span.start != span.start || self.is_annotation_node_id(candidate.idx) {
                    return;
                }

                let candidate_kind_rank =
                    if self.tree.get_node_type(candidate.idx) == NodeType::Expression {
                        1
                    } else {
                        0
                    };

                let should_replace = if let Some(current) = best_owner {
                    let current_kind_rank =
                        if self.tree.get_node_type(current.idx) == NodeType::Expression {
                            1
                        } else {
                            0
                        };

                    candidate.length < current.length
                        || (candidate.length == current.length
                            && candidate_kind_rank < current_kind_rank)
                        || (candidate.length == current.length
                            && candidate_kind_rank == current_kind_rank
                            && candidate.idx < current.idx)
                } else {
                    true
                };

                if should_replace {
                    best_owner = Some(candidate);
                }
            },
        );

        best_owner.map(|owner| owner.idx)
    }

    /// Resolve a prefix owner fallback from one token span.
    fn fallback_prefix_owner_for_token(&self, token: TokenSpan) -> Option<u32> {
        if token.token.ty == TokenType::End {
            return self.find_trivia_anchor_owner();
        }

        if let Some(owner) =
            self.find_node_starting_at(&token.span, NodeSearchMode::SmallestOutermost)
            && !self.is_annotation_node_id(owner.idx)
        {
            return Some(owner.idx);
        }

        if let Some(owner) = self.find_node_enclosing_at(
            &token.span,
            NodeSearchMode::SmallestOutermost,
            |candidate| !self.is_annotation_node_id(candidate.idx),
        ) {
            return Some(owner.idx);
        }

        self.find_node_enclosing_at(&token.span, NodeSearchMode::BiggestOutermost, |candidate| {
            !self.is_annotation_node_id(candidate.idx)
        })
        .map(|owner| owner.idx)
    }

    /// Return whether one node id belongs to one annotation node type.
    fn is_annotation_node_id(&self, node_id: u32) -> bool {
        ANNOTATION_NODE_TYPES.contains(&self.tree.get_node_type(node_id))
    }

    /// Return true when one semantic token can own trivia seams.
    fn is_attachable_semantic_token(token_type: TokenType) -> bool {
        !matches!(token_type, TokenType::Newline | TokenType::End)
    }

    /// Return true when one side token is any comment form.
    fn is_comment_like_token(token_type: TokenType) -> bool {
        matches!(
            token_type,
            TokenType::LineComment
                | TokenType::DocLineComment
                | TokenType::BlockComment
                | TokenType::DocBlockComment
        )
    }

    /// Return true when one side token is a documentation comment.
    fn is_documentation_token(token_type: TokenType) -> bool {
        matches!(
            token_type,
            TokenType::DocLineComment | TokenType::DocBlockComment
        )
    }

    /// Return whether docs should skip this separator to find a forward target.
    fn is_documentation_forward_separator(token_type: TokenType) -> bool {
        matches!(
            token_type,
            TokenType::ElementwiseOr | TokenType::ElementwiseAnd
        )
    }

    /// Return whether source text contains a line terminator in one byte range.
    fn has_line_terminator_between(&self, start: u32, end: u32) -> bool {
        if start >= end {
            return false;
        }

        !self.file.is_same_line(start, end.min(self.file.len))
    }

    /// Return a compact encoded token index with sentinel for none.
    fn encode_token_index(token_index: Option<usize>) -> u32 {
        token_index
            .map(|index| index as u32)
            .unwrap_or(NO_TOKEN_INDEX)
    }

    /// Classify one comment directive marker from raw and cleaned payload text.
    fn classify_comment_directive(raw: &str, cleaned: &str) -> CommentDirective {
        let trimmed = cleaned.trim();

        // format ignore directives
        if matches!(
            trimmed,
            "prettier-ignore" | "oxfmt-ignore" | "format-ignore" | "fmt-ignore" | "deno-fmt-ignore"
        ) || (trimmed.starts_with("biome-ignore") && trimmed.contains("format"))
        {
            return CommentDirective::FormatIgnore;
        }

        if matches!(
            trimmed,
            "prettier-ignore-file"
                | "oxfmt-ignore-file"
                | "format-ignore-file"
                | "fmt-ignore-file"
                | "deno-fmt-ignore-file"
        ) {
            return CommentDirective::FormatIgnoreFile;
        }

        if matches!(
            trimmed,
            "prettier-ignore-start"
                | "oxfmt-ignore-start"
                | "format-ignore-start"
                | "fmt-ignore-start"
                | "biome-ignore-start"
        ) {
            return CommentDirective::FormatIgnoreStart;
        }

        if matches!(
            trimmed,
            "prettier-ignore-end"
                | "oxfmt-ignore-end"
                | "format-ignore-end"
                | "fmt-ignore-end"
                | "biome-ignore-end"
        ) {
            return CommentDirective::FormatIgnoreEnd;
        }

        // purity directives
        if matches!(trimmed, "#__PURE__" | "@__PURE__") {
            return CommentDirective::Pure;
        }

        if matches!(trimmed, "#__NO_SIDE_EFFECTS__" | "@__NO_SIDE_EFFECTS__") {
            return CommentDirective::NoSideEffects;
        }

        // typescript directives
        if trimmed.starts_with("@ts-") || trimmed.starts_with("ts-") {
            return CommentDirective::TypeScript;
        }

        // legal header comment forms
        if raw.starts_with("//!") || raw.starts_with("/*!") {
            return CommentDirective::Legal;
        }

        CommentDirective::None
    }

    /// Return whether one documentation token should remain a semantic doc node.
    fn documentation_token_should_stay_semantic(token_type: TokenType, raw: &str) -> bool {
        if token_type != TokenType::DocBlockComment {
            return true;
        }

        let trimmed = raw.trim();
        if trimmed == "/**/" {
            return false;
        }
        if !trimmed.starts_with("/**") || !trimmed.ends_with("*/") || trimmed.len() < 5 {
            return true;
        }

        let inner = trimmed[3..trimmed.len() - 2].trim();

        !(inner.is_empty() || inner.chars().all(|character| character == '*'))
    }
}
