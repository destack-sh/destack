use destack_dir as dir;
use destack_source::{FileId, Span};

use crate::{Module, ModuleQueryContext, ProgramQueryContext};

/// Reference collection filter for one target symbol.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ReferenceFilter<'a> {
    /// Whether to include direct expression references.
    pub include_expressions: bool,
    /// Whether to include member access references.
    pub include_members: bool,
    /// Whether to include dependency item references.
    pub include_dependency_items: bool,
    /// Whether to include namespace receiver references.
    pub include_namespace_receivers: bool,
    /// Whether to skip dependency aliases that target the symbol.
    pub skip_dependency_aliases: bool,
    /// An optional target name for name span resolution.
    pub target_name: Option<&'a str>,
    /// Whether expression-like references must keep the same visible name.
    pub requires_target_name_match: bool,
    /// The optional file filter for collected spans.
    pub limit_file: Option<FileId>,
}

impl ReferenceFilter<'_> {
    /// Build a rename reference filter for one target name.
    pub(crate) fn rename(target_name: &str) -> ReferenceFilter<'_> {
        ReferenceFilter {
            include_expressions: true,
            include_members: true,
            include_dependency_items: true,
            include_namespace_receivers: true,
            skip_dependency_aliases: true,
            target_name: Some(target_name),
            requires_target_name_match: true,
            limit_file: None,
        }
    }

    /// Return whether one indexed reference kind is enabled.
    fn includes_kind(&self, kind: dir::ReferenceKind) -> bool {
        match kind {
            dir::ReferenceKind::Dependency => self.include_dependency_items,
            dir::ReferenceKind::Member => self.include_members,
            dir::ReferenceKind::Name
            | dir::ReferenceKind::Call
            | dir::ReferenceKind::Construct
            | dir::ReferenceKind::Type
            | dir::ReferenceKind::Storage => self.include_expressions,
        }
    }

    /// Return the exact target name required for source span filtering.
    fn required_target_name(&self) -> Option<&str> {
        if !self.requires_target_name_match {
            return None;
        }

        Some(
            self.target_name
                .unwrap_or_else(|| panic!("reference search requires a target name")),
        )
    }
}

