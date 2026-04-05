use crate::Parser;
use crate::lex::SideRange;
use crate::parse::timing::tags;
use destack_ast::{
    Comment, CommentDirective, CommentNewlines, CommentPosition, CommentStyle, TokenSpan, TokenType,
};

const NO_TOKEN_INDEX: u32 = u32::MAX;

#[derive(Debug)]
struct TokenNeighborIndex {
    previous_attachable: Vec<u32>,
    next_attachable: Vec<u32>,
    last_attachable: u32,
}

#[derive(Debug, Clone)]
struct PendingCommentRecord {
    comment: Comment,
}

impl Parser {
    /// Emit raw comments.
    pub(crate) fn attach_trivia_annotations(&mut self) {
        {
            // materialize stream once so trivia flags and token indexes are stable
            let _lex_to_end_timing = self.timing_scope(tags::PARSE_ANNOTATIONS_LEX_TO_END);
            self.token_stream.lex_to_end();
        }

        // quick skip when no trivia was observed during lexing
        if !self.token_stream.has_comment_tokens() {
            return;
        }

        // keep attach_trivia idempotent for repeated parser entrypoints
        {
            let _attached_check_timing = self.timing_scope(tags::PARSE_ANNOTATIONS_ATTACHED_CHECK);
            if self.has_attached_trivia_annotations() {
                return;
            }
        }

        // borrow token arrays directly for one sweep emission
        let (semantic_tokens, side_tokens, leading_side_ranges, comment_side_token_indexes) = {
            let _collect_tokens_timing = self.timing_scope(tags::PARSE_ANNOTATIONS_COLLECT_TOKENS);
            let semantic_tokens_len = self.token_stream.tokens().len();
            let side_tokens_len = self.token_stream.side_tokens().len();
            if semantic_tokens_len == 0 && side_tokens_len == 0 {
                return;
            }
            let semantic_tokens_ptr = self.token_stream.tokens().as_ptr();
            let side_tokens_ptr = self.token_stream.side_tokens().as_ptr();
            let leading_side_ranges_ptr = self.token_stream.leading_side_ranges().as_ptr();
            let leading_side_ranges_len = self.token_stream.leading_side_ranges().len();
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
                    std::slice::from_raw_parts(leading_side_ranges_ptr, leading_side_ranges_len),
                    std::slice::from_raw_parts(
                        comment_side_token_indexes_ptr,
                        comment_side_token_indexes_len,
                    ),
                )
            }
        };
        let comment_owner_token_indexes = Self::collect_comment_owner_token_indexes(
            leading_side_ranges,
            comment_side_token_indexes,
            semantic_tokens.len(),
        );

        // build token-neighbor indexes
        let neighbor_index = {
            let _collect_wrappers_timing =
                self.timing_scope(tags::PARSE_ANNOTATIONS_COLLECT_WRAPPERS);
            Self::build_token_neighbor_index(semantic_tokens)
        };
        let _attach_side_timing = self.timing_scope(tags::PARSE_ANNOTATIONS_ATTACH_SIDE);
        self.emit_comments(
            semantic_tokens,
            side_tokens,
            comment_side_token_indexes,
            &comment_owner_token_indexes,
            &neighbor_index,
        );
    }

    /// Return true when trivia output already exists.
    fn has_attached_trivia_annotations(&self) -> bool {
        !self.tree.comments().is_empty()
    }

    /// Emit raw comment records.
    fn emit_comments(
        &mut self,
        semantic_tokens: &[TokenSpan],
        side_tokens: &[TokenSpan],
        comment_side_token_indexes: &[u32],
        comment_owner_token_indexes: &[u32],
        neighbor_index: &TokenNeighborIndex,
    ) {
        let mut pending_comments =
            Vec::<PendingCommentRecord>::with_capacity(comment_side_token_indexes.len());

        {
            let _scan_timing = self.timing_scope(tags::PARSE_ANNOTATIONS_ATTACH_SIDE_SCAN);

            for (comment_offset, &comment_side_index) in
                comment_side_token_indexes.iter().enumerate()
            {
                let side_index = comment_side_index as usize;
                debug_assert!(side_index < side_tokens.len());
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
                let token_before_type = Self::decode_token_index(token_before_index)
                    .and_then(|index| semantic_tokens.get(index))
                    .map(|token| token.token.ty);
                let prefers_leading =
                    Self::comment_prefers_forward_owner(has_leading_newline, token_before_type);
                let position = if prefers_leading {
                    CommentPosition::Leading
                } else {
                    CommentPosition::Trailing
                };
                let attached_to = if position == CommentPosition::Leading {
                    Self::decode_token_index(token_after_index)
                        .map(|index| semantic_tokens[index].span.start)
                        .unwrap_or_default()
                } else {
                    0
                };

                let directive = Self::classify_comment_directive(self.file.span_str(token.span));
                let style = if matches!(
                    token.token.ty,
                    TokenType::LineComment | TokenType::DocLineComment
                ) {
                    CommentStyle::Slash
                } else {
                    CommentStyle::Star
                };
                pending_comments.push(PendingCommentRecord {
                    comment: Comment {
                        span: token.span,
                        style,
                        attached_to,
                        position,
                        newlines: CommentNewlines::from_bools(
                            has_leading_newline,
                            has_trailing_newline,
                        ),
                        directive,
                    },
                });
            }
        }

        // emit raw comments after lookup phase to keep source order stable
        {
            let _emit_comments_timing =
                self.timing_scope(tags::PARSE_ANNOTATIONS_ATTACH_SIDE_EMIT_COMMENTS);
            for pending in pending_comments {
                self.tree.push_comment(pending.comment);
            }
        }
    }

    /// Resolve owner semantic token indexes for sorted comment side token indexes.
    fn collect_comment_owner_token_indexes(
        leading_side_ranges: &[SideRange],
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
                let side_end = leading_side_ranges
                    .get(owner_token_index)
                    .map(|range| range.end)
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

            let side_start = leading_side_ranges
                .get(owner_token_index)
                .map(|range| range.start)
                .unwrap_or(0) as usize;
            let side_end = leading_side_ranges
                .get(owner_token_index)
                .map(|range| range.end)
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

    /// Return true when one semantic token can own trivia seams.
    fn is_attachable_semantic_token(token_type: TokenType) -> bool {
        !matches!(token_type, TokenType::Newline | TokenType::End)
    }

    /// Return whether one comment seam should attach to the following token.
    fn comment_prefers_forward_owner(
        has_leading_newline: bool,
        token_before_type: Option<TokenType>,
    ) -> bool {
        if has_leading_newline || token_before_type.is_none() {
            return true;
        }

        matches!(
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
                    | TokenType::ElementwiseOr
                    | TokenType::ElementwiseAnd
            )
        )
    }

    /// Return whether source text contains a line terminator in one byte range.
    fn has_line_terminator_between(&self, start: u32, end: u32) -> bool {
        if start >= end {
            return false;
        }

        !self.file.is_same_line(start, end.min(self.file.len))
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
}
