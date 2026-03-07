use crate::Parser;
use destack_ast::{
    ANNOTATION_NODE_TYPES, Annotation, AnnotationPosition, Blank, BlankTrivia, Comment,
    CommentDirective, CommentStyle, CommentTrivia, Doc, DocStyle, Expression, LocalNodeId,
    NodeType, StringId, TokenSpan, TokenType, TriviaBoundary, TriviaNewlineFlags,
    normalize_comment_payload,
};
use destack_source::{NodeSearchMode, Span};

const NO_TOKEN_INDEX: u32 = u32::MAX;
const NO_OWNER_NODE_ID: u32 = u32::MAX;

#[derive(Debug)]
struct TokenNeighborIndex {
    previous_attachable: Vec<u32>,
    next_attachable: Vec<u32>,
    last_attachable: u32,
}

#[derive(Debug)]
struct DocumentationOwnerIndex {
    owner_start_by_token: Vec<u32>,
}

impl DocumentationOwnerIndex {
    #[inline]
    fn owner_start(&self, token_index: usize) -> Option<u32> {
        let owner_id = self
            .owner_start_by_token
            .get(token_index)
            .copied()
            .unwrap_or(NO_OWNER_NODE_ID);
        if owner_id == NO_OWNER_NODE_ID {
            None
        } else {
            Some(owner_id)
        }
    }
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

        // borrow token arrays directly for one sweep emission
        let (
            semantic_tokens,
            side_tokens,
            leading_side_start_indexes,
            leading_side_end_indexes,
            comment_side_token_indexes,
        ) = {
            let _collect_tokens_timing =
                self.timing_scope(crate::parse::timing::tags::PARSE_ANNOTATIONS_COLLECT_TOKENS);
            let semantic_tokens_len = self.token_stream.tokens().len();
            let side_tokens_len = self.token_stream.side_tokens().len();
            if semantic_tokens_len == 0 && side_tokens_len == 0 {
                return;
            }
            let semantic_tokens_ptr = self.token_stream.tokens().as_ptr();
            let side_tokens_ptr = self.token_stream.side_tokens().as_ptr();
            let leading_side_start_indexes_ptr =
                self.token_stream.leading_side_start_indexes().as_ptr();
            let leading_side_start_indexes_len =
                self.token_stream.leading_side_start_indexes().len();
            let leading_side_end_indexes_ptr =
                self.token_stream.leading_side_end_indexes().as_ptr();
            let leading_side_end_indexes_len = self.token_stream.leading_side_end_indexes().len();
            let comment_side_token_indexes_ptr =
                self.token_stream.comment_side_token_indexes().as_ptr();
            let comment_side_token_indexes_len =
                self.token_stream.comment_side_token_indexes().len();

            // safety: attach_trivia_annotations does not mutate token stream vectors,
            // and the returned slices remain valid for this function scope
            unsafe {
                (
                    std::slice::from_raw_parts(semantic_tokens_ptr, semantic_tokens_len),
                    std::slice::from_raw_parts(side_tokens_ptr, side_tokens_len),
                    std::slice::from_raw_parts(
                        leading_side_start_indexes_ptr,
                        leading_side_start_indexes_len,
                    ),
                    std::slice::from_raw_parts(
                        leading_side_end_indexes_ptr,
                        leading_side_end_indexes_len,
                    ),
                    std::slice::from_raw_parts(
                        comment_side_token_indexes_ptr,
                        comment_side_token_indexes_len,
                    ),
                )
            }
        };
        let comment_owner_token_indexes = Self::collect_comment_owner_token_indexes(
            leading_side_start_indexes,
            leading_side_end_indexes,
            comment_side_token_indexes,
            semantic_tokens.len(),
        );

        // build seam indexes
        let neighbor_index = {
            let _collect_wrappers_timing =
                self.timing_scope(crate::parse::timing::tags::PARSE_ANNOTATIONS_COLLECT_WRAPPERS);
            Self::build_token_neighbor_index(semantic_tokens)
        };
        let documentation_target_token_indexes = self.collect_documentation_target_token_indexes(
            semantic_tokens,
            side_tokens,
            comment_side_token_indexes,
            &comment_owner_token_indexes,
            &neighbor_index,
        );
        let documentation_owner_index = {
            let _build_owner_index_timing = self.timing_scope(
                crate::parse::timing::tags::PARSE_ANNOTATIONS_ATTACH_SIDE_BUILD_OWNER_INDEX,
            );
            self.build_documentation_owner_index(
                semantic_tokens,
                &documentation_target_token_indexes,
            )
        };

