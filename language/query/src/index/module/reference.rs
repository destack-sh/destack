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
    /// References keyed by their selected target.
    target_entries: Vec<dir::ReferenceEntry>,
    /// References keyed by their lexical declaration.
    declaration_entries: Vec<dir::ReferenceEntry>,
    /// Final selected targets keyed by their authored source node.
    targets_by_source: FxHashMap<dir::GlobalNodeIdAny, Vec<dir::GlobalSymbolId>>,
    /// Lower-level resolution nodes superseded by selected targets.
    superseded_sources: FxHashSet<dir::GlobalNodeIdAny>,
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
            targets_by_source: FxHashMap::default(),
            superseded_sources: FxHashSet::default(),
        };

        // select call and member targets before indexing ordinary resolutions
        indexer.collect_selections()?;
        indexer.collect_resolutions()?;
        indexer.collect_reference_declarations()?;
        indexer.collect_dependency_bindings()?;
        indexer.collect_dependency_names()?;

        Ok(dir::ReferenceIndex::new(
            indexer.target_entries,
            indexer.declaration_entries,
        ))
    }

    /// Collect targets selected after name and member lookup.
    fn collect_selections(&mut self) -> ProviderResult<()> {
        for (source, resolution) in self.module.decisions().decision_entries() {
            match resolution {
                // record explicit generic selections
                dir::Decision::Instantiation(resolution) => {
                    let sources = self.instantiation_sources(source)?;
                    self.record_selection(sources, vec![resolution.symbol])?;
                }
                // record symbol-backed call selections
                dir::Decision::Call(resolution) => {
                    let targets = resolution
                        .iter()
                        .filter_map(|call| call.target.symbol())
                        .collect::<Vec<_>>();
                    if targets.is_empty() {
                        continue;
                    }
                    let sources = self.call_sources(source)?;
                    self.record_selection(sources, targets)?;
                }
                // record nominal construction selections
                dir::Decision::Construct(resolution) => {
                    let Some(symbol) = resolution.target.symbol() else {
                        continue;
                    };
                    let sources = self.construct_sources(source)?;
                    self.record_selection(sources, vec![symbol])?;
                }
                // record the exact variant selected by nominal patterns
                dir::Decision::Pattern(dir::PatternDecision::Variant(resolution)) => {
                    let pattern =
                        source
                            .local_id
                            .try_into_typed::<dir::Pattern>()
                            .map_err(|_| {
                                ProviderError::internal(format!(
                                    "pattern decision source is not a pattern: {source:?}"
                                ))
                            })?;
                    let source = match self.module.view().get(pattern) {
                        dir::Pattern::NominalTuple { ty, .. }
                        | dir::Pattern::NominalObject { ty, .. } => {
                            ty.into_global_any(self.module.module_id())
                        }
                        dir::Pattern::Expression { value } => {
                            value.into_global_any(self.module.module_id())
                        }
                        pattern => {
                            return Err(ProviderError::internal(format!(
                                "variant decision has incompatible pattern {pattern:?}: {source:?}"
                            ))
                            .into());
                        }
                    };
                    self.record_selection(vec![source], vec![resolution.case.variant])?;
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Collect reference occurrences recorded by the checker.
    fn collect_resolutions(&mut self) -> ProviderResult<()> {
        // collect resolved lexical names
        let names = self
            .module
            .resolutions()
            .name_entries()
            .map(|(source, name)| (source, name.clone()))
            .collect::<Vec<_>>();
        for (source, resolution) in names {
            // dependency names use their exact imported-name occurrence
            if source.local_id.ty == dir::NodeType::DependencyItem {
                continue;
            }

            // final selections replace inner resolutions
            if self.superseded_sources.contains(&source) {
                continue;
            }

            for symbol in resolution.symbols() {
                self.index_reference(*symbol, source)?;
            }
        }

        for (source, resolution) in self.module.decisions().decision_entries() {
            match resolution {
                // collect resolved receiver declarations
                dir::Decision::Receiver(resolution) => {
                    self.index_reference(resolution.declaration, source)?;
                }
                // collect resolved members
                dir::Decision::Member(resolution) => {
                    if !self.superseded_sources.contains(&source) {
                        self.index_member(source, resolution)?;
                    }
                }
                // collect protocol operator targets
                dir::Decision::Operator(resolution) => {
                    for application in resolution.iter() {
                        if let Some(call) = application.call() {
                            self.index_call(source, call)?;
                        }
                    }
                }
                // collect subscript read targets
                dir::Decision::Subscript(resolution) => {
                    self.index_subscript(source, resolution)?;
                }
                // collect assignment read and write targets
                dir::Decision::Assignment(resolution) => {
                    if let Some(read) = &resolution.read {
                        self.index_read(source, read)?;
                    }
                    self.index_write(source, &resolution.write)?;
                }
                // collect type guard targets
                dir::Decision::Guard(resolution) => {
                    if let dir::GuardDecision::InstanceOf(guard) = resolution {
                        self.index_reference(guard.target, source)?;
                    }
                }
                _ => {}
            }
        }

        // emit the final selections once per authored occurrence
        for (source, targets) in mem::take(&mut self.targets_by_source) {
            for target in targets {
                self.index_reference(target, source)?;
            }
        }

        Ok(())
    }

    /// Collect authored declarations that differ from their targets.
    fn collect_reference_declarations(&mut self) -> ProviderResult<()> {
        // FUGU #Incomplete: DIR must retain declaration symbols for namespace references
        for (source, reference) in &self.module.resolved().references.declaration_by_node {
            // dependency names use their exact imported-name occurrence
            if source.local_id.ty == dir::NodeType::DependencyItem {
                continue;
            }

            let dir::Reference::Bound(declarations) = reference else {
                continue;
            };
            if declarations.is_empty() {
                return Err(ProviderError::internal(format!(
                    "reference declaration {source:?} has no symbol"
                ))
                .into());
            }

            // index the authored occurrence by every exact declaration
            let span = self.declaration_span(*source)?;
            for declaration in declarations {
                let entry = dir::ReferenceEntry {
                    symbol: *declaration,
                    source: *source,
                    span,
                    is_alias: false,
                };
                self.declaration_entries.push(entry);
            }
        }

        Ok(())
    }

    /// Collect local bindings declared by dependency items.
    fn collect_dependency_bindings(&mut self) -> ProviderResult<()> {
        // retain only local dependency bindings
        for (source, symbol_id) in self.module.bindings().declaration_symbols() {
            if source.local_id.ty != dir::NodeType::DependencyItem {
                continue;
            }
            let symbol = self.module.bindings().get_symbol(symbol_id);
            if !matches!(
                symbol.kind,
                dir::SymbolKind::Import | dir::SymbolKind::ExportAlias
            ) {
                continue;
            }

            let span = self.declaration_span(source)?;
            self.declaration_entries.push(dir::ReferenceEntry {
                symbol: symbol_id.into_global(self.module.module_id()),
                source,
                span,
                is_alias: false,
            });
        }

        Ok(())
    }

    /// Collect imported names from resolved dependency items.
    fn collect_dependency_names(&mut self) -> ProviderResult<()> {
        let imported_name = NodeSpanType::Region(NodeSpanRegion::Type);

        // record each concrete dependency target with an authored remote name
        for (source, target) in &self.module.resolved().references.target_by_node {
            if source.local_id.ty != dir::NodeType::DependencyItem {
                continue;
            }
            let declaration = self
                .module
                .resolved()
                .references
                .declaration(*source)
                .ok_or_else(|| {
                    ProviderError::internal(format!(
                        "dependency reference {source:?} has no declaration"
                    ))
                })?;
            let targets = Self::dependency_symbols(target, *source)?;
            let declarations = Self::dependency_symbols(declaration, *source)?;
            if targets.is_empty() && declarations.is_empty() {
                continue;
            }

            // require an authored dependency item
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

                // renamed default and namespace re-exports expose their only identifier
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

            // preserve whether the imported name denotes a public alias
            let is_alias = alias.is_none() && declaration != target;
            self.index_dependency(&targets, &declarations, *source, span, is_alias);
        }

        Ok(())
    }

    /// Return symbol identities from one dependency reference.
    fn dependency_symbols(
        reference: &dir::Reference,
        source: dir::GlobalNodeIdAny,
    ) -> ProviderResult<Vec<dir::GlobalSymbolId>> {
        let symbols = match reference {
            dir::Reference::Bound(symbols) => symbols.to_vec(),
            dir::Reference::Ambiguous(targets) => targets
                .iter()
                .filter_map(|target| match target {
                    dir::ReferenceTarget::Symbol(symbol) => Some(*symbol),
                    dir::ReferenceTarget::Namespace(_) => None,
                })
                .collect(),
            dir::Reference::Namespace { .. } | dir::Reference::Missing => Vec::new(),
            dir::Reference::Projected { .. } => {
                return Err(ProviderError::internal(format!(
                    "dependency reference {source:?} has a projected target"
                ))
                .into());
            }
        };

        Ok(symbols)
    }

    /// Index one authored reference occurrence.
    fn index_reference(
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

        // require one authored span for every recorded source reference
        let span = self.reference_span(source, source_id, target)?;

        self.index_reference_span(target, source, span)?;

        Ok(())
    }

    /// Index one authored reference occurrence at its exact selection span.
    fn index_reference_span(
        &mut self,
        target: dir::GlobalSymbolId,
        source: dir::GlobalNodeIdAny,
        span: Span,
    ) -> ProviderResult<()> {
        let is_alias = self.reference_names_alias(source, target)?;
        self.target_entries.push(dir::ReferenceEntry {
            symbol: target,
            source,
            span,
            is_alias,
        });

        Ok(())
    }

    /// Return whether one occurrence names an explicit import or export alias.
    fn reference_names_alias(
        &self,
        source: dir::GlobalNodeIdAny,
        target: dir::GlobalSymbolId,
    ) -> ProviderResult<bool> {
        // inspect the exact authored declarations
        let Some(dir::Reference::Bound(declarations)) =
            self.module.resolved().references.declaration(source)
        else {
            return Ok(false);
        };
        if declarations.contains(&target) {
            return Ok(false);
        }

        // classify declarations that name another target
        let bindings = self.module.bindings();
        for declaration in declarations {
            if declaration.module_id != self.module.module_id() {
                return Ok(true);
            }

            let symbol = bindings.get_symbol(declaration.local_id);
            if symbol.kind == dir::SymbolKind::ExportAlias
                || self.module.is_local_import_alias(*declaration)
            {
                return Ok(true);
            }
            if symbol.kind != dir::SymbolKind::Import {
                continue;
            }

            // follow the exact declaration and target retained for this import
            let resolution = self
                .module
                .resolved()
                .imports
                .symbol_resolution(declaration.local_id)
                .ok_or_else(|| {
                    ProviderError::internal(format!(
                        "import declaration {declaration:?} has no resolution"
                    ))
                })?;
            if Self::import_names_alias(resolution)? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Return whether one import names an exported alias for the given target.
    fn import_names_alias(resolution: &dir::ImportResolution) -> ProviderResult<bool> {
        let resolutions = match resolution {
            dir::ImportResolution::Resolved(resolution) => std::slice::from_ref(resolution),
            dir::ImportResolution::Ambiguous(resolutions) => resolutions.as_slice(),
            dir::ImportResolution::Missing => {
                return Err(ProviderError::internal("import declaration has no target").into());
            }
        };

        // compare every authored declaration with its paired target
        let is_alias = resolutions
            .iter()
            .any(|resolution| resolution.declaration != resolution.target);

        Ok(is_alias)
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
    ) -> ProviderResult<Span> {
        // selected targets replace projected prefix bindings
        if self.superseded_sources.contains(&source) {
            return self
                .module
                .source_index()
                .get_main(source_id)
                .ok_or_else(|| {
                    ProviderError::internal(format!(
                        "selected reference {source:?} has no authored main span"
                    ))
                    .into()
                });
        }

        // projected type paths bind the namespace prefix, not the projected source segment
        if source.local_id.ty == dir::NodeType::TypeExpression
            && let Some(dir::Reference::Projected {
                base: dir::ReferenceTarget::Symbol(base),
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

            return Ok(span);
        }

        self.module
            .source_index()
            .get_main(source_id)
            .ok_or_else(|| {
                ProviderError::internal(format!(
                    "reference {source:?} to {target:?} has no authored main span"
                ))
                .into()
            })
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

    /// Index imported target and declaration occurrences.
    fn index_dependency(
        &mut self,
        targets: &[dir::GlobalSymbolId],
        declarations: &[dir::GlobalSymbolId],
        source: dir::GlobalNodeIdAny,
        span: Span,
        is_alias: bool,
    ) {
        // index target identities
        for symbol in targets {
            self.target_entries.push(dir::ReferenceEntry {
                symbol: *symbol,
                source,
                span,
                is_alias,
            });
        }

        // index authored declaration identities
        for symbol in declarations {
            self.declaration_entries.push(dir::ReferenceEntry {
                symbol: *symbol,
                source,
                span,
                is_alias: false,
            });
        }
    }

    /// Index the symbol targets recorded by one member resolution.
    fn index_member(
        &mut self,
        source: dir::GlobalNodeIdAny,
        resolution: &dir::MemberDecision,
    ) -> ProviderResult<()> {
        for access in resolution.iter() {
            self.index_member_target(source, &access.target)?;
        }

        Ok(())
    }

    /// Index the symbol targets selected by one member target.
    fn index_member_target(
        &mut self,
        source: dir::GlobalNodeIdAny,
        target: &dir::MemberTarget,
    ) -> ProviderResult<()> {
        match target {
            dir::MemberTarget::Symbol(candidate) => {
                self.index_reference(candidate.symbol, source)?
            }
            dir::MemberTarget::Existential(targets) | dir::MemberTarget::Intersection(targets) => {
                for target in targets {
                    self.index_member_target(source, target)?;
                }
            }
            dir::MemberTarget::Call(call) => self.index_call(source, call)?,
            dir::MemberTarget::Field(field) => {
                if let dir::FieldTarget::Member { symbol, .. } = field.target {
                    self.index_reference(symbol, source)?;
                }
            }
            dir::MemberTarget::Projection { .. } | dir::MemberTarget::Index(_) => {}
        }

        Ok(())
    }

    /// Index the symbol target selected by one protocol call.
    fn index_call(&mut self, source: dir::GlobalNodeIdAny, call: &dir::Call) -> ProviderResult<()> {
        if let Some(symbol) = call.target.symbol() {
            self.index_reference(symbol, source)?;
        }

        Ok(())
    }

    /// Index the symbol targets recorded by one subscript resolution.
    fn index_subscript(
        &mut self,
        source: dir::GlobalNodeIdAny,
        resolution: &dir::SubscriptDecision,
    ) -> ProviderResult<()> {
        let member_span = self.string_subscript_span(source)?;
        let mut member_symbols = Vec::new();

        for subscript in resolution.iter() {
            match &subscript.target {
                dir::SubscriptTarget::Member(member) if member_span.is_some() => {
                    member.target.collect_symbols(&mut member_symbols)
                }
                dir::SubscriptTarget::Member(member) => {
                    self.index_member_target(source, &member.target)?
                }
                dir::SubscriptTarget::Call(call) => self.index_call(source, call)?,
                dir::SubscriptTarget::Index(read) => self.index_call(source, &read.call)?,
            }
        }

        // emit every member selected by one authored string key
        if let Some(span) = member_span {
            member_symbols.sort();
            member_symbols.dedup();
            for symbol in member_symbols {
                self.index_reference_span(symbol, source, span)?;
            }
        }

        Ok(())
    }

    /// Return the authored contents of one string subscript key.
    fn string_subscript_span(&self, source: dir::GlobalNodeIdAny) -> ProviderResult<Option<Span>> {
        let expression_id = source
            .local_id
            .try_into_typed::<dir::Expression>()
            .map_err(|_| {
                ProviderError::internal(format!(
                    "subscript resolution source is not an expression: {source:?}"
                ))
            })?;
        let dir::Expression::Index { index, .. } = self.module.view().get(expression_id) else {
            return Err(ProviderError::internal(format!(
                "subscript resolution source is not an index expression: {source:?}"
            ))
            .into());
        };
        let Some(index) = index else {
            return Ok(None);
        };

        // retain only static string member keys
        if !matches!(
            self.module.view().get(*index),
            dir::Expression::ScalarLiteral(dir::ScalarLiteral::String(_))
        ) {
            return Ok(None);
        }

        let span = self
            .module
            .view()
            .get_source_extent_by_id(index.id)
            .ok_or_else(|| {
                ProviderError::internal(format!(
                    "subscript string key has no authored source span: {source:?}"
                ))
            })?;
        if span.end < span.start + 2 {
            return Err(ProviderError::internal(format!(
                "subscript string key has an invalid source span: {span:?}"
            ))
            .into());
        }

        Ok(Some(Span::new(span.file, span.start + 1, span.end - 1)))
    }

    /// Index the symbol targets recorded by one dereference resolution.
    fn index_dereference(
        &mut self,
        source: dir::GlobalNodeIdAny,
        resolution: &dir::DereferenceResolution,
    ) -> ProviderResult<()> {
        for dereference in resolution.iter() {
            if let dir::DereferenceTarget::Call(call) = &dereference.target {
                self.index_call(source, call)?;
            }
        }

        Ok(())
    }

    /// Index the symbol targets recorded by one place read.
    fn index_read(
        &mut self,
        source: dir::GlobalNodeIdAny,
        read: &dir::ReadResolution,
    ) -> ProviderResult<()> {
        match read {
            dir::ReadResolution::Binding { symbol, .. } => self.index_reference(*symbol, source)?,
            dir::ReadResolution::Member(member) => self.index_member(source, member)?,
            dir::ReadResolution::Subscript(subscript) => self.index_subscript(source, subscript)?,
            dir::ReadResolution::Dereference(dereference) => {
                self.index_dereference(source, dereference)?;
            }
        }

        Ok(())
    }

    /// Index the symbol targets recorded by one place write.
    fn index_write(
        &mut self,
        source: dir::GlobalNodeIdAny,
        write: &dir::WriteResolution,
    ) -> ProviderResult<()> {
        match write {
            dir::WriteResolution::Binding { symbol, .. } => {
                self.index_reference(*symbol, source)?
            }
            dir::WriteResolution::Member(member) => self.index_member(source, member)?,
            dir::WriteResolution::Subscript(subscript) => {
                self.index_subscript(source, subscript)?
            }
            dir::WriteResolution::Dereference(dereference) => {
                self.index_dereference(source, dereference)?;
            }
        }

        Ok(())
    }

    /// Record one selected occurrence and every resolution node it replaces.
    fn record_selection(
        &mut self,
        sources: Vec<dir::GlobalNodeIdAny>,
        mut targets: Vec<dir::GlobalSymbolId>,
    ) -> ProviderResult<()> {
        let Some(source) = sources.last().copied() else {
            return Err(ProviderError::internal("selected reference has no source node").into());
        };

        // order and deduplicate the selected targets
        targets.sort();
        targets.dedup();
        if targets.is_empty() {
            return Err(ProviderError::internal(format!(
                "selected reference {source:?} has no symbol target"
            ))
            .into());
        }

        // reject conflicting targets for one authored occurrence
        if let Some(previous) = self.targets_by_source.get(&source)
            && previous != &targets
        {
            return Err(ProviderError::internal(format!(
                "reference {source:?} has conflicting selected targets: {previous:?} and \
                 {targets:?}"
            ))
            .into());
        }

        self.targets_by_source.insert(source, targets);
        self.superseded_sources.extend(sources);

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
