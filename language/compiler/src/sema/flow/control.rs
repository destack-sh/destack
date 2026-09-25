use tspp_dir as dir;

use crate::sema::{
    CheckState, ControlLabel, ControlTarget, ControlTargetForm, FlowBranch, TryTarget,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Return the exact binding for one optional control label.
    pub(in crate::sema) fn control_label(
        &self,
        source: dir::GlobalNodeId<dir::Expression>,
        name: Option<dir::StringId>,
    ) -> CompilerResult<Option<ControlLabel>> {
        let Some(name) = name else {
            return Ok(None);
        };

        // require the binding declared for this labeled control target
        let module = self.module(source.module_id);
        let symbol = module
            .declaration_symbol(source.local_id.into_any())
            .ok_or_else(|| CompilerError::Internal {
                message: format!("control label {source:?} has no binding"),
            })?;
        let bindings = module.binding_table();
        let binding = bindings.get_symbol(symbol.local_id);
        if binding.kind != dir::SymbolKind::Label || binding.name() != Some(name) {
            return Err(CompilerError::Internal {
                message: format!("control label {source:?} has incompatible binding {symbol:?}"),
            });
        }

        Ok(Some(ControlLabel { name, symbol }))
    }

    /// Enter one break or continue target.
    pub(in crate::sema) fn enter_control_target(
        &mut self,
        source: dir::GlobalNodeId<dir::Expression>,
        label: Option<ControlLabel>,
        form: ControlTargetForm,
    ) {
        // capture flow state before the control body
        let checkpoint = self.flow.fork();
        let target = ControlTarget {
            source,
            label,
            form,
            break_branches: Vec::new(),
            continue_branches: Vec::new(),
            checkpoint,
        };

        // expose target to nested break and continue expressions
        self.flow.push_target(target);
    }

    /// Leave one break or continue target and return its break branch flow.
    pub(in crate::sema) fn leave_control_target(&mut self) -> Vec<FlowBranch> {
        let target = self.flow.pop_target();

        target.break_branches
    }

    /// Enter one try failure target.
    pub(in crate::sema) fn enter_try_target(&mut self, node: dir::GlobalNodeId<dir::Expression>) {
        // expose target to nested try propagation
        self.flow.push_try(TryTarget {
            node,
            failures: Vec::new(),
        });
    }

    /// Leave one try failure target and compute its collected failure type.
    pub(in crate::sema) fn leave_try_target(&mut self) -> CompilerResult<dir::GlobalTypeId> {
        let target = self.flow.pop_try();

        // the caught value is the union of the body's failures
        match target.failures.as_slice() {
            [] => Ok(self.intern_type(dir::Type::Never)?),
            [failure] => Ok(*failure),
            failures => {
                let failures = failures.to_vec();

                self.normalized_union_type(failures)
            }
        }
    }

    /// Continue to one enclosing loop target.
    pub(in crate::sema) fn continue_to_control_target(
        &mut self,
        source: dir::GlobalNodeId<dir::Expression>,
        label: Option<dir::StringId>,
    ) -> CompilerResult<()> {
        // resolve the chosen loop target
        let Some(index) = self.flow.continue_target_index(label) else {
            self.report_continue_outside_loop(source.module_id, source.local_id.into_any());

            // unbound jumps already emitted diagnostics
            self.flow.insert_unbound_jump(source.local_id.into_any());

            return Ok(());
        };

        // record the selected control target
        self.commit_transfer_target(source, index)?;

        // capture branch flow at the continue site
        let checkpoint = self.flow.control_target_checkpoint(index);
        let branch = self.flow.branch(checkpoint);

        self.flow.push_continue_branch(index, branch);

        Ok(())
    }

    /// Commit the target selected by one control transfer.
    pub(in crate::sema) fn commit_transfer_target(
        &mut self,
        source: dir::GlobalNodeId<dir::Expression>,
        index: usize,
    ) -> CompilerResult<()> {
        let target = self.flow.control_target_source(index);

        self.commit_decision(source.into_any(), dir::Decision::Transfer(target))
    }

    /// Take continue branches collected by the current control target.
    pub(in crate::sema) fn take_current_continue_branches(&mut self) -> Vec<FlowBranch> {
        self.flow.take_continue_branches()
    }

    /// Collect the failure one try site propagates into the open try target.
    pub(in crate::sema) fn collect_try_failure(
        &mut self,
        failure: dir::GlobalTypeId,
    ) -> Option<dir::GlobalNodeId<dir::Expression>> {
        match self.flow.current_try_mut() {
            Some(target) => {
                target.failures.push(failure);

                Some(target.node)
            }
            None => None,
        }
    }
}
