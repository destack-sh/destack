use destack_dir as dir;

use crate::check::{CheckModuleState, TypeInferId};

/// One callable shape ready for overload selection.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct CallCandidate {
    /// The callable symbol when the callee came from a declaration.
    pub(in crate::check) symbol: Option<dir::GlobalSymbolId>,
    /// The callable parameter types.
    pub(in crate::check) parameters: Vec<dir::LocalTypeId>,
    /// The callable return type.
    pub(in crate::check) return_type: Option<dir::LocalTypeId>,
}

impl CheckModuleState {
    /// Solve one call expression.
    pub(in crate::check) fn solve_call(
        &mut self,
        node: dir::GlobalNodeIdAny,
        callee: TypeInferId,
        arguments: &[TypeInferId],
        result: TypeInferId,
    ) -> Option<TypeInferId> {
        if self.infer_type_id(result).is_some() {
            return None;
        }
        let callee_type = self.infer_type_id(callee)?;
        let mut argument_types = Vec::new();
        for argument in arguments {
            argument_types.push(self.infer_type_id(*argument)?);
        }
        let callee_symbol = self.call_callee_symbol(node);
        let candidates = self.resolve_call_candidates(callee_type, callee_symbol);
        let selected = self.select_call_candidate(&candidates, &argument_types)?;
        let return_type = selected.return_type.unwrap_or_else(|| {
            self.types_mut()
                .insert_type_from_any(dir::Type::Void, node.local_id)
        });

        for (argument, parameter) in arguments.iter().zip(&selected.parameters) {
            self.replace_infer_type(*argument, *parameter);
        }
        self.set_call_resolution(node, selected, return_type);

        self.set_infer_type(result, return_type).then_some(result)
    }

    /// Resolve call candidates for one solved callee type.
    pub(in crate::check) fn resolve_call_candidates(
        &self,
        callee: dir::LocalTypeId,
        symbol: Option<dir::GlobalSymbolId>,
    ) -> Vec<CallCandidate> {
        match self.get_type(callee) {
            dir::Type::Function(function) => vec![CallCandidate {
                symbol,
                parameters: function.parameters,
                return_type: function.return_type,
            }],
            _ => Vec::new(),
        }
    }

    /// Select the first call candidate compatible with solved arguments.
    pub(in crate::check) fn select_call_candidate(
        &self,
        candidates: &[CallCandidate],
        arguments: &[dir::LocalTypeId],
    ) -> Option<CallCandidate> {
        candidates
            .iter()
            .find(|candidate| self.is_call_candidate_compatible(candidate, arguments))
            .cloned()
    }

    /// Return whether one call candidate accepts the argument types.
    fn is_call_candidate_compatible(
        &self,
        candidate: &CallCandidate,
        arguments: &[dir::LocalTypeId],
    ) -> bool {
        if candidate.parameters.len() != arguments.len() {
            return false;
        }

        candidate
            .parameters
            .iter()
            .zip(arguments)
            .all(|(parameter, argument)| self.is_type_assignable(*argument, *parameter))
    }

    /// Return the selected symbol for a call callee expression.
    fn call_callee_symbol(&self, node: dir::GlobalNodeIdAny) -> Option<dir::GlobalSymbolId> {
        if node.module_id != self.module() {
            return None;
        }
        let id = node.local_id.try_into_typed::<dir::Expression>().ok()?;
        let dir::Expression::Call { left, .. } = self.parsed().tree.get(id) else {
            return None;
        };
        let left = left.into_global_any(self.module());

        self.resolutions().symbol_resolution(left)
    }

    /// Commit one selected call resolution.
    fn set_call_resolution(
        &mut self,
        node: dir::GlobalNodeIdAny,
        selected: CallCandidate,
        return_type: dir::LocalTypeId,
    ) {
        let target = match selected.symbol {
            Some(symbol) => dir::CallTarget::Symbol(dir::CallCandidate {
                receiver: None,
                symbol,
                instance: None,
            }),
            None => dir::CallTarget::Value,
        };
        let resolution = dir::CallResolution::new(target, selected.parameters, Some(return_type));

        self.resolutions_mut().set_call_resolution(node, resolution);
    }
}
