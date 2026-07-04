use destack_dir as dir;
use destack_source::Span;

use crate::ModuleQueryContext;

/// Builder for one reference index from checked DIR.
pub(super) struct ReferenceIndexer<'context, 'query> {
    /// The indexed module context.
    module: &'context ModuleQueryContext<'query>,
    /// The collected index entries.
    entries: Vec<dir::ReferenceEntry>,
}

impl<'context, 'query> ReferenceIndexer<'context, 'query> {
    /// Build the reference index.
    pub(super) fn build(module: &'context ModuleQueryContext<'query>) -> dir::ReferenceIndex {
        let mut indexer = Self {
            module,
            entries: Vec::new(),
        };

        // collect checked reference families
        indexer.collect_resolution_references();
        indexer.collect_expression_references();
        indexer.collect_dependency_references();

        dir::ReferenceIndex::new(indexer.entries)
    }

    /// Collect references from checked resolution tables.
    fn collect_resolution_references(&mut self) {
        // collect name resolution targets
        for (source, resolution) in self.module.resolutions().name_entries() {
            for symbol in resolution.symbols() {
                self.push_reference(*symbol, source, dir::ReferenceKind::Name);
            }
        }

        // collect type instantiation targets
        for (source, resolution) in self.module.resolutions().instantiation_entries() {
            self.push_reference(resolution.symbol, source, dir::ReferenceKind::Type);
        }

        // collect label targets that resolve to symbols
        for (source, resolution) in self.module.resolutions().label_entries() {
            if let dir::LabelResolution::Symbol(symbol) = resolution {
                self.push_reference(*symbol, source, dir::ReferenceKind::Name);
            }
        }

        // collect receiver type declarations
        for (source, resolution) in self.module.resolutions().receiver_entries() {
            self.push_reference(resolution.declaration, source, dir::ReferenceKind::Type);
        }

        // collect member resolution targets
        for (source, resolution) in self.module.resolutions().member_entries() {
            self.push_member_resolution(source, resolution);
        }

        // collect call resolution targets
        for (source, resolution) in self.module.resolutions().call_entries() {
            self.push_call_resolution(source, resolution);
        }

        // collect writable storage targets
        for (source, resolution) in self.module.resolutions().place_entries() {
            self.push_storage(source, &resolution.storage);
        }

        // collect type guard targets
        for (source, resolution) in self.module.resolutions().guard_entries() {
            if let dir::GuardResolution::InstanceOf(guard) = resolution {
                self.push_reference(guard.target, source, dir::ReferenceKind::Type);
            }
        }

        // collect construct resolution targets
        for (source, resolution) in self.module.resolutions().construct_entries() {
            self.push_construct_resolution(source, resolution);
        }
    }

    /// Collect references from expression nodes.
    fn collect_expression_references(&mut self) {
        let view = self.module.view();
        let module_id = self.module.module_id();

        // collect reference targets and path segments from each expression
        for (expression_id, expression) in view.iter_nodes_of_type::<dir::Expression>() {
            let source = expression_id.into_global_any(module_id);

            self.collect_expression_target(expression_id, expression, source);
            self.collect_path_segments(expression_id, source);
        }
    }

    /// Collect references from one expression target.
    fn collect_expression_target(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
        source: dir::GlobalNodeIdAny,
    ) {
        // collect direct expression target
        if let Some(target_symbol) = self.module.expression_reference_target(expression_id) {
            self.push_reference(target_symbol, source, dir::ReferenceKind::Name);
        }

        // collect namespace receiver target on the left side
        if let dir::Expression::Member { left, .. } = expression {
            if let Some(receiver_symbol) = self.module.namespace_receiver_symbol_target(*left) {
                let source = left.into_global_any(self.module.module_id());

                self.push_reference(receiver_symbol, source, dir::ReferenceKind::Name);
            }
        }

        // collect checked direct member access target
        if let dir::Expression::Member { .. } = expression {
            if let Some(member_symbol) = self.module.member_access_symbol_target(expression_id) {
                self.push_reference(member_symbol, source, dir::ReferenceKind::Member);
            }
        }

        // collect member candidate targets attached to this expression
        if let Some(resolution) = self.module.resolutions().member_resolution(source) {
            self.push_member_candidates(source, resolution);
        }

        // collect call target attached to this expression
        if let Some(resolution) = self.module.resolutions().call_resolution(source) {
            self.push_call_resolution(source, resolution);
        }

        // collect construct target attached to this expression
        if let Some(resolution) = self.module.resolutions().construct_resolution(source) {
            self.push_construct_resolution(source, resolution);
        }
    }

