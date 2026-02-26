use crate::format::analysis::{
    next_non_whitespace_token_after_annotation, previous_non_whitespace_token_before_annotation,
    token_is_keyword,
};
use crate::format::context::{
    ANNOTATION_STATE_CACHED, ANNOTATION_STATE_NONE, ANNOTATION_STATE_PRESENT, Annotation,
    AnnotationData, AnnotationPosition, Argument, ArgumentAnnotationCache,
    CallArgumentExpansionsCache, CallArgumentLayoutCache, Cell, Comment, DestackFormatContext,
    Expression, LocalNodeId, Node, NodeTree, NodeTreeImpl, Ref, Span, ast,
};

impl<'a> DestackFormatContext<'a> {
    /// Return whether one annotation position is one prefix position.
    #[inline]
    fn is_prefix_annotation_position(position: AnnotationPosition) -> bool {
        matches!(
            position,
            AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
        )
    }

    /// Get one formatter-owned annotation by id.
    #[inline]
    pub fn annotation(&self, annotation_id: LocalNodeId<Annotation>) -> Annotation {
        self.formatter_annotation_entries[annotation_id.id as usize].annotation
    }

    /// Get one formatter-owned annotation span by id.
    #[inline]
    pub fn annotation_span(&self, annotation_id: LocalNodeId<Annotation>) -> Span {
        self.formatter_annotation_entries[annotation_id.id as usize].span
    }

    /// Return whether one annotation starts on its own source line.
    #[inline]
    pub fn annotation_starts_on_own_line(&self, annotation_id: LocalNodeId<Annotation>) -> bool {
        self.span_starts_on_own_line(self.annotation_span(annotation_id))
    }

    /// Return whether one annotation source starts after at least one newline.
    #[inline]
    pub fn annotation_has_leading_newline(&self, annotation_id: LocalNodeId<Annotation>) -> bool {
        self.has_newline(self.annotation_span(annotation_id))
    }

    /// Return the nearest non-whitespace token before one annotation span.
    #[inline]
    pub fn annotation_previous_non_whitespace_token(
        &self,
        annotation_id: LocalNodeId<Annotation>,
    ) -> Option<ast::TokenSpan> {
        previous_non_whitespace_token_before_annotation(self, annotation_id)
    }

    /// Return the nearest non-whitespace token after one annotation span.
    #[inline]
    pub fn annotation_next_non_whitespace_token(
        &self,
        annotation_id: LocalNodeId<Annotation>,
    ) -> Option<ast::TokenSpan> {
        next_non_whitespace_token_after_annotation(self, annotation_id)
    }

    /// Return the previous non-whitespace token type before one annotation span.
    #[inline]
    pub fn annotation_previous_non_whitespace_token_type(
        &self,
        annotation_id: LocalNodeId<Annotation>,
    ) -> Option<ast::TokenType> {
        self.annotation_previous_non_whitespace_token(annotation_id)
            .map(|token| token.token.ty)
    }

    /// Return whether the next non-whitespace token after one annotation starts on the same line.
    pub fn annotation_next_token_is_on_same_line(
        &self,
        annotation_id: LocalNodeId<Annotation>,
    ) -> bool {
        let annotation_span = self.annotation_span(annotation_id);
        let Some(next_token) = self.annotation_next_non_whitespace_token(annotation_id) else {
            return false;
        };
        if annotation_span.file != next_token.span.file {
            return false;
        }

        self.file
            .is_same_line(annotation_span.end.saturating_sub(1), next_token.span.start)
    }

    /// Return whether the next non-whitespace token after one annotation matches one keyword.
    #[inline]
    pub fn annotation_next_token_is_keyword(
        &self,
        annotation_id: LocalNodeId<Annotation>,
        keyword: ast::Keyword,
    ) -> bool {
        self.annotation_next_non_whitespace_token(annotation_id)
            .is_some_and(|token| token_is_keyword(self, token, keyword))
    }

