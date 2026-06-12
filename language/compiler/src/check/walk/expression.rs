use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    ConditionBranch, ConstraintCause, Decision, FlowBranch, FlowCheckpoint, GuardOutcome,
    MatchCase, MatchObligation, Obligation, Origin, Place, PlaceObligation, PlaceTarget, Relation,
    WalkState,
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
                        self.declare_node_type(id, ty)?;
                    }
                } else {
                    let void = self.push_type(dir::Type::Void, id.into_any())?;
                    self.declare_node_type(id, void)?;
                }
            }
            // { ... }
            dir::Expression::Block(block) => {
                let block = *block;
                self.walk_block(block, self.tree.get(block))?;
                let ty = self.node_type(block)?;
                self.declare_node_type(id, ty)?;
            }
            // label: body
            dir::Expression::Label { label, body } => {
                let (label, body) = (*label, *body);
                self.walk_label_expression(id, label, body)?;
            }
            // import { item } from "module"
            dir::Expression::Import { items, .. } => {
                if let Some(items) = items.as_deref() {
                    for item in items {
                        self.walk_dependency_item(*item, self.tree.get(*item))?;
                    }
                }
                let void = self.push_type(dir::Type::Void, id.into_any())?;
                self.declare_node_type(id, void)?;
            }
            // export { item } from "module"
            dir::Expression::Export { items, .. } => {
                for item in items {
                    self.walk_dependency_item(*item, self.tree.get(*item))?;
                }
                let void = self.push_type(dir::Type::Void, id.into_any())?;
                self.declare_node_type(id, void)?;
            }
            // let x = value
            dir::Expression::Let { declarators, .. } => {
                for declarator in declarators {
                    self.walk_declarator(*declarator, self.tree.get(*declarator))?;
                    self.mark_declarator_assigned(self.tree.get(*declarator));
                }
                let void = self.push_type(dir::Type::Void, id.into_any())?;
                self.declare_node_type(id, void)?;
            }
            // using x = value
            dir::Expression::Using { declarators, .. } => {
                for declarator in declarators {
                    self.walk_declarator(*declarator, self.tree.get(*declarator))?;
                    self.mark_declarator_assigned(self.tree.get(*declarator));
                }
                let void = self.push_type(dir::Type::Void, id.into_any())?;
                self.declare_node_type(id, void)?;
            }
            // let pattern = value else { fallback }
            dir::Expression::LetElse {
                declarator,
                else_branch,
                ..
            } => {
                let (declarator, else_branch) = (*declarator, *else_branch);
                self.walk_let_else_expression(id, declarator, else_branch)?;
            }
            // if condition { then } else { otherwise }
            dir::Expression::If {
                condition,
                then_expression,
                else_expression,
                ..
            } => {
                let (then_expression, else_expression) = (*then_expression, *else_expression);
                self.walk_if_expression(id, condition, then_expression, else_expression)?;
            }
            // while condition { body }
            dir::Expression::While {
                condition, body, ..
            } => {
                let (condition, body) = (*condition, *body);
                self.walk_while_expression(id, None, condition, body)?;
            }
            // for item of iterator { body }
            dir::Expression::ForEach {
                operator,
                binding,
                iterator,
                body,
                ..
            } => {
                let (operator, iterator, body) = (*operator, *iterator, *body);
                self.walk_for_each_expression(id, None, operator, binding, iterator, body)?;
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
                let body = *body;
                self.walk_loop_expression(id, None, body)?;
            }
            // try body catch error finally cleanup
            dir::Expression::Try {
                body,
                catch,
                finally,
            } => {
                let (body, catch, finally) = (*body, *catch, *finally);
                self.walk_try_expression(id, body, catch, finally)?;
            }
            // match value { case pattern => body }
            dir::Expression::Match { value, cases, .. } => {
                let value = *value;
                let cases = cases.iter().copied().collect::<SmallVec<[_; 4]>>();
                self.walk_match_expression(id, value, &cases)?;
            }
            // break value
            dir::Expression::Break { label, value } => {
                let (label, value) = (*label, *value);
                if let Some(value) = value {
                    self.walk_expression(value, self.tree.get(value))?;
                }
                let value = value.map(|value| self.node_type(value)).transpose()?;
                self.break_to_control_target(id.into_any(), label, value)?;
                let never = self.push_type(dir::Type::Never, id.into_any())?;
                self.declare_node_type(id, never)?;
            }
            // continue
            dir::Expression::Continue { label } => {
                let label = *label;
                self.continue_to_control_target(id.into_any(), label);
                let never = self.push_type(dir::Type::Never, id.into_any())?;
                self.declare_node_type(id, never)?;
            }
            // await value
            dir::Expression::Await {
                expression: awaited,
            } => {
                let awaited = *awaited;
                self.walk_expression(awaited, self.tree.get(awaited))?;
                self.validate_await_context(id.into_any());
                let result = self.await_result(id, awaited)?;
                self.declare_node_type(id, result)?;
            }
            // throw value
            dir::Expression::Throw { value } => {
                let value = *value;
                self.walk_expression(value, self.tree.get(value))?;
                let never = self.push_type(dir::Type::Never, id.into_any())?;
                self.declare_node_type(id, never)?;
            }
            // return value
            dir::Expression::Return { value } => {
                let value = *value;
                if let Some(value) = value {
                    self.walk_expression(value, self.tree.get(value))?;
                    let ty = self.node_type(value)?;
                    self.constrain_return_value(id.into_any(), ty);
                } else {
                    self.constrain_void_return(id.into_any())?;
                }
                let never = self.push_type(dir::Type::Never, id.into_any())?;
                self.declare_node_type(id, never)?;
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
                self.constrain_yield_value(
                    id.into_any(),
                    cardinality,
                    value_type,
                    delegate_return,
                )?;

                // yield evaluates to the resumed value
                match self.current_resume_target() {
                    Some(resumed) => {
                        self.declare_node_type(id, resumed)?;
                    }
                    None => {
                        let void = self.push_type(dir::Type::Void, id.into_any())?;
                        self.declare_node_type(id, void)?;
                    }
                }
            }
            // value
            dir::Expression::Identifier { name } => {
                let name = *name;
                self.walk_identifier_expression(id, name)?;
            }
            // this
            dir::Expression::This => {
                let receiver = self.resolve_this_receiver(id.into_global_any(self.module))?;
                match receiver {
                    Some(receiver) => {
                        self.declare_node_type(id, receiver.ty)?;
                    }
                    None => {
                        self.check.report_invalid_control_flow(
                            self.module,
                            id.into_any(),
                            "this has no receiver",
                        );
                        let error = self.push_type(dir::Type::Error, id.into_any())?;
                        self.declare_node_type(id, error)?;
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
                self.declare_node_type(id, ty)?;
            }
            // super
            dir::Expression::Super => {
                let receiver = self.resolve_this_receiver(id.into_global_any(self.module))?;
                match receiver.and_then(|receiver| receiver.super_ty) {
                    Some(super_ty) => {
                        self.declare_node_type(id, super_ty)?;
                    }
                    None => {
                        self.check.report_invalid_control_flow(
                            self.module,
                            id.into_any(),
                            "super has no superclass",
                        );
                        let error = self.push_type(dir::Type::Error, id.into_any())?;
                        self.declare_node_type(id, error)?;
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
                self.declare_node_type(id, meta)?;
            }
            // #name, debugger, missing, stub, damaged syntax
            dir::Expression::PrivateIdentifier { .. }
            | dir::Expression::Debugger
            | dir::Expression::Missing
            | dir::Expression::Stub
            | dir::Expression::Error => {}
            // namespace.value<T>
            dir::Expression::QualifiedReference {
                path,
                generic_arguments,
            } => {
                let arguments = generic_arguments
                    .iter()
                    .copied()
                    .collect::<SmallVec<[_; 2]>>();
                self.walk_qualified_reference_expression(id, path, &arguments)?;
            }
            // start..end
            dir::Expression::RangeExpression {
                start,
                end,
                end_kind,
            } => {
                let (start, end, end_kind) = (*start, *end, *end_kind);
                self.walk_range_expression(id, start, end, end_kind)?;
            }
            // `text ${value}`
            dir::Expression::TemplateExpression { value } => {
                self.walk_template_literal(value)?;
                let string = self.push_type(
                    dir::Type::Primitive(dir::PrimitiveType::String),
                    id.into_any(),
                )?;
                self.declare_node_type(id, string)?;
            }
            // tag<T>`text ${value}`
            dir::Expression::TaggedTemplateExpression { tag, value, .. } => {
                let tag = *tag;
                self.walk_expression(tag, self.tree.get(tag))?;
                self.walk_template_literal(value)?;

                // tagged template calls resolve at selection
                self.node_type(id)?;
                self.queue_select(id.into_global_any(self.module));
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
                let count = self.lower_static_predicate(length)?;
                let array = self.push_type(
                    dir::Type::FixedArray(dir::FixedArrayType { element, count }),
                    id.into_any(),
                )?;
                self.declare_node_type(id, array)?;
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
                        self.declare_node_type(id, ty)?;
                    }
                    None => {
                        let void = self.push_type(dir::Type::Void, id.into_any())?;
                        self.declare_node_type(id, void)?;
                    }
                }
            }
            // { key: value }
            dir::Expression::ObjectExpression { properties } => {
                let properties = properties.iter().copied().collect::<SmallVec<[_; 4]>>();
                let (fields, has_spread) = self.walk_literal_properties(&properties)?;

                // spread surfaces merge at selection once they close
                if has_spread {
                    self.node_type(id)?;
                    self.queue_select(id.into_global_any(self.module));
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
                    self.declare_node_type(id, managed)?;
                }
            }
            // Type { key: value }
            dir::Expression::StructExpression { ty, properties } => {
                let ty = *ty;
                let properties = properties.iter().copied().collect::<SmallVec<[_; 4]>>();
                let target = self.walk_type_expression(ty)?;
                let (fields, has_spread) = self.walk_literal_properties(&properties)?;
                self.declare_node_type(id, target)?;

                // spread surfaces merge at selection once they close
                if has_spread {
                    self.queue_select(id.into_global_any(self.module));
                }
                // the written fields must fill the struct surface
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
                let left = *left;
                if let Some(left) = left {
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

                // tree construction resolves at selection
                self.node_type(id)?;
                self.queue_select(id.into_global_any(self.module));
            }
            // (value)
            dir::Expression::Parenthesized { expression: child } => {
                let child = *child;
                self.walk_expression(child, self.tree.get(child))?;
                let ty = self.node_type(child)?;
                self.declare_node_type(id, ty)?;
            }
            // type T
            dir::Expression::Type { value } => {
                let value = *value;
                let ty = self.walk_type_expression(value)?;
                self.declare_node_type(id, ty)?;
            }
            // comptime value
            dir::Expression::Comptime { body } => {
                let body = *body;

                // check comptime bodies in isolated flow
                let before_body = self.fork_flow();
                self.walk_expression(body, self.tree.get(body))?;
                self.restore_flow(before_body);

                let ty = self.node_type(body)?;
                self.declare_node_type(id, ty)?;
            }
            // value as T
            dir::Expression::As {
                expression: child,
                target_type,
            } => {
                let (child, target_type) = (*child, *target_type);
                self.walk_expression(child, self.tree.get(child))?;

                // value as const freezes the precise written type
                if matches!(self.tree.get(target_type), dir::TypeExpression::Const) {
                    let ty = self.node_type(child)?;
                    let asserted = self.const_asserted_type(child, ty)?;
                    self.declare_node_type(id, asserted)?;
                } else {
                    let target = self.walk_type_expression(target_type)?;
                    let value = self.node_type(child)?;
                    let origin = Origin::Node(id.into_global_any(self.module));
                    self.relate_type(origin, Relation::Castable, value, target);
                    self.declare_node_type(id, target)?;
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
                let origin = Origin::Node(id.into_global_any(self.module));
                self.relate_type(origin, Relation::Satisfies, value, target);

                // satisfies keeps the value's own type
                self.declare_node_type(id, value)?;
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
                self.declare_node_type(id, boolean)?;
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
                self.declare_node_type(id, boolean)?;
            }
            // !value, -value
            dir::Expression::Unary { right, .. } => {
                let right = *right;
                self.walk_expression(right, self.tree.get(right))?;

                // operator meaning resolves at selection
                self.node_type(id)?;
                self.queue_select(id.into_global_any(self.module));
            }
            // ^value
            dir::Expression::MoveOf { right, .. } => {
                let right = *right;
                self.walk_expression(right, self.tree.get(right))?;
                let ty = self.node_type(right)?;
                self.declare_node_type(id, ty)?;
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
                self.declare_node_type(id, borrowed)?;
            }
            // value.member, value.#member
            dir::Expression::Member { left, .. } | dir::Expression::PrivateMember { left, .. } => {
                let left = *left;
                self.walk_expression(left, self.tree.get(left))?;

                // member meaning resolves at selection
                self.node_type(id)?;
                self.queue_select(id.into_global_any(self.module));
            }
            // value[index]
            dir::Expression::Index { left, index, .. } => {
                let (left, index) = (*left, *index);
                self.walk_expression(left, self.tree.get(left))?;
                if let Some(index) = index {
                    self.walk_expression(index, self.tree.get(index))?;
                }

                // index meaning resolves at selection
                self.node_type(id)?;
                self.queue_select(id.into_global_any(self.module));
            }
            // value<T>
            dir::Expression::Instantiation {
                left,
                generic_arguments,
            } => {
                let left = *left;
                let arguments = generic_arguments
                    .iter()
                    .copied()
                    .collect::<SmallVec<[_; 2]>>();
                self.walk_instantiation_expression(id, left, &arguments)?;
            }
            // callee<T>(argument)
            dir::Expression::Call {
                left, arguments, ..
            } => {
                let left = *left;
                let arguments = arguments.iter().copied().collect::<SmallVec<[_; 4]>>();
                self.walk_expression(left, self.tree.get(left))?;
                for argument in &arguments {
                    self.walk_argument(*argument, self.tree.get(*argument))?;
                }

                // call meaning resolves at selection
                self.node_type(id)?;
                self.queue_select(id.into_global_any(self.module));
            }
            // new Type<T>(argument)
            dir::Expression::New { ty, arguments } => {
                let ty = *ty;
                let arguments = arguments.iter().copied().collect::<SmallVec<[_; 4]>>();
                self.walk_type_expression(ty)?;
                for argument in &arguments {
                    self.walk_argument(*argument, self.tree.get(*argument))?;
                }

                // construction resolves at selection
                self.node_type(id)?;
                self.queue_select(id.into_global_any(self.module));
            }
            // new? Type<T>(argument)
            dir::Expression::NewMaybe { ty, arguments } => {
                let ty = *ty;
                let arguments = arguments.iter().copied().collect::<SmallVec<[_; 4]>>();
                self.walk_type_expression(ty)?;
                for argument in &arguments {
                    self.walk_argument(*argument, self.tree.get(*argument))?;
                }

                // construction resolves at selection
                let constructed = self.node_type(id)?;
                self.queue_select(id.into_global_any(self.module));

                // fallible allocation propagates an allocation error
                let allocation_error = self.language_type_reference(
                    id.into_any(),
                    dir::LanguageItem::AllocationError,
                    Vec::new(),
                )?;
                let carrier = self.language_type_reference(
                    id.into_any(),
                    dir::LanguageItem::Result,
                    vec![constructed, allocation_error],
                )?;
                self.propagate_try(id.into_any(), carrier)?;
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
                self.declare_node_type(id, output)?;
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
                self.declare_node_type(id, output)?;
            }
            // value?
            dir::Expression::Maybe { left, .. } => {
                let left = *left;
                self.walk_expression(left, self.tree.get(left))?;
                let value = self.node_type(left)?;
                self.propagate_try(id.into_any(), value)?;
                let output = self.try_output(id, value)?;
                self.declare_node_type(id, output)?;
            }
            // value!
            dir::Expression::Must { left, .. } => {
                let left = *left;
                self.walk_expression(left, self.tree.get(left))?;
                let value = self.node_type(left)?;
                let output = self.try_output(id, value)?;
                self.declare_node_type(id, output)?;
            }
            // left + right
            dir::Expression::Binary {
                left,
                operator,
                right,
            } => {
                let (left, operator, right) = (*left, *operator, *right);
                self.walk_binary_expression(id, left, operator, right)?;
            }
            // target = value
            dir::Expression::Assign { left, right, .. } => {
                let (left, right) = (*left, *right);
                self.walk_assign_expression(id, left, right)?;
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
                let result = self.node_type(body)?;
                self.enter_control_target(Some(label), false, body, result);
                self.walk_expression(body, self.tree.get(body))?;
                let fallthrough = self.node_type(body)?;
                self.leave_control_target(Some(fallthrough))?;
            }
        }

        let ty = self.node_type(body)?;
        self.declare_node_type(id, ty)?;

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
        declarator: dir::LocalNodeId<dir::Declarator>,
        else_branch: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        self.walk_declarator(declarator, self.tree.get(declarator))?;

        // walk the diverging fallback in isolated flow
        let before_else = self.fork_flow();
        self.walk_expression(else_branch, self.tree.get(else_branch))?;
        self.restore_flow(before_else);

        // the fallback must leave the binding scope
        if self.expression_can_complete_normally(else_branch) {
            self.check.report_invalid_control_flow(
                self.module,
                else_branch.into_any(),
                "let else fallback must diverge",
            );
        }

        // continue with the matched bindings assigned
        self.mark_declarator_assigned(self.tree.get(declarator));
        self.narrow_declarator_match(declarator)?;

        let void = self.push_type(dir::Type::Void, id.into_any())?;
        self.declare_node_type(id, void)?;

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
        condition: &dir::IfCondition,
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
            let union = self.push_type(
                dir::Type::Union(dir::UnionType {
                    elements: vec![then_type, else_type],
                }),
                id.into_any(),
            )?;
            self.declare_node_type(id, union)?;

            // collect false completion
            if else_can_complete {
                branches.push(self.collect_flow_branch(before));
            }
        }
        // no false branch
        else {
            let void = self.push_type(dir::Type::Void, id.into_any())?;
            let union = self.push_type(
                dir::Type::Union(dir::UnionType {
                    elements: vec![then_type, void],
                }),
                id.into_any(),
            )?;
            self.declare_node_type(id, union)?;

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
    fn walk_if_condition(&mut self, condition: &dir::IfCondition) -> CompilerResult<()> {
        match condition {
            // if condition
            dir::IfCondition::Expression { condition } => {
                let condition = *condition;
                self.walk_expression(condition, self.tree.get(condition))?;
                self.expect_boolean_condition(condition)?;
            }
            // if let pattern = value
            dir::IfCondition::Let { declarator, .. } => {
                let declarator = *declarator;
                self.walk_declarator(declarator, self.tree.get(declarator))?;
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
        let result = self.node_type(id)?;
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
        let result = self.node_type(id)?;
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
            dir::ForEachOperator::In => self.push_type(
                dir::Type::Operation(dir::TypeOperation::KeyOf(dir::UnaryType {
                    target: iterator_type,
                })),
                id.into_any(),
            )?,
        };

        // flow the iterated value into the pattern holes
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
        let result = self.node_type(id)?;

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
        let result = self.node_type(id)?;
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

        // tie the expression value to its completing branches
        let body_type = self.node_type(body)?;
        match catch {
            Some(catch) => {
                let catch_body = self.tree.get(catch).body;
                let catch_type = self.node_type(catch_body)?;
                let union = self.push_type(
                    dir::Type::Union(dir::UnionType {
                        elements: vec![body_type, catch_type],
                    }),
                    id.into_any(),
                )?;
                self.declare_node_type(id, union)?;
            }
            None => {
                self.declare_node_type(id, body_type)?;
            }
        }

        // merge only branches that can continue normally
        let has_normal_flow = if let Some(catch) = catch {
            let catch_can_complete =
                self.expression_can_complete_normally(self.tree.get(catch).body);
            self.restore_flow(before);
            self.walk_catch(catch, catch_failure)?;
            let catch_flow = self.collect_flow_branch(before);

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

            // flow the caught value into the pattern holes
            if let Some(value) = expected.or(failure) {
                let origin = Origin::Node(pattern.into_global_any(self.module));
                let pattern_type = self.node_type(pattern)?;
                self.relate_type(origin, Relation::Assignable, value, pattern_type);
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
        let mut elements = Vec::new();
        let mut case_rows = Vec::new();
        let mut merged: Option<FlowBranch> = None;

        for (case, condition) in &active_cases {
            self.restore_flow(before);
            {
                let _guard = self.enter_static_guard(condition.clone());
                self.walk_match_case(
                    *case,
                    self.tree.get(*case),
                    Some((value_type, value_path.clone())),
                )?;
            }

            // collect the case body value
            let body = match self.tree.get(*case) {
                dir::MatchCase::Expression { body, .. } => *body,
                dir::MatchCase::Block { body, .. } => {
                    let body = *body;
                    elements.push(self.node_type(body)?);

                    // block bodies carry their own node types
                    let case_flow_done = ();
                    let _ = case_flow_done;
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
                    case_rows.extend(self.match_case_row(*case)?);

                    continue;
                }
            };
            elements.push(self.node_type(body)?);
            case_rows.extend(self.match_case_row(*case)?);

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
            .push_obligation(Obligation::Match(MatchObligation {
                source: id.into_global_any(self.module),
                condition,
                value: value_type,
                cases: case_rows,
            }));

        // the match evaluates to the union of its case values
        let union = self.push_type(dir::Type::Union(dir::UnionType { elements }), id.into_any())?;
        self.declare_node_type(id, union)?;

        Ok(())
    }

    /// Return the coverage row for one match case.
    fn match_case_row(
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
        self.declare_node_type(id, range)?;

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

        self.declare_node_type(id, array)?;

        Ok(())
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
        let mut rows = Vec::with_capacity(elements.len());

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

            rows.push(dir::TypeElement {
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
                elements: rows,
            }),
            id.into_any(),
        )?;
        self.declare_node_type(id, tuple)?;

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

        // operator meaning resolves at selection
        self.node_type(id)?;
        self.queue_select(id.into_global_any(self.module));

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
        right: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        // walk the assigned value first
        self.walk_expression(right, self.tree.get(right))?;
        let value = self.node_type(right)?;

        // flow the value into the assignment target
        let value_node = right.into_global_any(self.module);
        self.walk_assign_pattern(left, value, value_node)?;

        // assignments evaluate to their written value
        self.declare_node_type(id, value)?;

        Ok(())
    }

    /// Return one const-asserted value type.
    /// Array and tuple literals freeze into readonly tuples of their
    /// per-element asserted types; other values keep their written
    /// type, which preserve widening already keeps literal.
    fn const_asserted_type(
        &mut self,
        node: dir::LocalNodeId<dir::Expression>,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // only positional literal collections freeze into tuples
        let (form, arguments) = match self.tree.get(node) {
            dir::Expression::Parenthesized { expression } => {
                let expression = *expression;
                let inner = self.node_type(expression)?;

                return self.const_asserted_type(expression, inner);
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

    /// Return whether one written place infers its type from writes.
    /// Bindings declared without an annotation or initializer have no
    /// other type source, so their writes flow in as inference bounds.
    fn place_infers_from_writes(&self, place: &Place) -> bool {
        let PlaceTarget::Binding { symbol } = place.target else {
            return false;
        };
        if symbol.module_id != self.module {
            return false;
        }

        // climb from the bound pattern to its declarator
        let bindings = self.check.module(self.module).binding_table();
        let Some(declaration) = bindings.get_symbol(symbol.local_id).declaration else {
            return false;
        };
        let mut current = declaration.local_id.id;
        loop {
            let Some(parent) = self.tree.get_parent(current) else {
                return false;
            };
            match parent.ty {
                dir::NodeType::Pattern => current = parent.id,
                dir::NodeType::Declarator => {
                    let declarator = self
                        .tree
                        .get(dir::LocalNodeId::<dir::Declarator>::new(parent.id));

                    return declarator.ty.is_none() && declarator.value.is_none();
                }
                _ => return false,
            }
        }
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
    ) -> CompilerResult<()> {
        match self.tree.get(id) {
            // x = value, obj.x = value
            dir::AssignPattern::Expression { value: target } => {
                let target = *target;
                self.walk_assignment_target(target)?;

                if let Some(place) = self.assignment_place(target)? {
                    // writes check against settled places; only bindings
                    // with no declared or initialized type infer from them
                    let relation = if self.place_infers_from_writes(&place) {
                        Relation::Assignable
                    } else {
                        Relation::Writable
                    };
                    // anchor the check at the written value
                    self.relate_type(Origin::Node(value_node), relation, value, place.ty);

                    // the place must accept writes
                    let condition = self.active_static_guard();
                    self.check
                        .push_obligation(Obligation::Place(PlaceObligation { condition, place }));

                    self.mark_place_assigned(place);
                }

                // assignments invalidate narrowings under the target
                self.clear_mutated_expression_narrowings(target);
            }
            // x = default
            dir::AssignPattern::Assign {
                pattern,
                value: default,
            } => {
                let (pattern, default) = (*pattern, *default);
                self.walk_expression(default, self.tree.get(default))?;
                self.walk_assign_pattern(pattern, value, value_node)?;
            }
            // [a, , ...rest] = values, { x, y: z } = point
            dir::AssignPattern::Sequence { fields } | dir::AssignPattern::Object { fields } => {
                // destructured components resolve at selection
                // TODO(check): project sequence and object components onto
                // their assignment fields once assign selection lands.
                for field in fields.clone() {
                    self.walk_assign_pattern_field(field, value, value_node)?;
                }
            }
        }

        Ok(())
    }

    /// Walk one assignment pattern field against the destructured value.
    fn walk_assign_pattern_field(
        &mut self,
        id: dir::LocalNodeId<dir::AssignPatternField>,
        value: dir::GlobalTypeId,
        value_node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<()> {
        match self.tree.get(id) {
            dir::AssignPatternField::Named {
                pattern: Some(pattern),
                ..
            } => {
                let pattern = *pattern;
                self.walk_assign_pattern(pattern, value, value_node)?;
            }
            dir::AssignPatternField::Computed { pattern, .. }
            | dir::AssignPatternField::Positional { pattern } => {
                let pattern = *pattern;
                self.walk_assign_pattern(pattern, value, value_node)?;
            }
            dir::AssignPatternField::Spread {
                pattern: Some(pattern),
            } => {
                let pattern = *pattern;
                self.walk_assign_pattern(pattern, value, value_node)?;
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
            ConstraintCause::Condition,
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

    /// Return the lifetime of one identifier root place.
    fn borrow_root_lifetime(
        &mut self,
        node: dir::LocalNodeId<dir::Expression>,
        root: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // read the identifier's walked name decision
        let global = root.into_global_any(self.module);
        let symbol = match self.check.decisions.get(global) {
            Some(Decision::Name(resolution)) => match resolution.symbols() {
                [symbol] => Some(*symbol),
                _ => None,
            },
            _ => None,
        };
        let Some(symbol) = symbol else {
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
