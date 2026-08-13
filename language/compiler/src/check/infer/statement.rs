use std::sync::Arc;

use destack_artifact::{DirExpanded, DirParsed};
use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    BodyState, Cause, CauseKind, ConditionBranch, Constraint, ControlLabel, ControlTargetForm,
    Expectation, ExpectedType, FlowSite, GeneratorTargets, InferMode, Obligation, Origin,
    PatternCoverage, PatternCoverageObligation, PlaceUse, Relation, ValueUse, VariableRole,
    WalkState, Widening,
};
use crate::{CompilerError, CompilerResult};

impl BodyState<'_, '_> {
    /// Infer one statement-position expression.
    pub(in crate::check) fn infer_statement(
        &mut self,
        site: FlowSite,
        statement: &dir::Expression,
    ) -> CompilerResult<()> {
        let node = site.node;
        let module = node.module_id;

        match statement {
            // debugger
            dir::Expression::Debugger => {
                let void = self.check.intern_type(dir::Type::Void)?;
                self.check.commit_node_type(node, void)?;

                Ok(())
            }
            // let pattern = value else { ... }
            dir::Expression::LetElse {
                kind,
                declarator,
                else_branch,
                ..
            } => self.infer_let_else_statement(site, *kind, *declarator, *else_branch),
            // let x = value
            dir::Expression::Let {
                kind,
                declarators,
                export,
                is_ambient,
                ..
            } => {
                let exported = export.is_some();
                for declarator in declarators {
                    self.check_declarator(module, *declarator, Some(*kind), exported, *is_ambient)?;
                }
                let void = self.check.intern_type(dir::Type::Void)?;
                self.check.commit_node_type(node, void)?;

                Ok(())
            }
            // using resource = value
            dir::Expression::Using { declarators, .. } => {
                for declarator in declarators {
                    self.check_declarator(module, *declarator, None, false, false)?;
                }
                let void = self.check.intern_type(dir::Type::Void)?;
                self.check.commit_node_type(node, void)?;

                Ok(())
            }
            // return value
            dir::Expression::Return { value } => self.infer_return_statement(site, *value),
            // yield value / yield* value
            dir::Expression::Yield { cardinality, value } => {
                self.infer_yield_statement(site, *cardinality, *value)
            }
            // while (condition) { ... }
            dir::Expression::While {
                label,
                condition,
                body,
                ..
            } => self.infer_while_expression(site, *label, *condition, *body),
            // loop { ... }
            dir::Expression::Loop { label, body } => {
                self.infer_loop_expression(site, *label, *body)
            }
            // for (init; condition; increment) { ... }
            dir::Expression::For {
                label,
                initialization,
                condition,
                increment,
                body,
            } => self.infer_for_expression(
                site,
                *label,
                *initialization,
                *condition,
                *increment,
                *body,
            ),
            // break value
            dir::Expression::Break { label, value } => {
                self.infer_break_statement(site, *label, *value)
            }
            // continue
            dir::Expression::Continue { label } => self.infer_continue_statement(site, *label),
            // expressions that own no statement rule
            other => Err(CompilerError::Internal {
                message: format!("cannot infer statement {node:?}: {other:?}"),
            }),
        }
    }

    /// Infer one return statement against the body's return target.
    fn infer_return_statement(
        &mut self,
        site: FlowSite,
        value: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> CompilerResult<()> {
        let node = site.node;
        let module = node.module_id;

        // require an enclosing function body
        if self.check.flow.current_function().is_none() {
            self.check
                .report_return_outside_function(module, node.local_id);
        }

        // relate the returned value to the body's return target
        if let Some(value) = value {
            let value_site = self.check.visit_site(value.into_global_any(module))?;
            let expectation = self.return_type.map(|return_type| Expectation {
                target: return_type,
                relation: Relation::Assignable,
                cause: self.check.intern_cause(Cause::root(
                    value_site.origin(),
                    CauseKind::Return { annotation: None },
                )),
                use_: ValueUse::Output,
                mode: self.output_mode,
            });
            self.attempt_node(value_site, PlaceUse::Read, expectation)?;
        }
        // complete a bare return with void
        else if let Some(return_type) = self.return_type {
            let void = self.check.intern_type(dir::Type::Void)?;
            let cause = self.check.intern_cause(Cause::root(
                site.origin(),
                CauseKind::Return { annotation: None },
            ));
            self.check.push_constraint(Constraint::r#type(
                site.origin(),
                Relation::Assignable,
                void,
                return_type,
                cause,
            ))?;
        }

        // returns complete with never
        let never = self.check.intern_type(dir::Type::Never)?;
        self.check.commit_node_type(node, never)?;

        Ok(())
    }

