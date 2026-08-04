use std::mem;

use destack_dir as dir;
use destack_repository::{ProviderError, ProviderResult};
use destack_source::{NodeSpanList, NodeSpanRegion, NodeSpanType, Span};
use rustc_hash::{FxHashMap, FxHashSet};

use super::context::ModuleIndexContext;

/// Builder for one reference index from recorded resolutions.
pub(in crate::index) struct ReferenceIndexer<'context, 'index> {
    /// The indexed module context.
    module: &'context ModuleIndexContext<'index>,
    /// References keyed by their final resolved target.
    target_entries: Vec<dir::ReferenceEntry>,
    /// References keyed by their lexical declaration.
    declaration_entries: Vec<dir::ReferenceEntry>,
    /// The final selected targets keyed by their authored source node.
    selected_targets: FxHashMap<dir::GlobalNodeIdAny, Vec<dir::GlobalSymbolId>>,
    /// The resolution nodes represented by one selected authored occurrence.
    selected_sources: FxHashSet<dir::GlobalNodeIdAny>,
}

impl<'context, 'index> ReferenceIndexer<'context, 'index> {
    /// Build the reference index.
    pub(in crate::index) fn build(
        module: &'context ModuleIndexContext<'index>,
    ) -> ProviderResult<dir::ReferenceIndex> {
        let mut indexer = Self {
            module,
            target_entries: Vec::new(),
            declaration_entries: Vec::new(),
            selected_targets: FxHashMap::default(),
            selected_sources: FxHashSet::default(),
        };

        // select the final targets before transcribing ordinary resolutions
        indexer.collect_selected_targets()?;
        indexer.collect_resolutions()?;
        indexer.collect_declaration_references()?;
        indexer.collect_dependencies()?;

        Ok(dir::ReferenceIndex::new(
            indexer.target_entries,
            indexer.declaration_entries,
        ))
    }

    /// Collect targets selected after name and member lookup.
    fn collect_selected_targets(&mut self) -> ProviderResult<()> {
        // record explicit generic selections
        for (source, resolution) in self.module.resolutions().instantiation_entries() {
            let sources = self.instantiation_sources(source)?;
            self.select(sources, vec![resolution.symbol])?;
        }

        // record symbol-backed call selections
        for (source, resolution) in self.module.resolutions().call_entries() {
            let targets = resolution
                .iter()
                .filter_map(|call| call.target.symbol())
                .collect::<Vec<_>>();
            if targets.is_empty() {
                continue;
            }
            let sources = self.call_sources(source)?;
            self.select(sources, targets)?;
        }

        // record nominal construction selections
        for (source, resolution) in self.module.resolutions().construct_entries() {
            let Some(symbol) = resolution.target.symbol() else {
                continue;
            };
            let sources = self.construct_sources(source)?;
            self.select(sources, vec![symbol])?;
        }

        Ok(())
    }

    /// Collect reference occurrences recorded by the checker.
    fn collect_resolutions(&mut self) -> ProviderResult<()> {
        // collect resolved names
        for (source, resolution) in self.module.resolutions().name_entries() {
            // dependency names use their exact imported-name occurrence
            if source.local_id.ty == dir::NodeType::DependencyItem {
                continue;
            }

            // final call, construction, and instantiation selections replace inner resolutions
            if self.selected_sources.contains(&source) {
                continue;
            }

            for symbol in resolution.symbols() {
                self.push(*symbol, source)?;
            }
        }

        // collect resolved symbol labels
        for (source, resolution) in self.module.resolutions().label_entries() {
            if let dir::LabelResolution::Symbol(symbol) = resolution {
                self.push(*symbol, source)?;
            }
        }

        // collect resolved receiver declarations
        for (source, resolution) in self.module.resolutions().receiver_entries() {
            self.push(resolution.declaration, source)?;
        }

        // collect resolved members
        for (source, resolution) in self.module.resolutions().member_entries() {
            if !self.selected_sources.contains(&source) {
                self.push_member(source, resolution)?;
            }
        }

        // collect protocol operator targets
        for (source, resolution) in self.module.resolutions().operator_entries() {
            for application in resolution.iter() {
                if let Some(call) = application.call() {
                    self.push_call(source, call)?;
                }
            }
        }

        // collect subscript read targets
        for (source, resolution) in self.module.resolutions().subscript_entries() {
            self.push_subscript(source, resolution)?;
        }

        // collect assignment read and write targets
        for (source, resolution) in self.module.resolutions().assignment_entries() {
            if let Some(read) = &resolution.read {
                self.push_read(source, read)?;
            }
            self.push_write(source, &resolution.write)?;
        }

        // collect type guard targets
        for (source, resolution) in self.module.resolutions().guard_entries() {
            if let dir::GuardResolution::InstanceOf(guard) = resolution {
                self.push(guard.target, source)?;
            }
        }

        // emit the final selections once per authored occurrence
        for (source, targets) in mem::take(&mut self.selected_targets) {
            for target in targets {
                self.push(target, source)?;
            }
        }

        Ok(())
    }

