use destack_dir as dir;
use dir::NodeVisitor as _;

use crate::check::{
    ArgumentTerm, AwaitTerm, CallCandidate, CallTerm, CheckModuleState, ConstraintOrigin,
    FlowBranch, FlowCheckpoint, FormTerm, IdentityTerm, ImportMetaTerm, IndexSetTerm, IndexTerm,
    InstanceCheckTerm, KeyMembershipTerm, MemberCallTerm, MemberTerm, NewTerm, OperatorTerm,
    OperatorTermKind, PatternRelation, RangeValueTerm, ShapeMemberTerm, SuperTerm,
    TaggedTemplateTerm, TemplateTerm, TreeTerm, TryTerm, TryTermKind, TupleElementTerm,
    TypeLiteralTerm, TypeOperationTerm, TypeRelation, TypeTerm, TypeValueTerm, VariableId,
    VariableKind, YieldTerm,
};

/// The condition branch being entered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum ConditionBranch {
    /// The condition is known true.
    True,
    /// The condition is known false.
    False,
}

impl CheckModuleState {
    /// Walk one catch clause.
    pub(in crate::check) fn walk_catch(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Catch>,
        catch: &dir::Catch,
        failure: Option<VariableId>,
    ) {
        // apply static owner guards
        if !self.static_allows(tree, id.into_any()) {
            return;
        }

        self.visit_any(tree, dir::NodeType::Catch, id.id);

        // catch (error)
        if let Some(pattern) = catch.pattern {
            self.walk_pattern(tree, pattern, tree.get(pattern));

            if let (Some(failure), Some(ty)) = (failure, catch.ty) {
                let expected = self.intern_local_type_variable(ty);
                let origin = ConstraintOrigin::Node(ty.into_global_any(self.input.module_id));

                self.constrain_type(origin, TypeRelation::Assignable, failure, expected);
            }

            if let Some(value) = catch
                .ty
                .map(|ty| self.intern_local_type_variable(ty))
                .or(failure)
                && let Some(pattern_term) = self.build_pattern_term(pattern, tree)
            {
                self.constrain_pattern(
                    PatternRelation::Match(pattern_term),
                    pattern.into_any(),
                    value,
                );
            }

            self.mark_bindings_assigned(tree, pattern.into_any());
        }

        // catch (error: T)
        if let Some(ty) = catch.ty {
            self.walk_type_expression(tree, ty, tree.get(ty));
        }

        // catch match (failure)
        if catch.pattern.is_none()
            && catch.ty.is_none()
            && let Some(failure) = failure
            && let Some(symbol) = self.catch_match_failure_symbol(tree, catch.body)
        {
            let variable = self.intern_symbol_type_variable(symbol);

            self.define_type(variable, TypeTerm::Variable(failure));
            self.work.flow.mark_assigned(symbol);
        }

        // catch (...) { ... }
        self.walk_expression(tree, catch.body, tree.get(catch.body));
    }

    /// Walk one expression.
    pub(in crate::check) fn walk_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // apply static owner guards
        if !self.static_allows(tree, id.into_any()) {
            return;
        }

        self.visit_any(tree, dir::NodeType::Expression, id.id);