    /// Infer one yield statement against the enclosing generator targets.
    fn infer_yield_statement(
        &mut self,
        site: FlowSite,
        cardinality: dir::YieldCardinality,
        value: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> CompilerResult<()> {
        let node = site.node;
        let module = node.module_id;

        // require a surrounding generator body
        if !self.check.is_in_generator() {
            self.check
                .report_yield_outside_generator(module, node.local_id);
        }

        let generator = self.generator;
        let ty = match (cardinality, value) {
            // yield* value: evaluate to the delegate's inner return
            (dir::YieldCardinality::Generator, Some(value)) => {
                self.infer_yield_delegate(site, value)?
            }
            // yield*: a delegate requires a value
            (dir::YieldCardinality::Generator, None) => {
                self.check
                    .report_yield_delegate_missing_value(module, node.local_id);

                self.check.intern_type(dir::Type::Error)?
            }
            // yield value: send the value, resume with the resume target
            (dir::YieldCardinality::Scalar, Some(value)) => {
                let value_site = self.check.visit_site(value.into_global_any(module))?;
                let expectation = generator.map(|targets| Expectation {
                    target: targets.yielded,
                    relation: Relation::Assignable,
                    cause: self.check.intern_cause(Cause::root(
                        value_site.origin(),
                        CauseKind::Return { annotation: None },
                    )),
                    use_: ValueUse::Output,
                    mode: self.output_mode,
                });
                self.attempt_node(value_site, PlaceUse::Read, expectation)?;

                self.yield_resume_type(generator)?
            }
            // yield: send void, resume with the resume target
            (dir::YieldCardinality::Scalar, None) => {
                if let Some(targets) = generator {
                    let void = self.check.intern_type(dir::Type::Void)?;
                    let cause = self
                        .check
                        .intern_cause(Cause::root(site.origin(), CauseKind::Expression));
                    self.check.push_constraint(Constraint::r#type(
                        site.origin(),
                        Relation::Assignable,
                        void,
                        targets.yielded,
                        cause,
                    ))?;
                }

                self.yield_resume_type(generator)?
            }
        };
        self.check.commit_node_type(node, ty)?;

        Ok(())
    }

    /// Infer one delegated yield against the generator protocol.
    fn infer_yield_delegate(
        &mut self,
        site: FlowSite,
        value: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let module = site.node.module_id;
        let value_site = self.check.visit_site(value.into_global_any(module))?;

        // without generator targets the delegate value just infers
        let Some(targets) = self.generator else {
            self.attempt_node(value_site, PlaceUse::Read, None)?;

            return self.check.intern_type(dir::Type::Error);
        };

        // the delegate return becomes the yield's own output
        let variable =
            self.check
                .allocate_variable(site.origin(), Widening::Never, VariableRole::Regular);
        let output = self.check.variable_type(variable)?;

        // the delegate value must implement the generator protocol
        let item = match targets.asynchrony {
            dir::Asynchrony::Sync => dir::LanguageItem::Iterable,
            dir::Asynchrony::Async => dir::LanguageItem::AsyncIterable,
        };
        let expected = self
            .check
            .language_type(item, &[targets.yielded, output, targets.resumed])?;
        let expectation = Expectation {
            target: expected,
            relation: Relation::Assignable,
            cause: self.check.intern_cause(Cause::root(
                value_site.origin(),
                CauseKind::Return { annotation: None },
            )),
            use_: ValueUse::Output,
            mode: self.output_mode,
        };
        self.attempt_node(value_site, PlaceUse::Read, Some(expectation))?;

        Ok(output)
    }

    /// Return the type a scalar yield resumes with.
    fn yield_resume_type(
        &mut self,
        generator: Option<GeneratorTargets>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        match generator {
            Some(targets) => Ok(targets.resumed),
            // yields outside a generator already reported
            None => self.check.intern_type(dir::Type::Error),
        }
    }