        // emit side token comments and semantic docs
        let inserted_docs = {
            let _attach_side_timing =
                self.timing_scope(crate::parse::timing::tags::PARSE_ANNOTATIONS_ATTACH_SIDE);
            self.emit_comment_and_documentation_trivia(
                semantic_tokens,
                side_tokens,
                comment_side_token_indexes,
                &comment_owner_token_indexes,
                &neighbor_index,
                &documentation_owner_index,
            )
        };

        // emit blank runs
        {
            let _group_loop_timing = self
                .timing_scope(crate::parse::timing::tags::PARSE_ANNOTATIONS_ATTACH_SIDE_GROUP_LOOP);
            self.emit_blank_trivia(semantic_tokens, &neighbor_index);
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
        comment_side_token_indexes: &[u32],
        comment_owner_token_indexes: &[u32],
        neighbor_index: &TokenNeighborIndex,
        documentation_owner_index: &DocumentationOwnerIndex,
    ) -> bool {
        let mut pending_documentation =
            Vec::<PendingDocumentationAttachment>::with_capacity(side_tokens.len() / 16);
        let mut pending_comment_trivia =
            Vec::<PendingCommentTriviaRecord>::with_capacity(comment_side_token_indexes.len());

        {
            let _scan_timing =
                self.timing_scope(crate::parse::timing::tags::PARSE_ANNOTATIONS_ATTACH_SIDE_SCAN);

            for (comment_offset, &comment_side_index) in
                comment_side_token_indexes.iter().enumerate()
            {
                let side_index = comment_side_index as usize;
                debug_assert!(side_index < side_tokens.len());
                // safety: comment side indexes are emitted from the same side token stream
                let token = unsafe { *side_tokens.get_unchecked(side_index) };

                // normalize token seams from lexer side-owner indexes
                let side_owner = comment_owner_token_indexes
                    .get(comment_offset)
                    .copied()
                    .unwrap_or(NO_TOKEN_INDEX);
                let boundary_index = if side_owner == NO_TOKEN_INDEX {
                    semantic_tokens.len()
                } else {
                    side_owner as usize
                };
                let token_after_index = if boundary_index < semantic_tokens.len() {
                    let boundary_token = semantic_tokens[boundary_index];
                    if Self::is_attachable_semantic_token(boundary_token.token.ty) {
                        boundary_index as u32
                    } else {
                        neighbor_index.next_attachable[boundary_index]
                    }
                } else {
                    NO_TOKEN_INDEX
                };
                let token_before_index = if boundary_index < semantic_tokens.len() {
                    neighbor_index.previous_attachable[boundary_index]
                } else {
                    neighbor_index.last_attachable
                };

                // compute newline flags from source seams around the comment span
                let seam_before = if token_before_index == NO_TOKEN_INDEX {
                    0
                } else {
                    semantic_tokens[token_before_index as usize].span.end
                };
                let seam_after = if token_after_index == NO_TOKEN_INDEX {
                    self.file.len
                } else {
                    semantic_tokens[token_after_index as usize].span.start
                };
                let has_leading_newline =
                    self.has_line_terminator_between(seam_before, token.span.start);
                let has_trailing_newline =
                    self.has_line_terminator_between(token.span.end, seam_after);

                let boundary = TriviaBoundary {
                    token_before: token_before_index,
                    token_after: token_after_index,
                    newlines: TriviaNewlineFlags::from_bools(
                        has_leading_newline,
                        has_trailing_newline,
                    ),
                    is_leading_candidate: token_after_index != NO_TOKEN_INDEX,
                };

                let raw_text = self.file.span_str(token.span);
                let directive = Self::classify_comment_directive(raw_text);
                let token_before_type = Self::decode_token_index(token_before_index)
                    .and_then(|index| semantic_tokens.get(index))
                    .map(|token| token.token.ty);
                let token_before_text = Self::decode_token_index(token_before_index)
                    .and_then(|index| semantic_tokens.get(index))
                    .map(|token| self.get_span_str(token.span));
                let is_semantic_doc_token = Self::is_documentation_token(token.token.ty)
                    && Self::documentation_token_should_stay_semantic(
                        token.token.ty,
                        raw_text,
                        has_leading_newline,
                        token_before_type,
                        token_before_text,
                    );

                // documentation tokens stay semantic and preserve doc marker style
                if is_semantic_doc_token {
                    let doc_style = if token.token.ty == TokenType::DocLineComment {
                        DocStyle::Slash
                    } else {
                        DocStyle::Star
                    };
                    let token_after = Self::decode_token_index(token_after_index);
                    if let Some(target_id) = self.find_documentation_target(
                        token,
                        token_after,
                        semantic_tokens,
                        documentation_owner_index,
                    ) {
                        let cleaned_text = normalize_comment_payload(raw_text);
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
                    style,
                });
            }
        }

        let inserted_docs = !pending_documentation.is_empty();

        // emit semantic documentation after lookup phase to keep source-map index stable
        {
            let _emit_docs_timing = self
                .timing_scope(crate::parse::timing::tags::PARSE_ANNOTATIONS_ATTACH_SIDE_EMIT_DOCS);
            for pending in pending_documentation {
                let doc_id = self.insert_node(
                    Doc {
                        string: pending.string,
                        style: pending.style,
                    },
                    pending.span,
                );
                self.tree
                    .append_documentation(pending.target_id, doc_id, pending.position);
            }
        }

        // emit comment trivia after lookup phase to keep source-map index stable
        {
            let _emit_comments_timing = self.timing_scope(
                crate::parse::timing::tags::PARSE_ANNOTATIONS_ATTACH_SIDE_EMIT_COMMENTS,
            );
            for pending in pending_comment_trivia {
                let comment_id = self.insert_node(
                    Comment {
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
        }

        inserted_docs
    }

    /// Build a direct token-to-owner map for semantic documentation targets.
    fn build_documentation_owner_index(
        &self,
        semantic_tokens: &[TokenSpan],
        documentation_target_token_indexes: &[u32],
    ) -> DocumentationOwnerIndex {
        let mut owner_start_by_token = vec![NO_OWNER_NODE_ID; semantic_tokens.len()];
        if semantic_tokens.is_empty() || documentation_target_token_indexes.is_empty() {
            return DocumentationOwnerIndex {
                owner_start_by_token,
            };
        }

        let mut target_token_starts =
            Vec::<(u32, u32)>::with_capacity(documentation_target_token_indexes.len());
        for &token_index in documentation_target_token_indexes {
            let token_index = token_index as usize;
            let Some(token) = semantic_tokens.get(token_index).copied() else {
                continue;
            };
            if !Self::is_attachable_semantic_token(token.token.ty) {
                continue;
            }

            target_token_starts.push((token.span.start, token_index as u32));
        }
        target_token_starts.sort_unstable_by_key(|(token_start, _)| *token_start);
        target_token_starts.dedup_by_key(|(token_start, _)| *token_start);

        let node_count = self.tree.next_id();
        for node_id in 0..node_count {
            if self.is_annotation_node_id(node_id) {
                continue;
            }

            let owner_span = self.tree.get_span_by_id(node_id);
            let Ok(found_index) = target_token_starts
                .binary_search_by_key(&owner_span.start, |(token_start, _)| *token_start)
            else {
                continue;
            };
            let token_index = target_token_starts[found_index].1 as usize;
            let token_span = semantic_tokens[token_index].span;
            if owner_span.end < token_span.end {
                continue;
            }

            let current_owner = owner_start_by_token[token_index];
            if current_owner != NO_OWNER_NODE_ID
                && !self.documentation_owner_is_better(node_id, current_owner)
            {
                continue;
            }

            owner_start_by_token[token_index] = node_id;
        }

        DocumentationOwnerIndex {
            owner_start_by_token,
        }
    }

    /// Collect normalized doc target token indexes so owner indexing stays sparse.
    fn collect_documentation_target_token_indexes(
        &self,
        semantic_tokens: &[TokenSpan],
        side_tokens: &[TokenSpan],
        comment_side_token_indexes: &[u32],
        comment_owner_token_indexes: &[u32],
        neighbor_index: &TokenNeighborIndex,
    ) -> Vec<u32> {
        let mut target_token_indexes =
            Vec::<u32>::with_capacity(comment_side_token_indexes.len() / 8);

        for (comment_offset, &comment_side_index) in comment_side_token_indexes.iter().enumerate() {
            let side_index = comment_side_index as usize;
            let token = side_tokens[side_index];
            if !Self::is_documentation_token(token.token.ty) {
                continue;
            }

            let side_owner = comment_owner_token_indexes
                .get(comment_offset)
                .copied()
                .unwrap_or(NO_TOKEN_INDEX);
            let boundary_index = if side_owner == NO_TOKEN_INDEX {
                semantic_tokens.len()
            } else {
                side_owner as usize
            };
            let token_after_index = if boundary_index < semantic_tokens.len() {
                let boundary_token = semantic_tokens[boundary_index];
                if Self::is_attachable_semantic_token(boundary_token.token.ty) {
                    boundary_index as u32
                } else {
                    neighbor_index.next_attachable[boundary_index]
                }
            } else {
                NO_TOKEN_INDEX
            };
            let token_before_index = if boundary_index < semantic_tokens.len() {
                neighbor_index.previous_attachable[boundary_index]
            } else {
                neighbor_index.last_attachable
            };
            let seam_before = if token_before_index == NO_TOKEN_INDEX {
                0
            } else {
                semantic_tokens[token_before_index as usize].span.end
            };
            let has_leading_newline =
                self.has_line_terminator_between(seam_before, token.span.start);
            let token_before_type = Self::decode_token_index(token_before_index)
                .and_then(|index| semantic_tokens.get(index))
                .map(|token| token.token.ty);
            let token_before_text = Self::decode_token_index(token_before_index)
                .and_then(|index| semantic_tokens.get(index))
                .map(|token| self.get_span_str(token.span));
            let raw_text = self.file.span_str(token.span);
            if !Self::documentation_token_should_stay_semantic(
                token.token.ty,
                raw_text,
                has_leading_newline,
                token_before_type,
                token_before_text,
            ) {
                continue;
            }

            let token_after = Self::decode_token_index(token_after_index);
            let Some(token_after) =
                self.normalize_documentation_token_after(token_after, semantic_tokens)
            else {
                continue;
            };

            target_token_indexes.push(token_after as u32);
        }

        target_token_indexes.sort_unstable();
        target_token_indexes.dedup();
        target_token_indexes
    }

    /// Resolve owner semantic token indexes for sorted comment side token indexes.
    fn collect_comment_owner_token_indexes(
        leading_side_start_indexes: &[u32],
        leading_side_end_indexes: &[u32],
        comment_side_token_indexes: &[u32],
        semantic_tokens_len: usize,
    ) -> Vec<u32> {
        if comment_side_token_indexes.is_empty() {
            return Vec::new();
        }

        let mut owners = Vec::with_capacity(comment_side_token_indexes.len());
        let mut owner_token_index = 0usize;
        for &comment_side_index in comment_side_token_indexes {
            let side_index = comment_side_index as usize;

            while owner_token_index < semantic_tokens_len {
                let side_end = leading_side_end_indexes
                    .get(owner_token_index)
                    .copied()
                    .unwrap_or(0) as usize;
                if side_end > side_index {
                    break;
                }
                owner_token_index += 1;
            }

            if owner_token_index >= semantic_tokens_len {
                owners.push(NO_TOKEN_INDEX);
                continue;
            }

            let side_start = leading_side_start_indexes
                .get(owner_token_index)
                .copied()
                .unwrap_or(0) as usize;
            let side_end = leading_side_end_indexes
                .get(owner_token_index)
                .copied()
                .unwrap_or(0) as usize;
            let owner = if side_start <= side_index && side_index < side_end {
                owner_token_index as u32
            } else {
                NO_TOKEN_INDEX
            };
            owners.push(owner);
        }

        owners
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
                let token_before =
                    Self::decode_token_index(neighbor_index.previous_attachable[run_start]);
                let token_after = Self::decode_token_index(neighbor_index.next_attachable[run_end]);
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
            let blank_id = self.insert_node(
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
        let mut previous_attachable = vec![NO_TOKEN_INDEX; semantic_tokens.len()];
        let mut next_attachable = vec![NO_TOKEN_INDEX; semantic_tokens.len()];

        // previous attachable token before each semantic token
        let mut previous = NO_TOKEN_INDEX;
        for (index, token) in semantic_tokens.iter().enumerate() {
            previous_attachable[index] = previous;
            if Self::is_attachable_semantic_token(token.token.ty) {
                previous = index as u32;
            }
        }

        // next attachable token after each semantic token
        let mut next = NO_TOKEN_INDEX;
        for index in (0..semantic_tokens.len()).rev() {
            next_attachable[index] = next;
            if Self::is_attachable_semantic_token(semantic_tokens[index].token.ty) {
                next = index as u32;
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
        documentation_owner_index: &DocumentationOwnerIndex,
    ) -> Option<u32> {
        let token_after = self.normalize_documentation_token_after(token_after, semantic_tokens);

        // direct seam owner: use the precomputed owner that starts at the following token
        if let Some(token_after) = token_after
            && let Some(owner_id) = documentation_owner_index.owner_start(token_after)
        {
            let owner_id = self.normalize_documentation_owner(owner_id);
            let owner_id =
                self.normalize_documentation_target(owner_id, token_after, semantic_tokens);
            return Some(owner_id);
        }

        // fallback seam owner: resolve from the following token span
        if let Some(token_after) = token_after {
            let owner_token = semantic_tokens[token_after];
            if let Some(owner) = self.fallback_prefix_owner_for_token(owner_token) {
                let owner = self.normalize_documentation_owner(owner);
                let owner =
                    self.normalize_documentation_target(owner, token_after, semantic_tokens);
                return Some(owner);
            }
        }

        // side token owner fallback: resolve from the doc token span itself
        if let Some(owner) = self.fallback_prefix_owner_for_token(token) {
            return Some(self.normalize_documentation_owner(owner));
        }

        // trivia-only fallback: attach to stable anchor expression
        self.find_trivia_anchor_owner()
    }

    /// Return whether one owner should replace another in documentation ranking.
    fn documentation_owner_is_better(&self, candidate_id: u32, current_id: u32) -> bool {
        let candidate_span = self.tree.get_span_by_id(candidate_id);
        let current_span = self.tree.get_span_by_id(current_id);

        let candidate_length = candidate_span.end.saturating_sub(candidate_span.start);
        let current_length = current_span.end.saturating_sub(current_span.start);
        if candidate_length != current_length {
            return candidate_length < current_length;
        }

        let candidate_kind_rank = if self.tree.get_node_type(candidate_id) == NodeType::Expression {
            1
        } else {
            0
        };
        let current_kind_rank = if self.tree.get_node_type(current_id) == NodeType::Expression {
            1
        } else {
            0
        };
        if candidate_kind_rank != current_kind_rank {
            return candidate_kind_rank < current_kind_rank;
        }

        candidate_id < current_id
    }

    /// Normalize one documentation owner to the semantic declaration node when available.
    fn normalize_documentation_owner(&self, owner_id: u32) -> u32 {
        let mut current_id = owner_id;

        loop {
            if self.tree.get_node_type(current_id) != NodeType::Expression {
                return current_id;
            }

            let expression_id = LocalNodeId::<Expression>::new(current_id);
            match self.tree.get(expression_id) {
                Expression::Declaration(declaration_id) => return declaration_id.id,
                Expression::Statement(inner_expression_id) => {
                    current_id = inner_expression_id.id;
                }
                _ => return current_id,
            }
        }
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

    /// Decode one compact token index sentinel into an option.
    fn decode_token_index(token_index: u32) -> Option<usize> {
        if token_index == NO_TOKEN_INDEX {
            return None;
        }

        Some(token_index as usize)
    }

    /// Classify one comment directive marker from raw payload text.
    fn classify_comment_directive(raw: &str) -> CommentDirective {
        // legal header comment forms
        if raw.starts_with("//!") || raw.starts_with("/*!") {
            return CommentDirective::Legal;
        }

        let trimmed = raw.trim();
        let content = if let Some(rest) = trimmed.strip_prefix("//") {
            rest.trim_start_matches('/')
        } else if let Some(rest) = trimmed.strip_prefix("/*") {
            rest.strip_suffix("*/").unwrap_or(rest)
        } else {
            trimmed
        };

        let mut first_significant_line = None;
        let mut has_additional_significant_line = false;
        for line in content.lines() {
            let line = line.trim();
            let line = line.strip_prefix('*').unwrap_or(line).trim_start();
            if line.is_empty() {
                continue;
            }
            if first_significant_line.is_none() {
                first_significant_line = Some(line);
            } else {
                has_additional_significant_line = true;
                break;
            }
        }

        let Some(first_significant_line) = first_significant_line else {
            return CommentDirective::None;
        };

        // format ignore directives
        if (!has_additional_significant_line
            && matches!(
                first_significant_line,
                "prettier-ignore"
                    | "oxfmt-ignore"
                    | "format-ignore"
                    | "fmt-ignore"
                    | "deno-fmt-ignore"
            ))
            || (first_significant_line.starts_with("biome-ignore") && content.contains("format"))
        {
            return CommentDirective::FormatIgnore;
        }

        if !has_additional_significant_line
            && matches!(
                first_significant_line,
                "prettier-ignore-file"
                    | "oxfmt-ignore-file"
                    | "format-ignore-file"
                    | "fmt-ignore-file"
                    | "deno-fmt-ignore-file"
            )
        {
            return CommentDirective::FormatIgnoreFile;
        }

        if !has_additional_significant_line
            && matches!(
                first_significant_line,
                "prettier-ignore-start"
                    | "oxfmt-ignore-start"
                    | "format-ignore-start"
                    | "fmt-ignore-start"
                    | "biome-ignore-start"
            )
        {
            return CommentDirective::FormatIgnoreStart;
        }

        if !has_additional_significant_line
            && matches!(
                first_significant_line,
                "prettier-ignore-end"
                    | "oxfmt-ignore-end"
                    | "format-ignore-end"
                    | "fmt-ignore-end"
                    | "biome-ignore-end"
            )
        {
            return CommentDirective::FormatIgnoreEnd;
        }

        // purity directives
        if !has_additional_significant_line
            && matches!(first_significant_line, "#__PURE__" | "@__PURE__")
        {
            return CommentDirective::Pure;
        }

        if !has_additional_significant_line
            && matches!(
                first_significant_line,
                "#__NO_SIDE_EFFECTS__" | "@__NO_SIDE_EFFECTS__"
            )
        {
            return CommentDirective::NoSideEffects;
        }

        // typescript directives
        if first_significant_line.starts_with("@ts-") || first_significant_line.starts_with("ts-") {
            return CommentDirective::TypeScript;
        }

        CommentDirective::None
    }

    /// Return whether one documentation token should remain a semantic doc node.
    fn documentation_token_should_stay_semantic(
        token_type: TokenType,
        raw: &str,
        has_leading_newline: bool,
        token_before_type: Option<TokenType>,
        token_before_text: Option<&str>,
    ) -> bool {
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
        if !has_leading_newline && token_before_type.is_some() {
            let is_extends_seam = token_before_type == Some(TokenType::Identifier)
                && token_before_text == Some("extends");
            let is_doc_context_after_opening_token = matches!(
                token_before_type,
                Some(
                    TokenType::OpenParenthesis
                        | TokenType::OpenBracket
                        | TokenType::OpenBrace
                        | TokenType::Comma
                        | TokenType::Colon
                        | TokenType::Assign
                        | TokenType::Arrow
                        | TokenType::ArrowWide
                        | TokenType::LessThan
                )
            );
            if !is_doc_context_after_opening_token && !is_extends_seam {
                return false;
            }
        }

        !(inner.is_empty() || inner.chars().all(|character| character == '*'))
    }
}