impl ModuleQueryContext<'_> {
    /// Return indexed program reference spans for one symbol.
    pub(crate) fn program_symbol_references(
        &self,
        program: &ProgramQueryContext<'_>,
        target_symbol: dir::GlobalSymbolId,
        options: ReferenceFilter<'_>,
    ) -> Vec<(Module, Span)> {
        let mut references = Vec::new();

        for reference in program.symbol_program_references(target_symbol) {
            let entry = reference.entry;
            if !self.reference_entry_matches(entry, options) {
                continue;
            }

            let query_module = Module {
                module_id: entry.source.module_id,
                profile_id: reference.profile_id,
            };
            references.push((query_module, entry.span));
        }

        references.sort_by_key(|(module, span)| {
            (
                module.profile_id,
                module.module_id,
                span.file,
                span.start,
                span.end,
            )
        });
        references.dedup();

        references
    }

    /// Collect local reference spans by scanning checked DIR.
    pub(crate) fn symbol_references(
        &self,
        target_symbol: dir::GlobalSymbolId,
        options: ReferenceFilter<'_>,
    ) -> Vec<Span> {
        let mut spans = Vec::new();

        if options.include_expressions {
            self.collect_expression_references(target_symbol, options, &mut spans);
        }

        if options.include_members {
            self.collect_member_references(target_symbol, options, &mut spans);
        }

        if options.include_dependency_items {
            self.collect_dependency_references(target_symbol, options, &mut spans);
        }

        spans.sort_by_key(|span| (span.file, span.start, span.end));
        spans.dedup();

        spans
    }

    /// Return the target symbol for one expression reference.
    pub(crate) fn expression_reference_target(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::GlobalSymbolId> {
        if self.is_member_receiver_expression(expression_id) {
            return self.expression_symbol_target(expression_id);
        }

        self.expression_symbol_target(expression_id)
    }

    /// Return whether one indexed reference is allowed by the search options.
    fn reference_entry_matches(
        &self,
        entry: dir::ReferenceEntry,
        options: ReferenceFilter<'_>,
    ) -> bool {
        if options
            .limit_file
            .is_some_and(|file_id| entry.span.file != file_id)
        {
            return false;
        }

        if !options.includes_kind(entry.kind) {
            return false;
        }

        self.reference_span_matches_name(entry.span, options)
    }

    /// Collect expression references from the local checked DIR view.
    fn collect_expression_references(
        &self,
        target_symbol: dir::GlobalSymbolId,
        options: ReferenceFilter<'_>,
        spans: &mut Vec<Span>,
    ) {
        for (expression_id, expression) in self.view().iter_nodes_of_type::<dir::Expression>() {
            if options.include_members && matches!(expression, dir::Expression::Member { .. }) {
                continue;
            }

            if let Some(symbol_id) = self.expression_reference_target(expression_id) {
                self.push_node_reference_span(
                    symbol_id,
                    target_symbol,
                    options,
                    expression_id.into(),
                    spans,
                );
            }

            self.collect_path_segment_references(expression_id, target_symbol, options, spans);
        }
    }

    /// Collect path segment references from one expression.
    fn collect_path_segment_references(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        target_symbol: dir::GlobalSymbolId,
        options: ReferenceFilter<'_>,
        spans: &mut Vec<Span>,
    ) {
        let Some(path) = self.tree().reference_path(expression_id) else {
            return;
        };

        for segment_index in 0..path.segments.len() {
            let Ok(segment_index) = u16::try_from(segment_index) else {
                break;
            };
            let Some(symbol_id) = self.path_segment_symbol_target(expression_id, segment_index)
            else {
                continue;
            };
            let span = self
                .path_segment_span(expression_id, segment_index)
                .unwrap_or_else(|| {
                    panic!(
                        "missing source span for resolved path segment {expression_id:?}/{segment_index}"
                    )
                });

            self.push_span_if_matching(symbol_id, target_symbol, options, span, spans);
        }
    }

    /// Collect member references from the local checked DIR view.
    fn collect_member_references(
        &self,
        target_symbol: dir::GlobalSymbolId,
        options: ReferenceFilter<'_>,
        spans: &mut Vec<Span>,
    ) {
        for (expression_id, expression) in self.view().iter_nodes_of_type::<dir::Expression>() {
            let dir::Expression::Member { left, .. } = expression else {
                continue;
            };

            if let Some(symbol_id) = self.member_access_symbol_target(expression_id) {
                if let Some(span) = self.member_access_name_span(expression_id) {
                    self.push_span_if_matching(symbol_id, target_symbol, options, span, spans);
                }
            }

            if options.include_namespace_receivers {
                if let Some(symbol_id) = self.expression_symbol_target(*left) {
                    let span = self.get_main_span(self.view(), (*left).into());
                    self.push_span_if_matching(symbol_id, target_symbol, options, span, spans);
                }
            }
        }
    }

    /// Collect dependency references from the local checked DIR view.
    fn collect_dependency_references(
        &self,
        target_symbol: dir::GlobalSymbolId,
        options: ReferenceFilter<'_>,
        spans: &mut Vec<Span>,
    ) {
        for (item_id, _item) in self.view().iter_nodes_of_type::<dir::DependencyItem>() {
            if let Some(symbol_id) = self.dependency_symbol_target(item_id) {
                self.push_node_reference_span(
                    symbol_id,
                    target_symbol,
                    options,
                    item_id.into(),
                    spans,
                );
            }

            if options.skip_dependency_aliases {
                continue;
            }

            if let Some(symbol_id) = self.dependency_local_symbol(item_id) {
                self.push_node_reference_span(
                    symbol_id,
                    target_symbol,
                    options,
                    item_id.into(),
                    spans,
                );
            }
        }
    }

    /// Push one node reference span when it matches the target.
    fn push_node_reference_span(
        &self,
        symbol_id: dir::GlobalSymbolId,
        target_symbol: dir::GlobalSymbolId,
        options: ReferenceFilter<'_>,
        node_id: dir::LocalNodeIdAny,
        spans: &mut Vec<Span>,
    ) {
        let span = self.get_main_span(self.view(), node_id);

        self.push_span_if_matching(symbol_id, target_symbol, options, span, spans);
    }

    /// Push one span when the symbol and source text match.
    fn push_span_if_matching(
        &self,
        symbol_id: dir::GlobalSymbolId,
        target_symbol: dir::GlobalSymbolId,
        options: ReferenceFilter<'_>,
        span: Span,
        spans: &mut Vec<Span>,
    ) {
        if !self.symbol_matches_reference_target(symbol_id, target_symbol) {
            return;
        }

        if options
            .limit_file
            .is_some_and(|file_id| span.file != file_id)
        {
            return;
        }

        if !self.reference_span_matches_name(span, options) {
            return;
        }

        spans.push(span);
    }

    /// Return whether one source span matches the requested target name.
    fn reference_span_matches_name(&self, span: Span, options: ReferenceFilter<'_>) -> bool {
        let Some(target_name) = options.required_target_name() else {
            return true;
        };
        let file = self
            .repository()
            .file(self.revision(), span.file)
            .unwrap_or_else(|error| {
                panic!("failed to read reference file {:?}: {error}", span.file)
            })
            .unwrap_or_else(|| panic!("missing reference file {:?}", span.file));

        file.span_str(span) == target_name
    }

    /// Return whether an expression is used as a member access receiver.
    fn is_member_receiver_expression(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        let view = self.view();
        let Some(parent) = view.get_parent_for(expression_id) else {
            return false;
        };
        if parent.ty != dir::NodeType::Expression {
            return false;
        }
        let Ok(parent_expression_id) = parent.try_into() else {
            return false;
        };

        let parent_expression = view.get::<dir::Expression>(parent_expression_id);

        matches!(parent_expression, dir::Expression::Member { left, .. } if *left == expression_id)
    }
}