    /// Infer one while loop with its condition narrowing.
    fn infer_while_expression(
        &mut self,
        site: FlowSite,
        label: Option<dir::StringId>,
        condition: dir::LocalNodeId<dir::Expression>,
        body: dir::LocalNodeId<dir::Block>,
    ) -> CompilerResult<()> {
        let node = site.node;
        let module = node.module_id;
        self.check_condition(module, condition)?;

        // check the body under true condition flow inside the loop target
        let label = label.map(|name| ControlLabel { name, source: node });
        self.check
            .enter_control_target(label, ControlTargetForm::Iteration);
        let before_body = self.check.fork_flow();
        self.check
            .narrow_expression(condition, ConditionBranch::True)?;
        let body_site = self.check.visit_site(body.into_global_any(module))?;
        self.attempt_node(body_site, PlaceUse::Read, None)?;
        self.check.restore_flow(before_body);

        // collect the normal exit through the false condition
        self.check
            .narrow_expression(condition, ConditionBranch::False)?;
        let normal_flow = self.check.collect_flow_branch(before_body);
        let mut branches = self.check.leave_control_target();
        branches.push(normal_flow);
        self.check.merge_flow_branches_from(before_body, &branches);

        // complete the loop with void when the condition fails
        let void = self.check.intern_type(dir::Type::Void)?;
        self.check.commit_node_type(node, void)?;

        Ok(())
    }

    /// Infer one loop expression joined from its break values.
    fn infer_loop_expression(
        &mut self,
        site: FlowSite,
        label: Option<dir::StringId>,
        body: dir::LocalNodeId<dir::Block>,
    ) -> CompilerResult<()> {
        let node = site.node;
        let module = node.module_id;

        // open the loop output joined by break values
        let origin = site.origin();
        let variable = self
            .check
            .allocate_variable(origin, Widening::Never, VariableRole::Regular);
        let result = self.check.variable_type(variable)?;
        let label = label.map(|name| ControlLabel { name, source: node });
        self.check
            .enter_control_target(label, ControlTargetForm::Loop { result });

        // check the body with isolated flow
        let before_body = self.check.fork_flow();
        let body_site = self.check.visit_site(body.into_global_any(module))?;
        self.attempt_node(body_site, PlaceUse::Read, None)?;
        self.check.restore_flow(before_body);

        // restore only branches that leave the loop
        let branches = self.check.leave_control_target();

        // close break-free loops to never
        if branches.is_empty() {
            let never = self.check.intern_type(dir::Type::Never)?;
            let cause = self
                .check
                .intern_cause(Cause::root(origin, CauseKind::Expression));
            self.check.push_constraint(Constraint::r#type(
                origin,
                Relation::Equal,
                result,
                never,
                cause,
            ))?;
        }
        self.check.merge_flow_branches_from(before_body, &branches);
        self.check.commit_node_type(node, result)?;