    /// Return the next non-whitespace token type after one annotation span.
    #[inline]
    pub fn annotation_next_non_whitespace_token_type(
        &self,
        annotation_id: LocalNodeId<Annotation>,
    ) -> Option<ast::TokenType> {
        self.annotation_next_non_whitespace_token(annotation_id)
            .map(|token| token.token.ty)
    }

    /// Return the guard target token type after one annotation-following semicolon.
    #[inline]
    pub fn annotation_semicolon_guard_target_token_type(
        &self,
        annotation_id: LocalNodeId<Annotation>,
    ) -> Option<ast::TokenType> {
        let token_after_annotation = self.annotation_next_non_whitespace_token(annotation_id)?;
        if token_after_annotation.token.ty != ast::TokenType::Semicolon {
            return None;
        }

        self.next_non_trivia_token_after_span(token_after_annotation.span)
            .map(|token| token.token.ty)
    }

    /// Record one annotation cache hit when instrumentation is enabled.
    #[inline]
    fn increment_annotation_cache_hits(&self) {
        if self.instrumentation_enabled {
            self.cache_stats
                .annotation_cache_hits
                .set(self.cache_stats.annotation_cache_hits.get() + 1);
        }
    }

    /// Record one annotation cache miss when instrumentation is enabled.
    #[inline]
    fn increment_annotation_cache_misses(&self) {
        if self.instrumentation_enabled {
            self.cache_stats
                .annotation_cache_misses
                .set(self.cache_stats.annotation_cache_misses.get() + 1);
        }
    }

