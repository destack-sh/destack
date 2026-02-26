use crate::format::context::{
    Cell, Comment, Cow, DestackFormatContext, Expression, File, FxHashMap, Keyword, LocalNodeId,
    NODE_BOOL_STATE_FALSE, NODE_BOOL_STATE_TRUE, NODE_SPAN_CHAR_LEN_UNKNOWN, Node, NodeTree,
    NodeTreeImpl, NodeType, SmallVec, Span, TYPE_CONTEXT_STATE_FALSE, TYPE_CONTEXT_STATE_TRUE,
    TYPE_CONTEXT_STATE_UNKNOWN, TokenSpan, TokenType, normalize_comment_payload,
};

/// Build one keyword map for identifier tokens across main and side streams.
pub(crate) fn token_keyword_map(
    file: &File,
    tokens: &[TokenSpan],
    side_tokens: &[TokenSpan],
) -> FxHashMap<Span, Option<Keyword>> {
    let mut token_keyword_by_span = FxHashMap::default();

    for token in tokens.iter().copied().chain(side_tokens.iter().copied()) {
        if token.token.ty != TokenType::Identifier {
            continue;
        }

        let keyword = file.span_str(token.span).parse::<Keyword>().ok();
        token_keyword_by_span.insert(token.span, keyword);
    }

    token_keyword_by_span
}

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
    /// Return the nearest non-whitespace token after one span.
    pub fn next_non_whitespace_token_after_span(&self, span: Span) -> Option<TokenSpan> {
        let tokens = self.tokens;
        let mut index = tokens.partition_point(|token| token.span.start < span.end);

        while let Some(token) = tokens.get(index).copied() {
            if matches!(token.token.ty, TokenType::Whitespace | TokenType::Newline) {
                index += 1;
                continue;
            }

            return Some(token);
        }

        None
    }

    /// Return the nearest non-trivia token after one span.
    pub fn next_non_trivia_token_after_span(&self, span: Span) -> Option<TokenSpan> {
        let tokens = self.tokens;
        let mut index = tokens.partition_point(|token| token.span.start < span.end);

        while let Some(token) = tokens.get(index).copied() {
            if matches!(
                token.token.ty,
                TokenType::Whitespace
                    | TokenType::Newline
                    | TokenType::LineComment
                    | TokenType::BlockComment
                    | TokenType::DocLineComment
                    | TokenType::DocBlockComment
            ) {
                index += 1;
                continue;
            }

            return Some(token);
        }

        None
    }

    /// Return whether one span has a newline before its next non-whitespace token.
    pub fn span_has_newline_before_next_non_whitespace_token(&self, span: Span) -> bool {
        let Some(next_token) = self.next_non_whitespace_token_after_span(span) else {
            return false;
        };
        if next_token.span.file != span.file || next_token.span.start <= span.end {
            return false;
        }

        self.has_newline(Span::new(span.file, span.end, next_token.span.start))
    }

    /// Return the first non-trivia token that intersects one span.
    #[inline]
    pub fn first_non_trivia_token_in_span(&self, span: Span) -> Option<TokenSpan> {
        let mut index = self
            .tokens
            .partition_point(|token| token.span.end <= span.start);

        while let Some(token) = self.tokens.get(index).copied() {
            if token.span.start >= span.end {
                break;
            }

            index += 1;
            if matches!(
                token.token.ty,
                TokenType::Whitespace
                    | TokenType::Newline
                    | TokenType::LineComment
                    | TokenType::BlockComment
                    | TokenType::DocLineComment
                    | TokenType::DocBlockComment
            ) {
                continue;
            }

            return Some(token);
        }

        None
    }

    /// Return one literal token lexeme inside one span.
    #[inline]
    pub fn literal_lexeme_in_span(&self, span: Span) -> Option<&'a str> {
        let token = self.first_non_trivia_token_in_span(span)?;
        if token.token.ty != TokenType::Literal {
            return None;
        }

        Some(self.token_str(token))
    }

    /// Return byte offsets of all newline characters in the source file.
    #[inline]
    fn newline_offsets(&self) -> &[u32] {
        self.newline_offsets.get_or_init(|| {
            self.file
                .text()
                .bytes()
                .enumerate()
                .filter_map(|(index, byte)| (byte == b'\n').then_some(index as u32))
                .collect()
        })
    }

    /// Return the source line count using cached newline offsets.
    #[inline]
    pub fn file_line_count(&self) -> usize {
        let file_text = self.file.text();
        if file_text.is_empty() {
            return 0;
        }

        let newline_count = self.newline_offsets().len();
        if file_text
            .as_bytes()
            .last()
            .is_some_and(|byte| *byte == b'\n')
        {
            newline_count
        } else {
            newline_count + 1
        }
    }

    pub fn has_ignore_directive_markers(&self) -> bool {
        self.has_ignore_directive_markers
    }

    /// Return whether this file may contain template literals.
    #[inline]
    pub fn has_template_literal_markers(&self) -> bool {
        self.has_template_literal_markers
    }

    /// Mark that file-level ignore was applied.
    #[inline]
    pub fn mark_file_ignore_applied(&self) {
        self.file_ignore_applied.set(true);
    }

    /// Return whether file-level ignore was applied.
    #[inline]
    pub fn file_ignore_applied(&self) -> bool {
        self.file_ignore_applied.get()
    }

    /// Gets the str source backing a Span.
    #[inline]
    pub fn span_str(&self, span: Span) -> &'a str {
        if !self.instrumentation_enabled {
            return self.file.span_str(span);
        }

        {
            let cache = self.span_text_by_span.borrow();
            if let Some(span_str) = cache.get(&span) {
                if self.instrumentation_enabled {
                    self.cache_stats
                        .span_text_hits
                        .set(self.cache_stats.span_text_hits.get() + 1);
                }
                return span_str;
            }
        }

        if self.instrumentation_enabled {
            self.cache_stats
                .span_text_misses
                .set(self.cache_stats.span_text_misses.get() + 1);
        }
        let span_str = self.file.span_str(span);
        self.span_text_by_span.borrow_mut().insert(span, span_str);
        span_str
    }

    /// Gets the str source backing a TokenSpan.
    #[inline]
    pub fn token_str(&self, token: TokenSpan) -> &'a str {
        self.span_str(token.span)
    }

    /// Parse one identifier token as a language keyword.
    #[inline]
    pub fn token_keyword(&self, token: TokenSpan) -> Option<Keyword> {
        if token.token.ty != TokenType::Identifier {
            return None;
        }

        {
            let cache = self.token_keyword_by_span.borrow();
            if let Some(keyword) = cache.get(&token.span) {
                return *keyword;
            }
        }

        let keyword = self.token_str(token).parse::<Keyword>().ok();
        self.token_keyword_by_span
            .borrow_mut()
            .insert(token.span, keyword);
        keyword
    }

    /// Get one normalized comment payload string.
    #[inline]
    pub fn comment_text(&self, comment_id: LocalNodeId<Comment>) -> Cow<'a, str> {
        let comment_source = self.comment_raw_text(comment_id);
        normalize_comment_payload(comment_source)
    }

    /// Get one raw comment text slice.
    #[inline]
    pub fn comment_raw_text(&self, comment_id: LocalNodeId<Comment>) -> &'a str {
        self.span_str(self.span(comment_id))
    }

    /// Get the source position for one byte offset.
    #[inline]
    pub fn source_position(&self, offset: u32) -> Option<(u32, u32)> {
        self.file.get_position(offset)
    }

    /// Get the source span for one line index.
    #[inline]
    pub fn source_line_span(&self, line_index: u32) -> Option<Span> {
        self.file.get_line_span(line_index)
    }

    /// Return whether one line prefix has only whitespace trivia.
    #[inline]
    pub fn line_prefix_is_whitespace(&self, offset: u32) -> bool {
        let Some((line_index, _)) = self.source_position(offset) else {
            return false;
        };
        let Some(line_span) = self.source_line_span(line_index) else {
            return false;
        };
        if line_span.start >= offset {
            return true;
        }

        let prefix_span = Span::new(line_span.file, line_span.start, offset);
        !self.has_non_whitespace_content(prefix_span)
    }

    /// Get the source line distance between two byte offsets.
    #[inline]
    pub fn source_line_distance(&self, start_offset: u32, end_offset: u32) -> Option<u32> {
        let (start_line, _) = self.source_position(start_offset)?;
        let (end_line, _) = self.source_position(end_offset)?;
        end_line.checked_sub(start_line)
    }

    /// Get the raw line prefix string before one byte offset.
    #[inline]
    pub fn line_prefix_text(&self, offset: u32) -> Option<&'a str> {
        let (line_index, column) = self.source_position(offset)?;
        let line_span = self.source_line_span(line_index)?;
        let line_text = self.span_str(line_span);
        line_text.get(..column as usize)
    }

    /// Return whether one span ends with a newline byte.
    #[inline]
    pub fn span_ends_with_newline(&self, span: Span) -> bool {
        if span.start >= span.end {
            return false;
        }

        self.file
            .text()
            .as_bytes()
            .get(span.end.saturating_sub(1) as usize)
            .is_some_and(|byte| *byte == b'\n')
    }

    /// Get comment tokens sorted by source position.
    #[inline]
    pub fn comment_tokens(&self) -> &[TokenSpan] {
        self.comment_tokens_sorted.get_or_init(|| {
            let mut tokens: Vec<TokenSpan> = self
                .tokens
                .iter()
                .copied()
                .chain(self.side_tokens.iter().copied())
                .filter(|token| {
                    matches!(
                        token.token.ty,
                        TokenType::LineComment
                            | TokenType::BlockComment
                            | TokenType::DocLineComment
                            | TokenType::DocBlockComment
                    )
                })
                .collect();
            tokens.sort_by_key(|token| token.span.start);
            tokens
        })
    }

    /// Get the Unicode scalar count for a source span.
    #[inline]
    pub fn span_char_len(&self, span: Span) -> usize {
        {
            let cache = self.span_char_len_by_span.borrow();
            if let Some(len) = cache.get(&span) {
                return *len;
            }
        }

        let len = if self.source_is_ascii {
            span.len() as usize
        } else {
            self.span_str(span).chars().count()
        };
        self.span_char_len_by_span.borrow_mut().insert(span, len);
        len
    }

    /// Get the Unicode scalar count for one node span.
    #[inline]
    pub fn node_span_char_len<T>(&self, node_id: LocalNodeId<T>) -> usize
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        let node_index = node_id.id as usize;
        let cached = self.node_caches.node_span_char_len[node_index].get();
        if cached != NODE_SPAN_CHAR_LEN_UNKNOWN {
            return cached as usize;
        }

        let len = self.span_char_len(self.span(node_id));
        #[expect(clippy::cast_possible_truncation)]
        self.node_caches.node_span_char_len[node_index].set(len as u32);

        len
    }

    /// Return whether one node span contains a newline.
    #[inline]
    pub fn node_has_newline<T>(&self, node_id: LocalNodeId<T>) -> bool
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        let node_index = node_id.id as usize;
        let cached = self.node_caches.node_has_newline[node_index].get();
        if cached == NODE_BOOL_STATE_TRUE {
            return true;
        }
        if cached == NODE_BOOL_STATE_FALSE {
            return false;
        }

        let has_newline = self.has_newline(self.span(node_id));
        self.node_caches.node_has_newline[node_index].set(if has_newline {
            NODE_BOOL_STATE_TRUE
        } else {
            NODE_BOOL_STATE_FALSE
        });

        has_newline
    }

    /// Get a Node from the tree.
    #[inline]
    pub fn node<T>(&self, node_id: LocalNodeId<T>) -> &T
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.tree.get(node_id)
    }

    /// Get a Node from the tree.
    #[inline]
    pub fn node_type<T>(&self, node_id: LocalNodeId<T>) -> NodeType
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.tree.get_node_type(node_id.id)
    }

    /// Get a parent node id and its type from the tree.
    #[inline]
    pub fn parent<T>(&self, node_id: LocalNodeId<T>) -> Option<(u32, NodeType)>
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        let parent_id = self.parents.get(node_id);
        if let Some(parent_id) = parent_id {
            let parent_type = self.tree.get_node_type(parent_id);
            Some((parent_id, parent_type))
        } else {
            None
        }
    }

    /// Get a parent node id and its type from the tree.
    #[inline]
    pub fn parent_by_id(&self, node_id: u32) -> Option<(u32, NodeType)> {
        let parent_id = self.parents.get_by_id(node_id);
        if let Some(parent_id) = parent_id {
            let parent_type = self.tree.get_node_type(parent_id);
            Some((parent_id, parent_type))
        } else {
            None
        }
    }

    /// Get all ancestors of a node.
    #[inline]
    pub fn ancestors<T>(&self, node_id: LocalNodeId<T>) -> Vec<(u32, NodeType)>
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.parents
            .get_ancestors(node_id)
            .into_iter()
            .map(|parent_id| {
                let parent_type = self.tree.get_node_type(parent_id);
                (parent_id, parent_type)
            })
            .collect()
    }

    /// Return whether any ancestor of a node matches the predicate.
    #[inline]
    pub fn any_ancestor<T, F>(&self, node_id: LocalNodeId<T>, mut predicate: F) -> bool
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
        F: FnMut(u32, NodeType) -> bool,
    {
        let mut current_id = node_id.id;
        while let Some(parent_id) = self.parents.get_by_id(current_id) {
            let parent_type = self.tree.get_node_type(parent_id);
            if predicate(parent_id, parent_type) {
                return true;
            }
            current_id = parent_id;
        }

        false
    }

    /// Return the transparent inner expression for one expression node.
    #[inline]
    pub fn transparent_inner_expression(
        &self,
        node_id: LocalNodeId<Expression>,
    ) -> LocalNodeId<Expression> {
        let node_index = node_id.id as usize;

        if let Some(inner_expression_id) = self
            .node_caches
            .transparent_inner_expression
            .get(node_index)
            .and_then(Cell::get)
        {
            return inner_expression_id;
        }

        let mut current_id = node_id;
        let mut visited_expression_indices: SmallVec<[usize; 8]> = SmallVec::new();

        loop {
            let current_index = current_id.id as usize;
            visited_expression_indices.push(current_index);

            if self.has_annotation(current_id) {
                break;
            }

            let next_id = match self.tree.get(current_id) {
                Expression::Await { expression }
                | Expression::AwaitMaybe { expression }
                | Expression::Parenthesized { expression } => Some(*expression),
                _ => None,
            };

            let Some(next_id) = next_id else {
                break;
            };
            current_id = next_id;
        }

        for expression_index in visited_expression_indices {
            if let Some(state_cell) = self
                .node_caches
                .transparent_inner_expression
                .get(expression_index)
            {
                state_cell.set(Some(current_id));
            }
        }

        current_id
    }

    /// Return a cached type-context value for one expression node.
    #[inline]
    pub fn lookup_expression_type_context(&self, node_id: LocalNodeId<Expression>) -> Option<bool> {
        let node_index = node_id.id as usize;
        let state = self
            .node_caches
            .expression_type_context
            .get(node_index)
            .map(Cell::get)
            .unwrap_or(TYPE_CONTEXT_STATE_UNKNOWN);

        if state == TYPE_CONTEXT_STATE_TRUE {
            return Some(true);
        }
        if state == TYPE_CONTEXT_STATE_FALSE {
            return Some(false);
        }

        None
    }

    /// Store one type-context value for one expression node.
    #[inline]
    pub fn store_expression_type_context(
        &self,
        node_id: LocalNodeId<Expression>,
        is_type_context: bool,
    ) {
        let node_index = node_id.id as usize;
        let state = if is_type_context {
            TYPE_CONTEXT_STATE_TRUE
        } else {
            TYPE_CONTEXT_STATE_FALSE
        };
        if let Some(state_cell) = self.node_caches.expression_type_context.get(node_index) {
            state_cell.set(state);
        }
    }

    /// Return whether one expression appears in template-literal interpolation.
    #[inline]
    pub fn expression_is_in_template_literal_interpolation(
        &self,
        node_id: LocalNodeId<Expression>,
    ) -> bool {
        if !self.has_template_literal_markers() {
            return false;
        }

        let mut current_id = node_id.id;
        let mut visited_expression_indices: SmallVec<[usize; 8]> = SmallVec::new();

        let has_template_interpolation_ancestor = loop {
            let current_index = current_id as usize;
            let state = self
                .node_caches
                .expression_template_interpolation
                .get(current_index)
                .map(Cell::get)
                .unwrap_or(TYPE_CONTEXT_STATE_UNKNOWN);
            if state == TYPE_CONTEXT_STATE_TRUE {
                break true;
            }
            if state == TYPE_CONTEXT_STATE_FALSE {
                break false;
            }

            visited_expression_indices.push(current_index);

            let Some(parent_id) = self.parents.get_by_id(current_id) else {
                break false;
            };

            let is_template_parent = self.tree.get_node_type(parent_id) == NodeType::Expression
                && matches!(
                    self.tree.get(LocalNodeId::<Expression>::new(parent_id)),
                    Expression::TemplateExpression { .. } | Expression::TypeTemplateLiteral { .. }
                );
            if is_template_parent {
                break true;
            }

            current_id = parent_id;
        };

        let state = if has_template_interpolation_ancestor {
            TYPE_CONTEXT_STATE_TRUE
        } else {
            TYPE_CONTEXT_STATE_FALSE
        };
        for expression_index in visited_expression_indices {
            if let Some(state_cell) = self
                .node_caches
                .expression_template_interpolation
                .get(expression_index)
            {
                state_cell.set(state);
            }
        }

        has_template_interpolation_ancestor
    }

    /// Return whether one expression has a type-conditional ancestor.
    #[inline]
    pub fn expression_has_type_conditional_ancestor(
        &self,
        node_id: LocalNodeId<Expression>,
    ) -> bool {
        let mut current_id = node_id.id;
        let mut visited_expression_indices: SmallVec<[usize; 8]> = SmallVec::new();

        let has_type_conditional_ancestor = loop {
            let current_index = current_id as usize;
            let state = self
                .node_caches
                .expression_type_conditional_ancestor
                .get(current_index)
                .map(Cell::get)
                .unwrap_or(TYPE_CONTEXT_STATE_UNKNOWN);
            if state == TYPE_CONTEXT_STATE_TRUE {
                break true;
            }
            if state == TYPE_CONTEXT_STATE_FALSE {
                break false;
            }

            visited_expression_indices.push(current_index);

            let Some(parent_id) = self.parents.get_by_id(current_id) else {
                break false;
            };

            let is_type_conditional_parent = self.tree.get_node_type(parent_id)
                == NodeType::Expression
                && matches!(
                    self.tree.get(LocalNodeId::<Expression>::new(parent_id)),
                    Expression::TypeConditional { .. }
                );
            if is_type_conditional_parent {
                break true;
            }

            current_id = parent_id;
        };

        let state = if has_type_conditional_ancestor {
            TYPE_CONTEXT_STATE_TRUE
        } else {
            TYPE_CONTEXT_STATE_FALSE
        };
        for expression_index in visited_expression_indices {
            if let Some(state_cell) = self
                .node_caches
                .expression_type_conditional_ancestor
                .get(expression_index)
            {
                state_cell.set(state);
            }
        }

        has_type_conditional_ancestor
    }

    /// Return the first ancestor of a node that matches the predicate.
    #[inline]
    pub fn find_ancestor<T, F>(
        &self,
        node_id: LocalNodeId<T>,
        mut predicate: F,
    ) -> Option<(u32, NodeType)>
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
        F: FnMut(u32, NodeType) -> bool,
    {
        let mut current_id = node_id.id;
        while let Some(parent_id) = self.parents.get_by_id(current_id) {
            let parent_type = self.tree.get_node_type(parent_id);
            if predicate(parent_id, parent_type) {
                return Some((parent_id, parent_type));
            }
            current_id = parent_id;
        }

        None
    }

    /// Get a Span from the tree.
    #[inline]
    pub fn span<T>(&self, node_id: LocalNodeId<T>) -> Span
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.tree.get_span(node_id)
    }

    /// Get a Span from the tree.
    #[inline]
    pub fn span_by_id(&self, node_id: u32) -> Span {
        self.source_map.get(node_id)
    }

    /// Whether the given span has a newline.
    #[inline]
    pub fn has_newline(&self, span: Span) -> bool {
        if span.start >= span.end {
            return false;
        }

        {
            let cache = self.span_has_newline_by_span.borrow();
            if let Some(has_newline) = cache.get(&span) {
                if self.instrumentation_enabled {
                    self.cache_stats
                        .span_has_newline_hits
                        .set(self.cache_stats.span_has_newline_hits.get() + 1);
                }
                return *has_newline;
            }
        }

        if self.instrumentation_enabled {
            self.cache_stats
                .span_has_newline_misses
                .set(self.cache_stats.span_has_newline_misses.get() + 1);
        }
        let newline_offsets = self.newline_offsets();
        let newline_index = newline_offsets.partition_point(|offset| *offset < span.start);
        let has_newline = newline_offsets
            .get(newline_index)
            .is_some_and(|offset| *offset < span.end);
        self.span_has_newline_by_span
            .borrow_mut()
            .insert(span, has_newline);
        has_newline
    }

    /// Whether the given span contains one explicit blank line in trivia.
    #[inline]
    pub fn has_blank_line(&self, span: Span) -> bool {
        if span.start >= span.end || !self.has_newline(span) {
            return false;
        }

        let newline_offsets = self.newline_offsets();
        let mut newline_index = newline_offsets.partition_point(|offset| *offset < span.start);
        let Some(mut previous_newline_offset) = newline_offsets.get(newline_index).copied() else {
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

    /// Whether the given span contains non-whitespace token content.
    #[inline]
    pub fn has_non_whitespace_content(&self, span: Span) -> bool {
        token_stream_has_non_whitespace_content(self.tokens, span)
            || token_stream_has_non_whitespace_content(self.side_tokens, span)
    }

    /// Whether the given span starts on a line with only leading whitespace.
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
            .map_or(0, |offset| offset.saturating_add(1));
        if line_start >= span.start {
            return true;
        }

        let prefix_span = Span::new(span.file, line_start, span.start);
        !self.has_non_whitespace_content(prefix_span)
    }

    /// Whether the given span contains a comment token.
    #[inline]
    pub fn has_comment(&self, span: Span) -> bool {
        {
            let cache = self.span_has_comment_by_span.borrow();
            if let Some(has_comment) = cache.get(&span) {
                if self.instrumentation_enabled {
                    self.cache_stats
                        .span_has_comment_hits
                        .set(self.cache_stats.span_has_comment_hits.get() + 1);
                }
                return *has_comment;
            }
        }

        if self.instrumentation_enabled {
            self.cache_stats
                .span_has_comment_misses
                .set(self.cache_stats.span_has_comment_misses.get() + 1);
        }

        let first_relevant_index = self
            .comment_spans
            .partition_point(|comment_span| comment_span.end < span.start);

        let mut has_comment = false;
        for comment_span in &self.comment_spans[first_relevant_index..] {
            if comment_span.start > span.end {
                break;
            }

            if span.intersects(*comment_span) {
                has_comment = true;
                break;
            }
        }

        self.span_has_comment_by_span
            .borrow_mut()
            .insert(span, has_comment);
        has_comment
    }

    /// Whether the given node is at a line start.
    /// (With no other semantic spans between it and the previous newline / start).
    pub fn is_at_line_start(&self, node_id: u32) -> bool {
        // find the token starting the node's span
        let span = self.span_by_id(node_id);
        let Some(token_idx) = self
            .tokens
            .iter()
            .position(|token| token.span.start == span.start)
        else {
            return false; // not found
        };
        if token_idx == 0 {
            return true;
        }

        // walk backward from the previous token:
        // only side-span trivia is allowed before a newline / file start
        let mut previous_token_idx = token_idx - 1;
        loop {
            let Some(previous_token) = self.tokens.get(previous_token_idx) else {
                return true;
            };
            if previous_token.token.ty == TokenType::Newline {
                return true;
            }
            if !self.side_span.contains(&previous_token.span) {
                return false;
            }
            if previous_token_idx == 0 {
                return true;
            }
            previous_token_idx -= 1;
        }
    }
}