        Ok(())
    }

    /// Infer one traditional for loop with its condition narrowing.
    fn infer_for_expression(
        &mut self,
        site: FlowSite,
        label: Option<dir::StringId>,
        initialization: Option<dir::LocalNodeId<dir::Expression>>,
        condition: Option<dir::LocalNodeId<dir::Expression>>,
        increment: Option<dir::LocalNodeId<dir::Expression>>,
        body: dir::LocalNodeId<dir::Block>,
    ) -> CompilerResult<()> {
        let node = site.node;
        let module = node.module_id;

        // check initialization and condition before the loop flow splits
        if let Some(initialization) = initialization {
            let init_site = self
                .check
                .visit_site(initialization.into_global_any(module))?;
            self.attempt_node(init_site, PlaceUse::Read, None)?;
        }
        if let Some(condition) = condition {
            self.check_condition(module, condition)?;
        }

        // check the body under true condition flow inside the loop target
        let label = label.map(|name| ControlLabel { name, source: node });
        self.check
            .enter_control_target(label, ControlTargetForm::Iteration);
        let before_body = self.check.fork_flow();
        if let Some(condition) = condition {
            self.check
                .narrow_expression(condition, ConditionBranch::True)?;
        }
        let body_site = self.check.visit_site(body.into_global_any(module))?;
        self.attempt_node(body_site, PlaceUse::Read, None)?;

        // check the increment under the joined flows reaching the next iteration
        if let Some(increment) = increment {
            let body_flow = self
                .check
                .block_can_complete_normally(body)
                .then(|| self.check.collect_flow_branch(before_body));
            let mut reaching = self.check.take_current_continue_branches();
            reaching.extend(body_flow);
            if reaching.is_empty() {
                self.check.restore_flow(before_body);
            } else {
                self.check.merge_flow_branches_from(before_body, &reaching);
            }
            let increment_site = self.check.visit_site(increment.into_global_any(module))?;
            self.attempt_node(increment_site, PlaceUse::Read, None)?;
        }
        self.check.restore_flow(before_body);

        // collect the normal exit through the false condition
        let normal_flow = if let Some(condition) = condition {
            self.check
                .narrow_expression(condition, ConditionBranch::False)?;

            Some(self.check.collect_flow_branch(before_body))
        } else {
            None
        };
        let mut branches = self.check.leave_control_target();
        branches.extend(normal_flow);
        self.check.merge_flow_branches_from(before_body, &branches);

        // for loops evaluate to void
        let void = self.check.intern_type(dir::Type::Void)?;
        self.check.commit_node_type(node, void)?;

        Ok(())
    }

    /// Check one break against its enclosing control target.
    fn infer_break_statement(
        &mut self,
        site: FlowSite,
        label: Option<dir::StringId>,
        value: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> CompilerResult<()> {
        let node = site.node;
        let module = node.module_id;

        match self.check.flow.break_target_index(label) {
            // bind the carried value to the resolved target
            Some(index) => {
                // record the selected label target
                if label.is_some() {
                    self.check.commit_label_target(node.local_id, index)?;
                }

                let form = self.check.flow.control_target_form(index);
                match (value, form) {
                    // valued breaks check against the target output
                    (Some(value), ControlTargetForm::Loop { result }) => {
                        let value_site = self.check.visit_site(value.into_global_any(module))?;
                        let expectation = Expectation::assignable(
                            result,
                            self.check.intern_cause(Cause::root(
                                value_site.origin(),
                                CauseKind::Return { annotation: None },
                            )),
                            ValueUse::Output,
                        );
                        self.attempt_node(value_site, PlaceUse::Read, Some(expectation))?;
                    }
                    // values need a valued target
                    (Some(value), ControlTargetForm::Iteration | ControlTargetForm::Switch) => {
                        self.check
                            .report_break_value_outside_loop(module, node.local_id);
                        let value_site = self.check.visit_site(value.into_global_any(module))?;
                        self.attempt_node(value_site, PlaceUse::Read, None)?;
                    }
                    // bare breaks exit with void
                    (None, ControlTargetForm::Loop { result }) => {
                        let void = self.check.intern_type(dir::Type::Void)?;
                        let cause = self.check.intern_cause(Cause::root(
                            site.origin(),
                            CauseKind::Return { annotation: None },
                        ));
                        self.check.push_constraint(Constraint::r#type(
                            site.origin(),
                            Relation::Assignable,
                            void,
                            result,
                            cause,
                        ))?;
                    }
                    (None, ControlTargetForm::Iteration | ControlTargetForm::Switch) => {}
                }

                // capture branch flow at the break site
                let checkpoint = self.check.flow.control_target_checkpoint(index);
                let branch = self.check.flow.branch(checkpoint);
                self.check.flow.push_break_branch(index, branch);
            }
            // report unbound breaks
            None => {
                self.check
                    .report_break_outside_control_target(module, node.local_id);
                self.check.flow.mark_unbound_jump(node.local_id);
            }
        }

        // breaks complete with never
        let never = self.check.intern_type(dir::Type::Never)?;
        self.check.commit_node_type(node, never)?;

        Ok(())
    }

    /// Record one continue against its enclosing loop target.
    fn infer_continue_statement(
        &mut self,
        site: FlowSite,
        label: Option<dir::StringId>,
    ) -> CompilerResult<()> {
        let node = site.node;
        self.check
            .continue_to_control_target(node.local_id, label)?;

        // continues complete with never
        let never = self.check.intern_type(dir::Type::Never)?;
        self.check.commit_node_type(node, never)?;

        Ok(())
    }

    /// Infer one let-else statement with its diverging else branch.
    fn infer_let_else_statement(
        &mut self,
        site: FlowSite,
        kind: dir::LetKind,
        declarator: dir::LocalNodeId<dir::Declarator>,
        else_branch: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        let node = site.node;
        let module = node.module_id;
        self.check_declarator(module, declarator, Some(kind), false, false)?;

        // check the diverging else branch in failed-match flow
        let before_else = self.check.fork_flow();
        self.check.narrow_declarator_pattern(declarator, false)?;
        let else_site = self.check.visit_site(else_branch.into_global_any(module))?;
        self.attempt_node(else_site, PlaceUse::Read, None)?;
        self.check.restore_flow(before_else);

        // require the else branch to leave the binding scope
        if self.check.expression_can_complete_normally(else_branch) {
            self.check
                .report_let_else_branch_can_complete(module, else_branch.into_any());
        }

        // continue with the matched bindings assigned
        self.check.narrow_let_condition(declarator)?;
        let void = self.check.intern_type(dir::Type::Void)?;
        self.check.commit_node_type(node, void)?;

        Ok(())
    }

    /// Check one declarator at its source evaluation position.
    fn check_declarator(
        &mut self,
        module: ModuleId,
        id: dir::LocalNodeId<dir::Declarator>,
        binding_kind: Option<dir::LetKind>,
        exported: bool,
        is_ambient: bool,
    ) -> CompilerResult<()> {
        // skip statically absent declarators, they keep no entries
        if self.check.is_absent(id.into_global_any(module)) {
            return Ok(());
        }

        // read the written declarator shape once
        let declarator = self.module(module).view().get(id).clone();
        let pattern = declarator.pattern;

        // walk the written annotation at its first typing visit
        if let Some(ty) = declarator.ty {
            self.walk_body_type_expression(module, ty)?;
        }

        // read the written annotation as the binding's storage type
        let annotation = declarator.ty.map(|ty| ty.into_global_any(module));
        let written = match annotation {
            Some(annotation) => {
                let ty = self.require_node_type(annotation)?;
                // fall back to the pattern's origin for declared-stage annotations
                let origin = match self.check.node_origin_maybe(annotation) {
                    Some(origin) => origin,
                    None => self.visit_site(pattern.into_global_any(module))?.origin(),
                };

                Some(self.storage_type(origin, ty)?)
            }
            None => None,
        };

        // derive the type the pattern destructures from
        let target = match (declarator.value, written) {
            // transcribe the written type while declaring
            (Some(_), Some(written)) if self.is_declaration() => {
                match self.check.type_flags(written)?.has_variable() {
                    true => None,
                    false => Some(written),
                }
            }
            // check the initializer against the written type
            (Some(value), Some(written)) => {
                let site = self.visit_site(value.into_global_any(module))?;
                let cause = self.intern_cause(Cause::root(
                    site.origin(),
                    CauseKind::Initializer { annotation },
                ));
                let expectation = Expectation {
                    target: written,
                    relation: Relation::Assignable,
                    cause,
                    use_: ValueUse::Store,
                    mode: InferMode::Exact,
                };
                self.attempt_node(site, PlaceUse::Read, Some(expectation))?;

                Some(written)
            }
            // leave unexported initializers to the body pass
            (Some(_), None) if self.is_declaration() && !exported => None,
            // error for exported non-transcribable values
            (Some(value), None)
                if exported && !self.check.is_transcribable_literal(module, value) =>
            {
                // infer the body while checking
                if !self.is_declaration() {
                    let site = self.visit_site(value.into_global_any(module))?;
                    self.infer_node(site, PlaceUse::Read, InferMode::Widen)?;
                }

                Some(self.intern_type(dir::Type::Error)?)
            }
            // infer the binding type from its initializer
            (Some(value), None) => {
                let site = self.visit_site(value.into_global_any(module))?;
                let widening = self
                    .check
                    .declarator_widening(module, &declarator, binding_kind);
                let mode = match widening {
                    Widening::Never
                    | Widening::Aggregate
                    | Widening::Multiple
                    | Widening::Comptime => InferMode::Exact,
                    Widening::Always => InferMode::Widen,
                };
                let ty = self.infer_node(site, PlaceUse::Read, mode)?;
                let ty = self.flow_type_at(site, ty)?;

                // take the value's base type for a widening name binding
                let is_name_binding = matches!(
                    self.module(module).view().get(pattern),
                    dir::Pattern::Binding { .. }
                );
                let ty = match widening {
                    Widening::Always if is_name_binding => self.check.widen_type(ty)?,
                    _ => ty,
                };

                Some(ty)
            }
            // uninitialized declarators take their written type
            (None, Some(written)) => {
                match self.is_declaration() && self.check.type_flags(written)?.has_variable() {
                    true => None,
                    false => Some(written),
                }
            }
            // nothing written and nothing to infer
            (None, None) => None,
        };

        // check the pattern against the destructured value
        if let Some(target) = target {
            let site = self.visit_site(pattern.into_global_any(module))?;
            self.check_pattern(pattern.into_global(module), site.flow, site.scope, target)?;

            // non-matching positions must always match irrefutably
            let requires_irrefutable = {
                let view = self.module(module).view();
                match view.get_parent(id.into_any().id) {
                    Some(parent) if parent.ty == dir::NodeType::Expression => {
                        let expression =
                            view.get(dir::LocalNodeId::<dir::Expression>::new(parent.id));
                        !matches!(
                            expression,
                            dir::Expression::LetElse { declarator, .. }
                                if declarator == &id
                        ) && !matches!(
                            expression,
                            dir::Expression::If { condition, .. }
                                if condition
                                    .as_binding()
                                    .is_some_and(|(_, _, declarator)| declarator == id)
                        )
                    }
                    _ => true,
                }
            };
            if requires_irrefutable {
                let value = match declarator.value {
                    Some(value) => ExpectedType::Node(value.into_global_any(module)),
                    None => ExpectedType::Type(target),
                };
                let scope = self.check.flow.template_scope();
                self.check.push_obligation(
                    Obligation::PatternCoverage(PatternCoverageObligation {
                        source: pattern.into_global_any(module),
                        value,
                        coverage: PatternCoverage::Binding {
                            pattern: pattern.into_global(module),
                        },
                    }),
                    scope,
                );
            }
        }

        // mark the declared and ambient bindings assigned
        {
            let node = self.module(module).view().get(id).clone();
            self.check.mark_declarator_assigned(&node, is_ambient);
        }

        Ok(())
    }

    /// Check every expression operand of one condition.
    pub(in crate::check) fn check_condition_operands(
        &mut self,
        module: ModuleId,
        condition: &dir::Condition,
    ) -> CompilerResult<()> {
        for operand in &condition.operands {
            match operand {
                dir::ConditionOperand::Expression { condition } => {
                    self.check_condition(module, *condition)?;
                }
                dir::ConditionOperand::Binding {
                    kind, declarator, ..
                } => {
                    self.check_declarator(module, *declarator, Some(*kind), false, false)?;
                }
            }
        }

        Ok(())
    }

    /// Check one loop or branch condition against boolean.
    fn check_condition(
        &mut self,
        module: ModuleId,
        condition: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        let site = self.check.visit_site(condition.into_global_any(module))?;
        let boolean = self
            .check
            .intern_type(dir::Type::Primitive(dir::PrimitiveType::Boolean))?;
        let expectation = Expectation {
            target: boolean,
            relation: Relation::Assignable,
            cause: self.check.intern_cause(Cause::root(
                Origin::Node(condition.into_global_any(module), site.scope),
                CauseKind::Expression,
            )),
            use_: ValueUse::Condition,
            mode: InferMode::Exact,
        };
        self.attempt_node(site, PlaceUse::Read, Some(expectation))?;

        Ok(())
    }
}

