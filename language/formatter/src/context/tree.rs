use super::context::DestackFormatContext;
use destack_ast::{LocalNodeId, Node, NodeType, TokenSpan, TokenType, Tree, TreeImpl};
use destack_source::Span;

/// Return whether a token contributes non-whitespace content.
#[inline]
fn token_has_non_whitespace_content(token_type: TokenType) -> bool {
    !matches!(
        token_type,
        TokenType::Whitespace | TokenType::Newline | TokenType::End
    )
}

/// Return whether one token stream contains non-whitespace content inside one span.
fn token_stream_has_non_whitespace_content(tokens: &[TokenSpan], span: Span) -> bool {
    if span.start >= span.end {
        return false;
    }

    let mut index = tokens.partition_point(|token| token.span.end <= span.start);

    while let Some(token) = tokens.get(index).copied() {
        if token.span.start >= span.end {
            break;
        }

        if token_has_non_whitespace_content(token.token.ty) {
            return true;
        }

        index += 1;
    }

    false
}

impl<'a> DestackFormatContext<'a> {
    /// Get a span from the tree.
    #[inline]
    pub fn span<T>(&self, node_id: LocalNodeId<T>) -> Span
    where
        T: Node,
        Tree: TreeImpl<T>,
    {
        self.tree.get_span(node_id)
    }

    /// Get a span from the source map by raw node id.
    #[inline]
    pub fn span_by_id(&self, node_id: u32) -> Span {
        self.tree.source_map.get(node_id)
    }

    /// Get a node from the tree.
    #[inline]
    pub fn node<T>(&self, node_id: LocalNodeId<T>) -> &T
    where
        T: Node,
        Tree: TreeImpl<T>,
    {
        self.tree.get(node_id)
    }

    /// Get the parent id and parent type for one typed node.
    #[inline]
    pub fn parent<T>(&self, node_id: LocalNodeId<T>) -> Option<(u32, NodeType)>
    where
        T: Node,
        Tree: TreeImpl<T>,
    {
        let parent_id = self.parents.get(node_id)?;
        let parent_type = self.tree.get_node_type(parent_id);

        Some((parent_id, parent_type))
    }

    /// Get the parent id and parent type for one raw node id.
    #[inline]
    pub fn parent_by_id(&self, node_id: u32) -> Option<(u32, NodeType)> {
        let parent_id = self.parents.get_by_id(node_id)?;
        let parent_type = self.tree.get_node_type(parent_id);

        Some((parent_id, parent_type))
    }

    /// Return whether one span contains a newline.
    #[inline]
    pub fn has_newline(&self, span: Span) -> bool {
        if span.start >= span.end {
            return false;
        }

        let newline_offsets = self.newline_offsets();
        let newline_index = newline_offsets.partition_point(|offset| *offset < span.start);

        newline_offsets
            .get(newline_index)
            .is_some_and(|offset| *offset < span.end)
    }

    /// Return whether one span contains a blank line in its trivia.
    #[inline]
    pub fn has_blank_line(&self, span: Span) -> bool {
        if span.start >= span.end || !self.has_newline(span) {
            return false;
        }

        let newline_offsets = self.newline_offsets();
        let mut newline_index = newline_offsets.partition_point(|offset| *offset < span.start);

        let Some(mut previous_newline_offset): Option<u32> =
            newline_offsets.get(newline_index).copied()
        else {
            return false;
        };

        if previous_newline_offset >= span.end {
            return false;
        }

        newline_index += 1;

        while let Some(newline_offset) = newline_offsets.get(newline_index).copied() {
            if newline_offset >= span.end {
                break;
            }

            // check whether the current physical line contains any token content
            let line_span = Span::new(
                span.file,
                previous_newline_offset.saturating_add(1),
                newline_offset,
            );
            let line_has_content = token_stream_has_non_whitespace_content(self.tokens, line_span)
                || token_stream_has_non_whitespace_content(self.side_tokens, line_span);

            if !line_has_content {
                return true;
            }

            previous_newline_offset = newline_offset;
            newline_index += 1;
        }

        false
    }

    /// Return whether one span contains any non-whitespace token content.
    #[inline]
    pub fn has_non_whitespace_content(&self, span: Span) -> bool {
        token_stream_has_non_whitespace_content(self.tokens, span)
            || token_stream_has_non_whitespace_content(self.side_tokens, span)
    }

    /// Return whether one span starts on its own line.
    #[inline]
    pub fn span_starts_on_own_line(&self, span: Span) -> bool {
        if span.start == 0 {
            return true;
        }

        let newline_offsets = self.newline_offsets();
        let newline_index = newline_offsets.partition_point(|offset| *offset < span.start);

        let line_start = newline_index
            .checked_sub(1)
            .and_then(|index| newline_offsets.get(index).copied())
            .map_or(0_u32, |offset| offset.saturating_add(1));

        if line_start >= span.start {
            return true;
        }

        let prefix_span = Span::new(span.file, line_start, span.start);

        !self.has_non_whitespace_content(prefix_span)
    }

    /// Return whether one span contains an own-line or multiline comment.
    pub fn has_own_line_or_multiline_comment(&self, span: Span) -> bool {
        let comment_tokens = self.comment_tokens();
        let first_relevant_index =
            comment_tokens.partition_point(|token| token.span.end <= span.start);

        for token in &comment_tokens[first_relevant_index..] {
            let comment_span = token.span;

            // skip unrelated files
            if comment_span.file != span.file {
                continue;
            }

            // stop once comments move past the span
            if comment_span.start >= span.end {
                break;
            }

            // skip comments fully before the span
            if comment_span.end <= span.start {
                continue;
            }

            // own-line and multiline comments both force the same handling
            let is_multiline = !self.comment_is_line(*token) && self.has_newline(comment_span);

            if is_multiline || self.span_starts_on_own_line(comment_span) {
                return true;
            }
        }

        false
    }

    /// Return whether the given node begins at line start.
    pub fn is_at_line_start(&self, node_id: u32) -> bool {
        let span = self.span_by_id(node_id);

        // find the token that starts the node
        let Some(token_index) = self
            .tokens
            .iter()
            .position(|token| token.span.start == span.start)
        else {
            return false;
        };

        if token_index == 0 {
            return true;
        }

        // walk backward through trivia until a newline or real syntax appears
        let mut previous_token_index = token_index - 1;

        loop {
            let Some(previous_token) = self.tokens.get(previous_token_index) else {
                return true;
            };

            if previous_token.token.ty == TokenType::Newline {
                return true;
            }

            if !self.side_span.contains(&previous_token.span) {
                return false;
            }

            if previous_token_index == 0 {
                return true;
            }

            previous_token_index -= 1;
        }
    }
}