    /// Collect lexical declaration occurrences recorded during resolution.
    fn collect_declaration_references(&mut self) -> ProviderResult<()> {
        for (source, declarations) in &self.module.resolved().references.declarations_by_node {
            let span = self.declaration_span(*source)?;

            // index every exact lexical declaration at the authored occurrence
            for declaration in declarations {
                self.declaration_entries.push(dir::ReferenceEntry {
                    symbol: *declaration,
                    source: *source,
                    span,
                    is_import_alias: false,
                });
            }
        }

        Ok(())
    }

    /// Collect explicit imported-name occurrences from dependency references.
    fn collect_dependencies(&mut self) -> ProviderResult<()> {
        let imported_name = NodeSpanType::Region(NodeSpanRegion::Type);

        // transcribe each concrete dependency target with an authored remote name
        for (source, reference) in &self.module.resolved().references.target_by_node {
            if source.local_id.ty != dir::NodeType::DependencyItem {
                continue;
            }
            let dir::Reference::Bound(targets) = reference else {
                continue;
            };
            if targets.is_empty() {
                return Err(ProviderError::internal(format!(
                    "dependency reference {source:?} has no bound target"
                ))
                .into());
            }
            let view = self.module.view();
            let item_id = source
                .local_id
                .try_into_typed::<dir::DependencyItem>()
                .map_err(|_| {
                    ProviderError::internal(format!(
                        "dependency reference has incompatible node id: {source:?}"
                    ))
                })?;
            let item = view.get(item_id);
            let dir::DependencyItem::Binding {
                binding,
                name,
                alias,
                ..
            } = item
            else {
                return Err(ProviderError::internal(format!(
                    "dependency reference has malformed declaration: {source:?}"
                ))
                .into());
            };

            let source_id = view.get_source_any(source.local_id);
            let span = match (name, binding, alias) {
                // named dependencies retain their remote name separately
                (Some(_), _, _) => self
                    .module
                    .source_index()
                    .get_side(source_id, imported_name)
                    .ok_or_else(|| {
                        ProviderError::internal(format!(
                            "dependency reference {source:?} has no imported-name span"
                        ))
                    })?,

                // a renamed default re-export exposes its only authored identifier
                (
                    None,
                    dir::DependencyBinding::Default | dir::DependencyBinding::Namespace,
                    Some(_),
                ) => {
                    let Some(parent) = view.get_parent_for(item_id) else {
                        return Err(ProviderError::internal(format!(
                            "dependency reference {source:?} has no parent"
                        ))
                        .into());
                    };
                    if parent.ty != dir::NodeType::Expression {
                        return Err(ProviderError::internal(format!(
                            "dependency reference {source:?} has incompatible parent {parent:?}"
                        ))
                        .into());
                    }
                    let parent = dir::LocalNodeId::<dir::Expression>::new(parent.id);
                    if !matches!(view.get(parent), dir::Expression::Export { .. }) {
                        continue;
                    }

                    self.module
                        .source_index()
                        .get_main(source_id)
                        .ok_or_else(|| {
                            ProviderError::internal(format!(
                                "default re-export reference {source:?} has no alias span"
                            ))
                        })?
                }

                // bare default and namespace tokens have no symbol name occurrence
                (
                    None,
                    dir::DependencyBinding::Default
                    | dir::DependencyBinding::Namespace
                    | dir::DependencyBinding::Named,
                    None,
                ) => continue,
                (None, dir::DependencyBinding::Named, Some(_)) => {
                    return Err(ProviderError::internal(format!(
                        "named dependency reference {source:?} has no imported name"
                    ))
                    .into());
                }
            };

            // emit the authored remote name for every exact bound declaration
            for target in targets {
                self.push_dependency(*target, *source, span);
            }
        }

        Ok(())
    }

