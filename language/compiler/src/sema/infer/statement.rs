use std::sync::Arc;

use smallvec::SmallVec;
use tspp_artifact::{DirExpanded, DirParsed};
use tspp_dir as dir;
use tspp_source::ModuleId;

use crate::sema::{
    Cause, CauseKind, CheckState, ConditionBranch, ControlTargetForm, ElisionSite, Expectation,
    ExpectedType, FlowBranch, FlowSite, GeneratorTargets, InferMode, NodeForm, Obligation, Origin,
    PatternCoverage, PatternCoverageObligation, PlaceUse, Relation, RelationCheck,
    SharedStorageObligation, StoreTarget, ValueUse, WalkState,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Infer one statement-position expression.
    pub(in crate::sema) fn infer_statement(
        &mut self,
        site: FlowSite,
        statement: &dir::Expression,
        mode: InferMode,
    ) -> CompilerResult<()> {
        // read the statement's node
        let node = site.node;
        let module = node.module_id;

        // infer by the statement form
        match statement {
            // debugger
            dir::Expression::Debugger => {
                let void = self.intern_type(dir::Type::Void)?;
                self.commit_node_type(node, void)?;

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
                is_shared,
                ..
            } => {
                let is_exported = export.is_some();
                for declarator in declarators {
                    self.check_declarator(
                        module,
                        *declarator,
                        Some(*kind),
                        is_exported,
                        *is_ambient,
                        *is_shared,
                    )?;
                }

                let void = self.intern_type(dir::Type::Void)?;
                self.commit_node_type(node, void)?;

                Ok(())
            }
            // using resource = value
            dir::Expression::Using {
                asynchrony,
                declarators,
                ..
            } => {
                for declarator in declarators {
                    self.check_declarator(module, *declarator, None, false, false, false)?;

                    // select the disposal the binding runs at scope exit
                    let pattern = self.module(module).view().get(*declarator).pattern;
                    let anchor = declarator.into_global_any(module);
                    self.record_disposal(anchor, pattern, *asynchrony)?;
                }

                let void = self.intern_type(dir::Type::Void)?;
                self.commit_node_type(node, void)?;

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
            } => self.infer_while_expression(site, *label, condition, *body),
            // loop { ... }
            dir::Expression::Loop { label, body } => {
                self.infer_loop_expression(site, *label, *body, mode)
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
            // leave expressions without a statement rule
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
        // read the return's node
        let node = site.node;
        let module = node.module_id;

        // require an enclosing function body
        if self.flow.current_function().is_none() {
            self.report_return_outside_function(module, node.local_id);
        }

        // reject values returned from constructors
        if self.current_initializes().is_some() && value.is_some() {
            self.report_constructor_return_value(module, node.local_id);
        }

        // check the returned value against the body's contextual return
        if let Some(value) = value {
            let value_site = self.visit_site(value.into_global_any(module))?;
            let expectation = self.current_return_target().map(|return_type| Expectation {
                target: return_type,
                relation: Relation::Storable,
                cause: self.intern_cause(Cause::root(
                    value_site.origin(),
                    CauseKind::Return { annotation: None },
                )),
                use_: ValueUse::Output,
                mode: self.current_output_mode(),
                store: StoreTarget::Exact,
            });
            self.attempt_node(value_site, PlaceUse::Read, expectation)?;
        }
        // complete a bare return with void
        else if let Some(return_type) = self.current_return_target() {
            let void = self.intern_type(dir::Type::Void)?;
            let cause = self.intern_cause(Cause::root(
                site.origin(),
                CauseKind::Return { annotation: None },
            ));
            self.push_relation(RelationCheck::new(
                site.origin(),
                Relation::Storable,
                void,
                return_type,
                cause,
            ))?;
        }

        // returns complete with never
        let never = self.intern_type(dir::Type::Never)?;
        self.commit_node_type(node, never)?;

        Ok(())
    }

    /// Return the open slot one binding pattern declares.
    fn binding_slot(
        &mut self,
        module: ModuleId,
        pattern: dir::LocalNodeId<dir::Pattern>,
    ) -> CompilerResult<Option<dir::TypeVariableId>> {
        // require a declared symbol
        let Some(symbol) = self.module(module).declaration_symbol(pattern.into_any()) else {
            return Ok(None);
        };

        // name the slot by the committed type, an uncommitted symbol by its one variable
        if let Some(ty) = self
            .binding_type_maybe(symbol)
            .or(self.declaration_type_maybe(symbol))
        {
            return self.root_variable(ty);
        }

        Ok(Some(self.symbol_variable(symbol)))
    }

    /// Infer one yield statement against the enclosing generator targets.
    fn infer_yield_statement(
        &mut self,
        site: FlowSite,
        cardinality: dir::YieldCardinality,
        value: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> CompilerResult<()> {
        // read the yield's node
        let node = site.node;
        let module = node.module_id;

        // require a surrounding generator body
        if !self.is_in_generator() {
            self.report_yield_outside_generator(module, node.local_id);
        }

        // evaluate each yield form against the enclosing targets
        let generator = self.current_generator();
        let ty = match (cardinality, value) {
            // yield* value: evaluate to the delegate's inner return
            (dir::YieldCardinality::Generator, Some(value)) => {
                self.infer_yield_delegate(site, value)?
            }
            // yield*: a delegate requires a value
            (dir::YieldCardinality::Generator, None) => {
                self.report_yield_delegate_missing_value(module, node.local_id);

                self.intern_type(dir::Type::Error)?
            }
            // yield value: send the value, resume with the resume target
            (dir::YieldCardinality::Scalar, Some(value)) => {
                let value_site = self.visit_site(value.into_global_any(module))?;
                let expectation = generator.map(|targets| Expectation {
                    target: targets.yielded,
                    relation: Relation::Storable,
                    cause: self.intern_cause(Cause::root(
                        value_site.origin(),
                        CauseKind::Return { annotation: None },
                    )),
                    use_: ValueUse::Output,
                    mode: self.current_output_mode(),
                    store: StoreTarget::Exact,
                });
                self.attempt_node(value_site, PlaceUse::Read, expectation)?;

                self.yield_resume_type(generator)?
            }
            // yield: send void, resume with the resume target
            (dir::YieldCardinality::Scalar, None) => {
                if let Some(targets) = generator {
                    let void = self.intern_type(dir::Type::Void)?;
                    let cause =
                        self.intern_cause(Cause::root(site.origin(), CauseKind::Expression));
                    self.push_relation(RelationCheck::new(
                        site.origin(),
                        Relation::Storable,
                        void,
                        targets.yielded,
                        cause,
                    ))?;
                }

                self.yield_resume_type(generator)?
            }
        };

        // select and record the producer yield call this statement runs
        if let (Some(targets), Some(completed)) = (generator, self.current_return_target())
            && !matches!(cardinality, dir::YieldCardinality::Generator)
        {
            let bound = [targets.yielded, completed, targets.resumed];
            let value = value.map(|value| value.into_global_any(module));
            self.commit_yield_call(node, targets.asynchrony, &bound, value)?;
        }

        self.commit_node_type(node, ty)?;

        Ok(())
    }

    /// Infer one delegated yield against the generator protocol.
    fn infer_yield_delegate(
        &mut self,
        site: FlowSite,
        value: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // visit the delegate value
        let module = site.node.module_id;
        let value_site = self.visit_site(value.into_global_any(module))?;

        // infer the delegate value alone outside a generator body
        let Some(targets) = self.current_generator() else {
            self.attempt_node(value_site, PlaceUse::Read, None)?;

            return self.intern_type(dir::Type::Error);
        };

        // open the output of the yield for the delegate return
        let variable = self.open_variable(site.origin());
        let output = self.variable_type(variable)?;

        // require the generator protocol of the delegate value
        let item = match targets.asynchrony {
            dir::Asynchrony::Sync => dir::LanguageItem::Iterable,
            dir::Asynchrony::Async => dir::LanguageItem::AsyncIterable,
        };
        let expected = self.language_type(item, &[targets.yielded, output, targets.resumed])?;
        let expectation = Expectation {
            target: expected,
            relation: Relation::Storable,
            cause: self.intern_cause(Cause::root(
                value_site.origin(),
                CauseKind::Return { annotation: None },
            )),
            use_: ValueUse::Output,
            mode: self.current_output_mode(),
            store: StoreTarget::Exact,
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
            // a yield outside a generator, reported above
            None => self.intern_type(dir::Type::Error),
        }
    }

    /// Infer one while loop with its condition narrowing.
    fn infer_while_expression(
        &mut self,
        site: FlowSite,
        label: Option<dir::StringId>,
        condition: &dir::Condition,
        body: dir::LocalNodeId<dir::Block>,
    ) -> CompilerResult<()> {
        // read the loop's node
        let node = site.node;
        let module = node.module_id;

        // check the condition before the loop flow splits
        self.check_condition_operands(module, condition)?;

        // check the body under true condition flow inside the loop target
        let label = self.control_label(node.into_typed(), label)?;
        self.enter_control_target(node.into_typed(), label, ControlTargetForm::Iteration);
        let before_body = self.fork_flow();
        self.narrow_condition(condition, ConditionBranch::True)?;
        let body_site = self.visit_site(body.into_global_any(module))?;
        self.attempt_node(body_site, PlaceUse::Read, None)?;
        self.restore_flow(before_body);
        let continues = self.take_current_continue_branches();
        self.commit_single_pass_loop(node, body, &continues)?;

        // collect the normal exit through the false condition
        self.narrow_condition(condition, ConditionBranch::False)?;
        let normal_flow = self.collect_flow_branch(before_body);
        let mut branches = self.leave_control_target();
        branches.push(normal_flow);
        self.merge_flow_branches_from(before_body, &branches);

        // complete the loop with void when the condition fails
        let void = self.intern_type(dir::Type::Void)?;
        self.commit_node_type(node, void)?;

        Ok(())
    }

    /// Record one loop whose body never runs another iteration.
    pub(in crate::sema) fn commit_single_pass_loop(
        &mut self,
        node: dir::GlobalNodeIdAny,
        body: dir::LocalNodeId<dir::Block>,
        continues: &[FlowBranch],
    ) -> CompilerResult<()> {
        // mark a loop whose body runs no second pass
        let repeats = !continues.is_empty() || self.block_can_complete_normally(body);
        if !repeats {
            self.module_mut(node.module_id)
                .flows
                .set_single_pass(node.local_id);
        }

        Ok(())
    }

    /// Infer one loop expression joined from its break values.
    fn infer_loop_expression(
        &mut self,
        site: FlowSite,
        label: Option<dir::StringId>,
        body: dir::LocalNodeId<dir::Block>,
        mode: InferMode,
    ) -> CompilerResult<()> {
        // read the loop's node
        let node = site.node;
        let module = node.module_id;

        // open the loop output joined by break values
        let origin = site.origin();
        let variable = self.open_join_variable(origin)?;
        let result = self.variable_type(variable)?;
        let label = self.control_label(node.into_typed(), label)?;
        self.enter_control_target(
            node.into_typed(),
            label,
            ControlTargetForm::Loop { result, mode },
        );

        // check the body with isolated flow
        let before_body = self.fork_flow();
        let body_site = self.visit_site(body.into_global_any(module))?;
        self.attempt_node(body_site, PlaceUse::Read, None)?;
        self.restore_flow(before_body);
        let continues = self.take_current_continue_branches();
        self.commit_single_pass_loop(node, body, &continues)?;

        // restore the branches that leave the loop
        let branches = self.leave_control_target();

        // close break-free loops to never
        if branches.is_empty() {
            let never = self.intern_type(dir::Type::Never)?;
            let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
            self.push_relation(RelationCheck::new(
                origin,
                Relation::Equal,
                result,
                never,
                cause,
            ))?;
        }

        // merge the break branches and take the joined output
        self.merge_flow_branches_from(before_body, &branches);
        self.commit_node_type(node, result)?;

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
        // read the loop's node
        let node = site.node;
        let module = node.module_id;

        // check initialization and condition before the loop flow splits
        if let Some(initialization) = initialization {
            let init_site = self.visit_site(initialization.into_global_any(module))?;
            self.attempt_node(init_site, PlaceUse::Read, None)?;
        }
        if let Some(condition) = condition {
            self.check_condition(module, condition)?;
        }

        // check the body under true condition flow inside the loop target
        let label = self.control_label(node.into_typed(), label)?;
        self.enter_control_target(node.into_typed(), label, ControlTargetForm::Iteration);
        let before_body = self.fork_flow();
        if let Some(condition) = condition {
            self.narrow_expression(condition, ConditionBranch::True)?;
        }

        // check the body under the narrowed condition
        let body_site = self.visit_site(body.into_global_any(module))?;
        self.attempt_node(body_site, PlaceUse::Read, None)?;

        // check the increment under the joined flows reaching the next iteration
        let continues = self.take_current_continue_branches();
        self.commit_single_pass_loop(node, body, &continues)?;
        if let Some(increment) = increment {
            let body_flow = self
                .block_can_complete_normally(body)
                .then(|| self.collect_flow_branch(before_body));
            let mut reaching = continues;
            reaching.extend(body_flow);

            // reset to the pre-body flow when no path runs the increment
            if reaching.is_empty() {
                self.restore_flow(before_body);
            }
            // otherwise join every reaching path
            else {
                self.merge_flow_branches_from(before_body, &reaching);
            }

            let increment_site = self.visit_site(increment.into_global_any(module))?;
            self.attempt_node(increment_site, PlaceUse::Read, None)?;
        }

        // return to the flow standing before the body
        self.restore_flow(before_body);

        // collect the normal exit through the false condition
        let normal_flow = if let Some(condition) = condition {
            self.narrow_expression(condition, ConditionBranch::False)?;

            Some(self.collect_flow_branch(before_body))
        } else {
            None
        };

        // merge the break branches with the normal exit
        let mut branches = self.leave_control_target();
        branches.extend(normal_flow);
        self.merge_flow_branches_from(before_body, &branches);

        // complete the loop with void
        let void = self.intern_type(dir::Type::Void)?;
        self.commit_node_type(node, void)?;

        Ok(())
    }

    /// Check one break against its enclosing control target.
    fn infer_break_statement(
        &mut self,
        site: FlowSite,
        label: Option<dir::StringId>,
        value: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> CompilerResult<()> {
        // read the break's node
        let node = site.node;
        let module = node.module_id;

        // bind the break to the target its label selects
        match self.flow.break_target_index(label) {
            // bind the carried value to the resolved target
            Some(index) => {
                // record the selected control target
                self.commit_transfer_target(node.into_typed(), index)?;

                let form = self.flow.control_target_form(index);
                match (value, form) {
                    // valued breaks check against the target output
                    (Some(value), ControlTargetForm::Loop { result, mode }) => {
                        let value_site = self.visit_site(value.into_global_any(module))?;
                        let expectation = Expectation {
                            mode,
                            ..Expectation::assignable(
                                result,
                                self.intern_cause(Cause::root(
                                    value_site.origin(),
                                    CauseKind::Return { annotation: None },
                                )),
                                ValueUse::Output,
                            )
                        };
                        self.attempt_node(value_site, PlaceUse::Read, Some(expectation))?;
                    }
                    // values need a valued target
                    (Some(value), ControlTargetForm::Iteration | ControlTargetForm::Switch) => {
                        self.report_break_value_outside_loop(module, node.local_id);

                        let value_site = self.visit_site(value.into_global_any(module))?;
                        self.attempt_node(value_site, PlaceUse::Read, None)?;
                    }
                    // bare breaks exit with void
                    (None, ControlTargetForm::Loop { result, .. }) => {
                        let void = self.intern_type(dir::Type::Void)?;
                        let cause = self.intern_cause(Cause::root(
                            site.origin(),
                            CauseKind::Return { annotation: None },
                        ));
                        self.push_relation(RelationCheck::new(
                            site.origin(),
                            Relation::Storable,
                            void,
                            result,
                            cause,
                        ))?;
                    }
                    // bare breaks leave an unvalued target as they are
                    (None, ControlTargetForm::Iteration | ControlTargetForm::Switch) => {}
                }

                // capture branch flow at the break site
                let checkpoint = self.flow.control_target_checkpoint(index);
                let branch = self.flow.branch(checkpoint);
                self.flow.push_break_branch(index, branch);
            }
            // report unbound breaks
            None => {
                self.report_break_outside_control_target(module, node.local_id);

                self.flow.insert_unbound_jump(node.local_id);
            }
        }

        // breaks complete with never
        let never = self.intern_type(dir::Type::Never)?;
        self.commit_node_type(node, never)?;

        Ok(())
    }

    /// Record one continue against its enclosing loop target.
    fn infer_continue_statement(
        &mut self,
        site: FlowSite,
        label: Option<dir::StringId>,
    ) -> CompilerResult<()> {
        // bind the continue to its loop target
        let node = site.node;
        self.continue_to_control_target(node.into_typed(), label)?;

        // continues complete with never
        let never = self.intern_type(dir::Type::Never)?;
        self.commit_node_type(node, never)?;

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
        // read the statement's node
        let node = site.node;
        let module = node.module_id;

        // check the matched declarator before the flow splits
        self.check_declarator(module, declarator, Some(kind), false, false, false)?;

        // check the diverging else branch in failed-match flow
        let before_else = self.fork_flow();
        self.narrow_declarator_pattern(declarator, false)?;
        let else_site = self.visit_site(else_branch.into_global_any(module))?;
        self.attempt_node(else_site, PlaceUse::Read, None)?;
        self.restore_flow(before_else);

        // require the else branch to leave the binding scope
        if self.expression_can_complete_normally(else_branch) {
            self.report_let_else_branch_can_complete(module, else_branch.into_any());
        }

        // continue with the matched bindings assigned
        self.narrow_let_condition(declarator)?;
        let void = self.intern_type(dir::Type::Void)?;
        self.commit_node_type(node, void)?;

        Ok(())
    }

    /// Check one declarator at its source evaluation position.
    fn check_declarator(
        &mut self,
        module: ModuleId,
        id: dir::LocalNodeId<dir::Declarator>,
        binding_kind: Option<dir::LetKind>,
        is_exported: bool,
        is_ambient: bool,
        is_shared: bool,
    ) -> CompilerResult<()> {
        // skip statically absent declarators, they keep no entries
        if self.is_absent(id.into_global_any(module)) {
            return Ok(());
        }

        // read the written declarator shape once
        let declarator = self.module(module).view().get(id).clone();
        let pattern = declarator.pattern;

        // walk the written annotation at its first typing visit
        if let Some(ty) = declarator.ty {
            self.walk_body_binding_type(module, ty, is_ambient)?;
        }

        // read the written annotation as the binding type
        let annotation = declarator.ty.map(|ty| ty.into_global_any(module));
        let written = annotation
            .map(|annotation| self.require_node_type(annotation))
            .transpose()?;

        // derive the type the pattern destructures from
        let target = match (declarator.value, written) {
            // keep the written type while declaring
            (Some(_), Some(written)) if self.is_declaring() => {
                match self.type_flags(written)?.has_variable() {
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
                let is_composite = matches!(self.node_form(site.node), NodeForm::Composite);
                let mode = match binding_kind {
                    Some(dir::LetKind::Const)
                        if !is_composite && self.root_variable(written)?.is_some() =>
                    {
                        InferMode::Literal
                    }
                    _ => InferMode::Regular,
                };
                let expectation = Expectation {
                    target: written,
                    relation: Relation::Storable,
                    cause,
                    use_: ValueUse::Store,
                    mode,
                    store: StoreTarget::Exact,
                };
                self.attempt_node(site, PlaceUse::Read, Some(expectation))?;

                Some(written)
            }
            // leave unexported initializers to the body pass
            (Some(_), None) if self.is_declaring() && !is_exported => None,
            // report an exported value the declaration pass cannot read
            (Some(value), None) if is_exported && !self.is_literal_initializer(module, value) => {
                // infer the body while checking
                if !self.is_declaring() {
                    let site = self.visit_site(value.into_global_any(module))?;
                    self.infer_node(site, PlaceUse::Read, InferMode::Regular)?;
                }

                Some(self.intern_type(dir::Type::Error)?)
            }
            // bind a destructured or existing value at its type
            (Some(value), None)
                if Self::is_destructuring_pattern(self.module(module).view().get(pattern))
                    || !self.is_fresh_node(value.into_global_any(module))? =>
            {
                let site = self.visit_site(value.into_global_any(module))?;
                let ty = self.infer_node(site, PlaceUse::Read, InferMode::Regular)?;
                let ty = self.flow_type_at(site, ty)?;

                Some(ty)
            }
            // store a fresh value through the binding's slot
            (Some(value), None) => {
                let site = self.visit_site(value.into_global_any(module))?;

                // record a const binding initialized by a fresh value
                if binding_kind == Some(dir::LetKind::Const)
                    && let Some(symbol) = self.module(module).declaration_symbol(pattern.into_any())
                {
                    self.fresh_consts.insert(symbol);
                }

                let slot = match self.binding_slot(module, pattern)? {
                    Some(slot) => slot,
                    None => self.open_variable(site.origin()),
                };
                let slot = self.variable_type(slot)?;
                let cause = self.intern_cause(Cause::root(site.origin(), CauseKind::Expression));
                let is_composite = matches!(self.node_form(site.node), NodeForm::Composite);
                let mode = match binding_kind {
                    Some(dir::LetKind::Const) if !is_composite => InferMode::Literal,
                    _ => InferMode::Regular,
                };
                self.check_node(
                    site,
                    Expectation {
                        target: slot,
                        relation: Relation::Storable,
                        cause,
                        use_: ValueUse::Store,
                        mode,
                        store: StoreTarget::Exact,
                    },
                )?;

                Some(slot)
            }
            // uninitialized declarators take their written type
            (None, Some(written)) => {
                match self.is_declaring() && self.type_flags(written)?.has_variable() {
                    true => None,
                    false => Some(written),
                }
            }
            // report an unannotated binding on its first typing visit
            (None, None) => {
                let source = pattern.into_global_any(module);
                if self.committed_node_type(source).is_none() {
                    self.report_missing_type_annotation(module, pattern.into_any());
                }

                Some(self.intern_type(dir::Type::Error)?)
            }
        };

        // check the pattern against the destructured value
        if let Some(target) = target {
            let site = self.visit_site(pattern.into_global_any(module))?;
            self.check_pattern(pattern.into_global(module), site.flow, site.scope, target)?;

            // require a shared binding to store what shared space holds
            if is_shared {
                let scope = self.flow.template_scope();
                self.push_obligation(
                    Obligation::SharedStorage(SharedStorageObligation {
                        source: pattern.into_global_any(module),
                        ty: target,
                    }),
                    scope,
                )?;
            }

            // require irrefutable coverage outside a refutable position
            let requires_irrefutable = !self.is_refutable_pattern_position(module, id);
            if requires_irrefutable {
                let value = match declarator.value {
                    Some(value) => ExpectedType::Node(value.into_global_any(module)),
                    None => ExpectedType::Type(target),
                };
                let scope = self.flow.template_scope();
                self.push_obligation(
                    Obligation::PatternCoverage(PatternCoverageObligation {
                        source: pattern.into_global_any(module),
                        value,
                        coverage: PatternCoverage::Binding {
                            pattern: pattern.into_global(module),
                        },
                    }),
                    scope,
                )?;
            }
        }

        // commit a module constant's static term beside its checked initializer
        if !self.is_declaring()
            && binding_kind == Some(dir::LetKind::Const)
            && let Some(value) = declarator.value
            && let Some(symbol) = self.module(module).declaration_symbol(pattern.into_any())
        {
            self.commit_constant_term(symbol, value)?;
        }

        // mark the declared and ambient bindings assigned
        {
            let node = self.module(module).view().get(id).clone();
            self.assign_declarator_bindings(&node, is_ambient);
        }

        Ok(())
    }

    /// Check every operand of one condition.
    pub(in crate::sema) fn check_condition_operands(
        &mut self,
        module: ModuleId,
        condition: &dir::Condition,
    ) -> CompilerResult<()> {
        // check each operand of the condition
        for operand in &condition.operands {
            match operand {
                dir::ConditionOperand::Expression { condition } => {
                    self.check_condition(module, *condition)?;
                }
                dir::ConditionOperand::Binding {
                    kind, declarator, ..
                } => {
                    // reject a bare condition binding that shadows a visible type
                    let (pattern, value) = {
                        let node = self.module(module).view().get(*declarator);
                        (node.pattern, node.value)
                    };

                    let Some(value) = value else {
                        return Err(CompilerError::Internal {
                            message: "a condition binding has no matched value".to_string(),
                        });
                    };

                    // report a binding name that shadows a visible type
                    let origin = self.visit_site(pattern.into_global_any(module))?.origin();
                    self.report_type_shadowing_binding(module, value.into_any(), pattern, origin)?;

                    self.check_declarator(module, *declarator, Some(*kind), false, false, false)?;
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
        // visit the condition
        let site = self.visit_site(condition.into_global_any(module))?;

        // store the condition into the branch's boolean, a literal materializing
        let boolean = self.intern_type(dir::Type::Primitive(dir::PrimitiveType::Boolean))?;
        let expectation = Expectation {
            target: boolean,
            relation: Relation::Storable,
            cause: self.intern_cause(Cause::root(
                Origin::Node(condition.into_global_any(module), site.scope),
                CauseKind::Expression,
            )),
            use_: ValueUse::Condition,
            mode: InferMode::Regular,
            store: StoreTarget::Exact,
        };
        self.attempt_node(site, PlaceUse::Read, Some(expectation))?;

        Ok(())
    }
}

impl CheckState<'_> {
    /// Return the parsed and expanded inputs one patched module view reads.
    pub(in crate::sema) fn patched_inputs(
        &self,
        module: ModuleId,
    ) -> (Arc<DirParsed>, Arc<DirExpanded>) {
        let state = self.module(module);

        (state.parsed.clone(), state.expanded.clone())
    }

    /// Walk one body node's decorators at its typing visit, returning the presence gating it.
    pub(in crate::sema) fn walk_body_decorators(
        &mut self,
        module: ModuleId,
        decorated: dir::LocalNodeIdAny,
    ) -> CompilerResult<bool> {
        // keep undecorated nodes present
        if !self.module_view(module).has_decorators_any(decorated) {
            return Ok(true);
        }

        // walk the decorators over the patched view
        let (parsed, expanded) = self.patched_inputs(module);
        let tree = dir::View::new(&parsed.tree).patched(&expanded.patch);
        let mut walk = WalkState::new(module, tree, self).for_body();
        let present = walk.walk_decorators(decorated)?;
        walk.flush_flows()?;

        Ok(present)
    }

    /// Walk one argument list's decorators and return the arguments their conditions keep.
    pub(in crate::sema) fn walk_body_arguments(
        &mut self,
        module: ModuleId,
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) -> CompilerResult<SmallVec<[dir::LocalNodeId<dir::Argument>; 4]>> {
        // keep each argument its static condition decided present
        let mut present = SmallVec::with_capacity(arguments.len());
        for argument in arguments {
            if self.walk_body_decorators(module, argument.into_any())? {
                present.push(*argument);
            }
        }

        Ok(present)
    }

    /// Resolve an aggregate's written type and generic arguments.
    pub(in crate::sema) fn walk_body_construct_type(
        &mut self,
        module: ModuleId,
        ty: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<()> {
        // walk the construct type over the patched view
        let (parsed, expanded) = self.patched_inputs(module);
        let tree = dir::View::new(&parsed.tree).patched(&expanded.patch);
        let mut walk = WalkState::new(module, tree, self).for_body();
        walk.walk_construct_type_expression(ty)?;

        Ok(())
    }

    /// Walk one static term at its first typing visit.
    pub(in crate::sema) fn walk_body_static_term(
        &mut self,
        module: ModuleId,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // walk the term over the patched view
        let (parsed, expanded) = self.patched_inputs(module);
        let tree = dir::View::new(&parsed.tree).patched(&expanded.patch);
        let mut walk = WalkState::new(module, tree, self).for_body();

        walk.walk_static_term(expression)
    }

    /// Walk one body type at its first typing visit, its elided regions closing at the site.
    pub(in crate::sema) fn walk_body_type_expression(
        &mut self,
        module: ModuleId,
        ty: dir::LocalNodeId<dir::TypeExpression>,
        site: ElisionSite,
    ) -> CompilerResult<()> {
        self.walk_body_type(module, ty, |walk| {
            walk.walk_value_type_in(ty, site).map(|_| ())
        })
    }

    /// Walk one binding's annotation at its first typing visit.
    pub(in crate::sema) fn walk_body_binding_type(
        &mut self,
        module: ModuleId,
        ty: dir::LocalNodeId<dir::TypeExpression>,
        is_ambient: bool,
    ) -> CompilerResult<()> {
        self.walk_body_type(module, ty, |walk| {
            walk.walk_binding_type(ty, is_ambient).map(|_| ())
        })
    }

    /// Walk one annotation once per pass over the patched view.
    fn walk_body_type(
        &mut self,
        module: ModuleId,
        ty: dir::LocalNodeId<dir::TypeExpression>,
        walk_type: impl FnOnce(&mut WalkState<'_, '_>) -> CompilerResult<()>,
    ) -> CompilerResult<()> {
        // walk each type once per pass
        if self.own_node_type(ty.into_global_any(module)).is_some() {
            return Ok(());
        }

        // read the patched view
        let (parsed, expanded) = self.patched_inputs(module);
        let tree = dir::View::new(&parsed.tree).patched(&expanded.patch);
        let mut walk = WalkState::new(module, tree, self).for_body();

        walk_type(&mut walk)
    }

    /// Walk explicit generic arguments at their first typing visit.
    pub(in crate::sema) fn walk_body_generic_arguments(
        &mut self,
        module: ModuleId,
        arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    ) -> CompilerResult<()> {
        // reuse types an earlier walk already committed
        let has_unwalked = arguments.iter().any(|argument| {
            self.committed_node_type(argument.into_global_any(module))
                .is_none()
        });
        if !has_unwalked {
            return Ok(());
        }

        // walk the arguments over the patched view
        let (parsed, expanded) = self.patched_inputs(module);
        let tree = dir::View::new(&parsed.tree).patched(&expanded.patch);
        let mut walk = WalkState::new(module, tree, self).for_body();
        walk.walk_generic_arguments(arguments)?;

        Ok(())
    }

    /// Return whether one declaration node holds a function value.
    pub(in crate::sema) fn is_function_value_declaration(
        &self,
        node: dir::GlobalNodeIdAny,
        declaration: dir::LocalNodeId<dir::Declaration>,
    ) -> CompilerResult<bool> {
        // answer a committed body for its node
        if self.lambdas.contains_key(&node) {
            return Ok(true);
        }

        // read the declaration's written form
        let (parsed, expanded) = self.patched_inputs(node.module_id);
        let tree = dir::View::new(&parsed.tree).patched(&expanded.patch);

        Ok(matches!(
            tree.get(declaration),
            dir::Declaration::Function(function)
                if function.signature.form == dir::FunctionForm::Lambda
                    || function.name.is_none()
        ))
    }

    /// Declare one function value and register its body at its first typing visit.
    pub(in crate::sema) fn commit_function_value(
        &mut self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<()> {
        // read the declaration over the patched view
        let module = node.module_id;
        let declaration = self.function_value_declaration(node)?;
        let (parsed, expanded) = self.patched_inputs(module);
        let tree = dir::View::new(&parsed.tree).patched(&expanded.patch);

        // walk each function value once, a walked declaration keeping its symbol type
        let is_walked = match self
            .module(module)
            .declaration_symbol(declaration.into_any())
        {
            Some(symbol) => self.symbol_type_maybe(symbol)?.is_some(),
            None => false,
        };
        if is_walked || self.lambdas.contains_key(&node) {
            return Ok(());
        }
        let kind = tree.get(declaration).clone();

        // declare the signature, commit the body, and key it by value
        let mut walk = WalkState::new(module, tree, self).for_body();
        walk.walk_declaration(declaration, &kind)?;
        let symbol = walk
            .check
            .module(module)
            .declaration_symbol(declaration.into_any())
            .ok_or_else(|| CompilerError::Internal {
                message: format!("function value {node:?} has no declaration symbol"),
            })?;

        if let dir::Declaration::Function(function) = &kind {
            walk.walk_declared_function_body(declaration, function, symbol)?;
        }

        walk.flush_flows()?;

        // move the body check to its value expression
        let Some(body) = self.functions.swap_remove(&symbol) else {
            return Err(CompilerError::Internal {
                message: format!("function value {} has no body", self.node_label(node)),
            });
        };
        self.lambdas.insert(node, body);

        Ok(())
    }

    /// Type one declaration statement, committing the bodies it declares.
    pub(in crate::sema) fn infer_declaration_statement(
        &mut self,
        site: FlowSite,
        declaration: dir::LocalNodeId<dir::Declaration>,
    ) -> CompilerResult<()> {
        // read the declaration's node
        let node = site.node;
        let module = node.module_id;

        // type a function value as its callable
        if self.is_function_value_declaration(node, declaration)? {
            self.function_value_type(node, None)?;

            return Ok(());
        }

        // read the declaration over the patched view
        let (parsed, expanded) = self.patched_inputs(module);
        let tree = dir::View::new(&parsed.tree).patched(&expanded.patch);
        let kind = tree.get(declaration).clone();

        // type declaration statements as void
        let void = self.intern_type(dir::Type::Void)?;
        self.commit_node_type(node, void)?;

        // a function body nests only function and type declarations
        if self.flow.current_function().is_some()
            && !matches!(
                kind,
                dir::Declaration::Type(_) | dir::Declaration::Function(_)
            )
        {
            self.report_declaration_not_nestable(site.origin(), kind.kind_name())?;

            return Ok(());
        }

        // type module and global block members in authored order
        let members = match &kind {
            dir::Declaration::Global(block) => Some(block.expressions.clone()),
            dir::Declaration::Module(block) => Some(block.expressions.clone()),
            _ => None,
        };

        // type each member in authored order
        if let Some(members) = members {
            for member in members {
                let member_site = self.visit_site(member.into_global_any(module))?;
                self.attempt_node(member_site, PlaceUse::Read, None)?;
            }

            return Ok(());
        }

        // register the declared bodies of other declarations
        let mut walk = WalkState::new(module, tree, self).for_body();
        walk.visit_body_declaration_statement(declaration, &kind)?;

        Ok(())
    }
}