    /// Return cached annotation data for one node index.
    #[inline]
    fn annotation_data_from_cache(&self, node_index: usize) -> Ref<'_, AnnotationData> {
        let cache = self.annotation_data_by_node_id.borrow();
        Ref::map(cache, |cache| {
            cache
                .get(node_index)
                .and_then(|entry| entry.as_ref())
                .expect("annotation cache should contain requested node")
        })
    }

    /// Return borrowed cached annotation data for a node.
    #[inline]
    fn annotation_data_for_node<T>(
        &self,
        node_id: LocalNodeId<T>,
    ) -> Option<Ref<'_, AnnotationData>>
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        let node_index = node_id.id as usize;
        let state = self.annotation_state_by_node_id[node_index].get();

        // node has no annotations
        if state == ANNOTATION_STATE_NONE {
            self.increment_annotation_cache_hits();
            return None;
        }

        // cached metadata is already available
        if state == ANNOTATION_STATE_CACHED {
            self.increment_annotation_cache_hits();
            return Some(self.annotation_data_from_cache(node_index));
        }

        self.increment_annotation_cache_misses();

        // load annotations once and cache metadata
        let annotation_ids = self
            .formatter_annotation_ids_by_node_id
            .get(node_index)
            .map(|annotation_ids| annotation_ids.as_slice().to_vec())
            .unwrap_or_default();
        let annotation_data = AnnotationData::from_ids(annotation_ids, |annotation_id| {
            self.annotation(annotation_id)
        });
        {
            let mut cache = self.annotation_data_by_node_id.borrow_mut();
            cache[node_index] = Some(annotation_data);
        }
        self.annotation_state_by_node_id[node_index].set(ANNOTATION_STATE_CACHED);

        Some(self.annotation_data_from_cache(node_index))
    }

    /// Get annotations for a node. Annotations are sorted by position.
    #[inline]
    pub fn annotations<T>(&self, node_id: LocalNodeId<T>) -> Option<Vec<LocalNodeId<Annotation>>>
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.annotation_data_for_node(node_id)
            .map(|annotation_data| annotation_data.ids.clone())
    }

    /// Read node annotations without cloning.
    #[inline]
    pub fn visit_annotations<T, R, F>(&self, node_id: LocalNodeId<T>, f: F) -> Option<R>
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
        F: FnOnce(&[LocalNodeId<Annotation>]) -> R,
    {
        self.annotation_data_for_node(node_id)
            .map(|annotation_data| f(annotation_data.ids.as_slice()))
    }

    /// Check if a node has an annotation.
    #[inline]
    pub fn has_annotation<T>(&self, node_id: LocalNodeId<T>) -> bool
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        let node_index = node_id.id as usize;
        let state = self.annotation_state_by_node_id[node_index].get();

        // fast path when instrumentation is off
        if !self.instrumentation_enabled {
            return state != ANNOTATION_STATE_NONE;
        }

        // state lookup is one cache hit regardless of the presence result
        match state {
            ANNOTATION_STATE_NONE => {
                self.increment_annotation_cache_hits();
                false
            }
            ANNOTATION_STATE_PRESENT | ANNOTATION_STATE_CACHED => {
                self.increment_annotation_cache_hits();
                true
            }
            _ => false,
        }
    }

    /// Check if a node has a prefix annotation.
    #[inline]
    pub fn has_prefix_annotation<T>(&self, node_id: LocalNodeId<T>) -> bool
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.annotation_data_for_node(node_id)
            .is_some_and(|annotation_data| annotation_data.has_prefix)
    }

    /// Check if a node has a block infix annotation.
    #[inline]
    pub fn has_infix_annotation<T>(&self, node_id: LocalNodeId<T>) -> bool
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.annotation_data_for_node(node_id)
            .is_some_and(|annotation_data| annotation_data.has_infix)
    }

    /// Check if a node has a non-blank annotation.
    #[inline]
    pub fn has_non_blank_annotation<T>(&self, node_id: LocalNodeId<T>) -> bool
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.annotation_data_for_node(node_id)
            .is_some_and(|annotation_data| annotation_data.has_non_blank)
    }

    /// Check if a node has a non-blank block infix annotation.
    #[inline]
    pub fn has_non_blank_infix_annotation<T>(&self, node_id: LocalNodeId<T>) -> bool
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.annotation_data_for_node(node_id)
            .is_some_and(|annotation_data| annotation_data.has_non_blank_infix)
    }

    /// Check if a node has a postfix annotation.
    #[inline]
    pub fn has_postfix_annotation<T>(&self, node_id: LocalNodeId<T>) -> bool
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.annotation_data_for_node(node_id)
            .is_some_and(|annotation_data| annotation_data.has_postfix)
    }

    /// Check if a node has a non-blank postfix annotation.
    #[inline]
    pub fn has_non_blank_postfix_annotation<T>(&self, node_id: LocalNodeId<T>) -> bool
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.annotation_data_for_node(node_id)
            .is_some_and(|annotation_data| annotation_data.has_non_blank_postfix)
    }

    /// Check if a node has a blank block prefix annotation.
    #[inline]
    pub fn has_blank_prefix_annotation<T>(&self, node_id: LocalNodeId<T>) -> bool
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.annotation_data_for_node(node_id)
            .is_some_and(|annotation_data| annotation_data.has_blank_prefix)
    }

    /// Check if a node has a blank prefix annotation in first position.
    #[inline]
    pub fn has_blank_prefix_annotation_in_first_position<T>(&self, node_id: LocalNodeId<T>) -> bool
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.annotation_data_for_node(node_id)
            .is_some_and(|annotation_data| annotation_data.has_blank_prefix_first)
    }

    /// Return cached annotation data for one argument node.
    #[inline]
    pub fn argument_annotation_cache(
        &self,
        argument_id: LocalNodeId<Argument>,
    ) -> ArgumentAnnotationCache {
        if let Some(annotation_cache) = self
            .lookup_node_cache_value(&self.node_caches.argument_annotation_cache, argument_id.id)
        {
            self.increment_counter("cache.argument_annotation_cache.hits", 1);
            return annotation_cache;
        }

        self.increment_counter("cache.argument_annotation_cache.misses", 1);
        let annotation_cache = self.compute_argument_annotation_cache(argument_id);
        self.store_node_cache_value(
            &self.node_caches.argument_annotation_cache,
            argument_id.id,
            annotation_cache,
        );

        annotation_cache
    }

    /// Return cached compact simple unannotated argument predicate.
    #[inline]
    pub fn lookup_argument_compact_simple_unannotated(
        &self,
        argument_id: LocalNodeId<Argument>,
    ) -> Option<bool> {
        self.lookup_node_cache_value(
            &self.node_caches.argument_compact_simple_unannotated,
            argument_id.id,
        )
    }

    /// Store compact simple unannotated argument predicate.
    #[inline]
    pub fn store_argument_compact_simple_unannotated(
        &self,
        argument_id: LocalNodeId<Argument>,
        value: bool,
    ) {
        self.store_node_cache_value(
            &self.node_caches.argument_compact_simple_unannotated,
            argument_id.id,
            value,
        );
    }

    /// Return cached plain call argument predicate.
    #[inline]
    pub fn lookup_argument_plain_call_argument(
        &self,
        argument_id: LocalNodeId<Argument>,
    ) -> Option<bool> {
        self.lookup_node_cache_value(
            &self.node_caches.argument_plain_call_argument,
            argument_id.id,
        )
    }

    /// Store plain call argument predicate.
    #[inline]
    pub fn store_argument_plain_call_argument(
        &self,
        argument_id: LocalNodeId<Argument>,
        value: bool,
    ) {
        self.store_node_cache_value(
            &self.node_caches.argument_plain_call_argument,
            argument_id.id,
            value,
        );
    }

    /// Return cached call argument layout data for one call expression node.
    #[inline]
    pub fn lookup_call_argument_layout_cache(
        &self,
        call_node_id: LocalNodeId<Expression>,
    ) -> Option<CallArgumentLayoutCache> {
        self.lookup_node_cache_value(
            &self.node_caches.call_argument_layout_cache,
            call_node_id.id,
        )
    }

    /// Store call argument layout data for one call expression node.
    #[inline]
    pub fn store_call_argument_layout_cache(
        &self,
        call_node_id: LocalNodeId<Expression>,
        layout_cache: CallArgumentLayoutCache,
    ) {
        self.store_node_cache_value(
            &self.node_caches.call_argument_layout_cache,
            call_node_id.id,
            layout_cache,
        );
    }

    /// Return cached call boundary-comment state for one call expression node.
    #[inline]
    pub fn lookup_call_argument_boundary_comments(
        &self,
        call_node_id: LocalNodeId<Expression>,
    ) -> Option<bool> {
        self.lookup_node_cache_value(
            &self.node_caches.call_argument_boundary_comments,
            call_node_id.id,
        )
    }

    /// Store call boundary-comment state for one call expression node.
    #[inline]
    pub fn store_call_argument_boundary_comments(
        &self,
        call_node_id: LocalNodeId<Expression>,
        has_boundary_comments: bool,
    ) {
        self.store_node_cache_value(
            &self.node_caches.call_argument_boundary_comments,
            call_node_id.id,
            has_boundary_comments,
        );
    }

    /// Return cached chain call force-expand state for one call expression node.
    #[inline]
    pub fn lookup_call_argument_chain_force_expand(
        &self,
        call_node_id: LocalNodeId<Expression>,
    ) -> Option<bool> {
        self.lookup_node_cache_value(
            &self.node_caches.call_argument_chain_force_expand,
            call_node_id.id,
        )
    }

    /// Store one chain call force-expand state for one call expression node.
    #[inline]
    pub fn store_call_argument_chain_force_expand(
        &self,
        call_node_id: LocalNodeId<Expression>,
        force_expand: bool,
    ) {
        self.store_node_cache_value(
            &self.node_caches.call_argument_chain_force_expand,
            call_node_id.id,
            force_expand,
        );
    }

    /// Return cached regular and chain call argument expansion data for one call node.
    #[inline]
    pub fn lookup_call_argument_expansion_cache(
        &self,
        call_node_id: LocalNodeId<Expression>,
    ) -> Option<CallArgumentExpansionsCache> {
        self.lookup_node_cache_value(
            &self.node_caches.call_argument_expansions_cache,
            call_node_id.id,
        )
    }

    /// Store regular and chain call argument expansion data for one call node.
    #[inline]
    pub fn store_call_argument_expansion_cache(
        &self,
        call_node_id: LocalNodeId<Expression>,
        expansions: CallArgumentExpansionsCache,
    ) {
        self.store_node_cache_value(
            &self.node_caches.call_argument_expansions_cache,
            call_node_id.id,
            expansions,
        );
    }

    /// Compute annotation data for one argument node.
    fn update_argument_annotation_cache_from_argument_annotations(
        &self,
        argument_id: LocalNodeId<Argument>,
        argument_end: u32,
        annotation_cache: &mut ArgumentAnnotationCache,
    ) {
        // argument annotations
        if !self.has_annotation(argument_id) {
            return;
        }

        self.visit_annotations(argument_id, |annotations| {
            for annotation_id in annotations {
                let annotation = self.annotation(*annotation_id);

                match annotation {
                    Annotation::Blank { .. } => {}
                    Annotation::Doc { position, .. }
                    | Annotation::Decorator { position, .. }
                    | Annotation::Comment { position, .. } => {
                        if Self::is_prefix_annotation_position(position) {
                            annotation_cache.has_prefix_annotation = true;
                        }

                        let Annotation::Comment { node, .. } = annotation else {
                            continue;
                        };
                        annotation_cache.has_comment = true;

                        let comment = self.tree.get::<Comment>(node);
                        if comment.style != ast::CommentStyle::Slash {
                            continue;
                        }

                        let annotation_span = self.annotation_span(*annotation_id);
                        if annotation_span.start >= argument_end {
                            annotation_cache.has_line_comment = true;
                        }
                        if Self::is_prefix_annotation_position(position) {
                            annotation_cache.has_prefix_line_comment = true;
                        }
                    }
                }
            }
        });
    }

    /// Update one argument annotation cache from one argument value expression.
    fn update_argument_annotation_cache_from_value_annotations(
        &self,
        argument_id: LocalNodeId<Argument>,
        annotation_cache: &mut ArgumentAnnotationCache,
    ) {
        // value annotations
        let argument_value_expression = match self.tree.get(argument_id) {
            Argument::Named { value, .. }
            | Argument::Labeled { value, .. }
            | Argument::Positional { value, .. }
            | Argument::Spread { value, .. } => *value,
        };
        let value_id = self.transparent_inner_expression(argument_value_expression);
        let declaration_annotation_target = match self.tree.get(value_id) {
            Expression::Declaration(declaration_id) => Some(*declaration_id),
            _ => None,
        };

        // direct value annotation state
        if self.has_annotation(value_id) {
            annotation_cache.has_comment = true;
            if self.has_prefix_annotation(value_id) {
                annotation_cache.has_prefix_annotation = true;
            }
        }

        // wrapped declaration annotation state
        if let Some(declaration_id) = declaration_annotation_target
            && self.has_annotation(declaration_id)
        {
            annotation_cache.has_comment = true;
            if self.has_prefix_annotation(declaration_id) {
                annotation_cache.has_prefix_annotation = true;
            }
        }
    }

    /// Compute annotation data for one argument node.
    fn compute_argument_annotation_cache(
        &self,
        argument_id: LocalNodeId<Argument>,
    ) -> ArgumentAnnotationCache {
        let mut annotation_cache = ArgumentAnnotationCache::default();
        let argument_span = self.span(argument_id);
        let argument_end = argument_span.end;

        self.update_argument_annotation_cache_from_argument_annotations(
            argument_id,
            argument_end,
            &mut annotation_cache,
        );
        self.update_argument_annotation_cache_from_value_annotations(
            argument_id,
            &mut annotation_cache,
        );

        annotation_cache
    }

    /// Read one copyable value from an index-addressed optional cache.
    fn lookup_node_cache_value<T: Copy>(
        &self,
        cache: &[Cell<Option<T>>],
        node_id: u32,
    ) -> Option<T> {
        cache.get(node_id as usize).and_then(Cell::get)
    }

    /// Write one copyable value into an index-addressed optional cache.
    fn store_node_cache_value<T: Copy>(&self, cache: &[Cell<Option<T>>], node_id: u32, value: T) {
        if let Some(state_cell) = cache.get(node_id as usize) {
            state_cell.set(Some(value));
        }
    }
}