    /// Collect references from reference path segments.
    fn collect_path_segments(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        source: dir::GlobalNodeIdAny,
    ) {
        // skip expressions without a reference path
        let Some(path) = self.module.tree().reference_path(expression_id) else {
            return;
        };

        // collect each path segment target
        for segment_index in 0..path.segments.len() {
            // stop if the path exceeds the DIR segment index width
            let Ok(segment_index) = u16::try_from(segment_index) else {
                break;
            };

            // skip unresolved path segments
            let Some(segment_symbol) = self
                .module
                .path_segment_symbol_target(expression_id, segment_index)
            else {
                continue;
            };

            // read the checked source span for the resolved segment
            let span = self
                .module
                .path_segment_span(expression_id, segment_index)
                .unwrap_or_else(|| {
                    panic!(
                        "missing source span for resolved path segment {expression_id:?}/{segment_index}"
                    )
                });

            self.push_reference_at_span(segment_symbol, source, span, dir::ReferenceKind::Name);
        }
    }

    /// Collect references from dependency item nodes.
    fn collect_dependency_references(&mut self) {
        let view = self.module.view();
        let module_id = self.module.module_id();

        // collect dependency local and resolved symbol targets
        for (item_id, _item) in view.iter_nodes_of_type::<dir::DependencyItem>() {
            let source = item_id.into_global_any(module_id);

            if let Some(symbol_id) = self.module.dependency_local_symbol(item_id) {
                self.push_reference(symbol_id, source, dir::ReferenceKind::Dependency);
            }

            if let Some(target_symbol) = self.module.dependency_symbol_target(item_id) {
                self.push_reference(target_symbol, source, dir::ReferenceKind::Dependency);
            }
        }
    }

    /// Push reference row for a symbol.
    fn push_reference(
        &mut self,
        symbol_id: dir::GlobalSymbolId,
        source: dir::GlobalNodeIdAny,
        kind: dir::ReferenceKind,
    ) {
        // use the source node main span
        let span = self
            .module
            .get_main_span(self.module.view(), source.local_id);

        self.push_reference_at_span(symbol_id, source, span, kind);
    }

    /// Push reference row for a symbol using an explicit span.
    fn push_reference_at_span(
        &mut self,
        symbol_id: dir::GlobalSymbolId,
        source: dir::GlobalNodeIdAny,
        span: Span,
        kind: dir::ReferenceKind,
    ) {
        self.entries.push(dir::ReferenceEntry {
            target: symbol_id,
            source,
            span,
            kind,
        });
    }

    /// Push reference rows from one member resolution.
    fn push_member_resolution(
        &mut self,
        source: dir::GlobalNodeIdAny,
        resolution: &dir::MemberResolution,
    ) {
        match &resolution.target {
            dir::MemberTarget::Symbol(candidate) => {
                // collect owner type plus concrete member
                self.push_reference(candidate.owner, source, dir::ReferenceKind::Type);
                self.push_reference(candidate.symbol, source, dir::ReferenceKind::Member);
            }
            dir::MemberTarget::Existential(candidates)
            | dir::MemberTarget::Universal(candidates) => {
                // collect each overload candidate owner and member
                for candidate in candidates {
                    self.push_reference(candidate.owner, source, dir::ReferenceKind::Type);
                    self.push_reference(candidate.symbol, source, dir::ReferenceKind::Member);
                }
            }
            dir::MemberTarget::Field(_)
            | dir::MemberTarget::Element(_)
            | dir::MemberTarget::Index(_) => {}
        }
    }