    /// Push one authored reference occurrence.
    fn push(
        &mut self,
        target: dir::GlobalSymbolId,
        source: dir::GlobalNodeIdAny,
    ) -> ProviderResult<()> {
        let view = self.module.view();
        let source_id = view.get_source_any(source.local_id);

        // generated nodes do not represent source reference occurrences
        if self.module.source_index().try_get(source_id).is_none() {
            return Ok(());
        }

        // skip references without an authored main span
        let Some(span) = self.reference_span(source, source_id, target)? else {
            return Ok(());
        };

        // record explicit local aliases without interpreting dependency chains
        let declarations = self.module.resolved().references.declarations(source);
        let is_import_alias = declarations.is_some_and(|declarations| {
            declarations
                .iter()
                .any(|declaration| self.module.is_local_import_alias(*declaration))
        });
        self.target_entries.push(dir::ReferenceEntry {
            symbol: target,
            source,
            span,
            is_import_alias,
        });

        Ok(())
    }

    /// Return the authored span carrying one lexical declaration identity.
    fn declaration_span(&self, source: dir::GlobalNodeIdAny) -> ProviderResult<Span> {
        let view = self.module.view();
        let source_id = view.get_source_any(source.local_id);

        // qualified type paths retain their declaration identity on the root
        if source.local_id.ty == dir::NodeType::TypeExpression
            && let Some(span) = self.qualified_type_root_span(view, source, source_id)?
        {
            return Ok(span);
        }

        self.module
            .source_index()
            .get_main(source_id)
            .ok_or_else(|| {
                ProviderError::internal(format!(
                    "declaration reference {source:?} has no authored span"
                ))
                .into()
            })
    }

    /// Return the authored span carrying one resolved reference target.
    fn reference_span(
        &self,
        source: dir::GlobalNodeIdAny,
        source_id: u32,
        target: dir::GlobalSymbolId,
    ) -> ProviderResult<Option<Span>> {
        // projected type paths bind the final namespace prefix, not the final source segment
        if source.local_id.ty == dir::NodeType::TypeExpression
            && let Some(dir::Reference::Projected {
                base: dir::ImportTarget::Symbol(base),
                from,
            }) = self.module.resolved().references.get(source)
        {
            if *base != target {
                return Err(ProviderError::internal(format!(
                    "projected type reference {source:?} selected {target:?} instead of {base:?}"
                ))
                .into());
            }

            let segment = from.checked_sub(1).ok_or_else(|| {
                ProviderError::internal(format!(
                    "projected type reference {source:?} starts after no prefix"
                ))
            })?;
            let segment = u16::try_from(segment).map_err(|_| {
                ProviderError::internal(format!(
                    "projected type reference {source:?} segment exceeds source index limits"
                ))
            })?;
            let span_type = NodeSpanType::ListItem(NodeSpanList::Segment, segment);
            let span = self.module.source_index().get_side(source_id, span_type);
            let span = span.ok_or_else(|| {
                ProviderError::internal(format!(
                    "projected type reference {source:?} has no bound-prefix span"
                ))
            })?;

            return Ok(Some(span));
        }

        Ok(self.module.source_index().get_main(source_id))
    }

    /// Return the lexical root span of one qualified type path.
    fn qualified_type_root_span(
        &self,
        view: dir::View<'_>,
        source: dir::GlobalNodeIdAny,
        source_id: u32,
    ) -> ProviderResult<Option<Span>> {
        let type_id = source
            .local_id
            .try_into_typed::<dir::TypeExpression>()
            .map_err(|_| {
                ProviderError::internal(format!(
                    "type reference has incompatible node id: {source:?}"
                ))
            })?;
        let dir::TypeExpression::Reference { path, .. } = view.get(type_id) else {
            return Ok(None);
        };
        if path.segments.len() <= 1 {
            return Ok(None);
        }

        let span = self
            .module
            .source_index()
            .get_side(source_id, NodeSpanType::Head)
            .ok_or_else(|| {
                ProviderError::internal(format!(
                    "qualified type reference {source:?} has no root span"
                ))
            })?;

        Ok(Some(span))
    }

