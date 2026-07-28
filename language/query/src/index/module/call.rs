use destack_dir as dir;
use destack_repository::{ProviderError, ProviderResult};
use destack_source::Span;

use crate::ModuleQueryContext;

/// Builder for one call index from checked DIR.
pub(super) struct CallIndexer<'context, 'query> {
    /// The indexed module context.
    module: &'context ModuleQueryContext<'query>,
    /// The collected index entries.
    entries: Vec<dir::CallEntry>,
}

impl<'context, 'query> CallIndexer<'context, 'query> {
    /// Build the call index.
    pub(super) fn build(
        module: &'context ModuleQueryContext<'query>,
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
            // resolve call source metadata
            let source = self.expression_source(node_id, "call")?;
            let Some(span) = self.module.view().get_span_by_id(node_id.local_id.id) else {
                continue;
            };
            let caller = self.containing_symbol(node_id.local_id)?;

            self.push_call_resolution(source, caller, span, resolution);
        }

        Ok(())
    }

    /// Collect getter calls stored by member resolutions.
    fn collect_member_calls(&mut self) -> ProviderResult<()> {
        for (node_id, resolution) in self.module.resolutions().member_entries() {
            let source = self.expression_source(node_id, "member")?;
            let Some(span) = self.module.view().get_span_by_id(node_id.local_id.id) else {
                continue;
            };
            let caller = self.containing_symbol(node_id.local_id)?;

            self.push_member_resolution(source, caller, span, resolution);
        }

        Ok(())
    }

    /// Collect protocol calls stored by operator resolutions.
    fn collect_operator_calls(&mut self) -> ProviderResult<()> {
        for (node_id, resolution) in self.module.resolutions().operator_entries() {
            let source = self.expression_source(node_id, "operator")?;
            let Some(span) = self.module.view().get_span_by_id(node_id.local_id.id) else {
                continue;
            };
            let caller = self.containing_symbol(node_id.local_id)?;

            for application in resolution.iter() {
                if let Some(call) = application.call() {
                    self.push_call(source, caller, span, call);
                }
            }
        }

        Ok(())
    }

    /// Collect protocol calls stored by subscript read resolutions.
    fn collect_subscript_calls(&mut self) -> ProviderResult<()> {
        for (node_id, resolution) in self.module.resolutions().subscript_entries() {
            let source = self.expression_source(node_id, "subscript")?;
            let Some(span) = self.module.view().get_span_by_id(node_id.local_id.id) else {
                continue;
            };
            let caller = self.containing_symbol(node_id.local_id)?;

            self.push_subscript_resolution(source, caller, span, resolution);
        }

        Ok(())
    }

    /// Collect accessor and protocol calls stored by place resolutions.
    fn collect_place_calls(&mut self) -> ProviderResult<()> {
        for (node_id, resolution) in self.module.resolutions().assignment_entries() {
            let source = self.expression_source(node_id, "assignment")?;
            let Some(span) = self.module.view().get_span_by_id(node_id.local_id.id) else {
                continue;
            };
            let caller = self.containing_symbol(node_id.local_id)?;

            if let Some(read) = &resolution.read {
                self.push_read(source, caller, span, read);
            }
            self.push_write(source, caller, span, &resolution.write);
        }

        Ok(())
    }

    /// Collect checked construct target edges.
    fn collect_constructs(&mut self) -> ProviderResult<()> {
        for (node_id, resolution) in self.module.resolutions().construct_entries() {
            // resolve construct source metadata
            let source = self.expression_source(node_id, "construct")?;
            let Some(span) = self.module.view().get_span_by_id(node_id.local_id.id) else {
                continue;
            };
            let caller = self.containing_symbol(node_id.local_id)?;

            // emit the resolved construct edge
            self.entries.push(dir::CallEntry {
                source,
                kind: dir::CallKind::Construct,
                caller,
                callee: resolution.target.call_symbol(),
                span,
            });
        }

        Ok(())
    }

    /// Return one resolution source as a typed expression node.
    fn expression_source(
        &self,
        node_id: dir::GlobalNodeIdAny,
        family: &str,
    ) -> ProviderResult<dir::GlobalNodeId<dir::Expression>> {
        node_id.try_into_typed::<dir::Expression>().map_err(|error| {
            ProviderError::internal(format!("{family} source is not an expression: {error}")).into()
        })
    }

    /// Push call edges selected by one call resolution.
    fn push_call_resolution(
        &mut self,
        source: dir::GlobalNodeId<dir::Expression>,
        caller: Option<dir::GlobalSymbolId>,
        span: Span,
        resolution: &dir::CallResolution,
    ) {
        for call in resolution.iter() {
            self.push_call(source, caller, span, call);
        }
    }

    /// Push getter call edges selected by one member resolution.
    fn push_member_resolution(
        &mut self,
        source: dir::GlobalNodeId<dir::Expression>,
        caller: Option<dir::GlobalSymbolId>,
        span: Span,
        resolution: &dir::MemberResolution,
    ) {
        for access in resolution.iter() {
            self.push_member_target(source, caller, span, &access.target);
        }
    }

    /// Push calls selected by one singular member access.
    fn push_member_access(
        &mut self,
        source: dir::GlobalNodeId<dir::Expression>,
        caller: Option<dir::GlobalSymbolId>,
        span: Span,
        access: &dir::MemberAccess,
    ) {
        self.push_member_target(source, caller, span, &access.target);
    }

    /// Push calls selected by one member target.
    fn push_member_target(
        &mut self,
        source: dir::GlobalNodeId<dir::Expression>,
        caller: Option<dir::GlobalSymbolId>,
        span: Span,
        target: &dir::MemberTarget,
    ) {
        match target {
            dir::MemberTarget::Call(call) => self.push_call(source, caller, span, call),
            dir::MemberTarget::Existential(targets) | dir::MemberTarget::Intersection(targets) => {
                for target in targets {
                    self.push_member_target(source, caller, span, target);
                }
            }
            dir::MemberTarget::Projection { .. }
            | dir::MemberTarget::Field(_)
            | dir::MemberTarget::Index(_)
            | dir::MemberTarget::Symbol(_) => {}
        }
    }

    /// Push calls selected by one subscript resolution.
    fn push_subscript_resolution(
        &mut self,
        source: dir::GlobalNodeId<dir::Expression>,
        caller: Option<dir::GlobalSymbolId>,
        span: Span,
        resolution: &dir::SubscriptResolution,
    ) {
        for subscript in resolution.iter() {
            match &subscript.target {
                dir::SubscriptTarget::Member(member) => {
                    self.push_member_access(source, caller, span, member);
                }
                dir::SubscriptTarget::Call(call) => self.push_call(source, caller, span, call),
                dir::SubscriptTarget::Index(read) => {
                    self.push_call(source, caller, span, &read.call)
                }
            }
        }
    }

    /// Push calls selected by one dereference resolution.
    fn push_dereference_resolution(
        &mut self,
        source: dir::GlobalNodeId<dir::Expression>,
        caller: Option<dir::GlobalSymbolId>,
        span: Span,
        resolution: &dir::DereferenceResolution,
    ) {
        for dereference in resolution.iter() {
            if let dir::DereferenceTarget::Call(call) = &dereference.target {
                self.push_call(source, caller, span, call);
            }
        }
    }

    /// Push calls selected by one place read.
    fn push_read(
        &mut self,
        source: dir::GlobalNodeId<dir::Expression>,
        caller: Option<dir::GlobalSymbolId>,
        span: Span,
        read: &dir::ReadResolution,
    ) {
        match read {
            dir::ReadResolution::Binding { .. } => {}
            dir::ReadResolution::Member(member) => {
                self.push_member_resolution(source, caller, span, member);
            }
            dir::ReadResolution::Subscript(subscript) => {
                self.push_subscript_resolution(source, caller, span, subscript);
            }
            dir::ReadResolution::Dereference(dereference) => {
                self.push_dereference_resolution(source, caller, span, dereference);
            }
        }
    }

    /// Push calls selected by one place write.
    fn push_write(
        &mut self,
        source: dir::GlobalNodeId<dir::Expression>,
        caller: Option<dir::GlobalSymbolId>,
        span: Span,
        write: &dir::WriteResolution,
    ) {
        match write {
            dir::WriteResolution::Binding { .. } => {}
            dir::WriteResolution::Member(member) => {
                self.push_member_resolution(source, caller, span, member);
            }
            dir::WriteResolution::Subscript(subscript) => {
                self.push_subscript_resolution(source, caller, span, subscript);
            }
            dir::WriteResolution::Dereference(dereference) => {
                self.push_dereference_resolution(source, caller, span, dereference);
            }
        }
    }

    /// Push one singular call edge.
    fn push_call(
        &mut self,
        source: dir::GlobalNodeId<dir::Expression>,
        caller: Option<dir::GlobalSymbolId>,
        span: Span,
        call: &dir::Call,
    ) {
        let Some(callee) = call.target.symbol() else {
            return;
        };

        self.entries.push(dir::CallEntry {
            source,
            kind: dir::CallKind::Call,
            caller,
            callee,
            span,
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