impl BodyState<'_, '_> {
    /// Return the parsed and expanded inputs one patched module view reads.
    pub(in crate::check) fn patched_inputs(
        &self,
        module: ModuleId,
    ) -> (Arc<DirParsed>, Arc<DirExpanded>) {
        let state = self.module(module);

        (state.parsed.clone(), state.expanded.clone())
    }

    /// Walk one body node's decorators at its typing visit.
    ///
    /// Ordinary decorators register their application for the pass
    /// apply; the returned presence gates the node.
    pub(in crate::check) fn walk_body_decorators(
        &mut self,
        module: ModuleId,
        decorated: dir::LocalNodeIdAny,
    ) -> CompilerResult<bool> {
        // undecorated nodes are present, no walk decides anything for them
        if !self.check.module_view(module).has_decorators_any(decorated) {
            return Ok(true);
        }

        let (parsed, expanded) = self.patched_inputs(module);
        let tree = dir::View::with_patches(&parsed.tree, std::slice::from_ref(&expanded.patch));
        let mut walk = WalkState::new(module, tree, self.check);
        let present = walk.walk_decorators(decorated)?;
        walk.flush_flows()?;

        Ok(present)
    }

    /// Walk one argument list's decorators and return the arguments their conditions keep.
    pub(in crate::check) fn walk_body_arguments(
        &mut self,
        module: ModuleId,
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) -> CompilerResult<SmallVec<[dir::LocalNodeId<dir::Argument>; 4]>> {
        let mut present = SmallVec::with_capacity(arguments.len());

        // keep each argument its static condition decided present
        for argument in arguments {
            if self.walk_body_decorators(module, argument.into_any())? {
                present.push(*argument);
            }
        }

        Ok(present)
    }

