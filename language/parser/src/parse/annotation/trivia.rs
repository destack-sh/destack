use crate::Parser;
use destack_ast::{
    ANNOTATION_NODE_TYPES, Annotation, AnnotationPosition, Blank, BlankTrivia, Comment,
    CommentDirective, CommentStyle, CommentTrivia, Doc, DocStyle, Expression, LocalNodeId,
    NodeType, TokenSpan, TokenType, TriviaBoundary, TriviaNewlineFlags,
};
use destack_source::{NodeSearchMode, Span};
use rustc_hash::FxHashMap;

const NO_TOKEN_INDEX: u32 = u32::MAX;

#[derive(Debug)]
struct TokenNeighborIndex {
    previous_attachable: Vec<Option<usize>>,
    next_attachable: Vec<Option<usize>>,
    last_attachable: Option<usize>,
}

impl Parser {
    /// Emit comment and blank trivia, and attach documentation semantics.
    pub(crate) fn attach_trivia_annotations(&mut self) {
        // materialize stream once so trivia flags and token indexes are stable
        self.token_stream.lex_to_end();

        // quick skip when no trivia was observed during lexing
        if !self.token_stream.has_comment_trivia_tokens()
            && !self.token_stream.has_blank_trivia_tokens()
        {
            return;
        }

        // keep attach_trivia idempotent for repeated parser entrypoints
        if self.has_attached_trivia_annotations() {
            return;
        }

        // collect semantic and side token snapshots for one-sweep emission
        let semantic_tokens = self.token_stream.tokens().to_vec();
        let side_tokens = self.token_stream.side_tokens().to_vec();
        if semantic_tokens.is_empty() && side_tokens.is_empty() {
            return;
        }

        // collect side owner token indexes once so we can mutate the tree freely
        let mut side_owner_token_indexes = Vec::with_capacity(side_tokens.len());
        for side_index in 0..side_tokens.len() {
            side_owner_token_indexes.push(self.token_stream.side_owner_token_index(side_index));
        }

        // precompute attachable token neighbors and start owners
        let neighbor_index = Self::build_token_neighbor_index(&semantic_tokens);
        let owner_start_by_token = self.build_owner_start_by_token(&semantic_tokens);

        // emit comments and documentation records from side tokens
        self.emit_comment_and_documentation_trivia(
            &semantic_tokens,
            &side_tokens,
            &side_owner_token_indexes,
            &neighbor_index,
            &owner_start_by_token,
        );

        // emit blank records from semantic newline runs
        self.emit_blank_trivia(&semantic_tokens, &neighbor_index);

        // keep semantic annotation order stable after documentation inserts
        self.tree.sort_annotations();
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

    /// Emit comment trivia records and documentation semantic attachments from side tokens.
    fn emit_comment_and_documentation_trivia(
        &mut self,
        semantic_tokens: &[TokenSpan],
        side_tokens: &[TokenSpan],
        side_owner_token_indexes: &[Option<usize>],
        neighbor_index: &TokenNeighborIndex,
        owner_start_by_token: &[Option<u32>],
    ) {
        for (side_index, token) in side_tokens.iter().copied().enumerate() {
            // only comment-like side tokens participate in trivia output
            if !Self::is_comment_like_token(token.token.ty) {
                continue;
            }

            // normalize owner seams to attachable semantic tokens
            let token_after = self.normalize_after_token_index(
                side_owner_token_indexes.get(side_index).copied().flatten(),
                semantic_tokens,
                neighbor_index,
            );
            let token_before = token_after
                .and_then(|index| neighbor_index.previous_attachable[index])
                .or(neighbor_index.last_attachable);

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

            // normalize payload text once for both semantic docs and comment trivia
            let raw_text = self.get_span_str(token.span).to_string();
            let cleaned_text = Self::clean_comment_string(token.token.ty, &raw_text);
            let directive = Self::classify_comment_directive(&raw_text, &cleaned_text);
            let string = self.strings.intern(cleaned_text);

            // documentation tokens stay semantic and preserve doc marker style
            if Self::is_documentation_token(token.token.ty)
                && Self::documentation_token_should_stay_semantic(token.token.ty, &raw_text)
            {
                let doc_style = if token.token.ty == TokenType::DocLineComment {
                    DocStyle::Slash
                } else {
                    DocStyle::Star
                };
                let doc_id = self.tree.insert(
                    Doc {
                        string,
                        style: doc_style,
                    },
                    token.span,
                );

                if let Some(target_id) = self.find_documentation_target(
                    token,
                    token_after,
                    semantic_tokens,
                    owner_start_by_token,
                ) {
                    let position = if has_leading_newline {
                        AnnotationPosition::BlockPrefix
                    } else {
                        AnnotationPosition::LinePrefix
                    };
                    self.tree.append_documentation(target_id, doc_id, position);
                    continue;
                }

                // fallback: keep unowned doc tokens as regular comments so text is never dropped
            }

            // non-doc comments stay in trivia storage
            let style = if matches!(
                token.token.ty,
                TokenType::LineComment | TokenType::DocLineComment
            ) {
                CommentStyle::Slash
            } else {
                CommentStyle::Star
            };
            let comment_id = self.tree.insert(Comment { string, style }, token.span);

            self.tree.push_comment_trivia(CommentTrivia {
                comment: comment_id,
                span: token.span,
                boundary,
                directive,
            });
        }
    }

    /// Emit blank trivia records from newline runs in semantic tokens.
    fn emit_blank_trivia(
        &mut self,
        semantic_tokens: &[TokenSpan],
        neighbor_index: &TokenNeighborIndex,
    ) {
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
                let blank_id = self.tree.insert(Blank { lines: blank_lines }, span);

                let token_before = neighbor_index.previous_attachable[run_start];
                let token_after = neighbor_index.next_attachable[run_end];
                let boundary = TriviaBoundary {
                    token_before: Self::encode_token_index(token_before),
                    token_after: Self::encode_token_index(token_after),
                    newlines: TriviaNewlineFlags::from_bools(true, true),
                    is_leading_candidate: token_after.is_some(),
                };

                self.tree.push_blank_trivia(BlankTrivia {
                    blank: blank_id,
                    span,
                    boundary,
                });
            }

            index = run_end + 1;
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

    /// Build best start-owner ids for attachable semantic token indexes.
    fn build_owner_start_by_token(&self, semantic_tokens: &[TokenSpan]) -> Vec<Option<u32>> {
        let mut start_token_by_offset = FxHashMap::<u32, usize>::default();
        start_token_by_offset.reserve(semantic_tokens.len());
        for (index, token) in semantic_tokens.iter().copied().enumerate() {
            if !Self::is_attachable_semantic_token(token.token.ty) {
                continue;
            }

            start_token_by_offset.insert(token.span.start, index);
        }

        let mut owner_start_by_token = vec![None; semantic_tokens.len()];
        let mut owner_length_by_token = vec![u32::MAX; semantic_tokens.len()];

        // choose smallest owner for one token start seam to avoid statement wrappers
        let mut node_id = 0u32;
        while node_id < self.tree.next_id() {
            if self.is_annotation_node_id(node_id) {
                node_id += 1;
                continue;
            }

            let span = self.tree.get_span_by_id(node_id);
            let Some(token_index) = start_token_by_offset.get(&span.start).copied() else {
                node_id += 1;
                continue;
            };

            let length = span.end.saturating_sub(span.start);
            let best_length = owner_length_by_token[token_index];
            let best_owner = owner_start_by_token[token_index];
            let should_replace = length < best_length
                || (length == best_length && best_owner.is_none_or(|best| node_id < best));
            if should_replace {
                owner_length_by_token[token_index] = length;
                owner_start_by_token[token_index] = Some(node_id);
            }

            node_id += 1;
        }

        owner_start_by_token
    }

    /// Resolve the attachable token index after one side token owner seam.
    fn normalize_after_token_index(
        &self,
        owner_token_index: Option<usize>,
        semantic_tokens: &[TokenSpan],
        neighbor_index: &TokenNeighborIndex,
    ) -> Option<usize> {
        let owner_token_index = owner_token_index?;
        if owner_token_index >= semantic_tokens.len() {
            return None;
        }

        let owner_token = semantic_tokens[owner_token_index];
        if owner_token.token.ty == TokenType::End {
            return None;
        }

        if Self::is_attachable_semantic_token(owner_token.token.ty) {
            return Some(owner_token_index);
        }

        neighbor_index.next_attachable[owner_token_index]
    }

    /// Resolve the semantic documentation target for one doc token.
    fn find_documentation_target(
        &self,
        token: TokenSpan,
        token_after: Option<usize>,
        semantic_tokens: &[TokenSpan],
        owner_start_by_token: &[Option<u32>],
    ) -> Option<u32> {
        let token_after =
            self.normalize_documentation_token_after(token_after, semantic_tokens);

        // direct seam owner: use the owner that starts at the following token
        if let Some(token_after) = token_after {
            if let Some(owner_id) = owner_start_by_token
                .get(token_after)
                .and_then(|owner| *owner)
            {
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

    /// Resolve a prefix owner fallback from one token span.
    fn fallback_prefix_owner_for_token(&self, token: TokenSpan) -> Option<u32> {
        if token.token.ty == TokenType::End {
            return self.find_trivia_anchor_owner();
        }

        if let Some(owner) =
            self.find_node_starting_at(&token.span, NodeSearchMode::SmallestOutermost)
        {
            if !self.is_annotation_node_id(owner.idx) {
                return Some(owner.idx);
            }
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
        matches!(token_type, TokenType::ElementwiseOr | TokenType::ElementwiseAnd)
    }

    /// Return whether a node id belongs to one annotation node type.
    fn is_annotation_node_id(&self, node_id: u32) -> bool {
        ANNOTATION_NODE_TYPES.contains(&self.tree.get_node_type(node_id))
    }

    /// Return whether source text contains a line terminator in one byte range.
    fn has_line_terminator_between(&self, start: u32, end: u32) -> bool {
        if start >= end {
            return false;
        }

        let span = Span::new(self.file_id, start, end.min(self.file.len));
        let source = self.get_span_str(span);
        source
            .as_bytes()
            .iter()
            .any(|byte| *byte == b'\n' || *byte == b'\r')
    }

    /// Return a compact encoded token index with sentinel for none.
    fn encode_token_index(token_index: Option<usize>) -> u32 {
        token_index
            .map(|index| index as u32)
            .unwrap_or(NO_TOKEN_INDEX)
    }

    /// Normalize one comment payload string from raw token source.
    fn clean_comment_string(token_type: TokenType, raw: &str) -> String {
        let mut inner = match token_type {
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
            _ => raw,
        };

        // strip one common leading space for slash comments
        if matches!(
            token_type,
            TokenType::LineComment | TokenType::DocLineComment
        ) && inner.starts_with(' ')
        {
            inner = &inner[1..];
        }

        // strip one common multiline star prefix for block comments
        if matches!(
            token_type,
            TokenType::BlockComment | TokenType::DocBlockComment
        ) && inner.contains('\n')
        {
            let has_trailing_newline = inner.ends_with('\n');
            let mut cleaned = inner
                .lines()
                .map(|line| {
                    let line = line.trim_end();
                    let line = line.trim_start();
                    let line = line.strip_prefix('*').unwrap_or(line);
                    line.strip_prefix(' ').unwrap_or(line)
                })
                .collect::<Vec<_>>()
                .join("\n");

            if has_trailing_newline {
                cleaned.push('\n');
            }

            return cleaned;
        }

        inner.trim_end().to_string()
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
