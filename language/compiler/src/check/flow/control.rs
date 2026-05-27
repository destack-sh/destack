use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{
    CheckState, ConstraintOrigin, ControlTarget, FlowBranch, Obligation, TryFailureTerm, TryTarget,
    TypeLiteralTerm, TypeOperand, TypeOperationTerm, TypeRelation, TypeTerm, VariableId,
    VariableKind,
};

impl CheckState<'_> {
    /// Enter one break or continue target.
    pub(in crate::check) fn enter_control_target(
        &mut self,
        module: ModuleId,
        label: Option<dir::StringId>,
        allows_continue: bool,
        result: VariableId,
    ) {
        let checkpoint = self.flow(module).checkpoint();
        let target = ControlTarget {
            label,
            allows_continue,
            result,
            break_values: Vec::new(),
            break_branches: Vec::new(),
            continue_branches: Vec::new(),
            checkpoint,
        };

        self.flow_mut(module).push_target(target);
    }

    /// Leave one break or continue target, define its result, and return break branch flow.
    pub(in crate::check) fn leave_control_target(
        &mut self,
        module: ModuleId,
        fallthrough: Option<TypeTerm>,
    ) -> Vec<FlowBranch> {
        let target = self.flow_mut(module).pop_target();
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
                elements.extend(values.iter().copied().map(TypeOperand::from));
                if let Some(fallthrough) = fallthrough {
                    let term = self.terms.push(fallthrough);

                    elements.push(term.into());
                }

                let operation = self.terms.push(TypeOperationTerm::BestCommon { elements });

                TypeTerm::Operation(operation)
            }
        };

        self.define_type(module, target.result, term);

        target.break_branches
    }

    /// Enter one try failure target.
    pub(in crate::check) fn enter_try_target(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) -> VariableId {
        let source = source.into_global(module);
        let origin = ConstraintOrigin::Node(source);
        let failure = self.allocate_intermediate_variable(module, VariableKind::Type, origin);
        let target = TryTarget {
            failure,
            failures: Vec::new(),
        };

        self.flow_mut(module).push_try(target);

        failure
    }

    /// Leave one try failure target and define its collected failure type.
    pub(in crate::check) fn leave_try_target(&mut self, module: ModuleId) -> VariableId {
        let target = self.flow_mut(module).pop_try();
        let term = match target.failures.as_slice() {
            // no propagated failure
            [] => TypeTerm::Literal(TypeLiteralTerm::Never),
            // single propagated failure
            [failure] => TypeTerm::Variable(*failure),
            // multiple propagated failures
            failures => {
                let operation = self.terms.push(TypeOperationTerm::BestCommon {
                    elements: failures.iter().copied().map(TypeOperand::from).collect(),
                });

                TypeTerm::Operation(operation)
            }
        };

        self.define_type(module, target.failure, term);

        target.failure
    }

    /// Record one break value in its control target.
    pub(in crate::check) fn record_break_value(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        label: Option<dir::StringId>,
        value: Option<VariableId>,
    ) {
        let value = match value {
            Some(value) => value,
            None => self.define_void_type(module, source),
        };
        let origin = ConstraintOrigin::Node(source.into_global(module));
        let Some(index) = self.flow(module).find_break_target_index(label) else {
            self.report_invalid_control_flow(module, source, "break has no target");

            return;
        };
        let checkpoint = self.flow(module).targets[index].checkpoint;
        let branch = self.flow(module).branch(checkpoint);
        let result = self.flow(module).targets[index].result;

        // record break value and captured branch flow
        let target = &mut self.flow_mut(module).targets[index];
        target.break_values.push(value);
        target.break_branches.push(branch);

        self.constrain_type(origin, TypeRelation::Assignable, value, result);
    }

    /// Record one continue branch in its control target.
    pub(in crate::check) fn record_continue_branch(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        label: Option<dir::StringId>,
    ) {
        let Some(index) = self.flow(module).find_continue_target_index(label) else {
            self.report_invalid_control_flow(module, source, "continue has no target");

            return;
        };
        let checkpoint = self.flow(module).targets[index].checkpoint;
        let branch = self.flow(module).branch(checkpoint);

        self.flow_mut(module).targets[index]
            .continue_branches
            .push(branch);
    }

    /// Take continue branches collected by the current control target.
    pub(in crate::check) fn take_current_continue_branches(
        &mut self,
        module: ModuleId,
    ) -> Vec<FlowBranch> {
        let Some(target) = self.flow_mut(module).targets.last_mut() else {
            return Vec::new();
        };

        std::mem::take(&mut target.continue_branches)
    }

    /// Record one try propagation against catch or the enclosing return type.
    pub(in crate::check) fn record_try_propagation(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        value: VariableId,
    ) {
        if self.flow_mut(module).current_try_mut().is_some() {
            let source = source.into_global(module);
            let origin = ConstraintOrigin::Node(source);
            let failure = self.allocate_intermediate_variable(module, VariableKind::Type, origin);
            let tried = self.terms.push(TryFailureTerm { source, value });

            self.define_type(module, failure, TypeTerm::TryFailure(tried));

            if let Some(target) = self.flow_mut(module).current_try_mut() {
                target.failures.push(failure);
            }

            return;
        }

        self.add_obligation(Obligation::TryPropagation {
            source: source.into_global(module),
            value,
            return_type: self.current_return_type(module),
            condition: self.active_static_condition(module),
        });
    }
}
