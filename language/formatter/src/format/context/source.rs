use super::*;

impl<'a> DestackFormatContext<'a> {
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

    /// Get one normalized comment payload string.
    #[inline]
    pub fn comment_text(&self, comment_id: LocalNodeId<Comment>) -> Cow<'a, str> {
        let comment_source = self.span_str(self.span(comment_id));
        normalize_comment_payload(comment_source)
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
        let newline_offsets = self.newline_offsets.get_or_init(|| {
            self.file
                .text()
                .bytes()
                .enumerate()
                .filter_map(|(index, byte)| (byte == b'\n').then_some(index as u32))
                .collect()
        });
        let newline_index = newline_offsets.partition_point(|offset| *offset < span.start);
        let has_newline = newline_offsets
            .get(newline_index)
            .is_some_and(|offset| *offset < span.end);
        self.span_has_newline_by_span
            .borrow_mut()
            .insert(span, has_newline);
        has_newline
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
        #[cfg(debug_assertions)]
        let _span_str = self.span_str(span);
        let Some(mut token_idx) = self
            .tokens
            .iter()
            .position(|token| token.span.start == span.start)
        else {
            return false; // not found
        };

        // can we reach newline or start before hitting something not in side span
        while let Some(prev_token) = self.tokens.get(token_idx) {
            if token_idx == 0 || prev_token.token.ty == TokenType::Newline {
                return true; // reached start
            } else if self.side_span.contains(&prev_token.span) {
                token_idx -= 1; // keep looking
            } else {
                return false; // hit something else
            }
        }

        // reached start
        true
    }
}
