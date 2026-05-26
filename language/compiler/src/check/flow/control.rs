use destack_dir as dir;

use crate::check::{
    CheckModuleState, ConstraintOrigin, ControlTarget, FlowBranch, Obligation, TryFailureTerm,
    TryTarget, TypeLiteralTerm, TypeOperationTerm, TypeRelation, TypeTerm, VariableId,
    VariableKind,
};

impl CheckModuleState {
    /// Enter one break or continue target.
    pub(in crate::check) fn enter_control_target(
        &mut self,
        source: ConstraintOrigin,
        label: Option<dir::StringId>,
        allows_continue: bool,
        result: VariableId,
    ) {
        let checkpoint = self.work.flow.checkpoint();
        let target = ControlTarget {
            source,
            label,
            allows_continue,
            result,
            break_values: Vec::new(),
            break_branches: Vec::new(),
            continue_branches: Vec::new(),
            checkpoint,
        };

        self.work.flow.push_target(target);
    }

    /// Leave one break or continue target, define its result, and return break branch flow.
    pub(in crate::check) fn leave_control_target(
        &mut self,
        fallthrough: Option<TypeTerm>,
    ) -> Vec<FlowBranch> {
        let Some(target) = self.work.flow.pop_target() else {
            return Vec::new();
        };
        let term = match (target.break_values.as_slice(), fallthrough) {
            // fall through without break
            ([], Some(fallthrough)) => fallthrough,
            // loop expression with no exit
            ([], None) => TypeTerm::Literal(TypeLiteralTerm::Never),
            // single break branch
            ([value], None) => TypeTerm::Variable(*value),
            // multiple break and fallthrough branches
            (values, fallthrough) => {
                let mut elements =
                    Vec::with_capacity(values.len() + usize::from(fallthrough.is_some()));
                elements.extend_from_slice(values);
                if let Some(fallthrough) = fallthrough {
                    let variable = self.define_anonymous_type(target.source, fallthrough);

                    elements.push(variable);
                }

                TypeTerm::Operation(TypeOperationTerm::BestCommon { elements })
            }
        };

        self.define_type(target.result, term);

        target.break_branches
    }

    /// Enter one try failure target.
    pub(in crate::check) fn enter_try_target(&mut self, source: dir::LocalNodeIdAny) -> VariableId {
        let source = source.into_global(self.input.module_id);
        let origin = ConstraintOrigin::Node(source);
        let failure = self.allocate_anonymous_variable(VariableKind::Type, origin);
        let target = TryTarget {
            failure,
            failures: Vec::new(),
        };

        self.work.flow.push_try(target);

        failure
    }

    /// Leave one try failure target and define its collected failure type.
    pub(in crate::check) fn leave_try_target(&mut self) -> Option<VariableId> {
        let target = self.work.flow.pop_try()?;
        let term = match target.failures.as_slice() {
            // no propagated failure
            [] => TypeTerm::Literal(TypeLiteralTerm::Never),
            // single propagated failure
            [failure] => TypeTerm::Variable(*failure),
            // multiple propagated failures
            failures => TypeTerm::Operation(TypeOperationTerm::BestCommon {
                elements: failures.to_vec(),
            }),
        };

        self.define_type(target.failure, term);

        Some(target.failure)
    }

    /// Record one break value in its control target.
    pub(in crate::check) fn record_break_value(
        &mut self,
        source: dir::LocalNodeIdAny,
        label: Option<dir::StringId>,
        value: Option<VariableId>,
    ) {
        let value = match value {
            Some(value) => value,
            None => self.define_void_type(source),
        };
        let origin = ConstraintOrigin::Node(source.into_global(self.input.module_id));
        let Some(index) = self.work.flow.find_break_target_index(label) else {
            self.report_invalid_control_flow(source, "break has no target");

            return;
        };
        let checkpoint = self.work.flow.targets[index].checkpoint;
        let branch = self.work.flow.branch(checkpoint);
        let result = self.work.flow.targets[index].result;

        // record break value and captured branch flow
        let target = &mut self.work.flow.targets[index];
        target.break_values.push(value);
        target.break_branches.push(branch);

        self.constrain_type(origin, TypeRelation::Assignable, value, result);
    }

    /// Record one continue branch in its control target.
    pub(in crate::check) fn record_continue_branch(
        &mut self,
        source: dir::LocalNodeIdAny,
        label: Option<dir::StringId>,
    ) {
        let Some(index) = self.work.flow.find_continue_target_index(label) else {
            self.report_invalid_control_flow(source, "continue has no target");

            return;
        };
        let checkpoint = self.work.flow.targets[index].checkpoint;
        let branch = self.work.flow.branch(checkpoint);

        self.work.flow.targets[index].continue_branches.push(branch);
    }

    /// Take continue branches collected by the current control target.
    pub(in crate::check) fn take_current_continue_branches(&mut self) -> Vec<FlowBranch> {
        let Some(target) = self.work.flow.targets.last_mut() else {
            return Vec::new();
        };

        std::mem::take(&mut target.continue_branches)
    }

    /// Record one try propagation against catch or the enclosing return type.
    pub(in crate::check) fn record_try_propagation(
        &mut self,
        source: dir::LocalNodeIdAny,
        value: VariableId,
    ) {
        if self.work.flow.current_try_mut().is_some() {
            let source = source.into_global(self.input.module_id);
            let origin = ConstraintOrigin::Node(source);
            let failure = self.allocate_anonymous_variable(VariableKind::Type, origin);

            self.define_type(
                failure,
                TypeTerm::TryFailure(TryFailureTerm { source, value }),
            );

            if let Some(target) = self.work.flow.current_try_mut() {
                target.failures.push(failure);
            }

            return;
        }

        self.add_obligation(Obligation::TryPropagation {
            source: source.into_global(self.input.module_id),
            value,
            return_type: self.current_return_type(),
        });
    }
}
