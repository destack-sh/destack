use tspp_dir as dir;
use tspp_repository::{ProviderError, ProviderResult};
use tspp_source::Span;

use super::context::ModuleIndexContext;

/// Builder for one call index from checked DIR.
pub(crate) struct CallIndexer<'context, 'index> {
    /// The indexed module context.
    module: &'context ModuleIndexContext<'index>,
    /// The collected index entries.
    entries: Vec<dir::CallEntry>,
}

impl<'context, 'index> CallIndexer<'context, 'index> {
    /// Build the call index.
    pub(crate) fn build(
        module: &'context ModuleIndexContext<'index>,
    ) -> ProviderResult<dir::CallIndex> {
        let mut indexer = Self {
            module,
            entries: Vec::new(),
        };

        // collect call and construct decisions
        indexer.collect_calls()?;

        Ok(dir::CallIndex::new(indexer.entries))
    }

    /// Collect call edges from every checked decision.
    fn collect_calls(&mut self) -> ProviderResult<()> {
        for (node_id, resolution) in self.module.decisions().expression_entries() {
            // resolve shared source metadata per resolved node
            let label = match resolution {
                dir::Decision::Call(_) => "call",
                dir::Decision::Member(_) => "member",
                dir::Decision::Operator(_) => "operator",
                dir::Decision::Subscript(_) => "subscript",
                dir::Decision::Assignment(_) => "assignment",
                dir::Decision::Construct(_) => "construct",
                _ => continue,
            };
            let source = self.expression_source(node_id, label)?;
            let Some(span) = self.module.view().get_span_by_id(node_id.local_id.id) else {
                continue;
            };
            let caller = self.caller_symbol(node_id.local_id)?;

            match resolution {
                // emit selected call targets
                dir::Decision::Call(resolution) => {
                    self.push_call_decision(source, caller, span, resolution);
                }
                // emit getter calls stored by member resolutions
                dir::Decision::Member(resolution) => {
                    self.push_member_decision(source, caller, span, resolution);
                }
                // emit protocol calls stored by operator resolutions
                dir::Decision::Operator(resolution) => {
                    for application in resolution.arms() {
                        if let Some(call) = application.call() {
                            self.push_call(source, caller, span, call);
                        }
                    }
                }
                // emit protocol calls stored by subscript read resolutions
                dir::Decision::Subscript(resolution) => {
                    self.push_subscript_decision(source, caller, span, resolution);
                }
                // emit accessor and protocol calls stored by place resolutions
                dir::Decision::Assignment(resolution) => {
                    if let Some(read) = &resolution.read {
                        self.push_read(source, caller, span, read);
                    }
                    self.push_write(source, caller, span, &resolution.write);
                }
                // emit the resolved construct edge
                dir::Decision::Construct(resolution) => {
                    let Some(callee) = resolution.target.call_symbol() else {
                        continue;
                    };
                    self.entries.push(dir::CallEntry {
                        source,
                        kind: dir::CallKind::Construct,
                        caller,
                        callee,
                        span,
                    });
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Return one resolution source as a typed expression node.
    fn expression_source(
        &self,
        node_id: dir::GlobalNodeIdAny,
        family: &str,
    ) -> ProviderResult<dir::GlobalNodeId<dir::Expression>> {
        node_id
            .try_into_typed::<dir::Expression>()
            .map_err(|error| {
                ProviderError::internal(format!("{family} source is not an expression: {error}"))
                    .into()
            })
    }

    /// Push call edges selected by one call resolution.
    fn push_call_decision(
        &mut self,
        source: dir::GlobalNodeId<dir::Expression>,
        caller: Option<dir::GlobalSymbolId>,
        span: Span,
        resolution: &dir::CallDecision,
    ) {
        // emit each exact declaration selected by the call
        for callee in resolution.target_symbols() {
            self.push_call_symbol(source, caller, span, callee);
        }
    }

    /// Push one declaration-backed call edge.
    fn push_call_symbol(
        &mut self,
        source: dir::GlobalNodeId<dir::Expression>,
        caller: Option<dir::GlobalSymbolId>,
        span: Span,
        callee: dir::GlobalSymbolId,
    ) {
        self.entries.push(dir::CallEntry {
            source,
            kind: dir::CallKind::Call,
            caller,
            callee,
            span,
        });
    }

    /// Push one call edge retained by another operation.
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

        self.push_call_symbol(source, caller, span, callee);
    }

    /// Push getter call edges selected by one member resolution.
    fn push_member_decision(
        &mut self,
        source: dir::GlobalNodeId<dir::Expression>,
        caller: Option<dir::GlobalSymbolId>,
        span: Span,
        resolution: &dir::MemberDecision,
    ) {
        for access in resolution.arms() {
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
            dir::MemberTarget::OverloadSet(targets) | dir::MemberTarget::Intersection(targets) => {
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
    fn push_subscript_decision(
        &mut self,
        source: dir::GlobalNodeId<dir::Expression>,
        caller: Option<dir::GlobalSymbolId>,
        span: Span,
        resolution: &dir::SubscriptDecision,
    ) {
        for subscript in resolution.arms() {
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
        for dereference in resolution.arms() {
            if let Some(call) = &dereference.protocol {
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
                self.push_member_decision(source, caller, span, member);
            }
            dir::ReadResolution::Subscript(subscript) => {
                self.push_subscript_decision(source, caller, span, subscript);
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
                self.push_member_decision(source, caller, span, member);
            }
            dir::WriteResolution::Subscript(subscript) => {
                self.push_subscript_decision(source, caller, span, subscript);
            }
            dir::WriteResolution::Dereference(dereference) => {
                self.push_dereference_resolution(source, caller, span, dereference);
            }
        }
    }

    /// Find the named callable that contains one node.
    fn caller_symbol(
        &self,
        node_id: dir::LocalNodeIdAny,
    ) -> ProviderResult<Option<dir::GlobalSymbolId>> {
        let bindings = self.module.bindings();
        let view = self.module.view();
        let scope = bindings.scope_at(&view, node_id);

        // select the nearest function scope
        let function =
            std::iter::successors(Some(scope), |scope| bindings.get_scope(*scope).parent)
                .find(|scope| bindings.get_scope(*scope).kind == dir::ScopeKind::Function);
        let Some(function) = function else {
            return Ok(None);
        };
        let owner = bindings.get_scope(function).owner.ok_or_else(|| {
            ProviderError::internal(format!("function scope has no owner: {:?}", function.id))
        })?;

        // classify the exact declaration that owns the function scope
        let symbol = bindings.get_symbol(owner);
        let Some(declaration) = symbol.declaration else {
            return Err(ProviderError::internal(format!(
                "function scope owner has no declaration: {owner:?}"
            ))
            .into());
        };
        let is_caller = match declaration.local_id.ty {
            dir::NodeType::Declaration => {
                let declaration_id =
                    dir::LocalNodeId::<dir::Declaration>::new(declaration.local_id.id);
                matches!(
                    view.get(declaration_id),
                    dir::Declaration::Function(function) if function.name.is_some()
                )
            }
            dir::NodeType::Member => {
                let member_id = dir::LocalNodeId::<dir::Member>::new(declaration.local_id.id);
                matches!(view.get(member_id), dir::Member::Method { .. })
            }
            _ => false,
        };
        if !is_caller {
            return Ok(None);
        }

        Ok(Some(dir::GlobalSymbolId::new(
            self.module.module_id(),
            owner,
        )))
    }
}
