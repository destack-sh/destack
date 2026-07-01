use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{
    ControlTarget, Expectation, FlowBranch, FlowSite, Origin, Relation, Task, TryPropagation,
    TryPropagationTarget, TryTarget, ValueUse, WalkState, Widening,
};

impl WalkState<'_, '_> {
    /// Enter one break or continue target.
    pub(in crate::check) fn enter_control_target(
        &mut self,
        label: Option<dir::StringId>,
        allows_continue: bool,
        source: dir::LocalNodeId<dir::Expression>,
    ) {
        // capture flow state before the control body
        let checkpoint = self.flow().fork();
        let target = ControlTarget {
            label,
            allows_continue,
            source: source.into_global_any(self.module),
            break_values: Vec::new(),
            break_branches: Vec::new(),
            continue_branches: Vec::new(),
            checkpoint,
        };

        // expose target to nested break and continue expressions
        self.flow_mut().push_target(target);
    }

    /// Leave one break or continue target and return its result with break branch flow.
    pub(in crate::check) fn leave_control_target(
        &mut self,
        fallthrough: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<(dir::GlobalTypeId, Vec<FlowBranch>)> {
        // remove target before resolving its result
        let target = self.flow_mut().pop_target();

        // collect exits that produce the control expression value
        let mut values = target.break_values;
        values.extend(fallthrough);

        // compute the result carried by all exiting paths
        let result = match values.as_slice() {
            [] => self.push_type(dir::Type::Never, target.source.local_id)?,
            [single] => *single,
            _ => self.normalized_union_type(values, target.source.local_id)?,
        };

        // return branches that escaped by break
        Ok((result, target.break_branches))
    }

    /// Enter one try failure target.
    pub(in crate::check) fn enter_try_target(
        &mut self,
        source: dir::LocalNodeIdAny,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // infer the failure result variable
        let failure = self.open_type_hole(source, Widening::Preserve)?;
        let target = TryTarget {
            failure,
            has_failure: false,
        };

        // expose target to nested try propagation
        self.flow_mut().push_try(target);

        Ok(failure)
    }

    /// Leave one try failure target and define its collected failure type.
    /// Collected failures bound the target at their propagation sites.
    pub(in crate::check) fn leave_try_target(
        &mut self,
        source: dir::LocalNodeIdAny,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // remove target before resolving its failure result
        let target = self.flow_mut().pop_try();

        // close failure-free try bodies to never
        if !target.has_failure {
            let origin = Origin::Node(source.into_global(self.module));
            let never = self.push_type(dir::Type::Never, source)?;

            self.relate_type(origin, Relation::Equal, target.failure, never);
        }

        Ok(target.failure)
    }

    /// Break to one control target.
    pub(in crate::check) fn break_to_control_target(
        &mut self,
        source: dir::LocalNodeIdAny,
        label: Option<dir::StringId>,
        value: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<()> {
        // use void for omitted break values
        let value = match value {
            Some(value) => value,
            None => self.push_type(dir::Type::Void, source)?,
        };

        // resolve the chosen control target
        let Some(index) = self.flow().break_target_index(label) else {
            self.check
                .report_break_outside_control_target(self.module, source);
            // unbound jumps already emitted diagnostics
            self.flow_mut().mark_unbound_jump(source);

            return Ok(());
        };

        // capture branch flow at the break site
        let checkpoint = self.flow().control_target_checkpoint(index);
        let branch = self.flow().branch(checkpoint);

        // store value and captured branch flow
        self.flow_mut().push_break_branch(index, value, branch);

        Ok(())
    }

    /// Return the output type carried by one control-flow value expression.
    pub(in crate::check) fn output_value_type(
        &mut self,
        source: dir::LocalNodeIdAny,
        value: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // bind the output edge once the value expression is checked
        let ty = self.open_type_hole(source, Widening::Preserve)?;
        let expectation = Expectation::assignable(
            ty,
            Origin::Node(value.into_global_any(self.module)),
            ValueUse::Output,
        );
        self.queue_node_check(value, expectation)?;

        Ok(ty)
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

    /// Propagate one try result to catch or the enclosing return type.
    pub(in crate::check) fn propagate_try(
        &mut self,
        source: dir::LocalNodeIdAny,
    ) -> CompilerResult<()> {
        let node = source.into_global(self.module);

        self.propagate_try_value(source, node)
    }

    /// Queue one try propagation task.
    pub(in crate::check) fn propagate_try_value(
        &mut self,
        source: dir::LocalNodeIdAny,
        value: dir::GlobalNodeIdAny,
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

        self.check.queue_task(Task::Propagate(TryPropagation {
            source,
            value: FlowSite {
                node: value,
                flow: self.flow().point(),
            },
            target,
        }));

        Ok(())
    }
}
