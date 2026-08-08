use destack_dir as dir;
use destack_repository::{ProviderError, ProviderResult};
use destack_source::Span;

use super::context::ModuleIndexContext;

/// Builder for one call index from checked DIR.
pub(in crate::index) struct CallIndexer<'context, 'index> {
    /// The indexed module context.
    module: &'context ModuleIndexContext<'index>,
    /// The collected index entries.
    entries: Vec<dir::CallEntry>,
}

/// One authored call-like source occurrence.
#[derive(Debug, Clone, Copy)]
struct CallSite {
    /// The call-like expression node.
    source: dir::GlobalNodeId<dir::Expression>,
    /// The containing callable symbol when present.
    caller: Option<dir::GlobalSymbolId>,
    /// The authored source range.
    span: Span,
}

impl<'context, 'index> CallIndexer<'context, 'index> {
    /// Build the call index.
    pub(in crate::index) fn build(
        module: &'context ModuleIndexContext<'index>,
    ) -> ProviderResult<dir::CallIndex> {
        let mut indexer = Self {
            module,
            entries: Vec::new(),
        };

        // collect checked call and construct resolutions
        indexer.collect_calls()?;
        indexer.collect_member_calls()?;
        indexer.collect_operator_calls()?;
        indexer.collect_subscript_calls()?;
        indexer.collect_place_calls()?;
        indexer.collect_constructs()?;

        Ok(dir::CallIndex::new(indexer.entries))
    }

    /// Collect checked call target edges.
    fn collect_calls(&mut self) -> ProviderResult<()> {
        for (node_id, resolution) in self.module.resolutions().call_entries() {
            // omit generated calls without source occurrences
            let Some(site) = self.call_site(node_id)? else {
                continue;
            };

            self.push_call_resolution(site, resolution);
        }

        Ok(())
    }

    /// Collect getter calls stored by member resolutions.
    fn collect_member_calls(&mut self) -> ProviderResult<()> {
        for (node_id, resolution) in self.module.resolutions().member_entries() {
            // omit generated calls without source occurrences
            let Some(site) = self.call_site(node_id)? else {
                continue;
            };

            self.push_member_resolution(site, resolution);
        }

        Ok(())
    }

    /// Collect protocol calls stored by operator resolutions.
    fn collect_operator_calls(&mut self) -> ProviderResult<()> {
        for (node_id, resolution) in self.module.resolutions().operator_entries() {
            // omit generated calls without source occurrences
            let Some(site) = self.call_site(node_id)? else {
                continue;
            };

            for application in resolution.iter() {
                if let Some(call) = application.call() {
                    self.push_call(site, call);
                }
            }
        }

        Ok(())
    }

    /// Collect protocol calls stored by subscript read resolutions.
    fn collect_subscript_calls(&mut self) -> ProviderResult<()> {
        for (node_id, resolution) in self.module.resolutions().subscript_entries() {
            // omit generated calls without source occurrences
            let Some(site) = self.call_site(node_id)? else {
                continue;
            };

            self.push_subscript_resolution(site, resolution);
        }

        Ok(())
    }

    /// Collect accessor and protocol calls stored by place resolutions.
    fn collect_place_calls(&mut self) -> ProviderResult<()> {
        for (node_id, resolution) in self.module.resolutions().assignment_entries() {
            // omit generated calls without source occurrences
            let Some(site) = self.call_site(node_id)? else {
                continue;
            };

            if let Some(read) = &resolution.read {
                self.push_read(site, read);
            }
            self.push_write(site, &resolution.write);
        }

        Ok(())
    }

    /// Collect checked construct target edges.
    fn collect_constructs(&mut self) -> ProviderResult<()> {
        for (node_id, resolution) in self.module.resolutions().construct_entries() {
            // omit generated calls without source occurrences
            let Some(site) = self.call_site(node_id)? else {
                continue;
            };

            // omit constructions without declaration-backed call targets
            let Some(callee) = resolution.target.call_symbol() else {
                continue;
            };

            // emit the resolved construct edge
            self.entries.push(dir::CallEntry {
                source: site.source,
                kind: dir::CallKind::Construct,
                caller: site.caller,
                callee,
                span: site.span,
            });
        }

        Ok(())
    }

    /// Return one authored call site for a checked resolution source.
    fn call_site(&self, node_id: dir::GlobalNodeIdAny) -> ProviderResult<Option<CallSite>> {
        let source = node_id
            .try_into_typed::<dir::Expression>()
            .map_err(|error| {
                ProviderError::internal(format!("call index source is not an expression: {error}"))
            })?;
        let view = self.module.view();
        let source_id = view.get_source_any(node_id.local_id);
        if self.module.source_index().try_get(source_id).is_none() {
            return Ok(None);
        }

        // require every authored call site to have a complete source range
        let span = view.get_span_by_id(node_id.local_id.id).ok_or_else(|| {
            ProviderError::internal(format!(
                "authored call index source has no span: {node_id:?}"
            ))
        })?;
        let caller = self.containing_symbol(node_id.local_id)?;

        Ok(Some(CallSite {
            source,
            caller,
            span,
        }))
    }

    /// Push call edges selected by one call resolution.
    fn push_call_resolution(&mut self, site: CallSite, resolution: &dir::CallResolution) {
        for call in resolution.iter() {
            self.push_call(site, call);
        }
    }

