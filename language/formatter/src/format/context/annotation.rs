use crate::format::context::{
    ANNOTATION_STATE_CACHED, ANNOTATION_STATE_NONE, ANNOTATION_STATE_PRESENT, Annotation,
    AnnotationData, AnnotationPosition, Argument, ArgumentAnnotationCache,
    CallArgumentExpansionsCache, CallArgumentLayoutCache, Cell, Comment, DestackFormatContext,
    Expression, LocalNodeId, Node, NodeTree, NodeTreeImpl, Ref, Span, ast,
};

impl<'a> DestackFormatContext<'a> {
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
        if state == ANNOTATION_STATE_NONE {
            if self.instrumentation_enabled {
                self.cache_stats
                    .annotation_cache_hits
                    .set(self.cache_stats.annotation_cache_hits.get() + 1);
            }
            return None;
        }

        if state == ANNOTATION_STATE_CACHED {
            if self.instrumentation_enabled {
                self.cache_stats
                    .annotation_cache_hits
                    .set(self.cache_stats.annotation_cache_hits.get() + 1);
            }
            let cache = self.annotation_data_by_node_id.borrow();
            return Some(Ref::map(cache, |cache| {
                cache
                    .get(node_index)
                    .and_then(|entry| entry.as_ref())
                    .expect("annotation cache should contain requested node")
            }));
        }

        if self.instrumentation_enabled {
            self.cache_stats
                .annotation_cache_misses
                .set(self.cache_stats.annotation_cache_misses.get() + 1);
        }

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

        let cache = self.annotation_data_by_node_id.borrow();
        Some(Ref::map(cache, |cache| {
            cache
                .get(node_index)
                .and_then(|entry| entry.as_ref())
                .expect("annotation cache should contain requested node")
        }))
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

        if !self.instrumentation_enabled {
            return state != ANNOTATION_STATE_NONE;
        }

        if state == ANNOTATION_STATE_NONE {
            self.cache_stats
                .annotation_cache_hits
                .set(self.cache_stats.annotation_cache_hits.get() + 1);
            return false;
        }
        if state == ANNOTATION_STATE_PRESENT || state == ANNOTATION_STATE_CACHED {
            self.cache_stats
                .annotation_cache_hits
                .set(self.cache_stats.annotation_cache_hits.get() + 1);
            return true;
        }
        false
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
    fn compute_argument_annotation_cache(
        &self,
        argument_id: LocalNodeId<Argument>,
    ) -> ArgumentAnnotationCache {
        let mut annotation_cache = ArgumentAnnotationCache::default();
        let argument_span = self.span(argument_id);
        let argument_end = argument_span.end;

        // argument annotations
        if self.has_annotation(argument_id) {
            self.visit_annotations(argument_id, |annotations| {
                for annotation_id in annotations {
                    let annotation = self.annotation(*annotation_id);

                    match annotation {
                        Annotation::Blank { .. } => {}
                        Annotation::Doc { position, .. }
                        | Annotation::Decorator { position, .. }
                        | Annotation::Comment { position, .. } => {
                            if matches!(
                                position,
                                AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
                            ) {
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
                            if matches!(
                                position,
                                AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
                            ) {
                                annotation_cache.has_prefix_line_comment = true;
                            }
                        }
                    }
                }
            });
        }

        // value annotations
        let argument_value_expression = match self.tree.get(argument_id) {
            Argument::Named { value, .. }
            | Argument::Labeled { value, .. }
            | Argument::Positional { value, .. }
            | Argument::Spread { value, .. } => Some(*value),
        };
        if let Some(value_id) = argument_value_expression {
            let value_id = self.transparent_inner_expression(value_id);
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
