use destack_dir as dir;
use smallvec::SmallVec;

use crate::sema::{AssignedPlace, ConditionBranch, ElisionSite, FlowBranch, PlaceUse, WalkState};
use crate::{CompilerError, CompilerResult};

impl WalkState<'_, '_> {
    /// Walk one expression.
    ///
    /// Example:
    /// ```ds
    /// value + 1
    /// ```
    pub(in crate::sema) fn walk_expression(
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

                if is_lambda {
                    let Some(symbol) = self.declared_symbol(declaration.into_any()) else {
                        return Err(CompilerError::Internal {
                            message: format!("function value {id:?} has no declaration symbol"),
                        });
                    };

                    // function values register their bodies at their expression
                    if let dir::Declaration::Function(function) = self.tree.get(declaration).clone()
                        && !self.walk_declared_function_body(declaration, &function, symbol)?
                    {
                        return Err(CompilerError::Internal {
                            message: format!(
                                "function value {id:?} was not declared before its body"
                            ),
                        });
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
                    // walk the declared body left behind by the declaration walk
                    if let Some(symbol) = self.declared_symbol(declaration.into_any())
                        && !self.walk_declared_body(
                            declaration,
                            &self.tree.get(declaration).clone(),
                            symbol,
                        )?
                    {
                        return Err(CompilerError::Internal {
                            message: format!(
                                "local declaration {declaration:?} was not declared before its body"
                            ),
                        });
                    }
                    let void = self.intern_type(dir::Type::Void)?;
                    self.commit_node_type(id, void)?;
                }
            }
            // { ... }
            dir::Expression::Block(block) => {
                let block = *block;
                self.walk_block(block, self.tree.get(block))?;
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
                declarators,
                is_ambient,
                ..
            } => {
                for declarator in declarators {
                    self.walk_declarator(*declarator, self.tree.get(*declarator), *is_ambient)?;
                    let node = self.tree.get(*declarator).clone();
                    self.assign_declarator_bindings(&node, *is_ambient);
                }
            }
            // using x = value
            dir::Expression::Using { declarators, .. } => {
                for declarator in declarators {
                    self.walk_declarator(*declarator, self.tree.get(*declarator), false)?;
                    let node = self.tree.get(*declarator).clone();
                    self.assign_declarator_bindings(&node, false);
                }
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
            // control statements type through the fused traversal only
            dir::Expression::While { .. }
            | dir::Expression::ForEach { .. }
            | dir::Expression::For { .. }
            | dir::Expression::Loop { .. }
            | dir::Expression::Try { .. }
            | dir::Expression::Match { .. }
            | dir::Expression::Switch { .. }
            | dir::Expression::Break { .. }
            | dir::Expression::Continue { .. }
            | dir::Expression::LetElse { .. } => {
                return Err(CompilerError::Internal {
                    message: format!("control statement {id:?} reached the value walk"),
                });
            }
            // await value
            dir::Expression::Await {
                expression: awaited,
            } => {
                let awaited = *awaited;
                self.walk_expression(awaited, self.tree.get(awaited))?;
            }
            // return value
            dir::Expression::Return { value } => {
                if let Some(value) = *value {
                    self.walk_expression(value, self.tree.get(value))?;
                }
            }
            // yield value
            dir::Expression::Yield { cardinality, value } => {
                let (cardinality, value) = (*cardinality, *value);

                match (cardinality, value) {
                    // yield value: the checker relates the value to the yield target
                    (dir::YieldCardinality::Scalar, Some(value)) => {
                        self.walk_expression(value, self.tree.get(value))?;
                    }
                    // bare yield: the checker relates void to the yield target
                    (dir::YieldCardinality::Scalar, None) => {}
                    // yield* value: the delegate implements the generator protocol
                    (dir::YieldCardinality::Generator, Some(value)) => {
                        self.walk_expression(value, self.tree.get(value))?;
                    }
                    // yield* without a delegate value
                    (dir::YieldCardinality::Generator, None) => {}
                }
            }
            // value
            dir::Expression::Identifier { name } => {
                self.walk_identifier_expression(id, *name)?;
            }
            // this
            dir::Expression::This => {
                let receiver = {
                    let node = id.into_global_any(self.module);
                    self.commit_active_receiver_decision(node, dir::ReceiverKind::This)?
                };
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
            dir::Expression::Literal(_) => {}
            // super
            dir::Expression::Super => {
                let receiver = {
                    let node = id.into_global_any(self.module);
                    self.commit_active_receiver_decision(node, dir::ReceiverKind::Super)?
                };
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
                let meta = self.language_type(dir::LanguageItem::ImportMeta, &[])?;
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
                // walk a capitalized tag as a value, lowercase tags name builder rows
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
                let represented = self.walk_type_expression_in(*value, ElisionSite::Body)?;
                let reflected = self
                    .check
                    .language_type(dir::LanguageItem::Type, &[represented])?;
                self.commit_node_type(id, reflected)?;
            }
            // const value
            dir::Expression::Const { body } => {
                let body = *body;

                // check const bodies in isolated flow
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
                // walk the written target type before the operand
                else {
                    self.walk_value_type_in(target_type, ElisionSite::Body)?;
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
                self.walk_value_type_in(target_type, ElisionSite::Body)?;
            }
            // value is T
            dir::Expression::Is { value, target_type } => {
                let (value, target_type) = (*value, *target_type);
                self.walk_expression(value, self.tree.get(value))?;
                self.walk_value_type_in(target_type, ElisionSite::Body)?;
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

                if let Some(place) = self.walk_assigned_place(right)? {
                    self.assign_place(place);
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

                for argument in &arguments {
                    self.walk_argument(*argument, self.tree.get(*argument))?;
                }
            }
            // _
            dir::Expression::Infer { .. } => {}
            // new Type<T>(argument)
            dir::Expression::New {
                left,
                generic_arguments,
                arguments,
            } => {
                let arguments = arguments.iter().copied().collect::<SmallVec<[_; 4]>>();
                self.walk_expression(*left, self.tree.get(*left))?;
                self.walk_generic_arguments(generic_arguments)?;
                for argument in &arguments {
                    self.walk_argument(*argument, self.tree.get(*argument))?;
                }
            }
            // await? value
            dir::Expression::AwaitMaybe {
                expression: awaited,
            } => {
                let awaited = *awaited;
                self.walk_expression(awaited, self.tree.get(awaited))?;
            }
            // await! value
            dir::Expression::AwaitMust {
                expression: awaited,
            } => {
                let awaited = *awaited;
                self.walk_expression(awaited, self.tree.get(awaited))?;
            }
            // value?
            dir::Expression::Maybe { left, .. } => {
                let left = *left;
                self.walk_expression(left, self.tree.get(left))?;
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
        self.walk_condition(condition)?;
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

        // walk the written false branch
        if let Some(else_expression) = else_expression {
            self.restore_flow(before);
            self.narrow_condition(condition, ConditionBranch::False)?;
            self.walk_expression(else_expression, self.tree.get(else_expression))?;
            let else_can_complete = self.expression_can_complete_normally(else_expression);

            // collect false completion
            if else_can_complete {
                branches.push(self.collect_flow_branch(before));
            }
        }
        // collect the implicit false completion
        else {
            self.restore_flow(before);
            self.narrow_condition(condition, ConditionBranch::False)?;
            branches.push(self.collect_flow_branch(before));
        }

        // merge normally completed branches
        self.merge_flow_branches_from(before, &branches);

        Ok(())
    }

    /// Walk one condition.
    ///
    /// Example:
    /// ```ds
    /// if let Some(value) = option { value }
    /// ```
    fn walk_condition(&mut self, condition: &dir::Condition) -> CompilerResult<()> {
        let before = self.fork_flow();
        let result = self.walk_condition_operands(&condition.operands);
        self.restore_flow(before);

        result
    }

    /// Walk one condition chain.
    fn walk_condition_operands(
        &mut self,
        operands: &[dir::ConditionOperand],
    ) -> CompilerResult<()> {
        for operand in operands {
            self.walk_condition_operand(operand)?;
        }

        Ok(())
    }

    /// Walk one operand in a condition chain.
    fn walk_condition_operand(&mut self, operand: &dir::ConditionOperand) -> CompilerResult<()> {
        match operand {
            // boolean condition
            dir::ConditionOperand::Expression { condition } => {
                let condition = *condition;
                self.walk_expression(condition, self.tree.get(condition))?;
            }
            // pattern binding condition
            dir::ConditionOperand::Binding { declarator, .. } => {
                let declarator = *declarator;
                self.walk_declarator(declarator, self.tree.get(declarator), false)?;
                self.narrow_let_condition(declarator)?;
            }
        }

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
        match ConditionBranch::from_short_circuit(operator) {
            Some(branch) => {
                let before = self.fork_flow();
                self.narrow_expression(left, branch)?;
                self.walk_expression(right, self.tree.get(right))?;
                self.restore_flow(before);
            }
            None => {
                self.walk_expression(right, self.tree.get(right))?;
            }
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
        let places = if is_place_pattern {
            places
        } else {
            self.walk_assign_pattern(left, None, access)?
        };

        // mark the places assigned by this expression
        self.assign_written_places(places);

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
                self.walk_assigned_place(target)?
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
            // { name: pattern }
            dir::AssignPatternField::Named { pattern, .. } => {
                let pattern = *pattern;
                self.walk_assign_pattern(pattern, None, access)?
            }
            // { [key]: pattern }
            dir::AssignPatternField::Computed { key, pattern } => {
                let (key, pattern) = (*key, *pattern);
                self.walk_expression(key, self.tree.get(key))?;
                self.walk_assign_pattern(pattern, None, access)?
            }
            // [pattern]
            dir::AssignPatternField::Positional { pattern } => {
                let pattern = *pattern;
                self.walk_assign_pattern(pattern, None, access)?
            }
            // { ...pattern }
            dir::AssignPatternField::Rest {
                pattern: Some(pattern),
            } => {
                let pattern = *pattern;
                self.walk_assign_pattern(pattern, None, access)?
            }
            // { ... }, [,]
            dir::AssignPatternField::Rest { pattern: None } | dir::AssignPatternField::Elision => {
                Vec::new()
            }
        };

        Ok(places)
    }

    /// Assign every place written by one assignment pattern.
    fn assign_written_places(
        &mut self,
        places: Vec<(dir::LocalNodeId<dir::Expression>, AssignedPlace)>,
    ) {
        for (expression, place) in places {
            self.assign_place(place);

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