    /// Push one imported target and declaration occurrence.
    fn push_dependency(
        &mut self,
        symbol: dir::GlobalSymbolId,
        source: dir::GlobalNodeIdAny,
        span: Span,
    ) {
        self.target_entries.push(dir::ReferenceEntry {
            symbol,
            source,
            span,
            is_import_alias: false,
        });
        self.declaration_entries.push(dir::ReferenceEntry {
            symbol,
            source,
            span,
            is_import_alias: false,
        });
    }

    /// Push the symbol targets recorded by one member resolution.
    fn push_member(
        &mut self,
        source: dir::GlobalNodeIdAny,
        resolution: &dir::MemberResolution,
    ) -> ProviderResult<()> {
        for access in resolution.iter() {
            self.push_member_target(source, &access.target)?;
        }

        Ok(())
    }

    /// Push the symbol targets selected by one member target.
    fn push_member_target(
        &mut self,
        source: dir::GlobalNodeIdAny,
        target: &dir::MemberTarget,
    ) -> ProviderResult<()> {
        match target {
            dir::MemberTarget::Symbol(candidate) => self.push(candidate.symbol, source)?,
            dir::MemberTarget::Existential(targets) | dir::MemberTarget::Intersection(targets) => {
                for target in targets {
                    self.push_member_target(source, target)?;
                }
            }
            dir::MemberTarget::Call(call) => self.push_call(source, call)?,
            dir::MemberTarget::Field(field) => {
                if let dir::FieldTarget::Member { symbol, .. } = field.target {
                    self.push(symbol, source)?;
                }
            }
            dir::MemberTarget::Projection { .. } | dir::MemberTarget::Index(_) => {}
        }

        Ok(())
    }

    /// Push the symbol target selected by one protocol call.
    fn push_call(&mut self, source: dir::GlobalNodeIdAny, call: &dir::Call) -> ProviderResult<()> {
        if let Some(symbol) = call.target.symbol() {
            self.push(symbol, source)?;
        }

        Ok(())
    }

    /// Push the symbol targets recorded by one subscript resolution.
    fn push_subscript(
        &mut self,
        source: dir::GlobalNodeIdAny,
        resolution: &dir::SubscriptResolution,
    ) -> ProviderResult<()> {
        for subscript in resolution.iter() {
            match &subscript.target {
                dir::SubscriptTarget::Member(member) => {
                    self.push_member_target(source, &member.target)?;
                }
                dir::SubscriptTarget::Call(call) => self.push_call(source, call)?,
                dir::SubscriptTarget::Index(read) => self.push_call(source, &read.call)?,
            }
        }

        Ok(())
    }

    /// Push the symbol targets recorded by one dereference resolution.
    fn push_dereference(
        &mut self,
        source: dir::GlobalNodeIdAny,
        resolution: &dir::DereferenceResolution,
    ) -> ProviderResult<()> {
        for dereference in resolution.iter() {
            if let dir::DereferenceTarget::Call(call) = &dereference.target {
                self.push_call(source, call)?;
            }
        }

        Ok(())
    }

    /// Push the symbol targets recorded by one place read.
    fn push_read(
        &mut self,
        source: dir::GlobalNodeIdAny,
        read: &dir::ReadResolution,
    ) -> ProviderResult<()> {
        match read {
            dir::ReadResolution::Binding { symbol, .. } => self.push(*symbol, source)?,
            dir::ReadResolution::Member(member) => self.push_member(source, member)?,
            dir::ReadResolution::Subscript(subscript) => self.push_subscript(source, subscript)?,
            dir::ReadResolution::Dereference(dereference) => {
                self.push_dereference(source, dereference)?;
            }
        }

        Ok(())
    }

    /// Push the symbol targets recorded by one place write.
    fn push_write(
        &mut self,
        source: dir::GlobalNodeIdAny,
        write: &dir::WriteResolution,
    ) -> ProviderResult<()> {
        match write {
            dir::WriteResolution::Binding { symbol, .. } => self.push(*symbol, source)?,
            dir::WriteResolution::Member(member) => self.push_member(source, member)?,
            dir::WriteResolution::Subscript(subscript) => self.push_subscript(source, subscript)?,
            dir::WriteResolution::Dereference(dereference) => {
                self.push_dereference(source, dereference)?;
            }
        }

        Ok(())
    }

