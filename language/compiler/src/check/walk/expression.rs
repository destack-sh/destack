use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{
    AssignedPlace, CauseKind, ConditionBranch, ControlTargetForm, ExpectedType, FlowBranch,
    FlowCheckpoint, Obligation, Origin, PatternCoverage, PatternCoverageObligation, PlaceUse,
    Relation, ValueUse, VariableRole, WalkState, Widening,
};
use crate::{CompilerError, CompilerResult};

impl WalkState<'_, '_> {
    /// Walk one expression.
    ///
    /// Example:
    /// ```ds
    /// value + 1
    /// ```
    pub(in crate::check) fn walk_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) -> CompilerResult<()> {
        if !self.walk_decorators(id.into_any())? {
            return Ok(());
        }
        self.enter_node(id)?;

        match expression {
            // function f() {}
            dir::Expression::Declaration(declaration) => {
                let declaration = *declaration;

                // identify function values checked by their receiving context
                let is_lambda = matches!(
                    self.tree.get(declaration),
                    dir::Declaration::Function(function)
                        if function.signature.form == dir::FunctionForm::Lambda
                            || function.name.is_none()
                );
                self.walk_declaration(declaration, self.tree.get(declaration))?;

                // walk the declared bodies the declaration walk no longer visits
                if !is_lambda
                    && let Some(symbol) = self
                        .check
                        .module(self.module)
                        .declaration_symbol(declaration.into_any())
                {
                    self.visit_declared_bodies(
                        declaration,
                        &self.tree.get(declaration).clone(),
                        symbol,
                    )?;
                }
                if is_lambda {
                    let Some(symbol) = self
                        .check
                        .module(self.module)
                        .declaration_symbol(declaration.into_any())
                    else {
                        return Err(CompilerError::Internal {
                            message: format!("function value {id:?} has no declaration symbol"),
                        });
                    };
                    // function values register their bodies at their expression
                    if let dir::Declaration::Function(function) = self.tree.get(declaration).clone()
                    {
                        self.walk_declared_function_body(declaration, &function, symbol)?;
                    }
                    // move the body from independent roots to its value expression
                    let Some(body) = self.check.functions.swap_remove(&symbol) else {
                        return Err(CompilerError::Internal {
                            message: format!("function value {id:?} has no body"),
                        });
                    };
                    if self
                        .check
                        .lambdas
                        .insert(id.into_global_any(self.module), body)
                        .is_some()
                    {
                        return Err(CompilerError::Internal {
                            message: format!("function value {id:?} has multiple bodies"),
                        });
                    }
                } else {
                    let void = self.intern_type(dir::Type::Void)?;
                    self.commit_node_type(id, void)?;
                }
            }
            // { ... }
            dir::Expression::Block(block) => {
                let block = *block;
                self.walk_block(block, self.tree.get(block))?;
            }
            // label: body
            dir::Expression::Label { label, body } => {
                self.walk_label_expression(id, *label, *body)?;
            }
            // import { item } from "module"
            dir::Expression::Import { items, .. } => {
                if let Some(items) = items.as_deref() {
                    for item in items {
                        self.walk_dependency_item(*item, self.tree.get(*item))?;
                    }
                }
                let void = self.intern_type(dir::Type::Void)?;
                self.commit_node_type(id, void)?;
            }
            // export { item } from "module"
            dir::Expression::Export { items, .. } => {
                for item in items {
                    self.walk_dependency_item(*item, self.tree.get(*item))?;
                }
                let void = self.intern_type(dir::Type::Void)?;
                self.commit_node_type(id, void)?;
            }
            // let x = value
            dir::Expression::Let {
                kind,
                declarators,
                is_ambient,
                ..
            } => {
                for declarator in declarators {
                    self.walk_declarator(
                        *declarator,
                        self.tree.get(*declarator),
                        Some(*kind),
                        *is_ambient,
                    )?;
                    self.mark_declarator_assigned(self.tree.get(*declarator), *is_ambient);
                }
            }
            // using x = value
            dir::Expression::Using { declarators, .. } => {
                for declarator in declarators {
                    self.walk_declarator(*declarator, self.tree.get(*declarator), None, false)?;
                    self.mark_declarator_assigned(self.tree.get(*declarator), false);
                }
            }
            // let pattern = value else { return }
            dir::Expression::LetElse {
                kind,
                declarator,
                else_branch,
                ..
            } => {
                self.walk_let_else_expression(*kind, *declarator, *else_branch)?;
            }
            // if condition { then } else { otherwise }
            dir::Expression::If {
                condition,
                then_expression,
                else_expression,
                ..
            } => {
                self.walk_if_expression(condition, *then_expression, *else_expression)?;
            }
            // while condition { body }
            dir::Expression::While {
                condition, body, ..
            } => {
                self.walk_while_expression(None, *condition, *body)?;
            }
            // for item of iterator { body }
            dir::Expression::ForEach {
                binding,
                iterator,
                body,
                ..
            } => {
                self.walk_for_each_expression(None, binding, *iterator, *body)?;
            }
            // for (initialization; condition; increment) { body }
            dir::Expression::For {
                initialization,
                condition,
                increment,
                body,
            } => {
                let (initialization, condition, increment, body) =
                    (*initialization, *condition, *increment, *body);
                self.walk_for_expression(None, initialization, condition, increment, body)?;
            }
            // loop { body }
            dir::Expression::Loop { body } => {
                self.walk_loop_expression(id, None, *body)?;
            }
            // try body catch error finally cleanup
            dir::Expression::Try {
                body,
                catch,
                finally,
            } => {
                self.walk_try_expression(id, *body, *catch, *finally)?;
            }
            // match value { case pattern => body }
            dir::Expression::Match { value, arms } => {
                let arms = arms.iter().copied().collect::<SmallVec<[_; 4]>>();
                self.walk_match_expression(id, *value, &arms)?;
            }
            // switch (value) { case value: body }
            dir::Expression::Switch { value, cases } => {
                let cases = cases.iter().copied().collect::<SmallVec<[_; 4]>>();
                self.walk_switch_statement(id, *value, &cases)?;
            }
            // break value
            dir::Expression::Break { label, value } => {
                let (label, value) = (*label, *value);
                if let Some(value) = value {
                    self.walk_expression(value, self.tree.get(value))?;
                }
                self.break_to_control_target(id.into_any(), label, value)?;
            }
            // continue
            dir::Expression::Continue { label } => {
                self.continue_to_control_target(id.into_any(), *label);
            }
            // await value
            dir::Expression::Await {
                expression: awaited,
            } => {
                let awaited = *awaited;
                self.walk_expression(awaited, self.tree.get(awaited))?;
                if self
                    .flow()
                    .current_function()
                    .is_none_or(|function| function.asynchrony != dir::Asynchrony::Async)
                {
                    self.check
                        .report_await_outside_async_context(self.module, id.into_any());
                }
            }
            // return value
            dir::Expression::Return { value } => {
                if self.flow().current_function().is_none() {
                    self.check
                        .report_return_outside_function(self.module, id.into_any());
                }
                if let Some(value) = *value {
                    self.walk_expression(value, self.tree.get(value))?;
                }
            }
            // yield value
            dir::Expression::Yield { cardinality, value } => {
                let (cardinality, value) = (*cardinality, *value);

                // require a surrounding generator body
                if !self.is_in_generator() {
                    self.check
                        .report_yield_outside_generator(self.module, id.into_any());
                }

                match (cardinality, value) {
                    // yield value: the checker relates the value to the yield target
                    (dir::YieldCardinality::Scalar, Some(value)) => {
                        self.walk_expression(value, self.tree.get(value))?;
                    }
                    // bare yield: the checker relates void to the yield target
                    (dir::YieldCardinality::Scalar, None) => {}
                    // yield* value: the delegate implements the generator protocol
                    (dir::YieldCardinality::Generator, Some(value)) => {
                        self.record_yield_delegate(id, value)?;
                        self.walk_expression(value, self.tree.get(value))?;
                    }
                    // yield* without a delegate value
                    (dir::YieldCardinality::Generator, None) => {
                        self.check
                            .report_yield_delegate_missing_value(self.module, id.into_any());
                    }
                }
            }
            // value
            dir::Expression::Identifier { name } => {
                self.walk_identifier_expression(id, *name)?;
            }
            // this
            dir::Expression::This => {
                let receiver =
                    self.commit_active_receiver_decision(id.into_global_any(self.module))?;
                match receiver {
                    Some(receiver) => {
                        self.commit_node_type(id, receiver.ty)?;
                    }
                    None => {
                        self.check
                            .report_this_outside_receiver(self.module, id.into_any());
                        let error = self.intern_type(dir::Type::Error)?;
                        self.commit_node_type(id, error)?;
                    }
                }
            }
            // 1, "text", true
            dir::Expression::ScalarLiteral(_) => {}
            // super
            dir::Expression::Super => {
                let receiver =
                    self.commit_active_receiver_decision(id.into_global_any(self.module))?;
                match receiver.and_then(|receiver| receiver.super_ty) {
                    Some(super_ty) => {
                        self.commit_node_type(id, super_ty)?;
                    }
                    None => {
                        self.check
                            .report_super_outside_class(self.module, id.into_any());
                        let error = self.intern_type(dir::Type::Error)?;
                        self.commit_node_type(id, error)?;
                    }
                }
            }
            // import.meta
            dir::Expression::ImportMeta => {
                let meta = self.language_type_reference(dir::LanguageItem::ImportMeta, &[])?;
                self.commit_node_type(id, meta)?;
            }
            // import.source resolves to its module source descriptor at lowering
            dir::Expression::ImportSource => {
                let source = self.intern_type(dir::Type::Error)?;
                self.commit_node_type(id, source)?;
            }
            // debugger, missing, and damaged nodes
            dir::Expression::Debugger | dir::Expression::Missing | dir::Expression::Error => {}
            // start..end
            dir::Expression::RangeExpression { start, end, .. } => {
                self.walk_range_expression(*start, *end)?;
            }
            // `text ${value}`
            dir::Expression::TemplateExpression { value } => {
                self.walk_template_literal(value)?;
            }
            // tag<T>`text ${value}`
            dir::Expression::TaggedTemplateExpression { tag, value, .. } => {
                self.walk_expression(*tag, self.tree.get(*tag))?;
                self.walk_template_literal(value)?;
            }
            // [a, b, c]
            dir::Expression::ArrayExpression { elements } => {
                for element in elements {
                    self.walk_argument(*element, self.tree.get(*element))?;
                }
            }
            // [value; length]
            dir::Expression::FixedArrayExpression { value, length } => {
                let (value, length) = (*value, *length);
                self.walk_expression(value, self.tree.get(value))?;
                self.walk_static_term(length)?;
            }
            // [a, label: b, ...rest]
            dir::Expression::TupleExpression { elements } => {
                for element in elements {
                    self.walk_argument(*element, self.tree.get(*element))?;
                }
            }
            // { key: value }
            dir::Expression::ObjectExpression { properties } => {
                let properties = properties.iter().copied().collect::<SmallVec<[_; 4]>>();
                self.walk_literal_properties(&properties)?;
            }
            // Type { key: value }
            dir::Expression::StructExpression { ty, properties } => {
                let properties = properties.iter().copied().collect::<SmallVec<[_; 4]>>();
                self.walk_construct_type_expression(*ty)?;
                self.walk_literal_properties(&properties)?;
            }
            // jsx like tree expression
            dir::Expression::TreeExpression {
                left,
                attributes,
                children,
                ..
            } => {
                // lowercase tags name builder rows, not lexical values
                if let Some(left) = *left
                    && !self.is_intrinsic_tree_tag(left)
                {
                    self.walk_expression(left, self.tree.get(left))?;
                }
                if let Some(attributes) = attributes.as_deref() {
                    for attribute in attributes {
                        if let Some(value) = self.tree.get(*attribute).value() {
                            self.walk_expression(value, self.tree.get(value))?;
                        }
                    }
                }
                if let Some(children) = children.as_deref() {
                    for child in children {
                        if let Some(value) = self.tree.get(*child).value() {
                            self.walk_expression(value, self.tree.get(value))?;
                        }
                    }
                }
            }
            // type T
            dir::Expression::Type { value } => {
                let represented = self.walk_frame_type_expression(*value)?;
                let reflected = self
                    .check
                    .language_type(dir::LanguageItem::Type, &[represented])?;
                self.commit_node_type(id, reflected)?;
            }
            // comptime value
            dir::Expression::Comptime { body } => {
                let body = *body;

                // check comptime bodies in isolated flow
                let before_body = self.fork_flow();
                self.walk_expression(body, self.tree.get(body))?;
                self.restore_flow(before_body);
            }
            // value as T
            dir::Expression::As {
                expression: child,
                target_type,
            } => {
                let (child, target_type) = (*child, *target_type);

                // const assertions freeze during expression inference
                if matches!(self.tree.get(target_type), dir::TypeExpression::Const) {
                    self.walk_expression(child, self.tree.get(child))?;
                }
                // the checker relates the cast; the walk only records shapes
                else {
                    self.walk_type_expression(target_type)?;
                    self.walk_expression(child, self.tree.get(child))?;
                }
            }
            // value satisfies T
            dir::Expression::Satisfies {
                expression: child,
                target_type,
            } => {
                let (child, target_type) = (*child, *target_type);
                self.walk_expression(child, self.tree.get(child))?;
                self.walk_frame_type_expression(target_type)?;
            }
            // value is T
            dir::Expression::Is { value, target_type } => {
                let (value, target_type) = (*value, *target_type);
                self.walk_expression(value, self.tree.get(value))?;
                self.walk_frame_type_expression(target_type)?;
            }
            // value instanceof Target
            dir::Expression::InstanceOf { value, target } => {
                let (value, target) = (*value, *target);
                self.walk_expression(value, self.tree.get(value))?;
                self.walk_expression(target, self.tree.get(target))?;
            }
            // value++, --value
            dir::Expression::Unary {
                operator:
                    dir::UnaryOperator::PostIncrement
                    | dir::UnaryOperator::PostDecrement
                    | dir::UnaryOperator::PreIncrement
                    | dir::UnaryOperator::PreDecrement,
                right,
            } => {
                let right = *right;

                if let Some(place) = self.walk_assigned_place(right, PlaceUse::Update)? {
                    self.mark_place_assigned(place);
                }

                // increments invalidate narrowings under the target
                self.clear_mutated_expression_narrowings(right);
            }
            // !value, -value
            dir::Expression::Unary { right, .. } => {
                let right = *right;
                self.walk_expression(right, self.tree.get(right))?;
            }
            // &value
            dir::Expression::BorrowOf { right, .. } => {
                let right = *right;
                self.walk_expression(right, self.tree.get(right))?;
            }
            // value.member, or a static name path resolved by the resolve phase
            dir::Expression::Member { left, .. } => {
                self.walk_member_expression(id, *left)?;
            }
            // value[index]
            dir::Expression::Index { left, index, .. } => {
                self.walk_expression(*left, self.tree.get(*left))?;
                if let Some(index) = *index {
                    self.walk_expression(index, self.tree.get(index))?;
                }
            }
            // value<T>
            dir::Expression::Instantiation {
                left,
                generic_arguments,
            } => {
                let arguments = generic_arguments
                    .iter()
                    .copied()
                    .collect::<SmallVec<[_; 2]>>();
                self.walk_instantiation_expression(id, *left, &arguments)?;
            }
            // callee<T>(argument)
            dir::Expression::Call {
                left,
                generic_arguments,
                arguments,
                ..
            } => {
                let callee = *left;
                let arguments = arguments.iter().copied().collect::<SmallVec<[_; 4]>>();
                self.walk_expression(callee, self.tree.get(callee))?;
                self.walk_generic_arguments(generic_arguments)?;

                // a member callee's receiver may be consumed by the candidate
                if let dir::Expression::Member { left: receiver, .. } = self.tree.get(callee) {
                    self.mark_moved_source(*receiver, Some(id), None);
                }
                for argument in &arguments {
                    self.walk_argument(*argument, self.tree.get(*argument))?;
                    // by-value arguments may consume their identifier sources
                    if let Some(value) = self.tree.get(*argument).value() {
                        self.mark_moved_argument(value, *argument, id);
                    }
                }
            }
            // _
            dir::Expression::Infer { .. } => {}
            // new Type<T>(argument)
            dir::Expression::New { ty, arguments } => {
                let arguments = arguments.iter().copied().collect::<SmallVec<[_; 4]>>();
                self.walk_construct_type_expression(*ty)?;
                for argument in &arguments {
                    self.walk_argument(*argument, self.tree.get(*argument))?;
                    // by-value arguments may consume their identifier sources
                    if let Some(value) = self.tree.get(*argument).value() {
                        self.mark_moved_argument(value, *argument, id);
                    }
                }
            }
            // await? value
            dir::Expression::AwaitMaybe {
                expression: awaited,
            } => {
                let awaited = *awaited;
                self.walk_expression(awaited, self.tree.get(awaited))?;
                if self
                    .flow()
                    .current_function()
                    .is_none_or(|function| function.asynchrony != dir::Asynchrony::Async)
                {
                    self.check
                        .report_await_outside_async_context(self.module, id.into_any());
                }
                self.propagate_try(id.into_any())?;
            }
            // await! value
            dir::Expression::AwaitMust {
                expression: awaited,
            } => {
                let awaited = *awaited;
                self.walk_expression(awaited, self.tree.get(awaited))?;
                if self
                    .flow()
                    .current_function()
                    .is_none_or(|function| function.asynchrony != dir::Asynchrony::Async)
                {
                    self.check
                        .report_await_outside_async_context(self.module, id.into_any());
                }
            }
            // value?
            dir::Expression::Maybe { left, .. } => {
                let left = *left;
                self.walk_expression(left, self.tree.get(left))?;
                self.propagate_try(id.into_any())?;
            }
            // value?.member
            dir::Expression::Chain { expression } => {
                let expression = *expression;
                self.walk_expression(expression, self.tree.get(expression))?;
            }
            // value!
            dir::Expression::Must { left, .. } => {
                let left = *left;
                self.walk_expression(left, self.tree.get(left))?;
            }
            // left + right
            dir::Expression::Binary {
                left,
                operator,
                right,
            } => {
                self.walk_binary_expression(*left, *operator, *right)?;
            }
            // target = value
            dir::Expression::Assign {
                left,
                operator,
                right,
            } => {
                self.walk_assign_expression(*left, *operator, *right)?;
            }
        }

        Ok(())
    }

    /// Walk one labeled expression.
    ///
    /// Example:
    /// ```ds
    /// outer: loop { break outer; }
    /// ```
    fn walk_label_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        label: dir::StringId,
        body: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        let is_loop = matches!(
            self.tree.get(body),
            dir::Expression::While { .. }
                | dir::Expression::ForEach { .. }
                | dir::Expression::For { .. }
                | dir::Expression::Loop { .. }
        );
        if !is_loop {
            let before = self.fork_flow();
            let result =
                self.open_type_hole(id.into_any(), Widening::Never, VariableRole::Regular)?;
            self.enter_control_target(Some(label), ControlTargetForm::Block { result });
            self.walk_expression(body, self.tree.get(body))?;

            // collect the normal exit through the block fallthrough
            let normal_flow = self
                .expression_can_complete_normally(body)
                .then(|| self.collect_flow_branch(before));
            self.restore_flow(before);

            // the checker relates the body value to the block output
            let mut branches = self.leave_control_target();
            self.check
                .control_results
                .insert(id.into_global_any(self.module), result);

            // merge break branches with normal exit
            branches.extend(normal_flow);
            self.merge_flow_branches_from(before, &branches);

            return Ok(());
        }
        if !self.walk_decorators(body.into_any())? {
            return Ok(());
        }

        // labeled loops keep their own continue targets
        self.enter_node(body)?;
        match self.tree.get(body) {
            dir::Expression::While {
                condition,
                body: loop_body,
                ..
            } => {
                let (condition, loop_body) = (*condition, *loop_body);
                self.walk_while_expression(Some(label), condition, loop_body)?;
            }
            dir::Expression::ForEach {
                binding,
                iterator,
                body: loop_body,
                ..
            } => {
                let (iterator, loop_body) = (*iterator, *loop_body);
                self.walk_for_each_expression(Some(label), binding, iterator, loop_body)?;
            }
            dir::Expression::For {
                initialization,
                condition,
                increment,
                body: loop_body,
            } => {
                let (initialization, condition, increment, loop_body) =
                    (*initialization, *condition, *increment, *loop_body);
                self.walk_for_expression(
                    Some(label),
                    initialization,
                    condition,
                    increment,
                    loop_body,
                )?;
            }
            dir::Expression::Loop { body: loop_body } => {
                let loop_body = *loop_body;
                self.walk_loop_expression(body, Some(label), loop_body)?;
            }
            _ => {
                return Err(CompilerError::Internal {
                    message: format!("labeled loop body {body:?} is not a loop"),
                });
            }
        }

        Ok(())
    }

    /// Walk one let-else expression.
    ///
    /// Example:
    /// ```ds
    /// let Some(value) = option else { return };
    /// ```
    fn walk_let_else_expression(
        &mut self,
        kind: dir::LetKind,
        declarator: dir::LocalNodeId<dir::Declarator>,
        else_branch: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        self.walk_declarator(declarator, self.tree.get(declarator), Some(kind), false)?;

        // walk the diverging else block in isolated flow
        let before_else = self.fork_flow();
        self.walk_expression(else_branch, self.tree.get(else_branch))?;
        self.restore_flow(before_else);

        // require the else block to leave the binding scope
        if self.expression_can_complete_normally(else_branch) {
            self.check
                .report_let_else_branch_can_complete(self.module, else_branch.into_any());
        }

        // continue with the matched bindings assigned
        self.mark_declarator_assigned(self.tree.get(declarator), false);
        self.narrow_declarator_match(declarator)?;

        Ok(())
    }

    /// Walk one if expression.
    ///
    /// Example:
    /// ```ds
    /// if condition { then } else { otherwise }
    /// ```
    fn walk_if_expression(
        &mut self,
        condition: &dir::Condition,
        then_expression: dir::LocalNodeId<dir::Expression>,
        else_expression: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> CompilerResult<()> {
        // walk branches with isolated flow
        self.walk_if_condition(condition)?;
        let before = self.fork_flow();

        // walk true branch
        self.restore_flow(before);
        self.narrow_condition(condition, ConditionBranch::True)?;
        self.walk_expression(then_expression, self.tree.get(then_expression))?;
        let then_can_complete = self.expression_can_complete_normally(then_expression);
        let mut branches = SmallVec::<[FlowBranch; 2]>::new();
        if then_can_complete {
            branches.push(self.collect_flow_branch(before));
        }

        // false branch
        if let Some(else_expression) = else_expression {
            // walk false branch
            self.restore_flow(before);
            self.narrow_condition(condition, ConditionBranch::False)?;
            self.walk_expression(else_expression, self.tree.get(else_expression))?;
            let else_can_complete = self.expression_can_complete_normally(else_expression);

            // collect false completion
            if else_can_complete {
                branches.push(self.collect_flow_branch(before));
            }
        }
        // no false branch
        else {
            // collect implicit false completion
            self.restore_flow(before);
            self.narrow_condition(condition, ConditionBranch::False)?;
            branches.push(self.collect_flow_branch(before));
        }

        // merge normally completed branches
        self.merge_flow_branches_from(before, &branches);

        Ok(())
    }

    /// Walk one if condition.
    ///
    /// Example:
    /// ```ds
    /// if let Some(value) = option { value }
    /// ```
    fn walk_if_condition(&mut self, condition: &dir::Condition) -> CompilerResult<()> {
        let before = self.fork_flow();
        let result = self.walk_if_condition_chain(&condition.operands);
        self.restore_flow(before);

        result
    }

    /// Walk one if condition chain.
    fn walk_if_condition_chain(
        &mut self,
        operands: &[dir::ConditionOperand],
    ) -> CompilerResult<()> {
        for operand in operands {
            self.walk_if_condition_operand(operand)?;
        }

        Ok(())
    }

    /// Walk one operand in an if condition chain.
    fn walk_if_condition_operand(&mut self, operand: &dir::ConditionOperand) -> CompilerResult<()> {
        match operand {
            // boolean condition
            dir::ConditionOperand::Expression { condition } => {
                let condition = *condition;
                self.walk_expression(condition, self.tree.get(condition))?;
            }
            // pattern binding condition
            dir::ConditionOperand::Binding {
                kind, declarator, ..
            } => {
                let declarator = *declarator;
                self.walk_declarator(declarator, self.tree.get(declarator), Some(*kind), false)?;
                self.narrow_let_condition(declarator)?;
            }
        }

        Ok(())
    }

    /// Walk one while expression.
    ///
    /// Example:
    /// ```ds
    /// while condition { body }
    /// ```
    fn walk_while_expression(
        &mut self,
        label: Option<dir::StringId>,
        condition: dir::LocalNodeId<dir::Expression>,
        body: dir::LocalNodeId<dir::Block>,
    ) -> CompilerResult<()> {
        // walk condition in incoming flow
        self.walk_expression(condition, self.tree.get(condition))?;

        // enter loop control target
        self.enter_control_target(label, ControlTargetForm::Iteration);

        // walk body under true condition flow
        let before_body = self.fork_flow();
        self.narrow_expression(condition, ConditionBranch::True)?;
        self.walk_block(body, self.tree.get(body))?;
        self.restore_flow(before_body);

        // collect normal exit through false condition
        self.narrow_expression(condition, ConditionBranch::False)?;
        let normal_flow = self.collect_flow_branch(before_body);
        let mut branches = self.leave_control_target();

        // merge break branches with normal exit
        branches.push(normal_flow);
        self.merge_flow_branches_from(before_body, &branches);

        Ok(())
    }

    /// Walk one for each expression.
    ///
    /// Example:
    /// ```ds
    /// for (item of items) { item }
    /// ```
    fn walk_for_each_expression(
        &mut self,
        label: Option<dir::StringId>,
        binding: &dir::ForEachBinding,
        iterator: dir::LocalNodeId<dir::Expression>,
        body: dir::LocalNodeId<dir::Block>,
    ) -> CompilerResult<()> {
        // walk binding and iterator in incoming flow
        let pattern = match binding {
            dir::ForEachBinding::Pattern { pattern, .. }
            | dir::ForEachBinding::Using { pattern, .. } => *pattern,
        };
        self.walk_pattern(pattern, self.tree.get(pattern), None)?;
        self.walk_expression(iterator, self.tree.get(iterator))?;

        // enter loop control target
        self.enter_control_target(label, ControlTargetForm::Iteration);

        // walk body with iteration binding assigned
        let before_body = self.fork_flow();
        self.mark_bindings_assigned(pattern.into_any());
        self.walk_block(body, self.tree.get(body))?;
        self.restore_flow(before_body);

        // collect normal loop exit
        let normal_flow = self.collect_flow_branch(before_body);
        let mut branches = self.leave_control_target();

        // merge break branches with normal exit
        branches.push(normal_flow);
        self.merge_flow_branches_from(before_body, &branches);

        Ok(())
    }

    /// Walk one traditional for expression.
    ///
    /// Example:
    /// ```ds
    /// for (let i = 0; i < n; i++) { i }
    /// ```
    fn walk_for_expression(
        &mut self,
        label: Option<dir::StringId>,
        initialization: Option<dir::LocalNodeId<dir::Expression>>,
        condition: Option<dir::LocalNodeId<dir::Expression>>,
        increment: Option<dir::LocalNodeId<dir::Expression>>,
        body: dir::LocalNodeId<dir::Block>,
    ) -> CompilerResult<()> {
        // walk initialization before loop flow splits
        if let Some(initialization) = initialization {
            self.walk_expression(initialization, self.tree.get(initialization))?;
        }

        // walk condition in incoming flow
        if let Some(condition) = condition {
            self.walk_expression(condition, self.tree.get(condition))?;
        }

        // enter loop control target
        self.enter_control_target(label, ControlTargetForm::Iteration);

        // walk body under true condition flow
        let before_body = self.fork_flow();
        if let Some(condition) = condition {
            self.narrow_expression(condition, ConditionBranch::True)?;
        }
        self.walk_block(body, self.tree.get(body))?;
        let body_flow = self
            .block_can_complete_normally(self.tree.get(body))
            .then(|| self.collect_flow_branch(before_body));
        let continue_flows = self.take_current_continue_branches();
        if let Some(increment) = increment {
            self.walk_for_increment_expression(increment, before_body, body_flow, &continue_flows)?;
        }
        self.restore_flow(before_body);

        // collect normal exit through false condition
        let normal_flow = if let Some(condition) = condition {
            self.narrow_expression(condition, ConditionBranch::False)?;

            Some(self.collect_flow_branch(before_body))
        } else {
            None
        };
        let mut branches = self.leave_control_target();

        // merge break branches with normal exit
        branches.extend(normal_flow);
        self.merge_flow_branches_from(before_body, &branches);

        Ok(())
    }

    /// Walk one traditional for increment from body and continue flows.
    fn walk_for_increment_expression(
        &mut self,
        increment: dir::LocalNodeId<dir::Expression>,
        before_body: FlowCheckpoint,
        body_flow: Option<FlowBranch>,
        continue_flows: &[FlowBranch],
    ) -> CompilerResult<()> {
        let mut flows = Vec::with_capacity(continue_flows.len() + usize::from(body_flow.is_some()));
        flows.extend_from_slice(continue_flows);
        if let Some(body_flow) = body_flow {
            flows.push(body_flow);
        }

        // check unreachable increments once
        if flows.is_empty() {
            self.walk_expression(increment, self.tree.get(increment))?;
            self.restore_flow(before_body);

            return Ok(());
        }

        // check the increment from each flow that reaches the next iteration
        for flow in flows {
            self.restore_flow_branch(before_body, &flow);
            self.walk_expression(increment, self.tree.get(increment))?;
            self.restore_flow(before_body);
        }

        Ok(())
    }

    /// Walk one loop expression.
    ///
    /// Example:
    /// ```ds
    /// loop { break value; }
    /// ```
    fn walk_loop_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        label: Option<dir::StringId>,
        body: dir::LocalNodeId<dir::Block>,
    ) -> CompilerResult<()> {
        // open the loop output joined by break values
        let result = self.open_type_hole(id.into_any(), Widening::Never, VariableRole::Regular)?;
        self.enter_control_target(label, ControlTargetForm::Loop { result });

        // walk body with isolated flow
        let before_body = self.fork_flow();
        self.walk_block(body, self.tree.get(body))?;
        self.restore_flow(before_body);

        // restore only branches that leave the loop
        let branches = self.leave_control_target();
        self.check
            .control_results
            .insert(id.into_global_any(self.module), result);

        // close break-free loops to never
        if branches.is_empty() {
            let origin = Origin::Node(
                id.into_global_any(self.module),
                self.flow().template_scope(),
            );
            let never = self.intern_type(dir::Type::Never)?;

            self.relate_type(
                origin,
                CauseKind::Expression,
                Relation::Equal,
                result,
                never,
            );
        }
        self.merge_flow_branches_from(before_body, &branches);

        Ok(())
    }

    /// Walk one try expression with branch flow for catch.
    ///
    /// Example:
    /// ```ds
    /// try body catch error finally cleanup
    /// ```
    fn walk_try_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        body: dir::LocalNodeId<dir::Expression>,
        catch: Option<dir::LocalNodeId<dir::Catch>>,
        finally: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> CompilerResult<()> {
        // walk try branches
        let before = self.fork_flow();

        // walk the body branch from the incoming flow
        self.restore_flow(before);
        if catch.is_some() {
            self.enter_try_target(id.into_any())?;
        }
        self.walk_expression(body, self.tree.get(body))?;
        let catch_failure = match catch {
            Some(_) => Some(self.leave_try_target(id.into_any())?),
            None => None,
        };
        let body_flow = self.collect_flow_branch(before);
        let body_can_complete = self.expression_can_complete_normally(body);

        // walk catch branch
        let catch_branch = if let Some(catch) = catch {
            self.restore_flow(before);
            self.walk_catch(catch, catch_failure)?;
            let catch_can_complete =
                self.expression_can_complete_normally(self.tree.get(catch).body);
            let catch_flow = self.collect_flow_branch(before);

            Some((catch, catch_can_complete, catch_flow))
        } else {
            None
        };

        // merge only branches that can continue normally
        let has_normal_flow = if let Some((_, catch_can_complete, catch_flow)) = catch_branch {
            match (body_can_complete, catch_can_complete) {
                (true, true) => {
                    self.merge_flow_branches(before, &body_flow, &catch_flow);

                    true
                }
                (true, false) => {
                    self.restore_flow_branch(before, &body_flow);

                    true
                }
                (false, true) => {
                    self.restore_flow_branch(before, &catch_flow);

                    true
                }
                (false, false) => {
                    self.restore_flow(before);

                    false
                }
            }
        } else if body_can_complete {
            self.restore_flow_branch(before, &body_flow);

            true
        } else {
            self.restore_flow(before);

            false
        };

        // walk finally even when no normal path remains
        if let Some(finally) = finally {
            self.walk_expression(finally, self.tree.get(finally))?;

            if !has_normal_flow || !self.expression_can_complete_normally(finally) {
                self.restore_flow(before);
            }
        } else if !has_normal_flow {
            self.restore_flow(before);
        }

        Ok(())
    }

    /// Walk one catch clause.
    ///
    /// Example:
    /// ```ds
    /// catch (error: Error) { error }
    /// ```
    fn walk_catch(
        &mut self,
        id: dir::LocalNodeId<dir::Catch>,
        failure: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<()> {
        if !self.walk_decorators(id.into_any())? {
            return Ok(());
        }
        let catch = self.tree.get(id);
        let (pattern, ty, body) = (catch.pattern, catch.ty, catch.body);

        // catch (error: T)
        let expected = ty.map(|ty| self.walk_type_expression(ty)).transpose()?;
        if let (Some(failure), Some(expected)) = (failure, expected) {
            let origin = Origin::Node(
                id.into_global_any(self.module),
                self.flow().template_scope(),
            );
            self.relate_type(
                origin,
                CauseKind::Expression,
                Relation::Assignable,
                failure,
                expected,
            );
        }

        // catch (error)
        if let Some(pattern) = pattern {
            self.walk_pattern(pattern, self.tree.get(pattern), None)?;

            // flow the caught value into the pattern type
            if let Some(value) = expected.or(failure) {
                self.queue_assignable(
                    pattern,
                    value,
                    CauseKind::Pattern {
                        pattern: pattern.into_global_any(self.module),
                    },
                    ValueUse::Store,
                )?;

                self.check.push_obligation(
                    Obligation::PatternCoverage(PatternCoverageObligation {
                        source: pattern.into_global_any(self.module),
                        value: ExpectedType::Type(value),
                        coverage: PatternCoverage::Catch {
                            pattern: pattern.into_global(self.module),
                        },
                    }),
                    self.flow().template_scope(),
                );
            }

            self.mark_bindings_assigned(pattern.into_any());
        }

        // defer the handler body until the caught bindings are typed
        let result = self.open_type_hole(id.into_any(), Widening::Never, VariableRole::Regular)?;
        self.check
            .catch_results
            .insert(id.into_global_any(self.module), result);
        // catch (...) { ... }
        self.walk_expression(body, self.tree.get(body))?;
        self.queue_assignable(body, result, CauseKind::Expression, ValueUse::Output)?;

        Ok(())
    }

    /// Walk one range expression.
    ///
    /// Example:
    /// ```ds
    /// start..end
    /// ```
    fn walk_range_expression(
        &mut self,
        start: Option<dir::LocalNodeId<dir::Expression>>,
        end: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> CompilerResult<()> {
        // walk range boundaries
        if let Some(start) = start {
            self.walk_expression(start, self.tree.get(start))?;
        }
        if let Some(end) = end {
            self.walk_expression(end, self.tree.get(end))?;
        }

        Ok(())
    }

    /// Walk one template literal's interpolated arguments.
    ///
    /// Example:
    /// ```ds
    /// `hello ${name}`
    /// ```
    fn walk_template_literal(&mut self, value: &dir::TemplateLiteral) -> CompilerResult<()> {
        if let dir::TemplateLiteral::InterpolatedString { arguments, .. } = value {
            for argument in arguments.clone() {
                self.walk_argument(argument, self.tree.get(argument))?;
            }
        }

        Ok(())
    }

    /// Walk one binary expression.
    ///
    /// Example:
    /// ```ds
    /// left + right
    /// ```
    fn walk_binary_expression(
        &mut self,
        left: dir::LocalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        self.walk_expression(left, self.tree.get(left))?;

        // short-circuit operators narrow their right operand
        match operator {
            dir::BinaryOperator::And => {
                let before = self.fork_flow();
                self.narrow_expression(left, ConditionBranch::True)?;
                self.walk_expression(right, self.tree.get(right))?;
                self.restore_flow(before);
            }
            dir::BinaryOperator::Or | dir::BinaryOperator::Coalesce => {
                let before = self.fork_flow();
                self.narrow_expression(left, ConditionBranch::False)?;
                self.walk_expression(right, self.tree.get(right))?;
                self.restore_flow(before);
            }
            _ => {
                self.walk_expression(right, self.tree.get(right))?;
            }
        }

        // `in` inference owns the predicate result
        if operator == dir::BinaryOperator::In {
            return Ok(());
        }

        Ok(())
    }

    /// Walk one assignment expression.
    ///
    /// Example:
    /// ```ds
    /// target = value
    /// ```
    fn walk_assign_expression(
        &mut self,
        left: dir::LocalNodeId<dir::AssignPattern>,
        operator: dir::AssignOperator,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        // choose the place use before walking either side
        let access = match operator {
            dir::AssignOperator::Assign => PlaceUse::Write,
            _ => PlaceUse::Update,
        };

        // walk simple place targets before the value expression
        let is_place_pattern = matches!(self.tree.get(left), dir::AssignPattern::Place { .. });
        let places = if is_place_pattern {
            self.walk_assign_pattern(left, None, access)?
        } else {
            Vec::new()
        };

        // walk the assigned value before destructuring defaults and writes
        self.walk_expression(right, self.tree.get(right))?;
        // assigning a binding elsewhere may consume an identifier source
        let target = match places.as_slice() {
            [(_, AssignedPlace::Symbol(symbol))] => Some(*symbol),
            _ => None,
        };
        self.mark_moved_source(right, None, target);
        let places = if is_place_pattern {
            places
        } else {
            self.walk_assign_pattern(left, None, access)?
        };

        // mark the places assigned by this expression
        self.mark_assign_pattern_places(places);

        Ok(())
    }

    /// Walk one assignment pattern against its assigned value.
    ///
    /// Example:
    /// ```ds
    /// { x, y: z } = point
    /// ```
    fn walk_assign_pattern(
        &mut self,
        id: dir::LocalNodeId<dir::AssignPattern>,
        value: Option<dir::GlobalTypeId>,
        access: PlaceUse,
    ) -> CompilerResult<Vec<(dir::LocalNodeId<dir::Expression>, AssignedPlace)>> {
        self.enter_node(id)?;

        if let Some(value) = value {
            self.commit_node_type(id, value)?;
        }

        let places = match self.tree.get(id) {
            // x = value, obj.x = value
            dir::AssignPattern::Place { expression: target } => {
                let target = *target;
                self.walk_assigned_place(target, access)?
                    .map(|place| vec![(target, place)])
                    .unwrap_or_default()
            }
            // x = default
            dir::AssignPattern::Default {
                pattern,
                value: default,
            } => {
                let (pattern, default) = (*pattern, *default);
                self.walk_expression(default, self.tree.get(default))?;
                self.walk_assign_pattern(pattern, None, access)?
            }
            // [a, ...rest] = values
            dir::AssignPattern::Sequence { fields } => {
                let mut places = Vec::new();
                for field in fields.clone() {
                    places.extend(self.walk_assign_pattern_field(field, access)?);
                }
                places
            }
            // (x, y) = point
            dir::AssignPattern::Tuple { fields } => {
                let mut places = Vec::new();
                for field in fields.clone() {
                    places.extend(self.walk_assign_pattern_field(field, access)?);
                }
                places
            }
            // { x, y: z } = point
            dir::AssignPattern::Object { fields } => {
                let mut places = Vec::new();
                for field in fields.clone() {
                    places.extend(self.walk_assign_pattern_field(field, access)?);
                }
                places
            }
        };

        Ok(places)
    }

    /// Walk one assignment pattern field before selection projects its input.
    fn walk_assign_pattern_field(
        &mut self,
        id: dir::LocalNodeId<dir::AssignPatternField>,
        access: PlaceUse,
    ) -> CompilerResult<Vec<(dir::LocalNodeId<dir::Expression>, AssignedPlace)>> {
        let places = match self.tree.get(id) {
            dir::AssignPatternField::Named { pattern, .. } => {
                let pattern = *pattern;
                self.walk_assign_pattern(pattern, None, access)?
            }
            dir::AssignPatternField::Computed { key, pattern } => {
                let (key, pattern) = (*key, *pattern);
                self.walk_expression(key, self.tree.get(key))?;
                self.walk_assign_pattern(pattern, None, access)?
            }
            dir::AssignPatternField::Positional { pattern } => {
                let pattern = *pattern;
                self.walk_assign_pattern(pattern, None, access)?
            }
            dir::AssignPatternField::Rest {
                pattern: Some(pattern),
            } => {
                let pattern = *pattern;
                self.walk_assign_pattern(pattern, None, access)?
            }
            dir::AssignPatternField::Rest { pattern: None } | dir::AssignPatternField::Elision => {
                Vec::new()
            }
        };

        Ok(places)
    }

    /// Mark all places assigned by one assignment pattern.
    fn mark_assign_pattern_places(
        &mut self,
        places: Vec<(dir::LocalNodeId<dir::Expression>, AssignedPlace)>,
    ) {
        for (expression, place) in places {
            self.mark_place_assigned(place);

            // assignments invalidate narrowings under the written expression
            self.clear_mutated_expression_narrowings(expression);
        }
    }
    /// Return whether one tree tag names a lowercase builder row.
    fn is_intrinsic_tree_tag(&self, tag: dir::LocalNodeId<dir::Expression>) -> bool {
        match self.tree.get(tag) {
            dir::Expression::Identifier { name } => self.check.is_intrinsic_tree_tag(*name),
            _ => false,
        }
    }
}
