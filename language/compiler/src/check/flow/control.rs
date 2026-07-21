use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{
    CauseKind, ControlTarget, ControlTargetForm, FlowBranch, Origin, Relation, TryTarget,
    VariableRole, WalkState, Widening,
};

/// Receiver of one propagated try failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) enum TryPropagationTarget {
    /// A local try target failure type.
    Failure {
        /// The local failure accumulator type.
        ty: dir::GlobalTypeId,
    },
    /// The enclosing function return type.
    Return {
        /// The function return type, if propagation is inside a function.
        ty: Option<dir::GlobalTypeId>,
    },
}

impl WalkState<'_, '_> {
    /// Enter one break or continue target.
    pub(in crate::check) fn enter_control_target(
        &mut self,
        label: Option<dir::StringId>,
        form: ControlTargetForm,
    ) {
        // capture flow state before the control body
        let checkpoint = self.flow().fork();
        let target = ControlTarget {
            label,
            form,
            break_branches: Vec::new(),
            continue_branches: Vec::new(),
            checkpoint,
        };

        // expose target to nested break and continue expressions
        self.flow_mut().push_target(target);
    }

    /// Leave one break or continue target and return its break branch flow.
    pub(in crate::check) fn leave_control_target(&mut self) -> Vec<FlowBranch> {
        let target = self.flow_mut().pop_target();

        target.break_branches
    }

    /// Enter one try failure target.
    pub(in crate::check) fn enter_try_target(
        &mut self,
        source: dir::LocalNodeIdAny,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // infer the failure result variable
        let failure = self.open_type_hole(source, Widening::Never, VariableRole::Regular)?;
        let target = TryTarget {
            failure,
            has_failure: false,
        };

        // expose target to nested try propagation
        self.flow_mut().push_try(target);

        Ok(failure)
    }

    /// Leave one try failure target and define its collected failure type.
    pub(in crate::check) fn leave_try_target(
        &mut self,
        source: dir::LocalNodeIdAny,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // remove target before resolving its failure result
        let target = self.flow_mut().pop_try();

        // close failure-free try bodies to never
        if !target.has_failure {
            let origin = Origin::Node(
                source.into_global(self.module),
                self.flow().template_scope(),
            );
            let never = self.intern_type(dir::Type::Never)?;

            self.relate_type(
                origin,
                CauseKind::Expression,
                Relation::Equal,
                target.failure,
                never,
            );
        }

        Ok(target.failure)
    }

    /// Break to one control target.
    pub(in crate::check) fn break_to_control_target(
        &mut self,
        source: dir::LocalNodeIdAny,
        label: Option<dir::StringId>,
        value: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> CompilerResult<()> {
        // resolve the chosen control target
        let Some(index) = self.flow().break_target_index(label) else {
            self.check
                .report_break_outside_control_target(self.module, source);
            // unbound jumps already emitted diagnostics
            self.flow_mut().mark_unbound_jump(source);

            return Ok(());
        };

        // bind the carried value to the target output
        match (value, self.flow().control_target_form(index)) {
            // valued breaks check against the output at inference
            (
                Some(value),
                ControlTargetForm::Block { result } | ControlTargetForm::Loop { result },
            ) => {
                self.check
                    .control_results
                    .insert(value.into_global_any(self.module), result);
            }
            (Some(_), ControlTargetForm::Iteration | ControlTargetForm::Switch) => {
                self.check
                    .report_break_value_outside_loop(self.module, source);
            }
            // bare breaks exit with void
            (None, ControlTargetForm::Block { result } | ControlTargetForm::Loop { result }) => {
                let origin = Origin::Node(
                    source.into_global(self.module),
                    self.flow().template_scope(),
                );
                let void = self.intern_type(dir::Type::Void)?;

                self.relate_type(
                    origin,
                    CauseKind::Return { annotation: None },
                    Relation::Assignable,
                    void,
                    result,
                );
            }
            (None, ControlTargetForm::Iteration | ControlTargetForm::Switch) => {}
        }

        // capture branch flow at the break site
        let checkpoint = self.flow().control_target_checkpoint(index);
        let branch = self.flow().branch(checkpoint);

        self.flow_mut().push_break_branch(index, branch);

        Ok(())
    }

    /// Continue to one control target.
    pub(in crate::check) fn continue_to_control_target(
        &mut self,
        source: dir::LocalNodeIdAny,
        label: Option<dir::StringId>,
    ) {
        // resolve the chosen loop target
        let Some(index) = self.flow().continue_target_index(label) else {
            self.check.report_continue_outside_loop(self.module, source);

            // unbound jumps already emitted diagnostics
            self.flow_mut().mark_unbound_jump(source);

            return;
        };

        // capture branch flow at the continue site
        let checkpoint = self.flow().control_target_checkpoint(index);
        let branch = self.flow().branch(checkpoint);

        self.flow_mut().push_continue_branch(index, branch);
    }

    /// Take continue branches collected by the current control target.
    pub(in crate::check) fn take_current_continue_branches(&mut self) -> Vec<FlowBranch> {
        self.flow_mut().take_continue_branches()
    }

    /// Record one try propagation target for its inference site.
    pub(in crate::check) fn propagate_try(
        &mut self,
        source: dir::LocalNodeIdAny,
    ) -> CompilerResult<()> {
        let source = source.into_global(self.module);
        let target = if let Some(target) = self.flow_mut().current_try_mut() {
            target.has_failure = true;
            TryPropagationTarget::Failure { ty: target.failure }
        } else {
            TryPropagationTarget::Return {
                ty: self.current_return_target(),
            }
        };

        self.check.try_propagations.insert(source, target);

        Ok(())
    }
}