    /// Push getter call edges selected by one member resolution.
    fn push_member_resolution(&mut self, site: CallSite, resolution: &dir::MemberResolution) {
        for access in resolution.iter() {
            self.push_member_target(site, &access.target);
        }
    }

    /// Push calls selected by one singular member access.
    fn push_member_access(&mut self, site: CallSite, access: &dir::MemberAccess) {
        self.push_member_target(site, &access.target);
    }

    /// Push calls selected by one member target.
    fn push_member_target(&mut self, site: CallSite, target: &dir::MemberTarget) {
        match target {
            dir::MemberTarget::Call(call) => self.push_call(site, call),
            dir::MemberTarget::Existential(targets) | dir::MemberTarget::Intersection(targets) => {
                for target in targets {
                    self.push_member_target(site, target);
                }
            }
            dir::MemberTarget::Projection { .. }
            | dir::MemberTarget::Field(_)
            | dir::MemberTarget::Index(_)
            | dir::MemberTarget::Symbol(_) => {}
        }
    }

    /// Push calls selected by one subscript resolution.
    fn push_subscript_resolution(&mut self, site: CallSite, resolution: &dir::SubscriptResolution) {
        for subscript in resolution.iter() {
            match &subscript.target {
                dir::SubscriptTarget::Member(member) => {
                    self.push_member_access(site, member);
                }
                dir::SubscriptTarget::Call(call) => self.push_call(site, call),
                dir::SubscriptTarget::Index(read) => self.push_call(site, &read.call),
            }
        }
    }

    /// Push calls selected by one dereference resolution.
    fn push_dereference_resolution(
        &mut self,
        site: CallSite,
        resolution: &dir::DereferenceResolution,
    ) {
        for dereference in resolution.iter() {
            if let dir::DereferenceTarget::Call(call) = &dereference.target {
                self.push_call(site, call);
            }
        }
    }

    /// Push calls selected by one place read.
    fn push_read(&mut self, site: CallSite, read: &dir::ReadResolution) {
        match read {
            dir::ReadResolution::Binding { .. } => {}
            dir::ReadResolution::Member(member) => {
                self.push_member_resolution(site, member);
            }
            dir::ReadResolution::Subscript(subscript) => {
                self.push_subscript_resolution(site, subscript);
            }
            dir::ReadResolution::Dereference(dereference) => {
                self.push_dereference_resolution(site, dereference);
            }
        }
    }

    /// Push calls selected by one place write.
    fn push_write(&mut self, site: CallSite, write: &dir::WriteResolution) {
        match write {
            dir::WriteResolution::Binding { .. } => {}
            dir::WriteResolution::Member(member) => {
                self.push_member_resolution(site, member);
            }
            dir::WriteResolution::Subscript(subscript) => {
                self.push_subscript_resolution(site, subscript);
            }
            dir::WriteResolution::Dereference(dereference) => {
                self.push_dereference_resolution(site, dereference);
            }
        }
    }

    /// Push one singular call edge.
    fn push_call(&mut self, site: CallSite, call: &dir::Call) {
        // omit dynamically dispatched calls without a declaration edge
        let Some(callee) = call.target.symbol() else {
            return;
        };

        self.entries.push(dir::CallEntry {
            source: site.source,
            kind: dir::CallKind::Call,
            caller: site.caller,
            callee,
            span: site.span,
        });
    }

    /// Find the containing declaration or member symbol for one node.
    fn containing_symbol(
        &self,
        node_id: dir::LocalNodeIdAny,
    ) -> ProviderResult<Option<dir::GlobalSymbolId>> {
        let mut current = Some(node_id);

        // stop at the nearest callable declaration
        while let Some(node_id) = current {
            if node_id.ty == dir::NodeType::Declaration {
                let declaration_id =
                    node_id.try_into_typed::<dir::Declaration>().map_err(|_| {
                        ProviderError::internal(format!(
                            "declaration node id has incompatible type: {node_id:?}"
                        ))
                    })?;
                let declaration = self.module.view().get(declaration_id);

                // anonymous functions have no call hierarchy item
                if let dir::Declaration::Function(function) = declaration {
                    if function.name.is_none() {
                        return Ok(None);
                    }
                    let symbol_id = self.module.node_symbol(node_id).ok_or_else(|| {
                        ProviderError::internal(format!(
                            "missing symbol for function declaration {declaration_id:?}"
                        ))
                    })?;

                    return Ok(Some(dir::GlobalSymbolId::new(
                        self.module.module_id(),
                        symbol_id,
                    )));
                }
            }

            // stop at the nearest method declaration
            if node_id.ty == dir::NodeType::Member {
                let member_id = node_id.try_into_typed::<dir::Member>().map_err(|_| {
                    ProviderError::internal(format!(
                        "member node id has incompatible type: {node_id:?}"
                    ))
                })?;
                if matches!(
                    self.module.view().get(member_id),
                    dir::Member::Method { .. }
                ) {
                    let symbol_id = self.module.node_symbol(node_id).ok_or_else(|| {
                        ProviderError::internal(format!(
                            "missing symbol for method member {member_id:?}"
                        ))
                    })?;

                    return Ok(Some(dir::GlobalSymbolId::new(
                        self.module.module_id(),
                        symbol_id,
                    )));
                }
            }

            // continue with the parent node
            current = self.module.view().get_parent_any(node_id);
        }

        Ok(None)
    }
}
