use destack_dir as dir;

use crate::check::{
    ControlTarget, FlowBranch, Obligation, Origin, TryFailureTerm, TryTarget, TypeLiteralTerm,
    TypeOperand, TypeOperationTerm, TypeRelation, TypeTerm, VariableId, VariableKind, WalkState,
};

impl WalkState<'_, '_> {
    /// Enter one break or continue target.
    pub(in crate::check) fn enter_control_target(
        &mut self,
        label: Option<dir::StringId>,
        allows_continue: bool,
        result: VariableId,
    ) {
        // capture flow state before the control body
        let checkpoint = self.flow().fork();
        let target = ControlTarget {
            label,
            allows_continue,
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
    pub(in crate::check) fn leave_control_target(
        &mut self,
        fallthrough: Option<TypeTerm>,
    ) -> Vec<FlowBranch> {
        // remove target before resolving its result
        let target = self.flow_mut().pop_target();
        let condition = self.flow().active_static_guard();

        // define the control expression result
        match (target.break_values.as_slice(), fallthrough) {
            // fall through without break
            ([], Some(fallthrough)) => {
                self.check
                    .equate_type(target.result, fallthrough, condition);
            }
            // loop expression with no exit
            ([], None) => {
                let term = TypeTerm::Literal(TypeLiteralTerm::Never);

                self.check.equate_type(target.result, term, condition);
            }
            // single break branch
            ([value], None) => {
                let origin = self.check.variable(target.result).source;

                self.check.relate_type(
                    origin,
                    TypeRelation::Equal,
                    target.result,
                    *value,
                    condition,
                );
            }
            // multiple break and fallthrough branches
            (values, fallthrough) => {
                let mut elements =
                    Vec::with_capacity(values.len() + usize::from(fallthrough.is_some()));
                elements.extend(values.iter().copied());

                // include the fallthrough value as another exit
                if let Some(fallthrough) = fallthrough {
                    let term = self.check.push_term(fallthrough);

                    elements.push(term.into());
                }

                // compute the common result of every exit
                let operation = self
                    .check
                    .push_term(TypeOperationTerm::BestCommon { elements });

                let term = TypeTerm::Operation(operation);

                self.check.equate_type(target.result, term, condition);
            }
        }

        // return branches that escaped by break
        target.break_branches
    }

    /// Enter one try failure target.
    pub(in crate::check) fn enter_try_target(&mut self, source: dir::LocalNodeIdAny) -> VariableId {
        // create the failure result variable
        let source = source.into_global(self.module);
        let origin = Origin::Node(source);
        let failure = self
            .check
            .allocate_variable(self.module, VariableKind::Type, origin);
        let target = TryTarget {
            failure,
            failures: Vec::new(),
        };

        // expose target to nested try propagation
        self.flow_mut().push_try(target);

        failure
    }

    /// Leave one try failure target and define its collected failure type.
    pub(in crate::check) fn leave_try_target(&mut self) -> VariableId {
        // remove target before resolving its failure result
        let target = self.flow_mut().pop_try();
        let condition = self.flow().active_static_guard();

        // define the propagated failure type
        match target.failures.as_slice() {
            // no propagated failure
            [] => {
                let term = TypeTerm::Literal(TypeLiteralTerm::Never);

                self.check.equate_type(target.failure, term, condition);
            }
            // single propagated failure
            [failure] => {
                let origin = self.check.variable(target.failure).source;

                self.check.relate_type(
                    origin,
                    TypeRelation::Equal,
                    target.failure,
                    *failure,
                    condition,
                );
            }
            // multiple propagated failures
            failures => {
                let operation = self.check.push_term(TypeOperationTerm::BestCommon {
                    elements: failures.to_vec(),
                });

                let term = TypeTerm::Operation(operation);

                self.check.equate_type(target.failure, term, condition);
            }
        }

        target.failure
    }

    /// Record one break on its control target.
    pub(in crate::check) fn record_break(
        &mut self,
        source: dir::LocalNodeIdAny,
        label: Option<dir::StringId>,
        value: Option<impl Into<TypeOperand>>,
    ) {
        // normalize omitted break values to void
        let value = value
            .map(Into::into)
            .unwrap_or_else(|| self.void_type_operand());
        let origin = Origin::Node(source.into_global(self.module));

        // resolve the selected control target
        let Some(index) = self.flow().find_break_target_index(label) else {
            self.check
                .report_invalid_control_flow(self.module, source, "break has no target");

            return;
        };

        // capture branch flow at the break site
        let checkpoint = self.flow().targets[index].checkpoint;
        let branch = self.flow().branch(checkpoint);
        let result = self.flow().targets[index].result;

        // store value and captured branch flow
        let target = &mut self.flow_mut().targets[index];
        target.break_values.push(value);
        target.break_branches.push(branch);

        let condition = self.flow().active_static_guard();

        // require the break value to match the target result
        self.check
            .relate_type(origin, TypeRelation::Assignable, value, result, condition);
    }

    /// Record one continue on its control target.
    pub(in crate::check) fn record_continue(
        &mut self,
        source: dir::LocalNodeIdAny,
        label: Option<dir::StringId>,
    ) {
        // resolve the selected loop target
        let Some(index) = self.flow().find_continue_target_index(label) else {
            self.check
                .report_invalid_control_flow(self.module, source, "continue has no target");

            return;
        };

        // capture branch flow at the continue site
        let checkpoint = self.flow().targets[index].checkpoint;
        let branch = self.flow().branch(checkpoint);

        self.flow_mut().targets[index]
            .continue_branches
            .push(branch);
    }

    /// Take continue branches collected by the current control target.
    pub(in crate::check) fn take_current_continue_branches(&mut self) -> Vec<FlowBranch> {
        // require a surrounding control target
        let Some(target) = self.flow_mut().targets.last_mut() else {
            panic!("continue branch collection requires an active control target");
        };

        std::mem::take(&mut target.continue_branches)
    }

    /// Propagate one try result to catch or the enclosing return type.
    pub(in crate::check) fn propagate_try(
        &mut self,
        source: dir::LocalNodeIdAny,
        value: impl Into<TypeOperand>,
    ) {
        let value = value.into();

        // collect local try failure
        if self.flow_mut().current_try_mut().is_some() {
            let source = source.into_global(self.module);
            let tried = self.check.push_term(TryFailureTerm { source, value });
            let failure = self.check.push_term(TypeTerm::TryFailure(tried));

            // record failure on the innermost try target
            if let Some(target) = self.flow_mut().current_try_mut() {
                target.failures.push(failure.into());
            }

            return;
        }

        // propagate to the enclosing function
        self.check.require(Obligation::TryPropagation {
            source: source.into_global(self.module),
            value,
            return_type: self.current_return_type(),
            condition: self.flow().active_static_guard(),
        });
    }
}
