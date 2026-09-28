use smallvec::SmallVec;
use tspp_dir as dir;
use tspp_source::ModuleId;

use crate::sema::{
    BodyCheck, Check, CheckState, Expectation, FlowState, GenericParameterId, GenericTemplateId,
    InferMode, Origin, Settle, TypeSubstitution, Value, Verdict,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Type one function value as the callable its context takes.
    pub(in crate::sema) fn function_value_type(
        &mut self,
        node: dir::GlobalNodeIdAny,
        context: Option<Expectation>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // type a function value once
        if let Some(callable) = self.committed_node_type(node) {
            return Ok(callable);
        }

        // type the closure at the arity its context calls it with
        self.commit_function_value(node)?;
        let symbol = self.function_value_symbol(node)?;
        let callable = self.adopt_contextual_arity(symbol, context)?;
        let directive = self.resolve_capture_directive(symbol)?;
        let callable = match directive.is_some_and(|directive| directive.borrows()) {
            true => self.frame_borrow(node, callable)?,
            false => self.contextual_form(callable, context)?,
        };
        self.commit_node_type(node, callable)?;
        self.queue_check_function_body(node)?;

        Ok(callable)
    }

    /// Commit one function value's type at the arity its context calls it with.
    fn adopt_contextual_arity(
        &mut self,
        symbol: dir::GlobalSymbolId,
        context: Option<Expectation>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let Some(written) = self.written_values.get(&symbol).copied() else {
            return Err(CompilerError::Internal {
                message: format!("function value {symbol:?} without its written type"),
            });
        };
        let adopted = match context.and_then(Expectation::contextual_target) {
            Some(target) => self.extended_callable(symbol, written, target, context)?,
            None => written,
        };

        self.commit_symbol_type(symbol, adopted)
    }

    /// Return the one callable signature a contextual target offers, through forms and union arms.
    fn contextual_signature(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // read each arm of the target
        let arms = match self.union_arms(origin, target)? {
            Some(arms) => arms,
            None => SmallVec::from_slice(&[target]),
        };

        // keep the signature of the single callable arm
        let mut signature = None;
        for arm in arms {
            let arm = self.form_chain(origin, arm)?.base();
            let arm = self.normalize(origin, arm)?;
            let arm = match self.ty(arm)? {
                dir::Type::Function(function) => function.signature,
                dir::Type::FunctionPointer(function) => function.signature,
                dir::Type::FunctionSignature(_) => arm,
                _ => continue,
            };
            if signature.replace(arm).is_some() {
                return Ok(None);
            }
        }

        Ok(signature)
    }

    /// Bind the target binders in trailing parameters to the lambda's own, none for a type binder.
    fn rebind_trailing(
        &mut self,
        symbol: dir::GlobalSymbolId,
        template: Option<GenericTemplateId>,
        binders: GenericTemplateId,
        trailing: Vec<dir::FunctionParameterType>,
        first: usize,
    ) -> CompilerResult<Option<(Option<GenericTemplateId>, Vec<dir::FunctionParameterType>)>> {
        let mut template = template;
        let mut rebound = Vec::with_capacity(trailing.len());
        for (offset, parameter) in trailing.into_iter().enumerate() {
            // keep a parameter without target binders
            let mentioned = self.template_binders(parameter.ty, binders)?;
            if mentioned.is_empty() {
                rebound.push(parameter);
                continue;
            }

            // open the lambda's template for its first binder
            let own_template = match template {
                Some(own_template) => own_template,
                None => {
                    let source = self.value_source(symbol)?;
                    let parent = self.flow.template_scope();
                    let opened = self.open_generic_template(source, parent)?;
                    template = Some(opened);

                    opened
                }
            };

            // bind each mentioned binder to the lambda's binder for this slot
            let mut substitution = TypeSubstitution::default();
            for (ordinal, binder) in mentioned.into_iter().enumerate() {
                let Some(own) =
                    self.slot_binder(symbol, own_template, first + offset, ordinal, binder)?
                else {
                    return Ok(None);
                };
                substitution.bindings.push(dir::GenericArgumentBinding {
                    parameter: binder,
                    argument: own,
                });
            }
            let ty = self.substitute_type(parameter.ty, &substitution)?;
            rebound.push(dir::FunctionParameterType { ty, ..parameter });
        }

        Ok(Some((template, rebound)))
    }

    /// Return the lambda's binder standing for one target binder at one slot, minted once.
    fn slot_binder(
        &mut self,
        symbol: dir::GlobalSymbolId,
        template: GenericTemplateId,
        slot: usize,
        ordinal: usize,
        binder: GenericParameterId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // reuse the binder a previous adoption minted
        let key = (symbol, slot, ordinal);
        let own = match self.slot_binders.get(&key) {
            Some(own) => *own,
            None => {
                // mint a memory binder, refusing a type binder
                let kind = self.generic_parameter(binder)?.map(|binding| binding.kind);
                let Some(dir::GenericParameterKind::Memory(memory)) = kind else {
                    return Ok(None);
                };
                let source = self.value_source(symbol)?;
                let own = self.push_induced_memory_parameter(template, source, memory)?;
                self.slot_binders.insert(key, own);

                own
            }
        };

        Ok(self.generic_parameter(own)?.map(|binding| binding.ty))
    }

    /// Return the parameters of one template a type mentions, in first-mention order.
    fn template_binders(
        &mut self,
        ty: dir::GlobalTypeId,
        template: GenericTemplateId,
    ) -> CompilerResult<Vec<GenericParameterId>> {
        let mut binders = Vec::new();
        let mut pending = vec![ty];
        while let Some(ty) = pending.pop() {
            // keep a parameter the template owns
            let ty = self.shallow_resolve(ty)?;
            let kind = self.ty(ty)?;
            if let dir::Type::Parameter(parameter) = kind
                && parameter.module_id == template.module_id
                && self
                    .generic_parameter(parameter)?
                    .is_some_and(|binding| binding.template == template.local_id)
                && !binders.contains(&parameter)
            {
                binders.push(parameter);
            }

            // walk the children
            self.for_each_type_child(ty.module_id, &kind, |child| pending.push(child))?;
        }

        Ok(binders)
    }

    /// Return the declaration node of one function value symbol.
    fn value_source(&self, symbol: dir::GlobalSymbolId) -> CompilerResult<dir::GlobalNodeIdAny> {
        let node = self
            .module(symbol.module_id)
            .symbol_declaration_node(symbol.local_id)?;

        Ok(node.into_global(symbol.module_id))
    }

    /// Return the written callable of one function value.
    fn written_callable(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        match self.written_values.get(&symbol) {
            Some(written) => Ok(*written),
            None => self.symbol_type(symbol),
        }
    }

    /// Return one written callable with the trailing parameters of a target signature appended.
    fn extended_callable(
        &mut self,
        symbol: dir::GlobalSymbolId,
        written: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        context: Option<Expectation>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // read the written signature and the target's
        let dir::Type::Function(function) = self.ty(written)? else {
            return Ok(written);
        };
        let origin = match context {
            Some(context) => self.cause_origin(context.cause),
            None => return Ok(written),
        };
        let Some(target) = self.contextual_signature(origin, target)? else {
            return Ok(written);
        };
        let (Some(head), Some(target_head)) = (
            self.signature_head(function.signature)?,
            self.signature_head(target)?,
        ) else {
            return Ok(written);
        };

        // require a longer fixed target list
        let parameters = self
            .signature_parameters(function.signature.module_id, head.parameters)?
            .to_vec();
        let target_parameters = self
            .signature_parameters(target.module_id, target_head.parameters)?
            .to_vec();
        let has_rest = parameters
            .iter()
            .chain(&target_parameters)
            .any(|parameter| parameter.is_rest);
        if has_rest || parameters.len() >= target_parameters.len() {
            return Ok(written);
        }

        // bind the trailing parameters under the lambda's own binders
        let trailing = target_parameters[parameters.len()..].to_vec();
        let (template, trailing) = match target_head.template {
            Some(binders) => {
                let rebound = self.rebind_trailing(
                    symbol,
                    head.template,
                    binders,
                    trailing,
                    parameters.len(),
                )?;
                let Some(rebound) = rebound else {
                    return Ok(written);
                };

                rebound
            }
            None => (head.template, trailing),
        };

        // append the target's trailing parameters
        let mut extended = parameters;
        extended.extend(trailing);
        let parameters = self.intern_parameters(&extended)?;
        let signature = self.intern_signature(dir::FunctionSignatureType {
            parameters,
            template,
            ..head
        })?;

        self.intern_type(dir::Type::Function(dir::FunctionType {
            signature,
            ..function
        }))
    }

    /// Return the borrow type of one closure's frame temporary.
    fn frame_borrow(
        &mut self,
        node: dir::GlobalNodeIdAny,
        callable: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // place the closure in a frame temporary
        let origin = Origin::Node(node, None);
        let owned = self.owned_type(callable)?;
        let value = Value {
            ty: owned,
            node: None,
            place: None,
            is_fresh: false,
        };
        let place = self.borrowed_place(origin, value, Some(dir::Access::BARE), true)?;

        // borrow the temporary with an open access
        let placement = self.shallow_resolve(place.placement)?;
        let region = self.intern_region(place.lifetime, placement)?;
        let access = self.open_memory_type(origin, dir::MemoryParameter::Access)?;
        let borrowed = self.borrow_value(region, access, owned)?;

        self.normalize(origin, borrowed)
    }

    /// Return the declaration one function value holds, read over the patched view.
    pub(in crate::sema) fn function_value_declaration(
        &self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<dir::LocalNodeId<dir::Declaration>> {
        let module = node.module_id;
        let (parsed, expanded) = self.patched_inputs(module);
        let tree = dir::View::new(&parsed.tree).patched(&expanded.patch);
        match *tree.get(node.local_id.into_typed::<dir::Expression>()) {
            dir::Expression::Declaration(declaration) => Ok(declaration),
            _ => Err(CompilerError::Internal {
                message: format!("function value {node:?} holds no declaration"),
            }),
        }
    }

    /// Return the declaration symbol of one function value.
    pub(in crate::sema) fn function_value_symbol(
        &self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<dir::GlobalSymbolId> {
        let module = node.module_id;
        let declaration = self.function_value_declaration(node)?;

        self.module(module)
            .declaration_symbol(declaration.into_any())
            .ok_or_else(|| CompilerError::Internal {
                message: format!("function value {node:?} has no declaration symbol"),
            })
    }

    /// Check one function value's body in place once its parameter types close.
    pub(in crate::sema) fn queue_check_function_body(
        &mut self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<()> {
        // leave a checked or stalled body, and every body inside a decision
        let Some(body) = self.lambdas.get(&node) else {
            return Ok(());
        };
        if body.flow.is_some() || self.infer.is_deciding() {
            return Ok(());
        }

        // check a body without open parameter types in place
        let slots = self.function_value_parameters(node)?;
        if slots.is_empty() {
            self.check_function_body(node)?;
        }
        // otherwise stall the body behind its open parameter types
        else {
            let snapshot = self.flow.snapshot();
            if let Some(body) = self.lambdas.get_mut(&node) {
                body.flow = Some(snapshot);
            }
            self.queue_check_stalled(Check::Body(BodyCheck { node }), &slots)?;
        }

        Ok(())
    }

    /// Return the open parameter roots of one function value.
    pub(in crate::sema) fn function_value_parameters(
        &mut self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<SmallVec<[dir::TypeVariableId; 2]>> {
        // read the parameter types of the written callable
        let symbol = self.function_value_symbol(node)?;
        let callable = self.written_callable(symbol)?;
        let parameters = match self.callable_signature_head(callable)? {
            Some((module, head)) => self
                .signature_parameters(module, head.parameters)?
                .iter()
                .map(|parameter| parameter.ty)
                .collect::<SmallVec<[dir::GlobalTypeId; 4]>>(),
            None => SmallVec::new(),
        };

        self.collect_open_variables(parameters.iter().copied())
    }

    /// Check one pending function value body under the parameters its context bound.
    pub(in crate::sema) fn check_function_body(
        &mut self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<()> {
        // take the pending body
        let Some(mut body) = self.lambdas.swap_remove(&node) else {
            return Err(CompilerError::Internal {
                message: format!("function value {node:?} has no body"),
            });
        };

        // fix the parameter types the body reads
        let roots = self.function_value_parameters(node)?;
        self.fix_variables(&roots)?;
        self.settle_variables(&roots, Settle::All)?;

        let symbol = body.symbol;

        // check a deferred body under the flow its function value saw
        match body.flow.take() {
            Some(snapshot) => {
                let past = FlowState::from_snapshot(snapshot);
                let current = std::mem::replace(&mut self.flow, past);
                let checked = body.check(self, InferMode::Regular, None);
                self.flow = current;
                checked?;
            }
            None => {
                body.check(self, InferMode::Regular, None)?;
            }
        }

        self.constrain_function_value_receiver(node, symbol)
    }

    /// Require one function value's receiver to grant the access its body takes on its captures.
    fn constrain_function_value_receiver(
        &mut self,
        node: dir::GlobalNodeIdAny,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        // read the written callable's receiver term
        let callable = self.written_callable(symbol)?;
        let callable = self.shallow_resolve(callable)?;
        let dir::Type::Function(function) = self.ty(callable)? else {
            return Ok(());
        };

        // leave an owned receiver alone
        if self.receiver_mode(function.receiver)? == dir::ReceiverMode::Owned {
            return Ok(());
        }

        // require the receiver to grant the access the body takes
        let required = self.captured_access(node.module_id, symbol)?;
        let origin = Origin::Node(node, None);
        let requested = self.access_literal(required)?;
        let verdict = self.constrain_access_assignable(origin, function.receiver, requested)?;
        if verdict == Verdict::Fails {
            let granted = self.receiver_mode(function.receiver)?;
            self.report_receiver_access_not_granted(origin, required, granted)?;
        }

        // require a frame borrow to grant that access
        if let Some(callable) = self.committed_node_type(node)
            && let dir::Type::Form(dir::FormType {
                form: dir::Form::Borrowed(borrow),
                ..
            }) = self.ty(callable)?
        {
            let borrow = self.type_borrow(callable.module_id, borrow)?;
            let requested = self.access_literal(required.join(dir::Access::Mutable))?;
            self.constrain_access_assignable(origin, borrow.access, requested)?;
        }

        Ok(())
    }

    /// Return the access one closure takes of its captures.
    fn captured_access(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<dir::Access> {
        // read the uses the body makes of each captured binding
        let capture = self
            .module(module)
            .pending_captures
            .iter()
            .rev()
            .find(|capture| capture.symbol == symbol)
            .cloned()
            .ok_or_else(|| CompilerError::Internal {
                message: format!("checked function value {symbol:?} without its captures"),
            })?;
        let uses = self.captured_uses(&capture);
        let directive = self.resolve_capture_directive(symbol)?;

        // join the access each capture takes
        let mut access = dir::Access::Readonly;
        for (captured, captured_uses) in &uses {
            let captured_access = captured_uses.borrowed_access();
            let mode = self.capture_mode(module, directive.as_ref(), *captured)?;
            if mode == dir::CaptureMode::Borrow && captured_access.writes() {
                access = access.join(dir::Access::Exclusive);
            } else if captured_access.writes() {
                access = access.join(dir::Access::Mutable);
            }
        }

        Ok(access)
    }

    /// Return the signature of one callable type, with the module that interns its rows.
    pub(in crate::sema) fn callable_signature_head(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<(ModuleId, dir::FunctionSignatureType)>> {
        // read the signature the callable names, a handle through its form
        let ty = self.shallow_resolve(ty)?;
        let signature = match self.ty(ty)? {
            dir::Type::Function(function) => function.signature,
            dir::Type::FunctionSignature(_) => ty,
            dir::Type::Form(form) if matches!(form.form, dir::Form::Owned) => {
                return self.callable_signature_head(form.value);
            }
            _ => return Ok(None),
        };
        let head = self.signature_head(signature)?;

        Ok(head.map(|head| (signature.module_id, head)))
    }
}
