use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{
    ControlTarget, FlowBranch, Obligation, Origin, Relation, TryObligation, TryTarget, WalkState,
};

impl WalkState<'_, '_> {
    /// Enter one break or continue target.
    pub(in crate::check) fn enter_control_target(
        &mut self,
        label: Option<dir::StringId>,
        allows_continue: bool,
        source: dir::LocalNodeId<dir::Expression>,
        result: dir::GlobalTypeId,
    ) {
        // capture flow state before the control body
        let checkpoint = self.flow().fork();
        let target = ControlTarget {
            label,
            allows_continue,
            source: source.into_global_any(self.module),
            result,
            break_values: Vec::new(),
            break_branches: Vec::new(),
            continue_branches: Vec::new(),
            checkpoint,
        };

        // expose target to nested break and continue expressions
        self.flow_mut().push_target(target);
    }

    /// Leave one break or continue target, define its result, and return break branch flow.
    /// Break values already bound the result at their break sites.
    pub(in crate::check) fn leave_control_target(
        &mut self,
        fallthrough: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Vec<FlowBranch>> {
        // remove target before resolving its result
        let target = self.flow_mut().pop_target();
        let origin = Origin::Node(target.source);

        match (target.break_values.as_slice(), fallthrough) {
            // fall through without break
            ([], Some(fallthrough)) => {
                self.relate_types(origin, Relation::Equal, target.result, fallthrough);
            }
            // loop expression with no exit
            ([], None) => {
                let never = self.push_type(dir::Type::Never, target.source.local_id)?;

                self.relate_types(origin, Relation::Equal, target.result, never);
            }
            // break exits already bound the result, add the fallthrough exit
            (_, Some(fallthrough)) => {
                self.relate_types(origin, Relation::Assignable, fallthrough, target.result);
            }
            // break exits already bound the result
            (_, None) => {}
        }

        // return branches that escaped by break
        Ok(target.break_branches)
    }

    /// Enter one try failure target.
    pub(in crate::check) fn enter_try_target(
        &mut self,
        source: dir::LocalNodeIdAny,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // open the failure result variable
        let failure = self.open_type(source)?;
        let target = TryTarget {
            failure,
            failures: Vec::new(),
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
        if target.failures.is_empty() {
            let origin = Origin::Node(source.into_global(self.module));
            let never = self.push_type(dir::Type::Never, source)?;

            self.relate_types(origin, Relation::Equal, target.failure, never);
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
        let origin = Origin::Node(source.into_global(self.module));

        // resolve the selected control target
        let Some(index) = self.flow().break_target_index(label) else {
            self.check
                .report_invalid_control_flow(self.module, source, "break has no target");

            return Ok(());
        };

        // capture branch flow at the break site
        let (checkpoint, result) = self.flow().control_target_result(index);
        let branch = self.flow().branch(checkpoint);

        // store value and captured branch flow
        self.flow_mut().push_break_branch(index, value, branch);

        // require the break value to match the target result
        self.relate_types(origin, Relation::Assignable, value, result);

        Ok(())
    }

    /// Continue to one control target.
    pub(in crate::check) fn continue_to_control_target(
        &mut self,
        source: dir::LocalNodeIdAny,
        label: Option<dir::StringId>,
    ) {
        // resolve the selected loop target
        let Some(index) = self.flow().continue_target_index(label) else {
            self.check
                .report_invalid_control_flow(self.module, source, "continue has no target");

            return;
        };

        // capture branch flow at the continue site
        let (checkpoint, _) = self.flow().control_target_result(index);
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
        value: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        // collect the local failure projection on the innermost try target
        if self.flow_mut().current_try_mut().is_some() {
            let operation = dir::TypeOperation::TryResidual { value };
            let failure = self.push_type(dir::Type::Operation(operation), source)?;
            let origin = Origin::Node(source.into_global(self.module));

            let mut result = None;
            if let Some(target) = self.flow_mut().current_try_mut() {
                target.failures.push(failure);
                result = Some(target.failure);
            }
            if let Some(result) = result {
                self.relate_types(origin, Relation::Assignable, failure, result);
            }

            return Ok(());
        }

        // propagate to the enclosing function
        let return_type = self.current_return_target();
        let condition = self.flow().active_static_guard();
        self.check.push_obligation(Obligation::Try(TryObligation {
            source: source.into_global(self.module),
            condition,
            value,
            return_type,
        }));

        Ok(())
    }
}
