use destack_mir::{Lifetime, LocalNodeIdAny, Origin, Path, Terminator, TypeId, Value};

use crate::verify::VerifyError;

use super::checker::FunctionChecker;

impl FunctionChecker<'_, '_> {
    /// Check one returned value.
    pub(super) fn check_return(&mut self, value: Value, anchor: LocalNodeIdAny) {
        if !self
            .tree
            .type_contains_borrowed_refs(self.function.return_type)
        {
            return;
        }

        let bindings = self.state.value_borrows(&self.context(), value);

        self.check_return_bindings(bindings, anchor);
    }

    /// Check tail-call result retention against the function return lifetime.
    pub(super) fn check_tail_call_return(
        &mut self,
        terminator: &Terminator,
        anchor: LocalNodeIdAny,
    ) {
        let Terminator::TailCall { call } = terminator else {
            return;
        };
        let arguments = self.tree.get_values(call.arguments);
        let bindings = self
            .context()
            .call_result(&self.state, call.signature, arguments);

        self.check_return_bindings(bindings, anchor);
    }

    /// Check returned borrow bindings against declared return lifetimes.
    fn check_return_bindings(&mut self, bindings: Vec<(Path, Origin)>, anchor: LocalNodeIdAny) {
        if self.check_frame_escape(&bindings, anchor) {
            return;
        }

        let return_type = self.function.return_type;
        let return_paths = self.tree.type_borrowed_paths(return_type);
        let return_lifetime = self.tree.type_lifetime(return_type);

        for (path, origin) in bindings {
            // require a proven origin
            let is_unproven = !origin.has_region();

            // check the matching explicit return lifetime when one exists
            let required = return_paths
                .iter()
                .find(|borrowed| borrowed.path == path)
                .map(|borrowed| borrowed.lifetime.clone());
            let required = if required.is_none() && path.is_root() {
                return_lifetime.clone()
            } else {
                required
            };
            let is_uncovered = match &required {
                Some(required) => !origin.is_covered_by(required, &self.function.lifetimes),
                None => true,
            };
            if is_unproven || is_uncovered {
                self.verification
                    .emit_error(VerifyError::BorrowOutlivesOrigin {
                        anchor: self.verification.anchor(anchor),
                    });

                return;
            }
        }
    }

    /// Check declared lifetime relations for one terminator call.
    pub(super) fn check_terminator_call_outlives(
        &mut self,
        terminator: &Terminator,
        anchor: LocalNodeIdAny,
    ) {
        match terminator {
            Terminator::Invoke { call, .. } | Terminator::TailCall { call } => {
                self.check_call_outlives(
                    &call.signature,
                    self.tree.get_values(call.arguments),
                    anchor,
                );
            }
            _ => {}
        }
    }

    /// Check declared lifetime relations against call arguments.
    pub(super) fn check_call_outlives(
        &mut self,
        signature: &TypeId,
        arguments: &[Value],
        anchor: LocalNodeIdAny,
    ) {
        let Some((lifetimes, parameters, _)) = self.tree.get(*signature).function_signature_parts()
        else {
            unreachable!("call has no function signature")
        };
        let parameter_types = parameters
            .iter()
            .map(|parameter| parameter.ty)
            .collect::<Vec<_>>();

        // prove every declared outlives bound from the actual argument origin
        for (slot, parameter) in lifetimes.iter().enumerate() {
            for target in &parameter.outlives {
                let longer = self.context().map_lifetime(
                    &self.state,
                    &Lifetime::slot(slot as u32),
                    lifetimes,
                    &parameter_types,
                    arguments,
                );
                let shorter = self.context().map_lifetime(
                    &self.state,
                    &Lifetime::slot(target.0),
                    lifetimes,
                    &parameter_types,
                    arguments,
                );

                // defer result-only slots to result mapping
                if shorter.is_empty() {
                    continue;
                }
                if !longer.outlives(&shorter, &self.function.lifetimes) {
                    self.verification
                        .emit_error(VerifyError::BorrowOutlivesOrigin {
                            anchor: self.verification.anchor(anchor),
                        });
                }
            }
        }
    }

    /// Check the arguments a tail call hands out of the frame it replaces.
    pub(super) fn check_tail_call(&mut self, arguments: &[Value], anchor: LocalNodeIdAny) {
        for argument in arguments.iter().copied() {
            let bindings = self.state.value_borrows(&self.context(), argument);
            self.check_frame_escape(&bindings, anchor);
        }
    }

    /// Reject bindings leaving the frame while borrowing it, reporting whether any does.
    fn check_frame_escape(&mut self, bindings: &[(Path, Origin)], anchor: LocalNodeIdAny) -> bool {
        let escaping = bindings
            .iter()
            .filter(|(_, origin)| origin.has_local_region())
            .collect::<Vec<_>>();
        if escaping.is_empty() {
            return false;
        }

        self.verification
            .emit_error(VerifyError::BorrowOutlivesOrigin {
                anchor: self.verification.anchor(anchor),
            });
        for (_, origin) in escaping {
            for loan in origin.loans() {
                self.rejected_loans.insert(loan.index());
            }
        }

        true
    }
}
