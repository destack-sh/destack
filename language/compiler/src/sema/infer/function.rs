use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::sema::{BodyCheck, Check, CheckState, FlowState, InferMode, Settle};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Check one function value's body in its receiving context.
    pub(in crate::sema) fn function_value_type(
        &mut self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // type a function value once
        if let Some(callable) = self.committed_node_type(node) {
            return Ok(callable);
        }

        // read the declared callable type of its body
        let Some(body) = self.lambdas.get(&node) else {
            return Err(CompilerError::Internal {
                message: format!("function value {node:?} has no body"),
            });
        };
        let callable = self.symbol_type(body.symbol)?;
        self.commit_node_type(node, callable)?;
        self.schedule_function_body(node)?;

        Ok(callable)
    }

    /// Check one function value's body in place once its slots close.
    ///
    /// A body with open slots waits behind them under the current flow.
    pub(in crate::sema) fn schedule_function_body(
        &mut self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<()> {
        // check a body with no open slots in place
        let slots = self.function_value_parameters(node)?;
        if slots.is_empty() {
            self.check_function_body(node)?;
        }
        // otherwise stall the body behind its slots under the current flow
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
        // read the parameter slots of the committed callable
        let Some(callable) = self.committed_node_type(node) else {
            return Ok(SmallVec::new());
        };
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

        // fix the parameter slots the body reads
        let roots = self.function_value_parameters(node)?;
        self.fix_variables(&roots)?;
        self.settle_variables(&roots, Settle::All)?;

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

        Ok(())
    }

    /// Return the signature one callable type carries, with the module owning its rows.
    pub(in crate::sema) fn callable_signature_head(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<(ModuleId, dir::FunctionSignatureType)>> {
        // read the signature the callable carries
        let ty = self.shallow_resolve(ty)?;
        let signature = match self.ty(ty)? {
            dir::Type::Function(function) => function.signature,
            dir::Type::FunctionSignature(_) => ty,
            _ => return Ok(None),
        };
        let head = self.signature_head(signature)?;

        Ok(head.map(|head| (signature.module_id, head)))
    }
}