    /// Record one selected occurrence and every resolution node it replaces.
    fn select(
        &mut self,
        sources: Vec<dir::GlobalNodeIdAny>,
        mut targets: Vec<dir::GlobalSymbolId>,
    ) -> ProviderResult<()> {
        let Some(source) = sources.last().copied() else {
            return Err(ProviderError::internal("selected reference has no source node").into());
        };
        targets.sort();
        targets.dedup();
        if targets.is_empty() {
            return Err(ProviderError::internal(format!(
                "selected reference {source:?} has no symbol target"
            ))
            .into());
        }

        // reject conflicting final selections for one authored occurrence
        if let Some(previous) = self.selected_targets.get(&source)
            && previous != &targets
        {
            return Err(ProviderError::internal(format!(
                "reference {source:?} has conflicting selected targets: {previous:?} and \
                 {targets:?}"
            ))
            .into());
        }

        self.selected_targets.insert(source, targets);
        self.selected_sources.extend(sources);

        Ok(())
    }

    /// Return the authored source chain for one generic instantiation.
    fn instantiation_sources(
        &self,
        source: dir::GlobalNodeIdAny,
    ) -> ProviderResult<Vec<dir::GlobalNodeIdAny>> {
        if source.local_id.ty != dir::NodeType::Expression {
            return Err(ProviderError::internal(format!(
                "instantiation resolution source is not an expression: {source:?}"
            ))
            .into());
        }

        self.expression_sources(source)
    }

    /// Return the authored callee source chain for one call resolution.
    fn call_sources(
        &self,
        source: dir::GlobalNodeIdAny,
    ) -> ProviderResult<Vec<dir::GlobalNodeIdAny>> {
        let expression_id = source
            .local_id
            .try_into_typed::<dir::Expression>()
            .map_err(|_| {
                ProviderError::internal(format!(
                    "call resolution source is not an expression: {source:?}"
                ))
            })?;
        let dir::Expression::Call { left, .. } = self.module.view().get(expression_id) else {
            return Err(ProviderError::internal(format!(
                "call resolution source is not a call expression: {source:?}"
            ))
            .into());
        };
        let source = left.into_global_any(self.module.module_id());

        self.expression_sources(source)
    }

    /// Return one expression and the generic wrappers leading to its authored name.
    fn expression_sources(
        &self,
        source: dir::GlobalNodeIdAny,
    ) -> ProviderResult<Vec<dir::GlobalNodeIdAny>> {
        let mut sources = vec![source];
        let mut expression_id = source
            .local_id
            .try_into_typed::<dir::Expression>()
            .map_err(|_| {
                ProviderError::internal(format!(
                    "expression reference source has incompatible node id: {source:?}"
                ))
            })?;

        // cross explicit generic application wrappers
        while let dir::Expression::Instantiation { left, .. } =
            self.module.view().get(expression_id)
        {
            expression_id = *left;
            sources.push(expression_id.into_global_any(self.module.module_id()));
        }

        Ok(sources)
    }

    /// Return the authored target source chain for one construction resolution.
    fn construct_sources(
        &self,
        source: dir::GlobalNodeIdAny,
    ) -> ProviderResult<Vec<dir::GlobalNodeIdAny>> {
        let expression_id = source
            .local_id
            .try_into_typed::<dir::Expression>()
            .map_err(|_| {
                ProviderError::internal(format!(
                    "construct resolution source is not an expression: {source:?}"
                ))
            })?;
        match self.module.view().get(expression_id) {
            dir::Expression::New {
                ty: type_expression,
                ..
            } => Ok(vec![
                type_expression.into_global_any(self.module.module_id()),
            ]),
            dir::Expression::Call { left, .. } => {
                let source = left.into_global_any(self.module.module_id());

                self.expression_sources(source)
            }
            _ => {
                let expression = self.module.view().get(expression_id);

                Err(ProviderError::internal(format!(
                    "construct resolution source is not a construct expression: {source:?}: \
                     {expression:?}"
                ))
                .into())
            }
        }
    }
}
