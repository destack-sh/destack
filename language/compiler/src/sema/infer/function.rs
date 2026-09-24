use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::sema::{
    BodyCheck, Check, CheckState, Expectation, FlowState, InferMode, Origin, Settle, Verdict,
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

        // type the closure of its declaration symbol under the context
        self.commit_function_value(node)?;
        let symbol = self.function_value_symbol(node)?;
        let callable = self.symbol_type(symbol)?;
        let callable = self.contextual_form(callable, context)?;
        self.commit_node_type(node, callable)?;
        self.queue_check_function_body(node)?;

        Ok(callable)
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
        // read the parameter types of the declared callable
        let callable = self.symbol_type(self.function_value_symbol(node)?)?;
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
        // read the declared callable's receiver term
        let callable = self.symbol_type(symbol)?;
        let callable = self.shallow_resolve(callable)?;
        let dir::Type::Function(function) = self.ty(callable)? else {
            return Ok(());
        };

        // leave an owned receiver alone
        if self.receiver_mode(function.receiver)? == dir::ReceiverMode::Owned {
            return Ok(());
        }

        // read the bindings the body captures
        let state = self.module(node.module_id);
        let captured = state
            .pending_captures
            .iter()
            .rev()
            .find(|capture| capture.symbol == symbol)
            .map(|capture| capture.symbols.as_slice())
            .unwrap_or_default();

        // take the strongest access the body makes of a captured binding
        let mut required = dir::Access::Readonly;
        for occurrence in state.flows.binding_occurrences() {
            if !captured.contains(&occurrence.symbol) {
                continue;
            }
            if occurrence.uses.contains(dir::BindingUse::WRITE)
                || occurrence.uses.contains(dir::BindingUse::MUTATE)
            {
                required = required.join(dir::Access::Mutable);
            }
        }

        let origin = Origin::Node(node, None);

        // require the receiver to grant the access the body takes
        let requested = self.access_literal(required)?;
        let verdict = self.constrain_access_assignable(origin, function.receiver, requested)?;
        if verdict == Verdict::Fails {
            let granted = self.receiver_mode(function.receiver)?;
            self.report_receiver_access_not_granted(origin, required, granted)?;
        }

        Ok(())
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
