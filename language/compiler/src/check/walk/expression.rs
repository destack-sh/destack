use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    ConditionBranch, ConstraintRole, Decision, FlowBranch, FlowCheckpoint, ForInSourceObligation,
    GuardOutcome, MatchCase, Obligation, Origin, PatternCoverage, PatternCoverageObligation, Place,
    PlaceUse, Relation, WalkState, Widening, WritablePlaceObligation,
};

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
        let Some(_guard) = self.enter_decorated_static_guard(id.into_any())? else {
            return Ok(());
        };

        match expression {
            // function f() {}
            dir::Expression::Declaration(declaration) => {
                let declaration = *declaration;
                self.walk_declaration(declaration, self.tree.get(declaration))?;

                // lambdas and anonymous functions are values
                let is_function_value = matches!(
                    self.tree.get(declaration),
                    dir::Declaration::Function(function)
                        if function.signature.form == dir::FunctionForm::Lambda
                            || function.name.is_none()
                );
                if is_function_value {
                    let symbol = self
                        .check
                        .module(self.module)
                        .declaration_symbol(declaration.into_any());
                    if let Some(symbol) = symbol {
                        let ty = self.symbol_type(symbol)?;
                        self.bind_node_type(id, ty)?;
                    }
                } else {
                    let void = self.push_type(dir::Type::Void, id.into_any())?;
                    self.bind_node_type(id, void)?;
                }
            }
            // { ... }
            dir::Expression::Block(block) => {
                let block = *block;
                self.walk_block(block, self.tree.get(block))?;
                self.copy_node_type(id, block)?;
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
                let void = self.push_type(dir::Type::Void, id.into_any())?;
                self.bind_node_type(id, void)?;
            }
            // export { item } from "module"
            dir::Expression::Export { items, .. } => {
                for item in items {
                    self.walk_dependency_item(*item, self.tree.get(*item))?;
                }
                let void = self.push_type(dir::Type::Void, id.into_any())?;
                self.bind_node_type(id, void)?;
            }
            // let x = value
            dir::Expression::Let {
                kind,
                declarators,
                is_ambient,
                ..
            } => {
                for declarator in declarators {
                    self.walk_declarator(*declarator, self.tree.get(*declarator), Some(*kind))?;
                    self.mark_declarator_assigned(self.tree.get(*declarator), *is_ambient);
                }
                let void = self.push_type(dir::Type::Void, id.into_any())?;
                self.bind_node_type(id, void)?;
            }
            // using x = value
            dir::Expression::Using { declarators, .. } => {
                for declarator in declarators {
                    self.walk_declarator(*declarator, self.tree.get(*declarator), None)?;
                    self.mark_declarator_assigned(self.tree.get(*declarator), false);
                }
                let void = self.push_type(dir::Type::Void, id.into_any())?;
                self.bind_node_type(id, void)?;
            }
            // let pattern = value else { return }
            dir::Expression::LetElse {
                kind,
                declarator,
                else_branch,
                ..
            } => {
                self.walk_let_else_expression(id, *kind, *declarator, *else_branch)?;
            }
            // if condition { then } else { otherwise }
            dir::Expression::If {
                condition,
                then_expression,
                else_expression,
                ..
            } => {
                self.walk_if_expression(id, condition, *then_expression, *else_expression)?;
            }
            // while condition { body }
            dir::Expression::While {
                condition, body, ..
            } => {
                self.walk_while_expression(id, None, *condition, *body)?;
            }
            // for item of iterator { body }
            dir::Expression::ForEach {
                operator,
                binding,
                iterator,
                body,
                ..
            } => {
                self.walk_for_each_expression(id, None, *operator, binding, *iterator, *body)?;
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
                self.walk_for_expression(id, None, initialization, condition, increment, body)?;
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
            dir::Expression::Match { value, cases, .. } => {
                let cases = cases.iter().copied().collect::<SmallVec<[_; 4]>>();
                self.walk_match_expression(id, *value, &cases)?;
            }
            // break value
            dir::Expression::Break { label, value } => {
                let (label, value) = (*label, *value);
                if let Some(value) = value {
                    self.walk_expression(value, self.tree.get(value))?;
                }
                let value = value.map(|value| self.node_type(value)).transpose()?;
                let never = self.push_type(dir::Type::Never, id.into_any())?;
                self.bind_node_type(id, never)?;
                self.break_to_control_target(id.into_any(), label, value)?;
            }
            // continue
            dir::Expression::Continue { label } => {
                let never = self.push_type(dir::Type::Never, id.into_any())?;
                self.bind_node_type(id, never)?;
                self.continue_to_control_target(id.into_any(), *label);
            }
            // await value
            dir::Expression::Await {
                expression: awaited,
            } => {
                let awaited = *awaited;
                self.walk_expression(awaited, self.tree.get(awaited))?;
                self.validate_await_context(id.into_any());
                let result = self.await_result(id, awaited)?;
                self.bind_node_type(id, result)?;
            }
            // throw value
            dir::Expression::Throw { value } => {
                self.walk_expression(*value, self.tree.get(*value))?;
                let never = self.push_type(dir::Type::Never, id.into_any())?;
                self.bind_node_type(id, never)?;
            }
            // return value
            dir::Expression::Return { value } => {
                let value = *value;
                let never = self.push_type(dir::Type::Never, id.into_any())?;
                self.bind_node_type(id, never)?;

                if let Some(value) = value {
                    self.walk_expression(value, self.tree.get(value))?;
                    let ty = self.node_type(value)?;
                    self.constrain_return_value(id.into_any(), ty);
                } else {
                    self.constrain_void_return(id.into_any())?;
                }
            }
            // yield value
            dir::Expression::Yield { cardinality, value } => {
                let (cardinality, value) = (*cardinality, *value);
                if let Some(value) = value {
                    self.walk_expression(value, self.tree.get(value))?;
                }
                let value_type = value.map(|value| self.node_type(value)).transpose()?;
                let delegate_return = match cardinality {
                    // yield* delegates resume into the inner return
                    dir::YieldCardinality::Generator => Some(self.open_type(id.into_any())?),
                    dir::YieldCardinality::Scalar => None,
                };

                // yield evaluates to the resumed value
                match self.current_resume_target() {
                    Some(resumed) => {
                        self.bind_node_type(id, resumed)?;
                    }
                    None => {
                        let void = self.push_type(dir::Type::Void, id.into_any())?;
                        self.bind_node_type(id, void)?;
                    }
                }

                self.constrain_yield_value(
                    id.into_any(),
                    cardinality,
                    value_type,
                    delegate_return,
                )?;
            }
            // value
            dir::Expression::Identifier { name } => {
                self.walk_identifier_expression(id, *name)?;
            }
            // this
            dir::Expression::This => {
                let receiver = self.select_active_receiver(id.into_global_any(self.module))?;
                match receiver {
                    Some(receiver) => {
                        self.bind_node_type(id, receiver.ty)?;
                    }
                    None => {
                        self.check
                            .report_this_outside_receiver(self.module, id.into_any());
                        let error = self.push_type(dir::Type::Error, id.into_any())?;
                        self.bind_node_type(id, error)?;
                    }
                }
            }
            // 1, "text", true
            dir::Expression::ScalarLiteral(value) => {
                let value = *value;
                let ty = match value {
                    // /pattern/
                    dir::ScalarLiteral::RegexString { .. } => self.language_type_reference(
                        id.into_any(),
                        dir::LanguageItem::RegExp,
                        Vec::new(),
                    )?,
                    // nullish literals write their canonical types
                    dir::ScalarLiteral::Null => self.push_type(dir::Type::Null, id.into_any())?,
                    dir::ScalarLiteral::Undefined => {
                        self.push_type(dir::Type::Undefined, id.into_any())?
                    }
                    // scalar literals are their own singleton types
                    value => self.push_type(dir::Type::Literal(value), id.into_any())?,
                };
                self.bind_node_type(id, ty)?;
            }
            // super
            dir::Expression::Super => {
                let receiver = self.select_active_receiver(id.into_global_any(self.module))?;
                match receiver.and_then(|receiver| receiver.super_ty) {
                    Some(super_ty) => {
                        self.bind_node_type(id, super_ty)?;
                    }
                    None => {
                        self.check
                            .report_super_outside_class(self.module, id.into_any());
                        let error = self.push_type(dir::Type::Error, id.into_any())?;
                        self.bind_node_type(id, error)?;
                    }
                }
            }
            // import.meta
            dir::Expression::ImportMeta => {
                let meta = self.language_type_reference(
                    id.into_any(),
                    dir::LanguageItem::ImportMeta,
                    Vec::new(),
                )?;
                self.bind_node_type(id, meta)?;
            }
            // import.source resolves to its module source descriptor at lowering
            dir::Expression::ImportSource => {
                let source = self.push_type(dir::Type::Error, id.into_any())?;
                self.bind_node_type(id, source)?;
            }
            // #name, debugger, missing, stub, damaged nodes
            dir::Expression::PrivateIdentifier { .. }
            | dir::Expression::Debugger
            | dir::Expression::Missing
            | dir::Expression::Stub
            | dir::Expression::Error => {}
            // start..end
            dir::Expression::RangeExpression {
                start,
                end,
                end_kind,
            } => {
                self.walk_range_expression(id, *start, *end, *end_kind)?;
            }
            // `text ${value}`
            dir::Expression::TemplateExpression { value } => {
                self.walk_template_literal(value)?;
                let string = self.push_type(
                    dir::Type::Primitive(dir::PrimitiveType::String),
                    id.into_any(),
                )?;
                self.bind_node_type(id, string)?;
            }
            // tag<T>`text ${value}`
            dir::Expression::TaggedTemplateExpression { tag, value, .. } => {
                self.walk_expression(*tag, self.tree.get(*tag))?;
                self.walk_template_literal(value)?;

                // tagged template calls resolve at selection
                self.queue_selection(id.into_global_any(self.module))?;
            }
            // [a, b, c]
            dir::Expression::ArrayExpression { elements } => {
                let elements = elements.iter().copied().collect::<SmallVec<[_; 4]>>();
                self.walk_array_expression(id, &elements)?;
            }
            // [value; length]
            dir::Expression::FixedArrayExpression { value, length } => {
                let (value, length) = (*value, *length);
                self.walk_expression(value, self.tree.get(value))?;
                self.walk_expression(length, self.tree.get(length))?;

                let element = self.node_type(value)?;
                let count = self.walk_static_term(length)?;
                let array = self.push_type(
                    dir::Type::FixedArray(dir::FixedArrayType { element, count }),
                    id.into_any(),
                )?;
                self.bind_node_type(id, array)?;
            }
            // [a, label: b, ...rest]
            dir::Expression::TupleExpression { elements } => {
                let elements = elements.iter().copied().collect::<SmallVec<[_; 4]>>();
                self.walk_tuple_expression(id, &elements)?;
            }
            // a, b, c
            dir::Expression::SequenceExpression { expressions } => {
                let expressions = expressions.iter().copied().collect::<SmallVec<[_; 4]>>();
                for expression in &expressions {
                    self.walk_expression(*expression, self.tree.get(*expression))?;
                }

                // sequences evaluate to their final expression
                match expressions.last() {
                    Some(last) => {
                        let ty = self.node_type(*last)?;
                        self.bind_node_type(id, ty)?;
                    }
                    None => {
                        let void = self.push_type(dir::Type::Void, id.into_any())?;
                        self.bind_node_type(id, void)?;
                    }
                }
            }
            // { key: value }
            dir::Expression::ObjectExpression { properties } => {
                let properties = properties.iter().copied().collect::<SmallVec<[_; 4]>>();
                let (fields, has_spread) = self.walk_literal_properties(&properties)?;

                // queue selection when spread properties need closed source types
                if has_spread {
                    self.queue_selection(id.into_global_any(self.module))?;
                }
                // object literal values are managed objects
                else {
                    let shape = self.push_type(
                        dir::Type::Shape(dir::ShapeType {
                            fields,
                            call_signatures: Vec::new(),
                            construct_signatures: Vec::new(),
                            index_signatures: Vec::new(),
                        }),
                        id.into_any(),
                    )?;
                    let managed = self.push_type(
                        dir::Type::Form(dir::FormType {
                            form: dir::Form::Managed,
                            value: shape,
                        }),
                        id.into_any(),
                    )?;
                    self.bind_node_type(id, managed)?;
                }
            }
            // Type { key: value }
            dir::Expression::StructExpression { ty, properties } => {
                let properties = properties.iter().copied().collect::<SmallVec<[_; 4]>>();
                let target = self.walk_type_expression(*ty)?;
                let (fields, has_spread) = self.walk_literal_properties(&properties)?;
                self.bind_node_type(id, target)?;

                // queue selection when spread properties need closed source types
                if has_spread {
                    self.queue_selection(id.into_global_any(self.module))?;
                }
                // the written fields must fill the declared struct fields
                else {
                    let shape = self.push_type(
                        dir::Type::Shape(dir::ShapeType {
                            fields,
                            call_signatures: Vec::new(),
                            construct_signatures: Vec::new(),
                            index_signatures: Vec::new(),
                        }),
                        id.into_any(),
                    )?;
                    let origin = Origin::Node(id.into_global_any(self.module));
                    self.relate_type(origin, Relation::Writable, shape, target);
                }
            }
            // jsx like tree expression
            dir::Expression::TreeExpression {
                left,
                arguments,
                elements,
                ..
            } => {
                if let Some(left) = *left {
                    self.walk_expression(left, self.tree.get(left))?;
                }
                if let Some(arguments) = arguments.as_deref() {
                    for argument in arguments {
                        self.walk_argument(*argument, self.tree.get(*argument))?;
                    }
                }
                if let Some(elements) = elements.as_deref() {
                    for element in elements {
                        self.walk_argument(*element, self.tree.get(*element))?;
                    }
                }

                // queue selection for tree construction
                self.queue_selection(id.into_global_any(self.module))?;
            }
            // (value)
            dir::Expression::Parenthesized { expression: child } => {
                let child = *child;
                self.walk_expression(child, self.tree.get(child))?;
                let ty = self.node_type(child)?;
                self.bind_node_type(id, ty)?;
            }
            // type T
            dir::Expression::Type { value } => {
                let ty = self.walk_type_expression(*value)?;
                self.bind_node_type(id, ty)?;
            }
            // comptime value
            dir::Expression::Comptime { body } => {
                let body = *body;

                // check comptime bodies in isolated flow
                let before_body = self.fork_flow();
                self.walk_expression(body, self.tree.get(body))?;
                self.restore_flow(before_body);

                let ty = self.node_type(body)?;
                self.bind_node_type(id, ty)?;
            }
            // value as T
            dir::Expression::As {
                expression: child,
                target_type,
            } => {
                let (child, target_type) = (*child, *target_type);

                // value as const freezes the precise written type
                if matches!(self.tree.get(target_type), dir::TypeExpression::Const) {
                    self.walk_expression(child, self.tree.get(child))?;
                    let ty = self.node_type(child)?;
                    let asserted = self.const_asserted_type(child, ty)?;
                    self.bind_node_type(id, asserted)?;
                } else {
                    let target = self.walk_type_expression(target_type)?;
                    self.walk_expression_with_relation(
                        child,
                        self.tree.get(child),
                        target,
                        Relation::Castable,
                    )?;
                    self.bind_node_type(id, target)?;
                }
            }
            // value satisfies T
            dir::Expression::Satisfies {
                expression: child,
                target_type,
            } => {
                let (child, target_type) = (*child, *target_type);
                self.walk_expression(child, self.tree.get(child))?;
                let target = self.walk_type_expression(target_type)?;
                let value = self.node_type(child)?;
                self.bind_node_type(id, value)?;

                // require the value to satisfy the target without changing its type
                let origin = Origin::Node(id.into_global_any(self.module));
                self.relate_type(origin, Relation::Satisfies, value, target);
            }
            // value is T
            dir::Expression::Is { value, target_type } => {
                let (value, target_type) = (*value, *target_type);
                self.walk_expression(value, self.tree.get(value))?;
                self.walk_type_expression(target_type)?;
                let boolean = self.push_type(
                    dir::Type::Primitive(dir::PrimitiveType::Boolean),
                    id.into_any(),
                )?;
                self.bind_node_type(id, boolean)?;
                self.queue_selection(id.into_global_any(self.module))?;
            }
            // value instanceof Target
            dir::Expression::InstanceOf { value, target } => {
                let (value, target) = (*value, *target);
                self.walk_expression(value, self.tree.get(value))?;
                self.walk_expression(target, self.tree.get(target))?;
                let boolean = self.push_type(
                    dir::Type::Primitive(dir::PrimitiveType::Boolean),
                    id.into_any(),
                )?;
                self.bind_node_type(id, boolean)?;
                self.queue_selection(id.into_global_any(self.module))?;
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

                // increments rewrite their place by one
                if let Some(place) = self.walk_assignment_place(right, PlaceUse::Update)? {
                    let value = self.node_type(id)?;
                    let target = self.node_type(right)?;
                    let origin = Origin::Node(id.into_global_any(self.module));
                    self.push_relation(
                        origin,
                        ConstraintRole::Value,
                        Relation::Assignable,
                        value,
                        target,
                    );

                    self.push_write_obligations(place, target);
                    self.mark_place_assigned(place);
                }

                // increments invalidate narrowings under the target
                self.clear_mutated_expression_narrowings(right);

                // queue selection for the increment operator
                self.queue_selection(id.into_global_any(self.module))?;
            }
            // !value, -value
            dir::Expression::Unary { right, .. } => {
                let right = *right;
                self.walk_expression(right, self.tree.get(right))?;

                // queue selection for the unary operator
                self.queue_selection(id.into_global_any(self.module))?;
            }
            // ^value
            dir::Expression::MoveOf { right, .. } => {
                let right = *right;
                self.walk_expression(right, self.tree.get(right))?;
                let ty = self.node_type(right)?;
                self.bind_node_type(id, ty)?;
            }
            // &value
            dir::Expression::BorrowOf {
                mutability, right, ..
            } => {
                let (mutability, right) = (*mutability, *right);
                self.walk_expression(right, self.tree.get(right))?;

                // derive the lifetime from the borrowed place, write the sigil's access
                let value = self.node_type(right)?;
                let lifetime = self.borrow_provenance(id, right)?;
                let access = match mutability {
                    Some(mutability) => self.push_type(
                        dir::Type::Memory(dir::MemoryLiteral::Access(mutability.access())),
                        id.into_any(),
                    )?,
                    // unqualified borrows are mutable by default
                    None => self.push_type(
                        dir::Type::Memory(dir::MemoryLiteral::Access(dir::Access::Mutable)),
                        id.into_any(),
                    )?,
                };
                let borrowed = self.push_type(
                    dir::Type::Form(dir::FormType {
                        form: dir::Form::Borrowed { lifetime, access },
                        value,
                    }),
                    id.into_any(),
                )?;
                self.bind_node_type(id, borrowed)?;
            }
            // value.member, or a static name path resolved by the resolve phase
            dir::Expression::Member { left, .. } => {
                self.walk_member_expression(id, *left)?;
            }
            // value.#member always projects at selection
            dir::Expression::PrivateMember { left, .. } => {
                self.walk_expression(*left, self.tree.get(*left))?;

                self.queue_selection(id.into_global_any(self.module))?;
            }
            // value[index]
            dir::Expression::Index { left, index, .. } => {
                self.walk_expression(*left, self.tree.get(*left))?;
                if let Some(index) = *index {
                    self.walk_expression(index, self.tree.get(index))?;
                }

                // queue selection for the index expression
                self.queue_selection(id.into_global_any(self.module))?;
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
                let arguments = arguments.iter().copied().collect::<SmallVec<[_; 4]>>();
                self.walk_expression(*left, self.tree.get(*left))?;
                self.walk_generic_arguments(generic_arguments)?;
                for argument in &arguments {
                    self.walk_argument(*argument, self.tree.get(*argument))?;
                }

                // queue selection for the call expression
                self.queue_selection(id.into_global_any(self.module))?;
            }
            // new Type<T>(argument)
            dir::Expression::New { ty, arguments } => {
                let arguments = arguments.iter().copied().collect::<SmallVec<[_; 4]>>();
                self.walk_type_expression(*ty)?;
                for argument in &arguments {
                    self.walk_argument(*argument, self.tree.get(*argument))?;
                }

                // queue selection for checked construction
                self.queue_selection(id.into_global_any(self.module))?;
            }
            // new? Type<T>(argument)
            dir::Expression::NewMaybe { ty, arguments } => {
                let arguments = arguments.iter().copied().collect::<SmallVec<[_; 4]>>();
                self.walk_type_expression(*ty)?;
                for argument in &arguments {
                    self.walk_argument(*argument, self.tree.get(*argument))?;
                }

                // queue selection for checked construction
                self.queue_selection(id.into_global_any(self.module))?;
                self.propagate_selected_try(id.into_any())?;
            }
            // await? value
            dir::Expression::AwaitMaybe {
                expression: awaited,
            } => {
                let awaited = *awaited;
                self.walk_expression(awaited, self.tree.get(awaited))?;
                self.validate_await_context(id.into_any());
                let result = self.await_result(id, awaited)?;
                self.propagate_try(id.into_any(), result)?;
                let output = self.try_output(id, result)?;
                self.bind_node_type(id, output)?;
            }
            // await! value
            dir::Expression::AwaitMust {
                expression: awaited,
            } => {
                let awaited = *awaited;
                self.walk_expression(awaited, self.tree.get(awaited))?;
                self.validate_await_context(id.into_any());
                let result = self.await_result(id, awaited)?;
                let output = self.try_output(id, result)?;
                self.bind_node_type(id, output)?;
            }
            // value?
            dir::Expression::Maybe { left, .. } => {
                let left = *left;
                self.walk_expression(left, self.tree.get(left))?;
                let value = self.node_type(left)?;
                self.propagate_try(id.into_any(), value)?;
                let output = self.try_output(id, value)?;
                self.bind_node_type(id, output)?;
            }
            // value!
            dir::Expression::Must { left, .. } => {
                let left = *left;
                self.walk_expression(left, self.tree.get(left))?;
                let value = self.node_type(left)?;
                let output = self.try_output(id, value)?;
                self.bind_node_type(id, output)?;
            }
            // left + right
            dir::Expression::Binary {
                left,
                operator,
                right,
            } => {
                self.walk_binary_expression(id, *left, *operator, *right)?;
            }
            // target = value
            dir::Expression::Assign {
                left,
                operator,
                right,
            } => {
                self.walk_assign_expression(id, *left, *operator, *right)?;
            }
        }

        Ok(())
    }

    /// Walk one expression against an expected value type.
    pub(in crate::check) fn walk_expression_expected(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
        expected: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.walk_expression_with_relation(id, expression, expected, Relation::Assignable)
    }

    /// Walk one expression against a target value type and relation.
    fn walk_expression_with_relation(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
        expected: dir::GlobalTypeId,
        relation: Relation,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let expected_type = self.check.ty(expected)?.clone();

        // let literal containers use the target representation
        let walked = match (expression, expected_type) {
            (dir::Expression::ArrayExpression { elements }, dir::Type::Array(array)) => {
                let elements = elements.iter().copied().collect::<SmallVec<[_; 4]>>();
                self.walk_array_expression_expected(id, &elements, array.element, None, relation)?
            }
            (dir::Expression::ArrayExpression { elements }, dir::Type::FixedArray(array)) => {
                let elements = elements.iter().copied().collect::<SmallVec<[_; 4]>>();
                self.walk_array_expression_expected(
                    id,
                    &elements,
                    array.element,
                    Some(array.count),
                    relation,
                )?
            }
            _ => {
                self.walk_expression(id, expression)?;
                self.node_type(id)?
            }
        };

        // keep the real source-target relation for diagnostics and coercions
        self.push_relation(
            Origin::Node(id.into_global_any(self.module)),
            ConstraintRole::Value,
            relation,
            walked,
            expected,
        );

        Ok(walked)
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
        // labeled loops keep their own continue targets
        match self.tree.get(body) {
            dir::Expression::While {
                condition,
                body: loop_body,
                ..
            } => {
                let (condition, loop_body) = (*condition, *loop_body);
                self.walk_while_expression(body, Some(label), condition, loop_body)?;
            }
            dir::Expression::ForEach {
                operator,
                binding,
                iterator,
                body: loop_body,
                ..
            } => {
                let (operator, iterator, loop_body) = (*operator, *iterator, *loop_body);
                self.walk_for_each_expression(
                    body,
                    Some(label),
                    operator,
                    binding,
                    iterator,
                    loop_body,
                )?;
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
                    body,
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
            // labeled blocks accept labeled breaks
            _ => {
                let result = self.open_inferred_node_type(body, Widening::Preserve)?;
                self.enter_control_target(Some(label), false, body, result);
                self.walk_expression(body, self.tree.get(body))?;
                let fallthrough = self.node_type(body)?;
                self.leave_control_target(Some(fallthrough))?;
            }
        }

        let ty = self.node_type(body)?;
        self.bind_node_type(id, ty)?;

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
        id: dir::LocalNodeId<dir::Expression>,
        kind: dir::LetKind,
        declarator: dir::LocalNodeId<dir::Declarator>,
        else_branch: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        self.walk_declarator(declarator, self.tree.get(declarator), Some(kind))?;

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

        let void = self.push_type(dir::Type::Void, id.into_any())?;
        self.bind_node_type(id, void)?;

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
        id: dir::LocalNodeId<dir::Expression>,
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
        let then_type = self.node_type(then_expression)?;
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
            let else_type = self.node_type(else_expression)?;
            let union = self.normalized_union_type([then_type, else_type], id.into_any())?;
            self.bind_node_type(id, union)?;

            // collect false completion
            if else_can_complete {
                branches.push(self.collect_flow_branch(before));
            }
        }
        // no false branch
        else {
            let void = self.push_type(dir::Type::Void, id.into_any())?;
            let union = self.normalized_union_type([then_type, void], id.into_any())?;
            self.bind_node_type(id, union)?;

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
                self.expect_boolean_condition(condition)?;
            }
            // pattern binding condition
            dir::ConditionOperand::Binding {
                kind, declarator, ..
            } => {
                let declarator = *declarator;
                self.walk_declarator(declarator, self.tree.get(declarator), Some(*kind))?;
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
        id: dir::LocalNodeId<dir::Expression>,
        label: Option<dir::StringId>,
        condition: dir::LocalNodeId<dir::Expression>,
        body: dir::LocalNodeId<dir::Block>,
    ) -> CompilerResult<()> {
        // walk condition in incoming flow
        self.walk_expression(condition, self.tree.get(condition))?;
        self.expect_boolean_condition(condition)?;

        // enter loop control target
        let result = self.open_inferred_node_type(id, Widening::Preserve)?;
        self.enter_control_target(label, true, id, result);

        // walk body under true condition flow
        let before_body = self.fork_flow();
        self.narrow_expression(condition, ConditionBranch::True)?;
        self.walk_block(body, self.tree.get(body))?;
        self.restore_flow(before_body);

        // collect normal exit through false condition
        self.narrow_expression(condition, ConditionBranch::False)?;
        let normal_flow = self.collect_flow_branch(before_body);
        let fallthrough = self.push_type(dir::Type::Void, id.into_any())?;
        let mut branches = self.leave_control_target(Some(fallthrough))?;

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
        id: dir::LocalNodeId<dir::Expression>,
        label: Option<dir::StringId>,
        operator: dir::ForEachOperator,
        binding: &dir::ForEachBinding,
        iterator: dir::LocalNodeId<dir::Expression>,
        body: dir::LocalNodeId<dir::Block>,
    ) -> CompilerResult<()> {
        // walk binding and iterator in incoming flow
        let pattern = match binding {
            dir::ForEachBinding::Pattern { pattern, .. }
            | dir::ForEachBinding::Using { pattern, .. } => *pattern,
        };
        self.walk_pattern(pattern, self.tree.get(pattern))?;
        self.walk_expression(iterator, self.tree.get(iterator))?;
        self.constrain_for_each_binding(id, operator, pattern, iterator)?;

        // enter loop control target
        let result = self.open_inferred_node_type(id, Widening::Preserve)?;
        self.enter_control_target(label, true, id, result);

        // walk body with iteration binding assigned
        let before_body = self.fork_flow();
        self.mark_bindings_assigned(pattern.into_any());
        self.walk_block(body, self.tree.get(body))?;
        self.restore_flow(before_body);

        // collect normal loop exit
        let normal_flow = self.collect_flow_branch(before_body);
        let fallthrough = self.push_type(dir::Type::Void, id.into_any())?;
        let mut branches = self.leave_control_target(Some(fallthrough))?;

        // merge break branches with normal exit
        branches.push(normal_flow);
        self.merge_flow_branches_from(before_body, &branches);

        Ok(())
    }

    /// Constrain one for each binding to its iterated value.
    ///
    /// Example:
    /// ```ds
    /// for (item of items) { item }
    /// ```
    fn constrain_for_each_binding(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        operator: dir::ForEachOperator,
        pattern: dir::LocalNodeId<dir::Pattern>,
        iterator: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        let origin = Origin::Node(id.into_global_any(self.module));
        let iterator_type = self.node_type(iterator)?;
        let value = match operator {
            // for (const item of iterable)
            dir::ForEachOperator::Of => {
                let value = self.open_type(id.into_any())?;
                let unknown = self.push_type(dir::Type::Unknown, id.into_any())?;
                let iterable = self.language_type_reference(
                    id.into_any(),
                    dir::LanguageItem::Iterable,
                    vec![value, unknown, unknown],
                )?;
                self.relate_type(origin, Relation::Assignable, iterator_type, iterable);

                value
            }
            // for (const key in object)
            dir::ForEachOperator::In => {
                let obligation = ForInSourceObligation {
                    source: id.into_global_any(self.module),
                    condition: self.active_static_guard(),
                    ty: iterator_type,
                };
                self.check
                    .push_obligation(Obligation::ForInSource(obligation));

                self.push_type(
                    dir::Type::Primitive(dir::PrimitiveType::String),
                    id.into_any(),
                )?
            }
        };

        // flow the iterated value into the pattern type
        let pattern_type = self.node_type(pattern)?;
        self.relate_type(origin, Relation::Assignable, value, pattern_type);

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
        id: dir::LocalNodeId<dir::Expression>,
        label: Option<dir::StringId>,
        initialization: Option<dir::LocalNodeId<dir::Expression>>,
        condition: Option<dir::LocalNodeId<dir::Expression>>,
        increment: Option<dir::LocalNodeId<dir::Expression>>,
        body: dir::LocalNodeId<dir::Block>,
    ) -> CompilerResult<()> {
        let result = self.open_inferred_node_type(id, Widening::Preserve)?;

        // walk initialization before loop flow splits
        if let Some(initialization) = initialization {
            self.walk_expression(initialization, self.tree.get(initialization))?;
        }

        // walk condition in incoming flow
        if let Some(condition) = condition {
            self.walk_expression(condition, self.tree.get(condition))?;
            self.expect_boolean_condition(condition)?;
        }

        // enter loop control target
        self.enter_control_target(label, true, id, result);

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
        let fallthrough = match condition {
            Some(_) => Some(self.push_type(dir::Type::Void, id.into_any())?),
            None => None,
        };
        let mut branches = self.leave_control_target(fallthrough)?;

        // merge break branches with normal exit
        if let Some(normal_flow) = normal_flow {
            branches.push(normal_flow);
        }
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
        // enter loop control target
        let result = self.open_inferred_node_type(id, Widening::Preserve)?;
        self.enter_control_target(label, true, id, result);

        // walk body with isolated flow
        let before_body = self.fork_flow();
        self.walk_block(body, self.tree.get(body))?;
        self.restore_flow(before_body);

        // restore only branches that leave the loop
        let branches = self.leave_control_target(None)?;
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

        // write the try expression type from completing branches
        let body_type = self.node_type(body)?;
        match catch_branch.as_ref() {
            Some((catch, _, _)) => {
                let catch_body = self.tree.get(*catch).body;
                let catch_type = self.node_type(catch_body)?;
                let union = self.normalized_union_type([body_type, catch_type], id.into_any())?;
                self.bind_node_type(id, union)?;
            }
            None => {
                self.bind_node_type(id, body_type)?;
            }
        }

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
        let Some(_guard) = self.enter_decorated_static_guard(id.into_any())? else {
            return Ok(());
        };
        let catch = self.tree.get(id);
        let (pattern, ty, body) = (catch.pattern, catch.ty, catch.body);

        // catch (error: T)
        let expected = ty.map(|ty| self.walk_type_expression(ty)).transpose()?;
        if let (Some(failure), Some(expected)) = (failure, expected) {
            let origin = Origin::Node(id.into_global_any(self.module));
            self.relate_type(origin, Relation::Assignable, failure, expected);
        }

        // catch (error)
        if let Some(pattern) = pattern {
            self.walk_pattern(pattern, self.tree.get(pattern))?;

            // flow the caught value into the pattern type
            if let Some(value) = expected.or(failure) {
                let origin = Origin::Node(pattern.into_global_any(self.module));
                let pattern_type = self.node_type(pattern)?;
                self.relate_type(origin, Relation::Assignable, value, pattern_type);

                let condition = self.active_static_guard();
                self.check.push_obligation(Obligation::PatternCoverage(
                    PatternCoverageObligation {
                        source: pattern.into_global_any(self.module),
                        condition,
                        value,
                        coverage: PatternCoverage::Catch {
                            pattern: pattern.into_global(self.module),
                        },
                    },
                ));
            }

            self.mark_bindings_assigned(pattern.into_any());
        }

        // catch (...) { ... }
        self.walk_expression(body, self.tree.get(body))?;

        Ok(())
    }

    /// Walk one match expression with isolated case flow.
    ///
    /// Example:
    /// ```ds
    /// match value { case Some(item) => item }
    /// ```
    fn walk_match_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        value: dir::LocalNodeId<dir::Expression>,
        cases: &[dir::LocalNodeId<dir::MatchCase>],
    ) -> CompilerResult<()> {
        // walk match discriminant
        self.walk_expression(value, self.tree.get(value))?;
        let value_type = self.node_type(value)?;
        let value_path = self.flow_path(value);

        // select cases whose static guards can hold
        let mut active_cases = Vec::new();
        for case in cases {
            match self.static_guard_condition(case.into_any())? {
                GuardOutcome::Absent => {}
                GuardOutcome::Present(condition) => active_cases.push((*case, condition)),
            }
        }

        let before = self.fork_flow();
        let mut result_types = Vec::new();
        let mut coverage_cases = Vec::new();
        let mut merged: Option<FlowBranch> = None;

        let mut remaining_type = value_type;
        for (case, condition) in &active_cases {
            self.restore_flow(before);
            {
                let _guard = self.enter_static_guard(condition.clone());
                self.walk_match_case(
                    *case,
                    self.tree.get(*case),
                    Some((remaining_type, value_path.clone())),
                )?;
            }

            // collect the case body value
            let body = match self.tree.get(*case) {
                dir::MatchCase::Expression { body, .. } => *body,
                dir::MatchCase::Block { body, .. } => {
                    let body = *body;
                    result_types.push(self.node_type(body)?);

                    if self.match_case_can_complete_normally(self.tree.get(*case)) {
                        let case_flow = self.collect_flow_branch(before);
                        merged = match merged.take() {
                            Some(previous) => {
                                self.merge_flow_branches(before, &previous, &case_flow);

                                Some(self.collect_flow_branch(before))
                            }
                            None => Some(case_flow),
                        };
                    }
                    coverage_cases.extend(self.match_case_coverage(*case)?);
                    remaining_type = self.next_match_input_type(*case, remaining_type)?;

                    continue;
                }
            };
            result_types.push(self.node_type(body)?);
            coverage_cases.extend(self.match_case_coverage(*case)?);
            remaining_type = self.next_match_input_type(*case, remaining_type)?;

            if !self.match_case_can_complete_normally(self.tree.get(*case)) {
                continue;
            }
            let case_flow = self.collect_flow_branch(before);
            merged = match merged.take() {
                Some(previous) => {
                    self.merge_flow_branches(before, &previous, &case_flow);

                    Some(self.collect_flow_branch(before))
                }
                None => Some(case_flow),
            };
        }

        if let Some(merged) = merged {
            self.restore_flow_branch(before, &merged);
        } else {
            self.restore_flow(before);
        }

        // matches must cover their scrutinee
        let condition = self.active_static_guard();
        self.check
            .push_obligation(Obligation::PatternCoverage(PatternCoverageObligation {
                source: id.into_global_any(self.module),
                condition,
                value: value_type,
                coverage: PatternCoverage::Match {
                    cases: coverage_cases,
                },
            }));

        // the match evaluates to the union of its case values
        let union = self.normalized_union_type(result_types, id.into_any())?;
        self.bind_node_type(id, union)?;

        Ok(())
    }

    /// Return the coverage case for one match arm.
    fn match_case_coverage(
        &mut self,
        case: dir::LocalNodeId<dir::MatchCase>,
    ) -> CompilerResult<Option<MatchCase>> {
        let selector = match self.tree.get(case) {
            dir::MatchCase::Expression { selector, .. }
            | dir::MatchCase::Block { selector, .. } => selector,
        };

        match selector {
            // default
            dir::MatchSelector::Default => Ok(Some(MatchCase::Default)),
            // case pattern if guard
            dir::MatchSelector::Pattern { pattern, guard } => {
                let (pattern, guard) = (*pattern, *guard);
                let guard = guard.map(|guard| self.node_type(guard)).transpose()?;

                Ok(Some(MatchCase::Pattern {
                    pattern: pattern.into_global(self.module),
                    guard,
                }))
            }
        }
    }

    /// Walk one range expression.
    ///
    /// Example:
    /// ```ds
    /// start..end
    /// ```
    fn walk_range_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        start: Option<dir::LocalNodeId<dir::Expression>>,
        end: Option<dir::LocalNodeId<dir::Expression>>,
        end_kind: dir::RangeEnd,
    ) -> CompilerResult<()> {
        // walk range boundaries
        if let Some(start) = start {
            self.walk_expression(start, self.tree.get(start))?;
        }
        if let Some(end) = end {
            self.walk_expression(end, self.tree.get(end))?;
        }

        // join both boundaries into one element type
        let origin = Origin::Node(id.into_global_any(self.module));
        let element = self.open_type(id.into_any())?;
        if let Some(start) = start {
            let start = self.node_type(start)?;
            self.relate_type(origin, Relation::Assignable, start, element);
        }
        if let Some(end) = end {
            let end = self.node_type(end)?;
            self.relate_type(origin, Relation::Assignable, end, element);
        }

        // pick the runtime range shape by its written bounds
        let (item, arguments) = match (start, end, end_kind) {
            (Some(_), Some(_), dir::RangeEnd::Open) => (dir::LanguageItem::Range, vec![element]),
            (Some(_), Some(_), dir::RangeEnd::Inclusive) => {
                (dir::LanguageItem::RangeInclusive, vec![element])
            }
            (Some(_), None, _) => (dir::LanguageItem::RangeFrom, vec![element]),
            (None, Some(_), dir::RangeEnd::Open) => (dir::LanguageItem::RangeTo, vec![element]),
            (None, Some(_), dir::RangeEnd::Inclusive) => {
                (dir::LanguageItem::RangeToInclusive, vec![element])
            }
            (None, None, _) => (dir::LanguageItem::RangeFull, Vec::new()),
        };
        let range = self.language_type_reference(id.into_any(), item, arguments)?;
        self.bind_node_type(id, range)?;

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

    /// Walk one array expression.
    ///
    /// Example:
    /// ```ds
    /// [a, b, c]
    /// ```
    fn walk_array_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        elements: &[dir::LocalNodeId<dir::Argument>],
    ) -> CompilerResult<()> {
        let element = self.open_type(id.into_any())?;
        let array = self.push_type(dir::Type::Array(dir::ArrayType { element }), id.into_any())?;

        // every item flows into the shared element type, each flow
        // anchored at its own value for coercions and diagnostics
        for item in elements {
            self.walk_argument(*item, self.tree.get(*item))?;

            match self.tree.get(*item) {
                // [...rest] spreads flow element-wise
                dir::Argument::Spread { value, .. } => {
                    let value = *value;
                    let origin = Origin::Node(value.into_global_any(self.module));
                    let spread = self.node_type(value)?;
                    self.relate_type(origin, Relation::Assignable, spread, array);
                }
                dir::Argument::Positional { value }
                | dir::Argument::Named { value, .. }
                | dir::Argument::Labeled { value, .. } => {
                    let value = *value;
                    let origin = Origin::Node(value.into_global_any(self.module));
                    let item = self.node_type(value)?;
                    self.relate_type(origin, Relation::Assignable, item, element);
                }
                dir::Argument::Error => {}
            }
        }

        self.bind_node_type(id, array)?;

        Ok(())
    }

    /// Walk one array literal whose target element type is known.
    fn walk_array_expression_expected(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        elements: &[dir::LocalNodeId<dir::Argument>],
        element: dir::GlobalTypeId,
        count: Option<dir::GlobalTypeId>,
        relation: Relation,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let mut values = SmallVec::<[_; 8]>::new();
        for argument in elements {
            match self.tree.get(*argument) {
                dir::Argument::Positional { value }
                | dir::Argument::Named { value, .. }
                | dir::Argument::Labeled { value, .. } => values.push(*value),
                dir::Argument::Spread { .. } | dir::Argument::Error => {
                    self.walk_array_expression(id, elements)?;

                    return self.node_type(id);
                }
            }
        }

        // walk each value under the expected element type
        for value in &values {
            self.walk_expression_with_relation(*value, self.tree.get(*value), element, relation)?;
        }

        let array = if count.is_some() {
            let actual_count = self.push_type(
                dir::Type::Literal(dir::ScalarLiteral::Integer(values.len() as i64)),
                id.into_any(),
            )?;

            self.push_type(
                dir::Type::FixedArray(dir::FixedArrayType {
                    element,
                    count: actual_count,
                }),
                id.into_any(),
            )?
        } else {
            self.push_type(dir::Type::Array(dir::ArrayType { element }), id.into_any())?
        };

        self.bind_node_type(id, array)
    }

    /// Walk one tuple expression.
    ///
    /// Example:
    /// ```ds
    /// (a, label: b)
    /// ```
    fn walk_tuple_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        elements: &[dir::LocalNodeId<dir::Argument>],
    ) -> CompilerResult<()> {
        let mut type_elements = Vec::with_capacity(elements.len());

        for element in elements {
            self.walk_argument(*element, self.tree.get(*element))?;

            // copy the element shape out of the tree
            let (label, value, is_rest) = match self.tree.get(*element) {
                dir::Argument::Positional { value } => (None, Some(*value), false),
                dir::Argument::Named { value, .. } => (None, Some(*value), false),
                dir::Argument::Labeled { label, value } => (Some(*label), Some(*value), false),
                dir::Argument::Spread { value, .. } => (None, Some(*value), true),
                dir::Argument::Error => (None, None, false),
            };
            let Some(value) = value else {
                continue;
            };

            type_elements.push(dir::TypeElement {
                label,
                ty: self.node_type(value)?,
                is_optional: false,
                is_readonly: false,
                is_rest,
            });
        }

        let tuple = self.push_type(
            dir::Type::Tuple(dir::TupleType {
                form: dir::TupleForm::Tuple,
                elements: type_elements,
            }),
            id.into_any(),
        )?;
        self.bind_node_type(id, tuple)?;

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
        id: dir::LocalNodeId<dir::Expression>,
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

        // write the fixed predicate result
        if operator == dir::BinaryOperator::In {
            let boolean = self.push_type(
                dir::Type::Primitive(dir::PrimitiveType::Boolean),
                id.into_any(),
            )?;
            self.bind_node_type(id, boolean)?;
            self.queue_selection(id.into_global_any(self.module))?;

            return Ok(());
        }

        // queue selection for the binary operator
        self.queue_selection(id.into_global_any(self.module))?;

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
        id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::AssignPattern>,
        operator: dir::AssignOperator,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        // choose the place use before walking either side
        let access = match operator {
            dir::AssignOperator::Assign => PlaceUse::Write,
            _ => PlaceUse::Update,
        };

        // queue arithmetic compound assignments as operator writes
        if operator.binary_operator().is_some()
            && let dir::AssignPattern::Expression { value: target } = self.tree.get(left)
        {
            let target = *target;

            return self.walk_compound_assignment(id, target, access);
        }

        // simple place assignments contextualize the assigned value
        if operator == dir::AssignOperator::Assign
            && let dir::AssignPattern::Expression { value: target } = self.tree.get(left)
        {
            let target = *target;
            let place = self.walk_assignment_place(target, access)?;
            if let Some(place) = place {
                let target_type = self.node_type(target)?;
                let value =
                    self.walk_expression_expected(right, self.tree.get(right), target_type)?;

                self.bind_node_type(id, value)?;
                self.push_write_obligations(place, target_type);
                self.mark_place_assigned(place);

                let resolution =
                    dir::AssignPatternResolution::Place(dir::AssignPatternPlaceResolution {
                        target: target.into_global_any(self.module),
                    });
                self.record_assign_pattern(left, resolution)?;
            } else {
                self.walk_expression(right, self.tree.get(right))?;
                let value = self.node_type(right)?;
                self.bind_node_type(id, value)?;
            };

            self.clear_mutated_expression_narrowings(target);

            return Ok(());
        }

        // walk the assigned value before matching complex assignment patterns
        self.walk_expression(right, self.tree.get(right))?;
        let value = self.node_type(right)?;
        let value_node = right.into_global_any(self.module);

        // assignment expressions evaluate to the assigned value
        self.bind_node_type(id, value)?;

        // flow the value into the assignment target
        self.walk_assign_pattern(left, value, value_node, access)?;

        Ok(())
    }

    /// Walk one arithmetic compound assignment target.
    /// The assignment node carries the operator result, which must
    /// write back into the place.
    ///
    /// Example:
    /// ```ds
    /// total += value
    /// ```
    fn walk_compound_assignment(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        target: dir::LocalNodeId<dir::Expression>,
        access: PlaceUse,
    ) -> CompilerResult<()> {
        if let Some(place) = self.walk_assignment_place(target, access)? {
            // the operator result writes back through the place
            self.queue_selection(id.into_global_any(self.module))?;
            let value = self.node_type(id)?;
            let target_type = self.node_type(target)?;
            let origin = Origin::Node(id.into_global_any(self.module));
            self.push_relation(
                origin,
                ConstraintRole::Value,
                Relation::Assignable,
                value,
                target_type,
            );

            self.push_write_obligations(place, target_type);
            self.mark_place_assigned(place);
        }

        // assignments invalidate narrowings under the target
        self.clear_mutated_expression_narrowings(target);

        Ok(())
    }

    /// Return one const-asserted value type.
    /// Array and tuple literals freeze into readonly tuples of their
    /// per-element asserted types, and object literals freeze their
    /// fields recursively.
    fn const_asserted_type(
        &mut self,
        node: dir::LocalNodeId<dir::Expression>,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // only positional literal collections freeze into tuples
        let (form, arguments) = match self.tree.get(node) {
            dir::Expression::Parenthesized { expression } => {
                let expression = *expression;
                let expression_type = self.node_type(expression)?;

                return self.const_asserted_type(expression, expression_type);
            }
            dir::Expression::ObjectExpression { properties } => {
                let properties = properties.iter().copied().collect::<SmallVec<[_; 4]>>();

                return self.const_asserted_object_type(node, &properties, ty);
            }
            dir::Expression::ArrayExpression { elements } => {
                (dir::TupleForm::Array, elements.clone())
            }
            dir::Expression::TupleExpression { elements } => {
                (dir::TupleForm::Tuple, elements.clone())
            }
            _ => return Ok(ty),
        };
        let mut values = SmallVec::<[_; 8]>::new();
        for argument in arguments {
            match self.tree.get(argument) {
                dir::Argument::Positional { value }
                | dir::Argument::Named { value, .. }
                | dir::Argument::Labeled { value, .. } => values.push(*value),
                // spread elements keep the plain collection type
                dir::Argument::Spread { .. } | dir::Argument::Error => return Ok(ty),
            }
        }

        // freeze the elements into one readonly tuple
        let mut elements = Vec::with_capacity(values.len());
        for value in values {
            let element = self.node_type(value)?;
            let element = self.const_asserted_type(value, element)?;
            elements.push(dir::TypeElement {
                label: None,
                ty: element,
                is_optional: false,
                is_readonly: false,
                is_rest: false,
            });
        }
        let tuple = self.push_type(
            dir::Type::Tuple(dir::TupleType { form, elements }),
            node.into_any(),
        )?;

        self.push_type(
            dir::Type::Form(dir::FormType {
                form: dir::Form::Readonly,
                value: tuple,
            }),
            node.into_any(),
        )
    }

    /// Return the const-asserted form of one object literal.
    fn const_asserted_object_type(
        &mut self,
        node: dir::LocalNodeId<dir::Expression>,
        properties: &[dir::LocalNodeId<dir::Property>],
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let mut fields = Vec::with_capacity(properties.len());
        for property in properties {
            match self.tree.get(*property) {
                dir::Property::Field { key, value, .. } => {
                    let Some(key) = key.direct_static_key() else {
                        return Ok(ty);
                    };
                    let value = *value;
                    let value_type = self.node_type(value)?;
                    let value_type = self.const_asserted_type(value, value_type)?;
                    fields.push(dir::TypeField {
                        key,
                        ty: value_type,
                        is_optional: false,
                        is_readonly: true,
                    });
                }
                dir::Property::Method { key, .. } => {
                    let Some(key) = key.and_then(dir::Key::direct_static_key) else {
                        return Ok(ty);
                    };
                    let Some(symbol) = self
                        .check
                        .module(self.module)
                        .declaration_symbol(property.into_any())
                    else {
                        return Ok(ty);
                    };
                    let method = self.symbol_type(symbol)?;
                    fields.push(dir::TypeField {
                        key,
                        ty: method,
                        is_optional: false,
                        is_readonly: true,
                    });
                }
                dir::Property::Spread { .. } | dir::Property::Error => return Ok(ty),
            }
        }

        let shape = self.push_type(
            dir::Type::Shape(dir::ShapeType {
                fields,
                call_signatures: Vec::new(),
                construct_signatures: Vec::new(),
                index_signatures: Vec::new(),
            }),
            node.into_any(),
        )?;

        self.push_type(
            dir::Type::Form(dir::FormType {
                form: dir::Form::Managed,
                value: shape,
            }),
            node.into_any(),
        )
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
        value: dir::GlobalTypeId,
        value_node: dir::GlobalNodeIdAny,
        access: PlaceUse,
    ) -> CompilerResult<()> {
        match self.tree.get(id) {
            // x = value, obj.x = value
            dir::AssignPattern::Expression { value: target } => {
                let target = *target;
                if let Some(place) = self.walk_assignment_place(target, access)? {
                    // value compatibility is separate from place mutability
                    let target_type = self.node_type(target)?;
                    let origin = Origin::Node(value_node);
                    self.push_relation(
                        origin,
                        ConstraintRole::Value,
                        Relation::Assignable,
                        value,
                        target_type,
                    );
                    self.push_write_obligations(place, target_type);
                    self.mark_place_assigned(place);
                }

                // assignments invalidate narrowings under the target
                self.clear_mutated_expression_narrowings(target);

                let resolution =
                    dir::AssignPatternResolution::Place(dir::AssignPatternPlaceResolution {
                        target: target.into_global_any(self.module),
                    });
                self.record_assign_pattern(id, resolution)?;
            }
            // x = default
            dir::AssignPattern::Assign {
                pattern,
                value: default,
            } => {
                let (pattern, default) = (*pattern, *default);
                self.walk_expression(default, self.tree.get(default))?;
                self.walk_assign_pattern(pattern, value, value_node, access)?;

                let resolution =
                    dir::AssignPatternResolution::Default(dir::AssignPatternDefaultResolution {
                        pattern: pattern.into_global_any(self.module),
                        value: default.into_global_any(self.module),
                    });
                self.record_assign_pattern(id, resolution)?;
            }
            // [a, , ...rest] = values
            dir::AssignPattern::Sequence { fields } => {
                for field in fields.clone() {
                    self.walk_assign_pattern_field(field, value, value_node, access)?;
                }

                let resolution = self.sequence_assign_pattern_resolution(fields);
                self.record_assign_pattern(id, resolution)?;
            }
            // { x, y: z } = point
            dir::AssignPattern::Object { fields } => {
                for field in fields.clone() {
                    self.walk_assign_pattern_field(field, value, value_node, access)?;
                }

                let resolution = self.object_assign_pattern_resolution(fields);
                self.record_assign_pattern(id, resolution)?;
            }
        }

        Ok(())
    }

    /// Record one assignment pattern decision.
    fn record_assign_pattern(
        &mut self,
        id: dir::LocalNodeId<dir::AssignPattern>,
        resolution: dir::AssignPatternResolution,
    ) -> CompilerResult<()> {
        self.check.record_decision(
            id.into_global_any(self.module),
            Decision::AssignPattern(resolution),
        )
    }

    /// Return the structural resolution for one sequence assignment target.
    fn sequence_assign_pattern_resolution(
        &self,
        fields: &[dir::LocalNodeId<dir::AssignPatternField>],
    ) -> dir::AssignPatternResolution {
        let mut projected = Vec::with_capacity(fields.len());
        let mut rest = None;
        let mut position = 0usize;
        for field in fields {
            match self.tree.get(*field) {
                dir::AssignPatternField::Positional { pattern } => {
                    projected.push(dir::AssignPatternFieldResolution {
                        source: field.into_global_any(self.module),
                        target: dir::AssignPatternFieldTarget::Index(position),
                        pattern: Some(pattern.into_global_any(self.module)),
                    });
                    position += 1;
                }
                dir::AssignPatternField::Spread { pattern } => {
                    rest = Some(dir::AssignPatternRestResolution {
                        source: field.into_global_any(self.module),
                        pattern: pattern.map(|pattern| pattern.into_global_any(self.module)),
                    });
                }
                dir::AssignPatternField::Elision => {
                    position += 1;
                }
                dir::AssignPatternField::Named { .. }
                | dir::AssignPatternField::Computed { .. } => {}
            }
        }

        dir::AssignPatternResolution::Sequence(dir::AssignPatternSequenceResolution {
            fields: projected,
            rest,
        })
    }

    /// Return the structural resolution for one object assignment target.
    fn object_assign_pattern_resolution(
        &self,
        fields: &[dir::LocalNodeId<dir::AssignPatternField>],
    ) -> dir::AssignPatternResolution {
        let mut projected = Vec::with_capacity(fields.len());
        let mut rest = None;
        for field in fields {
            match self.tree.get(*field) {
                dir::AssignPatternField::Named { name, pattern, .. } => {
                    projected.push(dir::AssignPatternFieldResolution {
                        source: field.into_global_any(self.module),
                        target: dir::AssignPatternFieldTarget::Key(name.static_key()),
                        pattern: pattern.map(|pattern| pattern.into_global_any(self.module)),
                    });
                }
                dir::AssignPatternField::Computed { key, pattern } => {
                    let Some(key) = self.tree.get(*key).static_key() else {
                        continue;
                    };
                    projected.push(dir::AssignPatternFieldResolution {
                        source: field.into_global_any(self.module),
                        target: dir::AssignPatternFieldTarget::Key(key),
                        pattern: Some(pattern.into_global_any(self.module)),
                    });
                }
                dir::AssignPatternField::Spread { pattern } => {
                    rest = Some(dir::AssignPatternRestResolution {
                        source: field.into_global_any(self.module),
                        pattern: pattern.map(|pattern| pattern.into_global_any(self.module)),
                    });
                }
                dir::AssignPatternField::Positional { .. } | dir::AssignPatternField::Elision => {}
            }
        }

        dir::AssignPatternResolution::Object(dir::AssignPatternObjectResolution {
            fields: projected,
            rest,
        })
    }

    /// Walk one assignment pattern field against the destructured value.
    fn walk_assign_pattern_field(
        &mut self,
        id: dir::LocalNodeId<dir::AssignPatternField>,
        value: dir::GlobalTypeId,
        value_node: dir::GlobalNodeIdAny,
        access: PlaceUse,
    ) -> CompilerResult<()> {
        match self.tree.get(id) {
            dir::AssignPatternField::Named {
                pattern: Some(pattern),
                ..
            } => {
                let pattern = *pattern;
                self.walk_assign_pattern(pattern, value, value_node, access)?;
            }
            dir::AssignPatternField::Computed { pattern, .. }
            | dir::AssignPatternField::Positional { pattern } => {
                let pattern = *pattern;
                self.walk_assign_pattern(pattern, value, value_node, access)?;
            }
            dir::AssignPatternField::Spread {
                pattern: Some(pattern),
            } => {
                let pattern = *pattern;
                self.walk_assign_pattern(pattern, value, value_node, access)?;
            }
            dir::AssignPatternField::Named { pattern: None, .. }
            | dir::AssignPatternField::Spread { pattern: None }
            | dir::AssignPatternField::Elision => {}
        }

        Ok(())
    }

    /// Expect one runtime condition to be boolean.
    ///
    /// Example:
    /// ```ds
    /// if condition { }
    /// ```
    fn expect_boolean_condition(
        &mut self,
        condition: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        let origin = Origin::Node(condition.into_global_any(self.module));
        let ty = self.node_type(condition)?;
        let boolean = self.push_type(
            dir::Type::Primitive(dir::PrimitiveType::Boolean),
            condition.into_any(),
        )?;
        self.push_relation(
            origin,
            ConstraintRole::Condition,
            Relation::Assignable,
            ty,
            boolean,
        );

        Ok(())
    }

    /// Return the awaited result of one promise-typed value.
    fn await_result(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        awaited: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let origin = Origin::Node(id.into_global_any(self.module));
        let result = self.open_type(id.into_any())?;
        let promised =
            self.language_type_reference(id.into_any(), dir::LanguageItem::Promise, vec![result])?;
        let value = self.node_type(awaited)?;
        self.relate_type(origin, Relation::Assignable, value, promised);

        Ok(result)
    }

    /// Return the try success projection of one tried value.
    fn try_output(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.push_type(
            dir::Type::Operation(dir::TypeOperation::TryOutput { value }),
            id.into_any(),
        )
    }

    /// Spell one borrow's provenance: the lifetime at the last borrow
    /// crossed along the place chain, or the root place's own lifetime.
    /// The chain reduces lazily as the composite's receiver types close.
    fn borrow_provenance(
        &mut self,
        node: dir::LocalNodeId<dir::Expression>,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // collect candidate borrow carriers, deepest first
        let mut steps = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        steps.push(self.node_type(right)?);
        let mut current = right;
        let root = loop {
            match self.tree.get(current) {
                dir::Expression::Member { left, .. }
                | dir::Expression::PrivateMember { left, .. }
                | dir::Expression::Index { left, .. } => {
                    let left = *left;
                    steps.push(self.node_type(left)?);
                    current = left;
                }
                dir::Expression::Unary {
                    operator: dir::UnaryOperator::Dereference,
                    right,
                    ..
                } => {
                    let right = *right;
                    steps.push(self.node_type(right)?);
                    current = right;
                }
                dir::Expression::Identifier { .. } => {
                    break self.borrow_root_lifetime(node, current)?;
                }
                _ => break self.temporary_lifetime(node)?,
            }
        };

        // fold the chain so the deepest carrier answers first
        let mut provenance = root;
        for step in steps.into_iter().rev() {
            provenance = self.language_type_reference(
                node.into_any(),
                dir::LanguageItem::LifetimeOr,
                vec![step, provenance],
            )?;
        }

        Ok(provenance)
    }

    /// Push the obligations for one successful place write.
    fn push_write_obligations(&mut self, place: Place, ty: dir::GlobalTypeId) {
        let condition = self.active_static_guard();
        self.check
            .push_obligation(Obligation::WritablePlace(WritablePlaceObligation {
                condition,
                place,
                ty,
            }));
    }

    /// Return the lifetime of one identifier root place.
    fn borrow_root_lifetime(
        &mut self,
        node: dir::LocalNodeId<dir::Expression>,
        root: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let global = root.into_global_any(self.module);
        let Some(symbol) = self.single_resolved_symbol(global) else {
            return self.temporary_lifetime(node);
        };

        // module-level places live in static storage
        let module = self.check.module(symbol.module_id);
        let is_static = module
            .binding_table()
            .get_symbol_maybe(symbol.local_id)
            .is_some_and(|declared| declared.scope.id == module.bound.namespace_scope);
        let lifetime = if is_static {
            dir::Lifetime::Static
        } else {
            dir::Lifetime::Symbol(symbol)
        };

        self.push_type(
            dir::Type::Memory(dir::MemoryLiteral::Lifetime(lifetime)),
            node.into_any(),
        )
    }

    /// Return the lifetime of one borrowed temporary.
    /// Lowering extends declarator-initializer temporaries to their
    /// bindings; every other temporary lives for the enclosing frame.
    fn temporary_lifetime(
        &mut self,
        node: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.push_type(
            dir::Type::Memory(dir::MemoryLiteral::Lifetime(dir::Lifetime::Frame)),
            node.into_any(),
        )
    }
}