    /// Push reference rows from expression member candidates.
    fn push_member_candidates(
        &mut self,
        source: dir::GlobalNodeIdAny,
        resolution: &dir::MemberResolution,
    ) {
        match &resolution.target {
            dir::MemberTarget::Symbol(candidate) => {
                // collect concrete member target only
                self.push_reference(candidate.symbol, source, dir::ReferenceKind::Member);
            }
            dir::MemberTarget::Existential(candidates)
            | dir::MemberTarget::Universal(candidates) => {
                // collect each candidate member target
                for candidate in candidates {
                    self.push_reference(candidate.symbol, source, dir::ReferenceKind::Member);
                }
            }
            dir::MemberTarget::Field(_)
            | dir::MemberTarget::Element(_)
            | dir::MemberTarget::Index(_) => {}
        }
    }

    /// Push reference rows from one call resolution.
    fn push_call_resolution(
        &mut self,
        source: dir::GlobalNodeIdAny,
        resolution: &dir::CallResolution,
    ) {
        match &resolution.target {
            dir::CallTarget::Symbol(candidate) => {
                // collect concrete callee
                self.push_reference(candidate.symbol, source, dir::ReferenceKind::Call);
            }
            dir::CallTarget::Universal(candidates) => {
                // collect each overload candidate callee
                for candidate in candidates {
                    self.push_reference(candidate.symbol, source, dir::ReferenceKind::Call);
                }
            }
            dir::CallTarget::Builtin(_) | dir::CallTarget::Expression { .. } => {}
        }
    }

    /// Push reference rows from one construct resolution.
    fn push_construct_resolution(
        &mut self,
        source: dir::GlobalNodeIdAny,
        resolution: &dir::ConstructResolution,
    ) {
        let symbol = match &resolution.target {
            dir::ConstructTarget::Class(candidate) => candidate.symbol,
            dir::ConstructTarget::Newtype(candidate) => candidate.symbol,
            dir::ConstructTarget::Variant(candidate) => candidate.case.member,
        };

        self.push_reference(symbol, source, dir::ReferenceKind::Construct);
    }

    /// Push reference rows from one writable storage selection.
    fn push_storage(&mut self, source: dir::GlobalNodeIdAny, storage: &dir::Storage) {
        match storage {
            dir::Storage::Binding { symbol } => {
                // collect local binding storage
                self.push_reference(*symbol, source, dir::ReferenceKind::Storage);
            }
            dir::Storage::Property { read, write } => {
                // collect optional read and required write property targets
                if let Some(read) = read {
                    self.push_member_resolution(source, read);
                }

                self.push_member_resolution(source, write);
            }
            dir::Storage::Subscript { read, write, .. } => {
                // collect optional read and required write subscript targets
                if let Some(read) = read {
                    self.push_subscript(source, read);
                }

                self.push_subscript(source, write);
            }
            dir::Storage::Dereference { read, write } => {
                // collect optional read and required write dereference targets
                if let Some(read) = read {
                    self.push_dereference(source, read);
                }

                self.push_dereference(source, write);
            }
            dir::Storage::Field { .. } => {}
        }
    }

    /// Push reference rows from one subscript operation.
    fn push_subscript(
        &mut self,
        source: dir::GlobalNodeIdAny,
        operation: &dir::SubscriptOperation,
    ) {
        match operation {
            dir::SubscriptOperation::Member(resolution) => {
                self.push_member_resolution(source, resolution);
            }
            dir::SubscriptOperation::Call(resolution) => {
                self.push_call_resolution(source, resolution);
            }
        }
    }

    /// Push reference rows from one dereference operation.
    fn push_dereference(
        &mut self,
        source: dir::GlobalNodeIdAny,
        operation: &dir::DereferenceOperation,
    ) {
        match operation {
            dir::DereferenceOperation::Direct => {}
            dir::DereferenceOperation::Call(resolution) => {
                self.push_call_resolution(source, resolution);
            }
        }
    }
}