        match expression {
            // function f() {}
            dir::Expression::Declaration(declaration) => {
                if tree.get(*declaration).symbol_kind().is_some() {
                    if let Some(symbol) = self.declaration_symbol((*declaration).into_any()) {
                        let term = TypeTerm::Variable(self.intern_symbol_type_variable(symbol));

                        self.define_expression_type(id, term);
                    }
                }

                self.walk_declaration(tree, *declaration, tree.get(*declaration));
            }
            // { ... }
            dir::Expression::Block(block) => {
                let term = TypeTerm::Variable(self.intern_local_type_variable(*block));

                self.define_expression_type(id, term);
                self.walk_block(tree, *block, tree.get(*block));
            }
            // label: body
            dir::Expression::Label { label, body } => {
                self.walk_label_expression(tree, id, *label, *body);
            }
            // import { item } from "module"
            dir::Expression::Import { items, .. } => {
                self.define_expression_type(id, TypeTerm::Literal(TypeLiteralTerm::Void));

                if let Some(items) = items {
                    for item in items {
                        self.walk_dependency_item(tree, *item, tree.get(*item));
                    }
                }
            }
            // export { item } from "module"
            dir::Expression::Export { items, .. } => {
                self.define_expression_type(id, TypeTerm::Literal(TypeLiteralTerm::Void));

                for item in items {
                    self.walk_dependency_item(tree, *item, tree.get(*item));
                }
            }
            // let x = value
            dir::Expression::Let { declarators, .. }
            // using x = value
            | dir::Expression::Using { declarators, .. } => {
                self.define_expression_type(id, TypeTerm::Literal(TypeLiteralTerm::Void));

                for declarator in declarators {
                    self.walk_declarator(tree, *declarator, tree.get(*declarator));
                    self.mark_declarator_assigned(tree, tree.get(*declarator));
                }
            }
            // let x = value else fallback
            dir::Expression::LetElse {
                declarator,
                else_branch,
                ..
            } => {
                self.define_expression_type(id, TypeTerm::Literal(TypeLiteralTerm::Void));
                self.walk_declarator(tree, *declarator, tree.get(*declarator));

                let before_else = self.checkpoint_flow();
                self.walk_expression(tree, *else_branch, tree.get(*else_branch));
                if self.expression_can_fall_through(tree, *else_branch) {
                    self.report_invalid_control_flow(
                        (*else_branch).into_any(),
                        "let else fallback must not fall through",
                    );
                }
                self.restore_flow(before_else);
                self.mark_declarator_assigned(tree, tree.get(*declarator));
                self.apply_declarator_pattern_narrowings(tree, *declarator);
            }
            // if condition { then } else { otherwise }
            dir::Expression::If {
                condition,
                then_expression,
                else_expression,
                ..
            } => {
                let then_type = self.intern_local_type_variable(*then_expression);
                let else_type = match else_expression {
                    Some(else_expression) => self.intern_local_type_variable(*else_expression),
                    None => self.define_void_type(id.into_any()),
                };
                let term = TypeTerm::Operation(TypeOperationTerm::BestCommon {
                    elements: vec![then_type, else_type],
                });

                self.define_expression_type(id, term);
                self.walk_if_expression(tree, condition, *then_expression, *else_expression);
            }
            // while condition { body }
            dir::Expression::While {
                condition, body, ..
            } => {
                self.intern_local_type_variable(id);
                self.walk_while_expression(tree, id, None, *condition, *body);
            }
            // for item of iterator { body }
            dir::Expression::ForEach {
                operator,
                binding,
                iterator,
                body,
                ..
            } => {
                self.intern_local_type_variable(id);
                self.walk_for_each_expression(tree, id, None, *operator, binding, *iterator, *body);
            }
            // for (initialization; condition; increment) { body }
            dir::Expression::For {
                initialization,
                condition,
                increment,
                body,
            } => {
                self.intern_local_type_variable(id);
                self.walk_for_expression(
                    tree,
                    id,
                    None,
                    *initialization,
                    *condition,
                    *increment,
                    *body,
                );
            }
            // loop { body }
            dir::Expression::Loop { body } => {
                self.intern_local_type_variable(id);
                self.walk_loop_expression(tree, id, None, *body);
            }
            // try body catch error finally cleanup
            dir::Expression::Try {
                body,
                catch,
                finally,
            } => {
                let body_type = self.intern_local_type_variable(*body);
                let term = match catch {
                    Some(catch) => {
                        let catch_type = self.intern_local_type_variable(tree.get(*catch).body);

                        TypeTerm::Operation(TypeOperationTerm::BestCommon {
                            elements: vec![body_type, catch_type],
                        })
                    }
                    None => TypeTerm::Variable(body_type),
                };

                self.define_expression_type(id, term);
                self.walk_try_expression(tree, id, *body, *catch, *finally);
            }
            // match value { case pattern => body }
            dir::Expression::Match { value, cases, .. } => {
                let elements = cases
                    .iter()
                    .map(|case| match tree.get(*case) {
                        // case pattern => expression
                        dir::MatchCase::Expression { body, .. } => {
                            self.intern_local_type_variable(*body)
                        }
                        // case pattern => { ... }
                        dir::MatchCase::Block { body, .. } => {
                            self.intern_local_type_variable(*body)
                        }
                    })
                    .collect();
                let term = TypeTerm::Operation(TypeOperationTerm::BestCommon { elements });

                self.define_expression_type(id, term);
                self.walk_match_expression(tree, id, *value, cases);
            }
            // break value
            dir::Expression::Break { label, value } => {
                self.define_expression_type(id, TypeTerm::Literal(TypeLiteralTerm::Never));

                let value = if let Some(value) = value {
                    self.walk_expression(tree, *value, tree.get(*value));

                    Some(self.intern_local_type_variable(*value))
                } else {
                    None
                };

                self.record_break_value(id.into_any(), *label, value);
            }
            // continue
            dir::Expression::Continue { label } => {
                self.define_expression_type(id, TypeTerm::Literal(TypeLiteralTerm::Never));
                self.record_continue_branch(id.into_any(), *label);
            }
            // await value
            dir::Expression::Await {
                expression: awaited,
            } => {
                self.validate_await_context(id.into_any());

                let term = TypeTerm::Await(AwaitTerm {
                    source: id.into_global_any(self.input.module_id),
                    value: self.intern_local_type_variable(*awaited),
                });

                self.define_expression_type(id, term);
                self.walk_expression(tree, *awaited, tree.get(*awaited));
            }
            // throw value
            dir::Expression::Throw { value } => {
                self.define_expression_type(id, TypeTerm::Literal(TypeLiteralTerm::Never));
                self.walk_expression(tree, *value, tree.get(*value));
            }
            // return value
            dir::Expression::Return { value } => {
                self.define_expression_type(id, TypeTerm::Literal(TypeLiteralTerm::Never));

                if let Some(value) = value {
                    self.walk_expression(tree, *value, tree.get(*value));

                    let value = self.intern_local_type_variable(*value);
                    self.constrain_return_value(id.into_any(), value);
                } else {
                    self.constrain_void_return(id.into_any());
                }
            }
            // yield value
            dir::Expression::Yield { cardinality, value } => {
                let source = id.into_global_any(self.input.module_id);
                let origin = ConstraintOrigin::Node(source);
                let value_type = value.map(|value| self.intern_local_type_variable(value));
                let delegate_return_type = if *cardinality == dir::YieldCardinality::Generator {
                    Some(self.allocate_anonymous_variable(VariableKind::Type, origin))
                } else {
                    None
                };
                let term = TypeTerm::Yield(YieldTerm {
                    source,
                    value: value_type,
                    yield_type: self.current_yield_type(),
                    resume_type: self.current_resume_type(),
                    delegate_return_type,
                    cardinality: *cardinality,
                });

                self.define_expression_type(id, term);
                self.constrain_yield_value(
                    id.into_any(),
                    *cardinality,
                    value_type,
                    delegate_return_type,
                );

                if let Some(value) = value {
                    self.walk_expression(tree, *value, tree.get(*value));
                }
            }
            // value
            dir::Expression::Identifier { name } => {
                if let Some(term) = self.resolve_identifier_expression(id, *name, tree) {
                    self.define_expression_type(id, term);
                }
            }
            // this
            dir::Expression::This => {
                if let Some(term) = self.resolve_this_expression(id) {
                    self.define_expression_type(id, term);
                }
            }
            // 1, "text", true
            dir::Expression::ScalarLiteral(value) => {
                let term = TypeTerm::Literal(TypeLiteralTerm::Scalar(value.clone()));

                self.define_expression_type(id, term);
            }
            // super
            dir::Expression::Super => {
                let source = id.into_global_any(self.input.module_id);
                let receiver = self
                    .resolve_this_receiver(source)
                    .map(|receiver| receiver.ty);
                let term = TypeTerm::Super(SuperTerm { source, receiver });

                self.define_expression_type(id, term);
            }
            // import.meta
            dir::Expression::ImportMeta => {
                let term = TypeTerm::ImportMeta(ImportMetaTerm {
                    source: id.into_global_any(self.input.module_id),
                });

                self.define_expression_type(id, term);
            }
            // #name
            dir::Expression::PrivateIdentifier { .. } => {}
            // debugger
            dir::Expression::Debugger => {}
            // missing expression
            dir::Expression::Missing => {}
            // stub expression
            dir::Expression::Stub => {}
            // ignore damaged syntax
            dir::Expression::Error => {}
            // namespace.value<T>
            dir::Expression::QualifiedReference {
                path,
                generic_arguments,
            } => {
                if let Some(term) =
                    self.resolve_reference_expression(id, path, generic_arguments, tree)
                {
                    self.define_expression_type(id, term);
                }

                for argument in generic_arguments {
                    self.walk_generic_argument(tree, *argument, tree.get(*argument));
                }
            }
            // start..end
            dir::Expression::RangeExpression {
                start,
                end,
                end_kind,
            } => {
                let term = TypeTerm::RangeValue(RangeValueTerm {
                    source: id.into_global_any(self.input.module_id),
                    start: start.map(|start| self.intern_local_type_variable(start)),
                    end: end.map(|end| self.intern_local_type_variable(end)),
                    end_kind: *end_kind,
                });

                self.define_expression_type(id, term);

                if let Some(start) = start {
                    self.walk_expression(tree, *start, tree.get(*start));
                }
                if let Some(end) = end {
                    self.walk_expression(tree, *end, tree.get(*end));
                }
            }
            // `text ${value}`
            dir::Expression::TemplateExpression { value } => {
                let (strings, spans) = self.template_parts(value, tree);
                let term = TypeTerm::Template(TemplateTerm {
                    source: id.into_global_any(self.input.module_id),
                    strings,
                    spans,
                });

                self.define_expression_type(id, term);
                self.walk_template_literal(tree, value);
            }
            // tag<T>`text ${value}`
            dir::Expression::TaggedTemplateExpression {
                tag,
                generic_arguments,
                value,
            } => {
                let (strings, spans) = self.template_parts(value, tree);
                let generic_arguments_term = self.build_generic_arguments(generic_arguments, tree);
                let term = TypeTerm::TaggedTemplate(TaggedTemplateTerm {
                    source: id.into_global_any(self.input.module_id),
                    tag: self.intern_local_type_variable(*tag),
                    generic_arguments: generic_arguments_term,
                    strings,
                    spans,
                });

                self.define_expression_type(id, term);
                self.walk_expression(tree, *tag, tree.get(*tag));

                for argument in generic_arguments {
                    self.walk_generic_argument(tree, *argument, tree.get(*argument));
                }

                self.walk_template_literal(tree, value);
            }
            // [a, b, c]
            dir::Expression::ArrayExpression { elements } => {
                let origin = ConstraintOrigin::Node(id.into_global_any(self.input.module_id));
                let element_types = self.argument_value_type_variables(elements, tree);
                let element = self.allocate_anonymous_variable(VariableKind::Type, origin);
                let length = dir::StaticTerm::ScalarLiteral {
                    value: dir::ScalarLiteral::Integer(element_types.len() as i64),
                };
                let length = self.define_static_literal(origin, length);
                let term = TypeTerm::Operation(TypeOperationTerm::BestCommon {
                    elements: element_types,
                });

                self.define_type(element, term);

                let term = TypeTerm::FixedArray {
                    element,
                    length,
                    is_readonly: false,
                };

                self.define_expression_type(id, term);

                for element in elements {
                    self.walk_argument(tree, *element, tree.get(*element));
                }
            }
            // [value; length]
            dir::Expression::FixedArrayExpression { value, length } => {
                let term = TypeTerm::FixedArray {
                    element: self.intern_local_type_variable(*value),
                    length: self.define_static_expression_variable(*length),
                    is_readonly: false,
                };

                self.define_expression_type(id, term);
                self.walk_expression(tree, *value, tree.get(*value));
                self.walk_expression(tree, *length, tree.get(*length));
            }
            // [a, label: b, ...rest]
            dir::Expression::TupleExpression { elements } => {
                let mut elements_term = Vec::new();
                for element in elements {
                    let argument = tree.get(*element);
                    let label = match argument {
                        // [label = value]
                        dir::Argument::Labeled { label, .. } => Some(*label),
                        // [...label: value]
                        dir::Argument::Spread { label, .. } => *label,
                        // [name: value], [value]
                        dir::Argument::Named { .. } | dir::Argument::Positional { .. } => None,
                        // ignore damaged syntax
                        dir::Argument::Error => continue,
                    };
                    let Some(value) = argument.value() else {
                        continue;
                    };

                    elements_term.push(TupleElementTerm {
                        label,
                        ty: self.intern_local_type_variable(value),
                        is_optional: false,
                        is_readonly: false,
                        is_rest: matches!(argument, dir::Argument::Spread { .. }),
                    });
                }
                let term = TypeTerm::Tuple {
                    form: dir::TupleForm::Tuple,
                    elements: elements_term,
                    is_readonly: false,
                };

                self.define_expression_type(id, term);

                for element in elements {
                    self.walk_argument(tree, *element, tree.get(*element));
                }
            }
            // a, b, c
            dir::Expression::SequenceExpression { expressions } => {
                let term = match expressions.last() {
                    Some(expression) => {
                        TypeTerm::Variable(self.intern_local_type_variable(*expression))
                    }
                    None => TypeTerm::Literal(TypeLiteralTerm::Void),
                };

                self.define_expression_type(id, term);

                for expression in expressions {
                    self.walk_expression(tree, *expression, tree.get(*expression));
                }
            }
            // { key: value }
            dir::Expression::ObjectExpression { properties } => {
                let mut members = Vec::new();
                for property in properties {
                    match tree.get(*property) {
                        // { key: value }
                        dir::Property::Field { key, value, .. } => {
                            let Some(key) = key.static_key(tree) else {
                                continue;
                            };

                            members.push(ShapeMemberTerm::Field {
                                key,
                                ty: self.intern_local_type_variable(*value),
                                is_optional: false,
                                is_readonly: false,
                            });
                        }
                        // { method() {} }
                        dir::Property::Method { key: Some(key), .. } => {
                            let Some(key) = key.static_key(tree) else {
                                continue;
                            };
                            let ty = self
                                .declaration_symbol((*property).into_any())
                                .map(|symbol| self.intern_symbol_type_variable(symbol))
                                .unwrap_or_else(|| self.intern_local_type_variable(*property));

                            members.push(ShapeMemberTerm::Field {
                                key,
                                ty,
                                is_optional: false,
                                is_readonly: false,
                            });
                        }
                        // { []() {} }
                        dir::Property::Method { key: None, .. } => {}
                        // { ...value }
                        dir::Property::Spread { .. } => {}
                        // ignore damaged syntax
                        dir::Property::Error => {}
                    }
                }
                let term = TypeTerm::Shape { members };

                self.define_expression_type(id, term);

                for property in properties {
                    self.walk_property(tree, *property, tree.get(*property));
                }
            }
            // Type { key: value }
            dir::Expression::StructExpression { ty, properties } => {
                let source = id.into_global_any(self.input.module_id);
                let owner = self.intern_local_type_variable(*ty);

                for property in properties {
                    if let dir::Property::Field { key, value, .. } = tree.get(*property)
                        && let Some(key) = key.static_key(tree)
                    {
                        let field = TypeTerm::Member(MemberTerm {
                            source: None,
                            owner,
                            key,
                            arguments: Vec::new(),
                        });
                        let field = self.define_anonymous_type(ConstraintOrigin::Node(source), field);
                        let value = self.intern_local_type_variable(*value);

                        self.constrain_type(
                            ConstraintOrigin::Node(source),
                            TypeRelation::Assignable,
                            value,
                            field,
                        );
                    }
                }

                self.define_expression_type(id, TypeTerm::Variable(owner));

                self.walk_type_expression(tree, *ty, tree.get(*ty));
                for property in properties {
                    self.walk_property(tree, *property, tree.get(*property));
                }
            }
            // jsx like tree expression
            dir::Expression::TreeExpression {
                left,
                generic_arguments,
                arguments,
                elements,
            } => {
                let term = TypeTerm::Tree(TreeTerm {
                    source: id.into_global_any(self.input.module_id),
                    tag: left.map(|left| self.intern_local_type_variable(left)),
                    generic_arguments: self.build_generic_arguments(generic_arguments, tree),
                    arguments: arguments
                        .as_ref()
                        .map(|arguments| self.argument_value_type_variables(arguments, tree))
                        .unwrap_or_default(),
                    elements: elements
                        .as_ref()
                        .map(|elements| self.argument_value_type_variables(elements, tree))
                        .unwrap_or_default(),
                });

                self.define_expression_type(id, term);

                if let Some(left) = left {
                    self.walk_expression(tree, *left, tree.get(*left));
                }
                for argument in generic_arguments {
                    self.walk_generic_argument(tree, *argument, tree.get(*argument));
                }
                if let Some(arguments) = arguments {
                    for argument in arguments {
                        self.walk_argument(tree, *argument, tree.get(*argument));
                    }
                }
                if let Some(elements) = elements {
                    for element in elements {
                        self.walk_argument(tree, *element, tree.get(*element));
                    }
                }
            }
            // (value)
            dir::Expression::Parenthesized { expression: child } => {
                let child_type = self.intern_local_type_variable(*child);

                self.define_expression_type(id, TypeTerm::Variable(child_type));
                self.walk_expression(tree, *child, tree.get(*child));
            }
            // type T
            dir::Expression::Type { value } => {
                let term = TypeTerm::TypeValue(TypeValueTerm {
                    source: id.into_global_any(self.input.module_id),
                    ty: self.intern_local_type_variable(*value),
                });

                self.define_expression_type(id, term);
                self.walk_type_expression(tree, *value, tree.get(*value));
            }
            // comptime value
            dir::Expression::Comptime { body } => {
                let body_type = self.intern_local_type_variable(*body);

                self.define_expression_type(id, TypeTerm::Variable(body_type));
                self.walk_expression(tree, *body, tree.get(*body));
            }
            // value as T
            dir::Expression::As {
                expression: child,
                target_type,
            } => {
                let child_type = self.intern_local_type_variable(*child);
                let target_type_variable = self.intern_local_type_variable(*target_type);
                let origin = ConstraintOrigin::Node(id.into_global_any(self.input.module_id));

                self.define_expression_type(id, TypeTerm::Variable(target_type_variable));
                self.constrain_type(
                    origin,
                    TypeRelation::Castable,
                    child_type,
                    target_type_variable,
                );
                self.walk_expression(tree, *child, tree.get(*child));
                self.walk_type_expression(tree, *target_type, tree.get(*target_type));
            }
            // value satisfies T
            dir::Expression::Satisfies {
                expression: child,
                target_type,
            } => {
                let child_type = self.intern_local_type_variable(*child);
                let target_type_variable = self.intern_local_type_variable(*target_type);
                let origin = ConstraintOrigin::Node(id.into_global_any(self.input.module_id));

                self.define_expression_type(id, TypeTerm::Variable(child_type));
                self.constrain_type(
                    origin,
                    TypeRelation::Satisfies,
                    child_type,
                    target_type_variable,
                );
                self.walk_expression(tree, *child, tree.get(*child));
                self.walk_type_expression(tree, *target_type, tree.get(*target_type));
            }
            // value is T
            dir::Expression::Is { value, target_type } => {
                self.define_expression_type(id, TypeTerm::Literal(TypeLiteralTerm::boolean()));
                self.walk_expression(tree, *value, tree.get(*value));
                self.walk_type_expression(tree, *target_type, tree.get(*target_type));
            }
            // value instanceof Target
            dir::Expression::InstanceOf { value, target } => {
                let term = TypeTerm::InstanceCheck(InstanceCheckTerm {
                    source: id.into_global_any(self.input.module_id),
                    value: self.intern_local_type_variable(*value),
                    target: self.intern_local_type_variable(*target),
                });

                self.define_expression_type(id, term);
                self.walk_expression(tree, *value, tree.get(*value));
                self.walk_expression(tree, *target, tree.get(*target));
            }
            // !value, ++value
            dir::Expression::Unary { operator, right } => {
                let is_update = matches!(
                    operator,
                    dir::UnaryOperator::PostIncrement
                        | dir::UnaryOperator::PostDecrement
                        | dir::UnaryOperator::PreIncrement
                        | dir::UnaryOperator::PreDecrement
                );

                let term = TypeTerm::Operator(OperatorTerm {
                    source: id.into_global_any(self.input.module_id),
                    kind: OperatorTermKind::Unary(*operator),
                    receiver: self.intern_local_type_variable(*right),
                    argument: None,
                });

                self.define_expression_type(id, term);
                self.walk_expression(tree, *right, tree.get(*right));

                // record writes after reading the updated place
                if is_update && let Some(place) = self.resolve_place(*right, tree) {
                    self.clear_written_expression_narrowings(tree, *right);
                    self.mark_place_assigned(place);
                    self.require_writable_place(place);
                }
            }
            // move value
            dir::Expression::MoveOf { right, .. } => {
                let term = TypeTerm::Form {
                    form: FormTerm::Owned,
                    payload: self.intern_local_type_variable(*right),
                };

                self.define_expression_type(id, term);
                self.walk_expression(tree, *right, tree.get(*right));
            }
            // &value
            dir::Expression::BorrowOf {
                mutability, right, ..
            } => {
                let origin = ConstraintOrigin::Node(id.into_global_any(self.input.module_id));
                let access = mutability
                    .map(dir::Mutability::access)
                    .unwrap_or(dir::Access::Mutable);
                let access = dir::StaticTerm::Access { access };
                let access = self.define_static_literal(origin, access);
                let lifetime = self.allocate_anonymous_variable(VariableKind::Static, origin);
                let term = TypeTerm::Form {
                    form: FormTerm::Borrowed { lifetime, access },
                    payload: self.intern_local_type_variable(*right),
                };

                self.define_expression_type(id, term);
                self.walk_expression(tree, *right, tree.get(*right));
            }
            // value.member
            dir::Expression::Member { left, name }
            // value.#member
            | dir::Expression::PrivateMember { left, name } => {
                if let Some(name) = name {
                    let term = if let Some(narrowed) = self.flow_path_narrowing(tree, id) {
                        TypeTerm::Variable(narrowed)
                    } else {
                        TypeTerm::Member(MemberTerm {
                            source: Some(id.into_global_any(self.input.module_id)),
                            owner: self.intern_local_type_variable(*left),
                            key: dir::StaticKey::Name(*name),
                            arguments: Vec::new(),
                        })
                    };

                    self.define_expression_type(id, term);
                }

                self.walk_expression(tree, *left, tree.get(*left));
            }
            // value[index]
            dir::Expression::Index { left, index, .. } => {
                if let Some(index) = index {
                    let term = if let Some(narrowed) = self.flow_path_narrowing(tree, id) {
                        TypeTerm::Variable(narrowed)
                    } else {
                        TypeTerm::Index(IndexTerm {
                            source: id.into_global_any(self.input.module_id),
                            receiver: self.intern_local_type_variable(*left),
                            index: self.intern_local_type_variable(*index),
                            key: tree.get(*index).static_key(),
                        })
                    };

                    self.define_expression_type(id, term);
                }

                self.walk_expression(tree, *left, tree.get(*left));

                if let Some(index) = index {
                    self.walk_expression(tree, *index, tree.get(*index));
                }
            }
            // value<T>
            dir::Expression::Instantiation {
                left,
                generic_arguments,
            } => {
                if let Some(symbol) = self.resolve_call_candidate_symbol(*left, tree) {
                    let source = id.into_global_any(self.input.module_id);
                    let arguments =
                        self.build_generic_arguments_for_owner(symbol, generic_arguments, tree);
                    let term = TypeTerm::Reference {
                        source: Some(source),
                        symbol,
                        arguments,
                    };

                    self.define_expression_type(id, term);
                }

                self.walk_expression(tree, *left, tree.get(*left));

                for argument in generic_arguments {
                    self.walk_generic_argument(tree, *argument, tree.get(*argument));
                }
            }
            // callee<T>(argument)
            dir::Expression::Call {
                left,
                generic_arguments,
                arguments,
                ..
            } => {
                let source = id.into_global_any(self.input.module_id);
                let callee = self.intern_local_type_variable(*left);

                // value.member()
                let member = if let dir::Expression::Member {
                    left,
                    name: Some(name),
                }
                // value.#member()
                | dir::Expression::PrivateMember {
                    left,
                    name: Some(name),
                } = tree.get(*left)
                {
                    Some(MemberCallTerm {
                        receiver: self.intern_local_type_variable(*left),
                        key: dir::StaticKey::Name(*name),
                        arguments: Vec::new(),
                        protocol: None,
                    })
                } else {
                    None
                };

                // seed direct identifier candidates
                let candidates = match self.resolve_call_candidate_symbol(*left, tree) {
                    Some(symbol) => {
                        let ty = self.intern_symbol_type_variable(symbol);

                        vec![CallCandidate { symbol, ty }]
                    }
                    None => Vec::new(),
                };
                let generic_arguments_term = match candidates.first() {
                    Some(candidate) => {
                        self.build_generic_arguments_for_owner(
                            candidate.symbol,
                            generic_arguments,
                            tree,
                        )
                    }
                    None => self.build_generic_arguments(generic_arguments, tree),
                };
                let arguments_term = self.argument_value_type_variables(arguments, tree);
                let term = TypeTerm::Call(CallTerm {
                    source,
                    callee,
                    member,
                    candidates,
                    generic_arguments: generic_arguments_term,
                    arguments: arguments_term,
                });

                self.define_expression_type(id, term);
                self.walk_expression(tree, *left, tree.get(*left));

                // arguments
                for argument in generic_arguments {
                    self.walk_generic_argument(tree, *argument, tree.get(*argument));
                }
                for argument in arguments {
                    self.walk_argument(tree, *argument, tree.get(*argument));
                }
            }
            // new Type<T>(argument)
            dir::Expression::New {
                left,
                generic_arguments,
                arguments,
            } => {
                let term = TypeTerm::New(NewTerm {
                    source: id.into_global_any(self.input.module_id),
                    callee: self.intern_local_type_variable(*left),
                    generic_arguments: self.build_generic_arguments(generic_arguments, tree),
                    arguments: self.argument_value_type_variables(arguments, tree),
                });

                self.define_expression_type(id, term);
                self.walk_expression(tree, *left, tree.get(*left));

                // arguments
                for argument in generic_arguments {
                    self.walk_generic_argument(tree, *argument, tree.get(*argument));
                }
                for argument in arguments {
                    self.walk_argument(tree, *argument, tree.get(*argument));
                }
            }
            // await? value
            dir::Expression::AwaitMaybe {
                expression: awaited,
            } => {
                self.validate_await_context(id.into_any());

                let source = id.into_global_any(self.input.module_id);
                let origin = ConstraintOrigin::Node(source);
                let awaited_type = self.allocate_anonymous_variable(VariableKind::Type, origin);
                let value = self.intern_local_type_variable(*awaited);

                self.define_type(awaited_type, TypeTerm::Await(AwaitTerm { source, value }));

                let term = TryTerm {
                    source,
                    value: awaited_type,
                    kind: TryTermKind::Maybe,
                };

                self.define_expression_type(id, TypeTerm::Try(term.clone()));
                self.walk_expression(tree, *awaited, tree.get(*awaited));

                self.record_try_propagation(id.into_any(), term.value);
            }
            // await! value
            dir::Expression::AwaitMust {
                expression: awaited,
            } => {
                self.validate_await_context(id.into_any());

                let source = id.into_global_any(self.input.module_id);
                let origin = ConstraintOrigin::Node(source);
                let awaited_type = self.allocate_anonymous_variable(VariableKind::Type, origin);
                let value = self.intern_local_type_variable(*awaited);

                self.define_type(awaited_type, TypeTerm::Await(AwaitTerm { source, value }));

                let term = TryTerm {
                    source,
                    value: awaited_type,
                    kind: TryTermKind::Must,
                };

                self.define_expression_type(id, TypeTerm::Try(term));
                self.walk_expression(tree, *awaited, tree.get(*awaited));
            }
            // value?
            dir::Expression::Maybe { left, .. } => {
                let tried = TryTerm {
                    source: id.into_global_any(self.input.module_id),
                    value: self.intern_local_type_variable(*left),
                    kind: TryTermKind::Maybe,
                };

                self.define_expression_type(id, TypeTerm::Try(tried.clone()));
                self.walk_expression(tree, *left, tree.get(*left));

                self.record_try_propagation(id.into_any(), tried.value);
            }
            // value!
            dir::Expression::Must { left, .. } => {
                let term = TypeTerm::Try(TryTerm {
                    source: id.into_global_any(self.input.module_id),
                    value: self.intern_local_type_variable(*left),
                    kind: TryTermKind::Must,
                });

                self.define_expression_type(id, term);
                self.walk_expression(tree, *left, tree.get(*left));
            }
            // left + right
            dir::Expression::Binary {
                left,
                operator,
                right,
            } => {
                let source = id.into_global_any(self.input.module_id);
                let left_type = self.intern_local_type_variable(*left);
                let right_type = self.intern_local_type_variable(*right);
                let term = match operator {
                    // left === right
                    dir::BinaryOperator::EqualStrict | dir::BinaryOperator::NotEqualStrict => {
                        TypeTerm::Identity(IdentityTerm {
                            source,
                            operator: *operator,
                            left: left_type,
                            right: right_type,
                        })
                    }
                    // key in receiver
                    dir::BinaryOperator::In => TypeTerm::KeyMembership(KeyMembershipTerm {
                        source,
                        key: left_type,
                        receiver: right_type,
                        static_key: tree.get(*left).static_key(),
                    }),
                    // overloaded operators
                    dir::BinaryOperator::Exponent
                    | dir::BinaryOperator::Multiply
                    | dir::BinaryOperator::Divide
                    | dir::BinaryOperator::Remainder
                    | dir::BinaryOperator::Add
                    | dir::BinaryOperator::Subtract
                    | dir::BinaryOperator::ShiftLeft
                    | dir::BinaryOperator::ShiftRight
                    | dir::BinaryOperator::UnsignedShiftRight
                    | dir::BinaryOperator::ElementwiseAnd
                    | dir::BinaryOperator::ElementwiseXor
                    | dir::BinaryOperator::ElementwiseOr
                    | dir::BinaryOperator::Equal
                    | dir::BinaryOperator::NotEqual
                    | dir::BinaryOperator::LessThan
                    | dir::BinaryOperator::LessThanOrEqual
                    | dir::BinaryOperator::GreaterThan
                    | dir::BinaryOperator::GreaterThanOrEqual
                    | dir::BinaryOperator::And
                    | dir::BinaryOperator::Or
                    | dir::BinaryOperator::Coalesce => TypeTerm::Operator(OperatorTerm {
                        source,
                        kind: OperatorTermKind::Binary(*operator),
                        receiver: left_type,
                        argument: Some(right_type),
                    }),
                };

                self.define_expression_type(id, term);

                match operator {
                    // left && right
                    dir::BinaryOperator::And => {
                        self.walk_expression(tree, *left, tree.get(*left));

                        let before_right = self.checkpoint_flow();

                        self.restore_flow(before_right);
                        self.apply_expression_narrowings(tree, *left, ConditionBranch::True);
                        self.walk_expression(tree, *right, tree.get(*right));
                        let right_flow = self
                            .expression_can_fall_through(tree, *right)
                            .then(|| self.collect_flow_branch(before_right));

                        self.restore_flow(before_right);
                        self.apply_expression_narrowings(tree, *left, ConditionBranch::False);
                        let skip_flow = self.collect_flow_branch(before_right);

                        if let Some(right_flow) = right_flow {
                            self.merge_flow_branches(before_right, &skip_flow, &right_flow);
                        } else {
                            self.apply_flow_branch(before_right, &skip_flow);
                        }
                    }
                    // left || right
                    dir::BinaryOperator::Or => {
                        self.walk_expression(tree, *left, tree.get(*left));

                        let before_right = self.checkpoint_flow();

                        self.restore_flow(before_right);
                        self.apply_expression_narrowings(tree, *left, ConditionBranch::False);
                        self.walk_expression(tree, *right, tree.get(*right));
                        let right_flow = self
                            .expression_can_fall_through(tree, *right)
                            .then(|| self.collect_flow_branch(before_right));

                        self.restore_flow(before_right);
                        self.apply_expression_narrowings(tree, *left, ConditionBranch::True);
                        let skip_flow = self.collect_flow_branch(before_right);

                        if let Some(right_flow) = right_flow {
                            self.merge_flow_branches(before_right, &skip_flow, &right_flow);
                        } else {
                            self.apply_flow_branch(before_right, &skip_flow);
                        }
                    }
                    // eager binary operators
                    _ => {
                        self.walk_expression(tree, *left, tree.get(*left));
                        self.walk_expression(tree, *right, tree.get(*right));
                    }
                }
            }
            // target = value
            dir::Expression::Assign {
                left,
                operator,
                right,
            } => {
                let value = self.intern_local_type_variable(*right);

                // direct assignment uses the right side value
                let assigned_value = if *operator == dir::AssignOperator::Assign {
                    Some(value)
                }
                // compound assignment reads and writes the same place
                else {
                    match tree.get(*left) {
                        // target
                        dir::AssignPattern::Expression { value: place } => {
                            let receiver = self.intern_local_type_variable(*place);
                            let source = id.into_global_any(self.input.module_id);
                            let term = if let Some(binary_operator) = operator.binary_operator() {
                                TypeTerm::Operator(OperatorTerm {
                                    source,
                                    kind: OperatorTermKind::Binary(binary_operator),
                                    receiver,
                                    argument: Some(value),
                                })
                            } else {
                                TypeTerm::Operation(TypeOperationTerm::BestCommon {
                                    elements: vec![receiver, value],
                                })
                            };
                            let result = self.allocate_anonymous_variable(
                                VariableKind::Type,
                                ConstraintOrigin::Node(source),
                            );

                            self.define_type(result, term);

                            Some(result)
                        }
                        // target = value
                        dir::AssignPattern::Assign { .. }
                        // [a, b]
                        | dir::AssignPattern::Sequence { .. }
                        // { a, b }
                        | dir::AssignPattern::Object { .. } => None,
                    }
                };

                // index assignment dispatches through index set
                if let Some(assigned_value) = assigned_value {
                    let term = match tree.get(*left) {
                        // target
                        dir::AssignPattern::Expression { value: place } => {
                            if let dir::Expression::Index {
                                left,
                                index: Some(index),
                                ..
                            } = tree.get(*place)
                            {
                                TypeTerm::IndexSet(IndexSetTerm {
                                    source: id.into_global_any(self.input.module_id),
                                    receiver: self.intern_local_type_variable(*left),
                                    index: self.intern_local_type_variable(*index),
                                    value: assigned_value,
                                    key: tree.get(*index).static_key(),
                                })
                            } else {
                                TypeTerm::Variable(assigned_value)
                            }
                        }
                        // target = value
                        dir::AssignPattern::Assign { .. }
                        // [a, b]
                        | dir::AssignPattern::Sequence { .. }
                        // { a, b }
                        | dir::AssignPattern::Object { .. } => TypeTerm::Variable(assigned_value),
                    };

                    self.define_expression_type(id, term);
                }

                if let Some(pattern) = self.build_assign_pattern_term(*left, tree) {
                    let value = self.intern_local_type_variable(id);

                    self.constrain_pattern(
                        PatternRelation::Assign(pattern),
                        (*left).into_any(),
                        value,
                    );
                }

                self.walk_assign_pattern(tree, *left, tree.get(*left));
                self.walk_expression(tree, *right, tree.get(*right));

                // record writes after reading the assigned value
                self.record_assign_pattern_writes(*left, tree);
            }
        }
    }

    /// Walk one where clause.
    pub(in crate::check) fn walk_where_clause(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::WhereClause>,
        where_clause: &dir::WhereClause,
    ) {
        self.visit_any(tree, dir::NodeType::WhereClause, id.id);
        self.walk_type_expression(tree, where_clause.left, tree.get(where_clause.left));
        self.walk_type_expression(tree, where_clause.right, tree.get(where_clause.right));
    }

    /// Walk one template literal's expression arguments.
    fn walk_template_literal(&mut self, tree: &dir::Tree, value: &dir::TemplateLiteral) {
        match value {
            // `text`
            dir::TemplateLiteral::String { .. } => {}
            // `text ${value}`
            dir::TemplateLiteral::InterpolatedString { arguments, .. } => {
                for argument in arguments {
                    self.walk_argument(tree, *argument, tree.get(*argument));
                }
            }
        }
    }

    /// Define one expression output type.
    fn define_expression_type(&mut self, id: dir::LocalNodeId<dir::Expression>, term: TypeTerm) {
        let variable = self.intern_local_type_variable(id);
        self.define_type(variable, term);
    }

    /// Return the synthetic catch match failure binding.
    fn catch_match_failure_symbol(
        &self,
        tree: &dir::Tree,
        body: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::GlobalSymbolId> {
        let dir::Expression::Match { value, .. } = tree.get(body) else {
            return None;
        };
        let dir::Expression::Identifier { .. } = tree.get(*value) else {
            return None;
        };

        self.declaration_symbol((*value).into_any())
    }

    /// Return template literal parts.
    fn template_parts(
        &mut self,
        value: &dir::TemplateLiteral,
        tree: &dir::Tree,
    ) -> (Vec<dir::StringId>, Vec<VariableId>) {
        match value {
            // `text`
            dir::TemplateLiteral::String { string } => (vec![*string], Vec::new()),
            // `text ${value}`
            dir::TemplateLiteral::InterpolatedString { strings, arguments } => {
                let spans = self.argument_value_type_variables(arguments, tree);

                (strings.clone(), spans)
            }
        }
    }

    /// Return value type variables for expression arguments.
    pub(in crate::check) fn argument_value_type_variables(
        &mut self,
        arguments: &[dir::LocalNodeId<dir::Argument>],
        tree: &dir::Tree,
    ) -> Vec<VariableId> {
        arguments
            .iter()
            .filter_map(|argument| tree.get(*argument).value())
            .map(|value| self.intern_local_type_variable(value))
            .collect()
    }

    /// Record all expression writes inside one assignment pattern.
    fn record_assign_pattern_writes(
        &mut self,
        id: dir::LocalNodeId<dir::AssignPattern>,
        tree: &dir::Tree,
    ) {
        match tree.get(id) {
            // target
            dir::AssignPattern::Expression { value } => {
                if let Some(place) = self.resolve_place(*value, tree) {
                    self.clear_written_expression_narrowings(tree, *value);
                    self.mark_place_assigned(place);
                    self.require_writable_place(place);
                }
            }
            // target = value
            dir::AssignPattern::Assign { pattern, .. } => {
                self.record_assign_pattern_writes(*pattern, tree);
            }
            // [a, b]
            dir::AssignPattern::Sequence { fields }
            // { a, b }
            | dir::AssignPattern::Object { fields } => {
                for field in fields {
                    self.record_assign_pattern_field_writes(*field, tree);
                }
            }
        }
    }

    /// Record all expression writes inside one assignment pattern field.
    fn record_assign_pattern_field_writes(
        &mut self,
        id: dir::LocalNodeId<dir::AssignPatternField>,
        tree: &dir::Tree,
    ) {
        match tree.get(id) {
            // { name: pattern }
            dir::AssignPatternField::Named {
                pattern: Some(pattern),
                ..
            }
            // { ...pattern }
            | dir::AssignPatternField::Spread {
                pattern: Some(pattern),
            } => {
                self.record_assign_pattern_writes(*pattern, tree);
            }
            // { [key]: pattern }
            dir::AssignPatternField::Computed { pattern, .. }
            // [pattern]
            | dir::AssignPatternField::Positional { pattern } => {
                self.record_assign_pattern_writes(*pattern, tree);
            }
            // { name }
            dir::AssignPatternField::Named { pattern: None, .. }
            // { ... }
            | dir::AssignPatternField::Spread { pattern: None }
            // [,]
            | dir::AssignPatternField::Elision => {}
        }
    }

    /// Walk one labeled expression target.
    fn walk_label_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        label: dir::StringId,
        body: dir::LocalNodeId<dir::Expression>,
    ) {
        // attach loop labels directly to their control target
        if Self::expression_allows_continue_label(tree.get(body)) {
            let term = TypeTerm::Variable(self.intern_local_type_variable(body));

            self.define_expression_type(id, term);
            self.walk_loop_expression_with_label(tree, body, label);

            return;
        }

        // enter labeled control target
        let result = self.intern_local_type_variable(id);
        let origin = ConstraintOrigin::Node(id.into_global_any(self.input.module_id));
        self.enter_control_target(origin, Some(label), false, result);

        // walk body with isolated flow
        let before_body = self.checkpoint_flow();
        self.walk_expression(tree, body, tree.get(body));
        let fallthrough = if self.expression_can_fall_through(tree, body) {
            Some(TypeTerm::Variable(self.intern_local_type_variable(body)))
        } else {
            None
        };

        // merge fallthrough and break branches
        let body_flow = fallthrough
            .is_some()
            .then(|| self.collect_flow_branch(before_body));
        let mut branches = self.leave_control_target(fallthrough);
        if let Some(body_flow) = body_flow {
            branches.push(body_flow);
        }

        self.merge_flow_branches_from(before_body, &branches);
    }

    /// Walk one labeled loop expression.
    fn walk_loop_expression_with_label(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        label: dir::StringId,
    ) {
        // apply static owner guards
        if !self.static_allows(tree, id.into_any()) {
            return;
        }

        self.visit_any(tree, dir::NodeType::Expression, id.id);

        match tree.get(id) {
            // label: while condition { body }
            dir::Expression::While {
                condition, body, ..
            } => {
                self.intern_local_type_variable(id);
                self.walk_while_expression(tree, id, Some(label), *condition, *body);
            }
            // label: for item of iterator { body }
            dir::Expression::ForEach {
                operator,
                binding,
                iterator,
                body,
                ..
            } => {
                self.intern_local_type_variable(id);
                self.walk_for_each_expression(
                    tree,
                    id,
                    Some(label),
                    *operator,
                    binding,
                    *iterator,
                    *body,
                );
            }
            // label: for (initialization; condition; increment) { body }
            dir::Expression::For {
                initialization,
                condition,
                increment,
                body,
            } => {
                self.intern_local_type_variable(id);
                self.walk_for_expression(
                    tree,
                    id,
                    Some(label),
                    *initialization,
                    *condition,
                    *increment,
                    *body,
                );
            }
            // label: loop { body }
            dir::Expression::Loop { body } => {
                self.intern_local_type_variable(id);
                self.walk_loop_expression(tree, id, Some(label), *body);
            }
            // non loop labels are handled by walk_label_expression
            _ => {}
        }
    }

    /// Walk one if expression with isolated branch flow.
    fn walk_if_expression(
        &mut self,
        tree: &dir::Tree,
        condition: &dir::IfCondition,
        then_expression: dir::LocalNodeId<dir::Expression>,
        else_expression: Option<dir::LocalNodeId<dir::Expression>>,
    ) {
        self.walk_if_condition(tree, condition);
        let before = self.checkpoint_flow();

        self.restore_flow(before);
        self.apply_condition_narrowings(tree, condition, ConditionBranch::True);
        self.walk_expression(tree, then_expression, tree.get(then_expression));
        let then_flow = self.collect_flow_branch(before);
        let then_can_fall_through = self.expression_can_fall_through(tree, then_expression);

        if let Some(else_expression) = else_expression {
            self.restore_flow(before);
            self.apply_condition_narrowings(tree, condition, ConditionBranch::False);
            self.walk_expression(tree, else_expression, tree.get(else_expression));
            let else_flow = self.collect_flow_branch(before);
            let else_can_fall_through = self.expression_can_fall_through(tree, else_expression);

            match (then_can_fall_through, else_can_fall_through) {
                (true, true) => self.merge_flow_branches(before, &then_flow, &else_flow),
                (true, false) => self.apply_flow_branch(before, &then_flow),
                (false, true) => self.apply_flow_branch(before, &else_flow),
                (false, false) => self.restore_flow(before),
            }
        } else {
            self.restore_flow(before);
            self.apply_condition_narrowings(tree, condition, ConditionBranch::False);
            let else_flow = self.collect_flow_branch(before);

            if then_can_fall_through {
                self.merge_flow_branches(before, &else_flow, &then_flow);
            } else {
                self.apply_flow_branch(before, &else_flow);
            }
        }
    }

    /// Walk one if condition.
    fn walk_if_condition(&mut self, tree: &dir::Tree, condition: &dir::IfCondition) {
        match condition {
            // if condition
            dir::IfCondition::Expression { condition } => {
                self.walk_expression(tree, *condition, tree.get(*condition));

                let variable = self.intern_local_type_variable(*condition);
                self.constrain_condition((*condition).into_any(), variable);
            }
            // if let pattern = value
            dir::IfCondition::Let { declarator, .. } => {
                self.walk_declarator(tree, *declarator, tree.get(*declarator));
            }
        }
    }

    /// Apply branch-local narrowings from one condition.
    fn apply_condition_narrowings(
        &mut self,
        tree: &dir::Tree,
        condition: &dir::IfCondition,
        branch: ConditionBranch,
    ) {
        match condition {
            // if condition
            dir::IfCondition::Expression { condition } => {
                self.apply_expression_narrowings(tree, *condition, branch);
            }
            // if let pattern = value
            dir::IfCondition::Let { declarator, .. } if branch == ConditionBranch::True => {
                self.mark_declarator_assigned(tree, tree.get(*declarator));
                self.apply_declarator_pattern_narrowings(tree, *declarator);
            }
            // if let pattern = value
            dir::IfCondition::Let { .. } => {}
        }
    }

    /// Apply branch-local narrowings from one expression condition.
    pub(in crate::check) fn apply_expression_narrowings(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        branch: ConditionBranch,
    ) {
        match tree.get(id) {
            // (value)
            dir::Expression::Parenthesized { expression } => {
                self.apply_expression_narrowings(tree, *expression, branch);
            }
            // !value
            dir::Expression::Unary {
                operator: dir::UnaryOperator::Not,
                right,
            } => {
                self.apply_expression_narrowings(tree, *right, branch.opposite());
            }
            // left && right
            dir::Expression::Binary {
                left,
                operator: dir::BinaryOperator::And,
                right,
            } if branch == ConditionBranch::True => {
                self.apply_expression_narrowings(tree, *left, branch);
                self.apply_expression_narrowings(tree, *right, branch);
            }
            // left || right
            dir::Expression::Binary {
                left,
                operator: dir::BinaryOperator::Or,
                right,
            } if branch == ConditionBranch::False => {
                self.apply_expression_narrowings(tree, *left, branch);
                self.apply_expression_narrowings(tree, *right, branch);
            }
            // left === right
            dir::Expression::Binary {
                left,
                operator,
                right,
            } if Self::is_equality_operator(*operator) => {
                let branch = if Self::is_negative_equality_operator(*operator) {
                    branch.opposite()
                } else {
                    branch
                };

                self.apply_equality_narrowing(tree, *left, *right, branch);
                self.apply_equality_narrowing(tree, *right, *left, branch);
                self.apply_typeof_narrowing(tree, *left, *right, branch);
                self.apply_typeof_narrowing(tree, *right, *left, branch);
            }
            // key in value
            dir::Expression::Binary {
                left,
                operator: dir::BinaryOperator::In,
                right,
            } => {
                self.apply_key_membership_narrowing(tree, *left, *right, branch);
            }
            // value is T
            dir::Expression::Is { value, target_type } => {
                self.apply_is_narrowing(tree, *value, *target_type, branch);
            }
            // value instanceof Target
            dir::Expression::InstanceOf { value, target } => {
                self.apply_instance_narrowing(tree, *value, *target, branch);
            }
            // expressions without flow facts
            _ => {}
        }
    }

    /// Apply one `is` expression narrowing.
    fn apply_is_narrowing(
        &mut self,
        tree: &dir::Tree,
        value: dir::LocalNodeId<dir::Expression>,
        target_type: dir::LocalNodeId<dir::TypeExpression>,
        branch: ConditionBranch,
    ) {
        let Some(path) = self.flow_path(tree, value) else {
            return;
        };
        let target = self.intern_local_type_variable(target_type);

        match branch {
            ConditionBranch::True => {
                self.narrow_flow_path(path, target);
            }
            ConditionBranch::False => {
                let original = self.intern_local_type_variable(value);

                self.narrow_flow_path_excluding(value.into_any(), path, original, target);
            }
        }
    }

    /// Apply one `typeof value == "kind"` expression narrowing.
    fn apply_typeof_narrowing(
        &mut self,
        tree: &dir::Tree,
        value: dir::LocalNodeId<dir::Expression>,
        target: dir::LocalNodeId<dir::Expression>,
        branch: ConditionBranch,
    ) {
        let dir::Expression::Unary {
            operator: dir::UnaryOperator::Typeof,
            right,
        } = tree.get(value)
        else {
            return;
        };
        let Some(path) = self.flow_path(tree, *right) else {
            return;
        };
        let Some(target) = self.typeof_target_type(tree, target) else {
            return;
        };

        match branch {
            ConditionBranch::True => {
                self.narrow_flow_path(path, target);
            }
            ConditionBranch::False => {
                let original = self.intern_local_type_variable(*right);

                self.narrow_flow_path_excluding((*right).into_any(), path, original, target);
            }
        }
    }

    /// Apply one `"key" in value` expression narrowing.
    fn apply_key_membership_narrowing(
        &mut self,
        tree: &dir::Tree,
        key: dir::LocalNodeId<dir::Expression>,
        value: dir::LocalNodeId<dir::Expression>,
        branch: ConditionBranch,
    ) {
        if branch == ConditionBranch::False {
            return;
        }
        let Some(path) = self.flow_path(tree, value) else {
            return;
        };
        let Some(key) = tree.get(key).static_key() else {
            return;
        };
        let source = value.into_global_any(self.input.module_id);
        let origin = ConstraintOrigin::Node(source);
        let ty = self.define_anonymous_type(origin, TypeTerm::Literal(TypeLiteralTerm::Unknown));
        let member = ShapeMemberTerm::Field {
            key,
            ty,
            is_optional: false,
            is_readonly: false,
        };
        let target = self.define_anonymous_type(
            origin,
            TypeTerm::Shape {
                members: vec![member],
            },
        );

        self.narrow_flow_path(path, target);
    }

    /// Apply one `value instanceof Target` expression narrowing.
    fn apply_instance_narrowing(
        &mut self,
        tree: &dir::Tree,
        value: dir::LocalNodeId<dir::Expression>,
        target: dir::LocalNodeId<dir::Expression>,
        branch: ConditionBranch,
    ) {
        let Some(path) = self.flow_path(tree, value) else {
            return;
        };
        let target = self.intern_local_type_variable(target);

        match branch {
            ConditionBranch::True => {
                self.narrow_flow_path(path, target);
            }
            ConditionBranch::False => {
                let original = self.intern_local_type_variable(value);

                self.narrow_flow_path_excluding(value.into_any(), path, original, target);
            }
        }
    }

    /// Apply one equality expression narrowing.
    fn apply_equality_narrowing(
        &mut self,
        tree: &dir::Tree,
        value: dir::LocalNodeId<dir::Expression>,
        target: dir::LocalNodeId<dir::Expression>,
        branch: ConditionBranch,
    ) {
        let Some(path) = self.flow_path(tree, value) else {
            return;
        };
        let Some(target) = self.equality_target_type(tree, target) else {
            return;
        };

        match branch {
            ConditionBranch::True => {
                self.narrow_flow_path(path, target);
            }
            ConditionBranch::False => {
                let original = self.intern_local_type_variable(value);

                self.narrow_flow_path_excluding(value.into_any(), path, original, target);
            }
        }
    }

    /// Return the literal type used by one equality test.
    fn equality_target_type(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<VariableId> {
        let source = id.into_global_any(self.input.module_id);
        let origin = ConstraintOrigin::Node(source);
        let term = match tree.get(id) {
            // literal
            dir::Expression::ScalarLiteral(value) => {
                TypeTerm::Literal(TypeLiteralTerm::Scalar(value.clone()))
            }
            // undefined
            dir::Expression::Identifier { name }
                if *name == dir::StringId::for_text("undefined") =>
            {
                TypeTerm::Literal(TypeLiteralTerm::Undefined)
            }
            // unsupported equality target
            _ => return None,
        };

        Some(self.define_anonymous_type(origin, term))
    }

    /// Return the primitive type named by one `typeof` result string.
    fn typeof_target_type(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<VariableId> {
        let dir::Expression::ScalarLiteral(dir::ScalarLiteral::String(value)) = tree.get(id) else {
            return None;
        };
        let text = self.input.strings.get(*value);
        let literal = match text {
            // typeof value == "undefined"
            "undefined" => TypeLiteralTerm::Undefined,
            // typeof value == "boolean"
            "boolean" => TypeLiteralTerm::boolean(),
            // typeof value == "string"
            "string" => TypeLiteralTerm::Primitive(dir::PrimitiveType::String),
            // typeof value == "number"
            "number" => TypeLiteralTerm::number(),
            // typeof value == "bigint"
            "bigint" => TypeLiteralTerm::bigint(),
            // typeof value == "symbol"
            "symbol" => TypeLiteralTerm::Primitive(dir::PrimitiveType::Symbol),
            // unsupported typeof string
            _ => return None,
        };
        let source = id.into_global_any(self.input.module_id);
        let origin = ConstraintOrigin::Node(source);
        let term = TypeTerm::Literal(literal);

        Some(self.define_anonymous_type(origin, term))
    }

    /// Return whether one binary operator tests equality.
    fn is_equality_operator(operator: dir::BinaryOperator) -> bool {
        matches!(
            operator,
            dir::BinaryOperator::Equal
                | dir::BinaryOperator::NotEqual
                | dir::BinaryOperator::EqualStrict
                | dir::BinaryOperator::NotEqualStrict
        )
    }

    /// Return whether one equality operator negates the relation.
    fn is_negative_equality_operator(operator: dir::BinaryOperator) -> bool {
        matches!(
            operator,
            dir::BinaryOperator::NotEqual | dir::BinaryOperator::NotEqualStrict
        )
    }

    /// Walk one while expression.
    fn walk_while_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        label: Option<dir::StringId>,
        condition: dir::LocalNodeId<dir::Expression>,
        body: dir::LocalNodeId<dir::Block>,
    ) {
        // walk condition in incoming flow
        self.walk_expression(tree, condition, tree.get(condition));
        let condition_type = self.intern_local_type_variable(condition);
        self.constrain_condition(condition.into_any(), condition_type);

        // enter loop control target
        let origin = ConstraintOrigin::Node(id.into_global_any(self.input.module_id));
        let result = self.intern_local_type_variable(id);
        self.enter_control_target(origin, label, true, result);

        // walk body under true condition facts
        let before_body = self.checkpoint_flow();
        self.apply_expression_narrowings(tree, condition, ConditionBranch::True);
        self.walk_block(tree, body, tree.get(body));
        self.restore_flow(before_body);

        // collect normal exit through false condition
        self.apply_expression_narrowings(tree, condition, ConditionBranch::False);
        let normal_flow = self.collect_flow_branch(before_body);
        let mut branches =
            self.leave_control_target(Some(TypeTerm::Literal(TypeLiteralTerm::Void)));

        // merge break branches with normal exit
        branches.push(normal_flow);
        self.merge_flow_branches_from(before_body, &branches);
    }

    /// Walk one for each expression.
    fn walk_for_each_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        label: Option<dir::StringId>,
        operator: dir::ForEachOperator,
        binding: &dir::ForEachBinding,
        iterator: dir::LocalNodeId<dir::Expression>,
        body: dir::LocalNodeId<dir::Block>,
    ) {
        // walk binding and iterator in incoming flow
        let pattern = match binding {
            dir::ForEachBinding::Pattern { pattern, .. }
            | dir::ForEachBinding::Using { pattern, .. } => *pattern,
        };
        self.walk_pattern(tree, pattern, tree.get(pattern));
        self.walk_expression(tree, iterator, tree.get(iterator));
        self.constrain_for_each_binding(id, operator, pattern, iterator, tree);

        // enter loop control target
        let origin = ConstraintOrigin::Node(id.into_global_any(self.input.module_id));
        let result = self.intern_local_type_variable(id);
        self.enter_control_target(origin, label, true, result);

        // walk body with iteration binding assigned
        let before_body = self.checkpoint_flow();
        self.mark_bindings_assigned(tree, pattern.into_any());
        self.walk_block(tree, body, tree.get(body));
        self.restore_flow(before_body);

        // collect normal loop exit
        let normal_flow = self.collect_flow_branch(before_body);
        let mut branches =
            self.leave_control_target(Some(TypeTerm::Literal(TypeLiteralTerm::Void)));

        // merge break branches with normal exit
        branches.push(normal_flow);
        self.merge_flow_branches_from(before_body, &branches);
    }

    /// Walk one traditional for expression.
    fn walk_for_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        label: Option<dir::StringId>,
        initialization: Option<dir::LocalNodeId<dir::Expression>>,
        condition: Option<dir::LocalNodeId<dir::Expression>>,
        increment: Option<dir::LocalNodeId<dir::Expression>>,
        body: dir::LocalNodeId<dir::Block>,
    ) {
        // walk initialization before loop flow splits
        if let Some(initialization) = initialization {
            self.walk_expression(tree, initialization, tree.get(initialization));
        }

        // walk condition in incoming flow
        if let Some(condition) = condition {
            self.walk_expression(tree, condition, tree.get(condition));

            let variable = self.intern_local_type_variable(condition);
            self.constrain_condition(condition.into_any(), variable);
        }

        // enter loop control target
        let origin = ConstraintOrigin::Node(id.into_global_any(self.input.module_id));
        let result = self.intern_local_type_variable(id);
        self.enter_control_target(origin, label, true, result);

        // walk body under true condition facts
        let before_body = self.checkpoint_flow();
        if let Some(condition) = condition {
            self.apply_expression_narrowings(tree, condition, ConditionBranch::True);
        }
        self.walk_block(tree, body, tree.get(body));
        let body_flow = self
            .block_can_fall_through(tree, tree.get(body))
            .then(|| self.collect_flow_branch(before_body));
        let continue_flows = self.take_current_continue_branches();
        if let Some(increment) = increment {
            self.walk_for_increment_expression(
                tree,
                increment,
                before_body,
                body_flow,
                &continue_flows,
            );
        }
        self.restore_flow(before_body);

        // collect normal exit through false condition
        let normal_flow = condition.map(|condition| {
            self.apply_expression_narrowings(tree, condition, ConditionBranch::False);

            self.collect_flow_branch(before_body)
        });
        let fallthrough = condition.map(|_| TypeTerm::Literal(TypeLiteralTerm::Void));
        let mut branches = self.leave_control_target(fallthrough);

        // merge break branches with normal exit
        if let Some(normal_flow) = normal_flow {
            branches.push(normal_flow);
        }

        self.merge_flow_branches_from(before_body, &branches);
    }

    /// Walk one traditional for increment from body and continue flows.
    fn walk_for_increment_expression(
        &mut self,
        tree: &dir::Tree,
        increment: dir::LocalNodeId<dir::Expression>,
        before_body: FlowCheckpoint,
        body_flow: Option<FlowBranch>,
        continue_flows: &[FlowBranch],
    ) {
        let mut flows = Vec::with_capacity(continue_flows.len() + usize::from(body_flow.is_some()));
        flows.extend_from_slice(continue_flows);
        if let Some(body_flow) = body_flow {
            flows.push(body_flow);
        }

        // check unreachable increment once
        if flows.is_empty() {
            self.walk_expression(tree, increment, tree.get(increment));
            self.restore_flow(before_body);

            return;
        }

        // check increment from each flow that reaches the next iteration
        for flow in flows {
            self.apply_flow_branch(before_body, &flow);
            self.walk_expression(tree, increment, tree.get(increment));
            self.restore_flow(before_body);
        }
    }

    /// Constrain one for each binding to its iterated value.
    fn constrain_for_each_binding(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        operator: dir::ForEachOperator,
        pattern: dir::LocalNodeId<dir::Pattern>,
        iterator: dir::LocalNodeId<dir::Expression>,
        tree: &dir::Tree,
    ) {
        let source = id.into_global_any(self.input.module_id);
        let origin = ConstraintOrigin::Node(source);
        let iterator_type = self.intern_local_type_variable(iterator);
        let value = match operator {
            // for (const item of iterable)
            dir::ForEachOperator::Of => {
                let value = self.allocate_anonymous_variable(VariableKind::Type, origin);
                let unknown =
                    self.define_anonymous_type(origin, TypeTerm::Literal(TypeLiteralTerm::Unknown));
                let Some(symbol) = self.language_symbol(dir::LanguageItem::Iterable) else {
                    self.report_internal_error(
                        id.into_any(),
                        "missing language item: iter.Iterable".to_owned(),
                    );

                    return;
                };
                let iterable = TypeTerm::Reference {
                    source: Some(source),
                    symbol,
                    arguments: vec![
                        ArgumentTerm::Type(value),
                        ArgumentTerm::Type(unknown),
                        ArgumentTerm::Type(unknown),
                    ],
                };
                let iterable = self.define_anonymous_type(origin, iterable);

                self.constrain_type(origin, TypeRelation::Assignable, iterator_type, iterable);

                value
            }
            // for (const key in object)
            dir::ForEachOperator::In => {
                let term = TypeTerm::Operation(TypeOperationTerm::KeyOf {
                    target: iterator_type,
                });

                self.define_anonymous_type(origin, term)
            }
        };

        if let Some(term) = self.build_pattern_term(pattern, tree) {
            self.constrain_pattern(PatternRelation::Match(term), pattern.into_any(), value);
        }
    }

    /// Walk one loop expression.
    fn walk_loop_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        label: Option<dir::StringId>,
        body: dir::LocalNodeId<dir::Block>,
    ) {
        // enter loop control target
        let origin = ConstraintOrigin::Node(id.into_global_any(self.input.module_id));
        let result = self.intern_local_type_variable(id);
        self.enter_control_target(origin, label, true, result);

        // walk body with isolated flow
        let before_body = self.checkpoint_flow();
        self.walk_block(tree, body, tree.get(body));
        self.restore_flow(before_body);

        // restore only branches that leave the loop
        let branches = self.leave_control_target(None);
        self.merge_flow_branches_from(before_body, &branches);
    }

    /// Walk one try expression with branch flow for catch.
    fn walk_try_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        body: dir::LocalNodeId<dir::Expression>,
        catch: Option<dir::LocalNodeId<dir::Catch>>,
        finally: Option<dir::LocalNodeId<dir::Expression>>,
    ) {
        let before = self.checkpoint_flow();

        // walk the body branch from the incoming facts
        self.restore_flow(before);
        if catch.is_some() {
            self.enter_try_target(id.into_any());
        }
        self.walk_expression(tree, body, tree.get(body));
        let catch_failure = catch.and_then(|_| self.leave_try_target());
        let body_flow = self.collect_flow_branch(before);
        let body_can_fall_through = self.expression_can_fall_through(tree, body);

        // merge only branches that can continue normally
        let has_normal_flow = if let Some(catch) = catch {
            let catch_can_fall_through = self.catch_can_fall_through(tree, tree.get(catch));

            self.restore_flow(before);
            self.walk_catch(tree, catch, tree.get(catch), catch_failure);
            let catch_flow = self.collect_flow_branch(before);

            match (body_can_fall_through, catch_can_fall_through) {
                (true, true) => {
                    self.merge_flow_branches(before, &body_flow, &catch_flow);

                    true
                }
                (true, false) => {
                    self.apply_flow_branch(before, &body_flow);

                    true
                }
                (false, true) => {
                    self.apply_flow_branch(before, &catch_flow);

                    true
                }
                (false, false) => {
                    self.restore_flow(before);

                    false
                }
            }
        } else if body_can_fall_through {
            self.apply_flow_branch(before, &body_flow);

            true
        } else {
            self.restore_flow(before);

            false
        };

        // finally is checked even when no normal path remains
        if let Some(finally) = finally {
            self.walk_expression(tree, finally, tree.get(finally));

            if !has_normal_flow || !self.expression_can_fall_through(tree, finally) {
                self.restore_flow(before);
            }
        } else if !has_normal_flow {
            self.restore_flow(before);
        }
    }

    /// Walk one match expression with isolated case flow.
    fn walk_match_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        value: dir::LocalNodeId<dir::Expression>,
        cases: &[dir::LocalNodeId<dir::MatchCase>],
    ) {
        self.walk_expression(tree, value, tree.get(value));
        let value_type = self.intern_local_type_variable(value);
        let value_path = self.flow_path(tree, value);

        self.require_exhaustive_match(id.into_any(), value_type, cases);

        let before = self.checkpoint_flow();
        let mut merged = None;

        for case in cases {
            self.restore_flow(before);
            self.walk_match_case(
                tree,
                *case,
                tree.get(*case),
                Some((value_type, value_path.clone())),
            );
            if !self.match_case_can_fall_through(tree, tree.get(*case)) {
                continue;
            }
            let case_flow = self.collect_flow_branch(before);

            if let Some(previous) = &merged {
                self.merge_flow_branches(before, previous, &case_flow);
                merged = Some(self.collect_flow_branch(before));
            } else {
                merged = Some(case_flow);
            }
        }

        if let Some(merged) = merged {
            self.apply_flow_branch(before, &merged);
        } else {
            self.restore_flow(before);
        }
    }

    /// Return whether one block can fall through normally.
    pub(in crate::check) fn block_can_fall_through(
        &self,
        tree: &dir::Tree,
        block: &dir::Block,
    ) -> bool {
        for expression in &block.leading_expressions {
            if !self.expression_can_fall_through(tree, *expression) {
                return false;
            }
        }

        match block.tail_expression {
            Some(expression) => self.expression_can_fall_through(tree, expression),
            None => true,
        }
    }

    /// Return whether one expression can fall through normally.
    pub(in crate::check) fn expression_can_fall_through(
        &self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        match tree.get(id) {
            // return, break, continue, throw
            dir::Expression::Return { .. }
            | dir::Expression::Break { .. }
            | dir::Expression::Continue { .. }
            | dir::Expression::Throw { .. } => false,
            // { ... }
            dir::Expression::Block(block) => self.block_can_fall_through(tree, tree.get(*block)),
            // if condition { then } else { otherwise }
            dir::Expression::If {
                then_expression,
                else_expression: Some(else_expression),
                ..
            } => {
                self.expression_can_fall_through(tree, *then_expression)
                    || self.expression_can_fall_through(tree, *else_expression)
            }
            dir::Expression::If {
                else_expression: None,
                ..
            } => true,
            // match value { case pattern => body }
            dir::Expression::Match { cases, .. } => cases
                .iter()
                .any(|case| self.match_case_can_fall_through(tree, tree.get(*case))),
            // try body catch error finally cleanup
            dir::Expression::Try {
                body,
                catch,
                finally,
            } => {
                let body = self.expression_can_fall_through(tree, *body);
                let catch = if let Some(catch) = catch {
                    self.catch_can_fall_through(tree, tree.get(*catch))
                } else {
                    true
                };
                let finally = if let Some(finally) = finally {
                    self.expression_can_fall_through(tree, *finally)
                } else {
                    true
                };

                finally && (body || catch)
            }
            // expressions that do not force control transfer by syntax
            dir::Expression::Declaration(_)
            | dir::Expression::Label { .. }
            | dir::Expression::Import { .. }
            | dir::Expression::Export { .. }
            | dir::Expression::Let { .. }
            | dir::Expression::LetElse { .. }
            | dir::Expression::Using { .. }
            | dir::Expression::While { .. }
            | dir::Expression::ForEach { .. }
            | dir::Expression::For { .. }
            | dir::Expression::Loop { .. }
            | dir::Expression::Await { .. }
            | dir::Expression::Yield { .. }
            | dir::Expression::Identifier { .. }
            | dir::Expression::This
            | dir::Expression::ScalarLiteral(_)
            | dir::Expression::PrivateIdentifier { .. }
            | dir::Expression::Super
            | dir::Expression::ImportMeta
            | dir::Expression::Debugger
            | dir::Expression::Missing
            | dir::Expression::Stub
            | dir::Expression::Error
            | dir::Expression::QualifiedReference { .. }
            | dir::Expression::RangeExpression { .. }
            | dir::Expression::TemplateExpression { .. }
            | dir::Expression::TaggedTemplateExpression { .. }
            | dir::Expression::ArrayExpression { .. }
            | dir::Expression::FixedArrayExpression { .. }
            | dir::Expression::TupleExpression { .. }
            | dir::Expression::SequenceExpression { .. }
            | dir::Expression::ObjectExpression { .. }
            | dir::Expression::StructExpression { .. }
            | dir::Expression::TreeExpression { .. }
            | dir::Expression::Parenthesized { .. }
            | dir::Expression::Type { .. }
            | dir::Expression::Comptime { .. }
            | dir::Expression::As { .. }
            | dir::Expression::Satisfies { .. }
            | dir::Expression::Is { .. }
            | dir::Expression::InstanceOf { .. }
            | dir::Expression::Unary { .. }
            | dir::Expression::MoveOf { .. }
            | dir::Expression::BorrowOf { .. }
            | dir::Expression::Member { .. }
            | dir::Expression::PrivateMember { .. }
            | dir::Expression::Index { .. }
            | dir::Expression::Instantiation { .. }
            | dir::Expression::Call { .. }
            | dir::Expression::New { .. }
            | dir::Expression::AwaitMaybe { .. }
            | dir::Expression::AwaitMust { .. }
            | dir::Expression::Maybe { .. }
            | dir::Expression::Must { .. }
            | dir::Expression::Binary { .. }
            | dir::Expression::Assign { .. } => true,
        }
    }

    /// Return whether one match case can fall through normally.
    fn match_case_can_fall_through(&self, tree: &dir::Tree, case: &dir::MatchCase) -> bool {
        match case {
            // case pattern => expression
            dir::MatchCase::Expression { body, .. } => {
                self.expression_can_fall_through(tree, *body)
            }
            // case pattern => { ... }
            dir::MatchCase::Block { body, .. } => {
                self.block_can_fall_through(tree, tree.get(*body))
            }
        }
    }

    /// Return whether one catch body can fall through normally.
    fn catch_can_fall_through(&self, tree: &dir::Tree, catch: &dir::Catch) -> bool {
        self.expression_can_fall_through(tree, catch.body)
    }

    /// Return whether one labeled expression can receive `continue`.
    fn expression_allows_continue_label(expression: &dir::Expression) -> bool {
        matches!(
            expression,
            dir::Expression::While { .. }
                | dir::Expression::ForEach { .. }
                | dir::Expression::For { .. }
                | dir::Expression::Loop { .. }
        )
    }
}

impl ConditionBranch {
    /// Return the opposite control branch.
    fn opposite(self) -> Self {
        match self {
            // true branch
            Self::True => Self::False,
            // false branch
            Self::False => Self::True,
        }
    }
}