    /// Walk one body type expression at its first typing visit.
    pub(in crate::check) fn walk_body_construct_type(
        &mut self,
        module: ModuleId,
        ty: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<()> {
        // reuse a full earlier visit, not a declared-stage decision
        if self.check.decision(ty.into_global_any(module)).is_some()
            && self
                .check
                .committed_node_type(ty.into_global_any(module))
                .is_some()
        {
            return Ok(());
        }
        let (parsed, expanded) = self.patched_inputs(module);
        let tree = dir::View::with_patches(&parsed.tree, std::slice::from_ref(&expanded.patch));
        let mut walk = WalkState::new(module, tree, self.check);
        walk.walk_construct_type_expression(ty)?;

        Ok(())
    }

    /// Walk one static term at its first typing visit.
    pub(in crate::check) fn walk_body_static_term(
        &mut self,
        module: ModuleId,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let (parsed, expanded) = self.patched_inputs(module);
        let tree = dir::View::with_patches(&parsed.tree, std::slice::from_ref(&expanded.patch));
        let mut walk = WalkState::new(module, tree, self.check);

        walk.walk_static_term(expression)
    }

    /// Walk one body annotation type at its first typing visit.
    pub(in crate::check) fn walk_body_type_expression(
        &mut self,
        module: ModuleId,
        ty: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<()> {
        // reuse the annotation an earlier pass walked, holes and all
        let node = ty.into_global_any(module);
        if let Some(declared) = &self.check.module.declared
            && declared.types.get_node_type_id(node).is_some()
        {
            return Ok(());
        }

        // reuse a settled commit; one still open re-walks fresh
        if let Some(committed) = self.check.committed_node_type(node)
            && !self.check.type_flags(committed)?.has_variable()
        {
            return Ok(());
        }

        let (parsed, expanded) = self.patched_inputs(module);
        let tree = dir::View::with_patches(&parsed.tree, std::slice::from_ref(&expanded.patch));

        // close elided borrows at static in module positions, at frame in bodies
        let is_body_position = self.is_body_annotation(module, ty);

        // walk the annotation in its closing position
        let mut walk = WalkState::new(module, tree, self.check);
        match is_body_position {
            // open lifetime holes for inference in body positions
            true => walk.walk_type_expression(ty)?,
            false => walk.walk_static_type_expression(ty)?,
        };

        Ok(())
    }

    /// Return whether one written annotation sits inside a function body.
    fn is_body_annotation(
        &self,
        module: ModuleId,
        ty: dir::LocalNodeId<dir::TypeExpression>,
    ) -> bool {
        let view = self.module(module).view();
        let mut current = view.get_parent(ty.into_any().id);
        while let Some(parent) = current {
            if parent.ty == dir::NodeType::Declaration {
                let declaration = view.get(dir::LocalNodeId::<dir::Declaration>::new(parent.id));
                if matches!(declaration, dir::Declaration::Function(_)) {
                    return true;
                }
            }
            current = view.get_parent(parent.id);
        }

        false
    }

    /// Walk one guard target type at its first typing visit.
    ///
    /// Guard targets describe runtime-narrowed values, so their elided
    /// borrows close at the frame regardless of position.
    pub(in crate::check) fn walk_body_guard_type_expression(
        &mut self,
        module: ModuleId,
        ty: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<()> {
        // reuse a closed type an earlier walk already committed
        if let Some(committed) = self.check.committed_node_type(ty.into_global_any(module))
            && self.check.type_variables(committed)?.is_empty()
        {
            return Ok(());
        }
        let (parsed, expanded) = self.patched_inputs(module);
        let tree = dir::View::with_patches(&parsed.tree, std::slice::from_ref(&expanded.patch));
        let mut walk = WalkState::new(module, tree, self.check);
        walk.walk_frame_type_expression(ty)?;

        Ok(())
    }

    /// Walk explicit generic arguments at their first typing visit.
    pub(in crate::check) fn walk_body_generic_arguments(
        &mut self,
        module: ModuleId,
        arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    ) -> CompilerResult<()> {
        // reuse types an earlier walk already committed
        let has_unwalked = arguments.iter().any(|argument| {
            self.check
                .committed_node_type(argument.into_global_any(module))
                .is_none()
        });
        if !has_unwalked {
            return Ok(());
        }
        let (parsed, expanded) = self.patched_inputs(module);
        let tree = dir::View::with_patches(&parsed.tree, std::slice::from_ref(&expanded.patch));
        let mut walk = WalkState::new(module, tree, self.check);
        walk.walk_generic_arguments(arguments)?;

        Ok(())
    }

    /// Register one function value's body at its first typing visit.
    ///
    /// Return whether the declaration is a function value; statements keep their declaration path.
    pub(in crate::check) fn register_function_value(
        &mut self,
        node: dir::GlobalNodeIdAny,
        declaration: dir::LocalNodeId<dir::Declaration>,
    ) -> CompilerResult<bool> {
        let module = node.module_id;
        if self.check.lambdas.contains_key(&node) {
            return Ok(true);
        }
        let (parsed, expanded) = self.patched_inputs(module);
        let tree = dir::View::with_patches(&parsed.tree, std::slice::from_ref(&expanded.patch));
        let kind = tree.get(declaration).clone();
        let is_lambda = matches!(
            &kind,
            dir::Declaration::Function(function)
                if function.signature.form == dir::FunctionForm::Lambda
                    || function.name.is_none()
        );
        if !is_lambda {
            return Ok(false);
        }

        // declare the signature, register the body, and key it by value
        let mut walk = WalkState::new(module, tree, self.check);
        walk.walk_declaration(declaration, &kind)?;
        let symbol = walk
            .check
            .module(module)
            .declaration_symbol(declaration.into_any())
            .ok_or_else(|| CompilerError::Internal {
                message: format!("function value {node:?} has no declaration symbol"),
            })?;
        if let dir::Declaration::Function(function) = &kind {
            let _registered = walk.walk_declared_function_body(declaration, function, symbol)?;
        }
        walk.flush_flows()?;

        // move the body to its value expression, which owns the check
        let Some(body) = self.check.functions.swap_remove(&symbol) else {
            return Err(CompilerError::Internal {
                message: format!("function value {} has no body", self.check.node_label(node)),
            });
        };
        self.check.lambdas.insert(node, body);

        Ok(true)
    }

    /// Register the function values among one call's arguments.
    ///
    /// Registration is durable syntax; it runs before candidate probes
    /// so rollbacks never unregister a body.
    pub(in crate::check) fn register_argument_function_values(
        &mut self,
        module: ModuleId,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
    ) -> CompilerResult<()> {
        for argument in argument_nodes {
            let Some(value) = self.argument_expression(module, *argument) else {
                continue;
            };
            let expression = self
                .module(value.module_id)
                .view()
                .get(value.local_id.into_typed::<dir::Expression>())
                .clone();
            if let dir::Expression::Declaration(declaration) = expression {
                self.register_function_value(value, declaration)?;
            }
        }

        Ok(())
    }

    /// Type one declaration statement, registering the bodies it owns.
    pub(in crate::check) fn infer_declaration_statement(
        &mut self,
        site: FlowSite,
        declaration: dir::LocalNodeId<dir::Declaration>,
    ) -> CompilerResult<()> {
        let node = site.node;
        let module = node.module_id;

        // type a function value with its expression
        if self.register_function_value(node, declaration)? {
            self.check_function_value(site, None, InferMode::Exact)?;

            return Ok(());
        }
        let (parsed, expanded) = self.patched_inputs(module);
        let tree = dir::View::with_patches(&parsed.tree, std::slice::from_ref(&expanded.patch));
        let kind = tree.get(declaration).clone();

        // type declaration statements as void
        let void = self.check.intern_type(dir::Type::Void)?;
        self.check.commit_node_type(node, void)?;

        // type module and global block members in authored order
        let members = match &kind {
            dir::Declaration::Global(block) => Some(block.expressions.clone()),
            dir::Declaration::Module(block) => Some(block.expressions.clone()),
            _ => None,
        };
        if let Some(members) = members {
            for member in members {
                let member_site = self.check.visit_site(member.into_global_any(module))?;
                self.attempt_node(member_site, PlaceUse::Read, None)?;
            }

            return Ok(());
        }

        // register the declared bodies of other declarations
        let mut walk = WalkState::new(module, tree, self.check);
        walk.visit_body_declaration_statement(declaration, &kind)?;

        Ok(())
    }
}
