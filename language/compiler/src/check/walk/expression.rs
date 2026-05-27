use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{
    AwaitTerm, CallCandidate, CallTerm, CheckState, ConstraintOrigin, ConstructTerm, FlowBranch,
    FlowCheckpoint, FormTerm, GenericArgument, IdentityTerm, ImportMetaTerm, IndexSetTerm,
    IndexTerm, InstanceCheckTerm, KeyMembershipTerm, MemberCallTerm, MemberTerm, OperatorTerm,
    OperatorTermKind, PatternRelation, RangeValueTerm, ShapeMember, StaticTerm, SuperTerm,
    TaggedTemplateTerm, TemplateTerm, TreeTerm, TryTerm, TryTermKind, TupleElement,
    TypeLiteralTerm, TypeOperand, TypeOperationTerm, TypeRelation, TypeTerm, TypeValueTerm,
    VariableId, VariableKind, YieldTerm,
};

/// The condition branch being entered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum ConditionBranch {
    /// The condition is known true.
    True,
    /// The condition is known false.
    False,
}

impl CheckState<'_> {
    /// Walk one expression.
    pub(in crate::check) fn walk_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        if !self.push_static_condition_for(tree, id.into_any(), None) {
            return;
        }
        match expression {
            // function f() {}
            dir::Expression::Declaration(declaration) => {
                if tree.get(*declaration).symbol_kind().is_some() {
                    if let Some(symbol) = self.declaration_symbol(tree.module_id, (*declaration).into_any()) {
                        let term = TypeTerm::Variable(self.intern_symbol_type_variable(tree.module_id, symbol));

                        self.define_expression_type(tree.module_id, id, term);
                    }
                }

                self.walk_declaration(tree, *declaration, tree.get(*declaration));
            }
            // { ... }
            dir::Expression::Block(block) => {
                let term = TypeTerm::Variable(self.intern_local_type_variable(tree.module_id, *block));

                self.define_expression_type(tree.module_id, id, term);
                self.walk_block(tree, *block, tree.get(*block));
            }
            // label: body
            dir::Expression::Label { label, body } => {
                self.walk_label_expression(tree, id, *label, *body);
            }
            // import { item } from "module"
            dir::Expression::Import { items, .. } => {
                self.define_expression_type(tree.module_id, id, TypeTerm::Literal(TypeLiteralTerm::Void));

                if let Some(items) = items {
                    for item in items {
                        self.walk_dependency_item(tree, *item, tree.get(*item));
                    }
                }
            }
            // export { item } from "module"
            dir::Expression::Export { items, .. } => {
                self.define_expression_type(tree.module_id, id, TypeTerm::Literal(TypeLiteralTerm::Void));

                for item in items {
                    self.walk_dependency_item(tree, *item, tree.get(*item));
                }
            }
            // let x = value
            dir::Expression::Let { declarators, .. }
            // using x = value
            | dir::Expression::Using { declarators, .. } => {
                self.define_expression_type(tree.module_id, id, TypeTerm::Literal(TypeLiteralTerm::Void));

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
                self.define_expression_type(tree.module_id, id, TypeTerm::Literal(TypeLiteralTerm::Void));
                self.walk_declarator(tree, *declarator, tree.get(*declarator));

                let before_else = self.checkpoint_flow(tree.module_id);
                self.walk_expression(tree, *else_branch, tree.get(*else_branch));
                if self.expression_can_fall_through(tree, *else_branch) {
                    self.report_invalid_control_flow(
                        tree.module_id,
                        (*else_branch).into_any(),
                        "let else fallback must not fall through",
                    );
                }
                self.restore_flow(tree.module_id, before_else);
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
                let then_type = self.intern_local_type_variable(tree.module_id, *then_expression);
                let else_type = match else_expression {
                    Some(else_expression) => self.intern_local_type_variable(tree.module_id, *else_expression),
                    None => self.define_void_type(tree.module_id, id.into_any()),
                };
                let operation = self.terms.push(TypeOperationTerm::BestCommon {
                    elements: vec![then_type.into(), else_type.into()],
                });
                let term = TypeTerm::Operation(operation);

                self.define_expression_type(tree.module_id, id, term);
                self.walk_if_expression(tree, condition, *then_expression, *else_expression);
            }
            // while condition { body }
            dir::Expression::While {
                condition, body, ..
            } => {
                self.intern_local_type_variable(tree.module_id, id);
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
                self.intern_local_type_variable(tree.module_id, id);
                self.walk_for_each_expression(tree, id, None, *operator, binding, *iterator, *body);
            }
            // for (initialization; condition; increment) { body }
            dir::Expression::For {
                initialization,
                condition,
                increment,
                body,
            } => {
                self.intern_local_type_variable(tree.module_id, id);
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
                self.intern_local_type_variable(tree.module_id, id);
                self.walk_loop_expression(tree, id, None, *body);
            }
            // try body catch error finally cleanup
            dir::Expression::Try {
                body,
                catch,
                finally,
            } => {
                let body_type = self.intern_local_type_variable(tree.module_id, *body);
                let term = match catch {
                    Some(catch) => {
                        let catch_type = self.intern_local_type_variable(tree.module_id, tree.get(*catch).body);

                        let operation = self.terms.push(TypeOperationTerm::BestCommon {
                            elements: vec![body_type.into(), catch_type.into()],
                        });

                        TypeTerm::Operation(operation)
                    }
                    None => TypeTerm::Variable(body_type),
                };

                self.define_expression_type(tree.module_id, id, term);
                self.walk_try_expression(tree, id, *body, *catch, *finally);
            }
            // match value { case pattern => body }
            dir::Expression::Match { value, cases, .. } => {
                let elements = cases
                    .iter()
                    .map(|case| match tree.get(*case) {
                        // case pattern => expression
                        dir::MatchCase::Expression { body, .. } => {
                            self.intern_local_type_variable(tree.module_id, *body)
                        }
                        // case pattern => { ... }
                        dir::MatchCase::Block { body, .. } => {
                            self.intern_local_type_variable(tree.module_id, *body)
                        }
                    })
                    .map(TypeOperand::from)
                    .collect();
                let operation = self.terms.push(TypeOperationTerm::BestCommon { elements });
                let term = TypeTerm::Operation(operation);

                self.define_expression_type(tree.module_id, id, term);
                self.walk_match_expression(tree, id, *value, cases);
            }
            // break value
            dir::Expression::Break { label, value } => {
                self.define_expression_type(tree.module_id, id, TypeTerm::Literal(TypeLiteralTerm::Never));

                let value = if let Some(value) = value {
                    self.walk_expression(tree, *value, tree.get(*value));

                    Some(self.intern_local_type_variable(tree.module_id, *value))
                } else {
                    None
                };

                self.record_break_value(tree.module_id, id.into_any(), *label, value);
            }
            // continue
            dir::Expression::Continue { label } => {
                self.define_expression_type(tree.module_id, id, TypeTerm::Literal(TypeLiteralTerm::Never));
                self.record_continue_branch(tree.module_id, id.into_any(), *label);
            }
            // await value
            dir::Expression::Await {
                expression: awaited,
            } => {
                self.validate_await_context(tree.module_id, id.into_any());

                let value = self.intern_local_type_variable(tree.module_id, *awaited);
                let await_term = self.terms.push(AwaitTerm {
                    source: id.into_global_any(tree.module_id),
                    value,
                });
                let term = TypeTerm::Await(await_term);

                self.define_expression_type(tree.module_id, id, term);
                self.walk_expression(tree, *awaited, tree.get(*awaited));
            }
            // throw value
            dir::Expression::Throw { value } => {
                self.define_expression_type(tree.module_id, id, TypeTerm::Literal(TypeLiteralTerm::Never));
                self.walk_expression(tree, *value, tree.get(*value));
            }
            // return value
            dir::Expression::Return { value } => {
                self.define_expression_type(tree.module_id, id, TypeTerm::Literal(TypeLiteralTerm::Never));

                if let Some(value) = value {
                    self.walk_expression(tree, *value, tree.get(*value));

                    let value = self.intern_local_type_variable(tree.module_id, *value);
                    self.constrain_return_value(tree.module_id, id.into_any(), value);
                } else {
                    self.constrain_void_return(tree.module_id, id.into_any());
                }
            }
            // yield value
            dir::Expression::Yield { cardinality, value } => {
                let source = id.into_global_any(tree.module_id);
                let origin = ConstraintOrigin::Node(source);
                let value_type = value.map(|value| self.intern_local_type_variable(tree.module_id, value));
                let delegate_return_type = if *cardinality == dir::YieldCardinality::Generator {
                    Some(self.allocate_intermediate_variable(tree.module_id, VariableKind::Type, origin))
                } else {
                    None
                };
                let yield_term = self.terms.push(YieldTerm {
                    source,
                    value: value_type,
                    yield_type: self.current_yield_type(tree.module_id),
                    resume_type: self.current_resume_type(tree.module_id),
                    delegate_return_type,
                    cardinality: *cardinality,
                });
                let term = TypeTerm::Yield(yield_term);

                self.define_expression_type(tree.module_id, id, term);
                self.constrain_yield_value(
                    tree.module_id,
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
                    self.define_expression_type(tree.module_id, id, term);
                }
            }
            // this
            dir::Expression::This => {
                if let Some(receiver) =
                    self.define_this_receiver_variable(id.into_global_any(tree.module_id))
                {
                    self.define_expression_type(tree.module_id, id, TypeTerm::Variable(receiver));
                }
            }
            // 1, "text", true
            dir::Expression::ScalarLiteral(value) => {
                let term = TypeTerm::Literal(TypeLiteralTerm::Scalar(value.clone()));

                self.define_expression_type(tree.module_id, id, term);
            }
            // super
            dir::Expression::Super => {
                let source = id.into_global_any(tree.module_id);
                let receiver = self
                    .resolve_this_receiver(source)
                    .map(|receiver| receiver.ty);
                let super_term = self.terms.push(SuperTerm { source, receiver });
                let term = TypeTerm::Super(super_term);

                self.define_expression_type(tree.module_id, id, term);
            }
            // import.meta
            dir::Expression::ImportMeta => {
                let import_meta = self.terms.push(ImportMetaTerm {
                    source: id.into_global_any(tree.module_id),
                });
                let term = TypeTerm::ImportMeta(import_meta);

                self.define_expression_type(tree.module_id, id, term);
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
                    self.define_expression_type(tree.module_id, id, term);
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
                let start_variable =
                    start.map(|start| self.intern_local_type_variable(tree.module_id, start));
                let end_variable =
                    end.map(|end| self.intern_local_type_variable(tree.module_id, end));
                let range = self.terms.push(RangeValueTerm {
                    source: id.into_global_any(tree.module_id),
                    start: start_variable,
                    end: end_variable,
                    end_kind: *end_kind,
                });
                let term = TypeTerm::RangeValue(range);

                self.define_expression_type(tree.module_id, id, term);

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
                let template = self.terms.push(TemplateTerm {
                    source: id.into_global_any(tree.module_id),
                    strings,
                    spans,
                });
                let term = TypeTerm::Template(template);

                self.define_expression_type(tree.module_id, id, term);
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
                let tag_variable = self.intern_local_type_variable(tree.module_id, *tag);
                let template = self.terms.push(TaggedTemplateTerm {
                    source: id.into_global_any(tree.module_id),
                    tag: tag_variable,
                    generic_arguments: generic_arguments_term,
                    strings,
                    spans,
                });
                let term = TypeTerm::TaggedTemplate(template);

                self.define_expression_type(tree.module_id, id, term);
                self.walk_expression(tree, *tag, tree.get(*tag));

                for argument in generic_arguments {
                    self.walk_generic_argument(tree, *argument, tree.get(*argument));
                }

                self.walk_template_literal(tree, value);
            }
            // [a, b, c]
            dir::Expression::ArrayExpression { elements } => {
                let origin = ConstraintOrigin::Node(id.into_global_any(tree.module_id));
                let element_types = self.argument_value_type_variables(elements, tree);
                let element = self.allocate_intermediate_variable(tree.module_id, VariableKind::Type, origin);
                let length = dir::StaticTerm::ScalarLiteral {
                    value: dir::ScalarLiteral::Integer(element_types.len() as i64),
                };
                let length_variable =
                    self.allocate_intermediate_variable(tree.module_id, VariableKind::Static, origin);
                self.define_static(tree.module_id, length_variable, StaticTerm::Literal(length));
                let operation = self.terms.push(TypeOperationTerm::BestCommon {
                    elements: element_types.into_iter().map(TypeOperand::from).collect(),
                });
                let term = TypeTerm::Operation(operation);

                self.define_type(tree.module_id, element, term);

                let term = TypeTerm::FixedArray {
                    element: element.into(),
                    length: length_variable,
                    is_readonly: false,
                };

                self.define_expression_type(tree.module_id, id, term);

                for element in elements {
                    self.walk_argument(tree, *element, tree.get(*element));
                }
            }
            // [value; length]
            dir::Expression::FixedArrayExpression { value, length } => {
                let term = TypeTerm::FixedArray {
                    element: self.intern_local_type_variable(tree.module_id, *value).into(),
                    length: self.define_static_expression_variable(tree.module_id, *length),
                    is_readonly: false,
                };

                self.define_expression_type(tree.module_id, id, term);
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

                    let element = TupleElement {
                        label,
                        ty: self.intern_local_type_variable(tree.module_id, value).into(),
                        is_optional: false,
                        is_readonly: false,
                        is_rest: matches!(argument, dir::Argument::Spread { .. }),
                    };

                    elements_term.push(element);
                }
                let term = TypeTerm::Tuple {
                    form: dir::TupleForm::Tuple,
                    elements: elements_term.into(),
                    is_readonly: false,
                };

                self.define_expression_type(tree.module_id, id, term);

                for element in elements {
                    self.walk_argument(tree, *element, tree.get(*element));
                }
            }
            // a, b, c
            dir::Expression::SequenceExpression { expressions } => {
                let term = match expressions.last() {
                    Some(expression) => {
                        TypeTerm::Variable(self.intern_local_type_variable(tree.module_id, *expression))
                    }
                    None => TypeTerm::Literal(TypeLiteralTerm::Void),
                };

                self.define_expression_type(tree.module_id, id, term);

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

                            let member = ShapeMember::Field {
                                key,
                                ty: self.intern_local_type_variable(tree.module_id, *value).into(),
                                is_optional: false,
                                is_readonly: false,
                            };

                            members.push(member);
                        }
                        // { method() {} }
                        dir::Property::Method { key: Some(key), .. } => {
                            let Some(key) = key.static_key(tree) else {
                                continue;
                            };
                            let ty = self
                                .declaration_symbol(tree.module_id, (*property).into_any())
                                .map(|symbol| self.intern_symbol_type_variable(tree.module_id, symbol))
                                .unwrap_or_else(|| self.intern_local_type_variable(tree.module_id, *property));

                            let member = ShapeMember::Field {
                                key,
                                ty: ty.into(),
                                is_optional: false,
                                is_readonly: false,
                            };

                            members.push(member);
                        }
                        // { []() {} }
                        dir::Property::Method { key: None, .. } => {}
                        // { ...value }
                        dir::Property::Spread { .. } => {}
                        // ignore damaged syntax
                        dir::Property::Error => {}
                    }
                }
                let term = TypeTerm::Shape {
                    members: members.into(),
                };

                self.define_expression_type(tree.module_id, id, term);

                for property in properties {
                    self.walk_property(tree, *property, tree.get(*property));
                }
            }
            // Type { key: value }
            dir::Expression::StructExpression { ty, properties } => {
                let source = id.into_global_any(tree.module_id);
                let owner = self.intern_local_type_variable(tree.module_id, *ty);

                for property in properties {
                    if let dir::Property::Field { key, value, .. } = tree.get(*property)
                        && let Some(key) = key.static_key(tree)
                    {
                        let member = self.terms.push(MemberTerm {
                            source: None,
                            owner,
                            key,
                            arguments: Vec::new().into(),
                        });
                        let field = TypeTerm::Member(member);
                        let field_variable = self.allocate_intermediate_variable(
                            tree.module_id,
                            VariableKind::Type,
                            ConstraintOrigin::Node(source),
                        );
                        self.define_type(tree.module_id, field_variable, field);
                        let value = self.intern_local_type_variable(tree.module_id, *value);

                        self.constrain_type(
                            ConstraintOrigin::Node(source),
                            TypeRelation::Assignable,
                            value,
                            field_variable,
                        );
                    }
                }

                self.define_expression_type(tree.module_id, id, TypeTerm::Variable(owner));

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
                let tag = left.map(|left| self.intern_local_type_variable(tree.module_id, left));
                let generic_argument_terms = self.build_generic_arguments(generic_arguments, tree);
                let argument_types = arguments
                    .as_ref()
                    .map(|arguments| self.argument_value_type_variables(arguments, tree))
                    .unwrap_or_default();
                let element_types = elements
                    .as_ref()
                    .map(|elements| self.argument_value_type_variables(elements, tree))
                    .unwrap_or_default();
                let tree_term = self.terms.push(TreeTerm {
                    source: id.into_global_any(tree.module_id),
                    tag,
                    generic_arguments: generic_argument_terms,
                    arguments: argument_types,
                    elements: element_types,
                });
                let term = TypeTerm::Tree(tree_term);

                self.define_expression_type(tree.module_id, id, term);

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
                let child_type = self.intern_local_type_variable(tree.module_id, *child);

                self.define_expression_type(tree.module_id, id, TypeTerm::Variable(child_type));
                self.walk_expression(tree, *child, tree.get(*child));
            }
            // type T
            dir::Expression::Type { value } => {
                let ty = self.intern_local_type_variable(tree.module_id, *value);
                let type_value = self.terms.push(TypeValueTerm {
                    source: id.into_global_any(tree.module_id),
                    ty,
                });
                let term = TypeTerm::TypeValue(type_value);

                self.define_expression_type(tree.module_id, id, term);
                self.walk_type_expression(tree, *value, tree.get(*value));
            }
            // comptime value
            dir::Expression::Comptime { body } => {
                let body_type = self.intern_local_type_variable(tree.module_id, *body);

                self.define_expression_type(tree.module_id, id, TypeTerm::Variable(body_type));
                self.walk_expression(tree, *body, tree.get(*body));
            }
            // value as T
            dir::Expression::As {
                expression: child,
                target_type,
            } => {
                let child_type = self.intern_local_type_variable(tree.module_id, *child);
                let target_type_variable = self.intern_local_type_variable(tree.module_id, *target_type);
                let origin = ConstraintOrigin::Node(id.into_global_any(tree.module_id));

                self.define_expression_type(tree.module_id, id, TypeTerm::Variable(target_type_variable));
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
                let child_type = self.intern_local_type_variable(tree.module_id, *child);
                let target_type_variable = self.intern_local_type_variable(tree.module_id, *target_type);
                let origin = ConstraintOrigin::Node(id.into_global_any(tree.module_id));

                self.define_expression_type(tree.module_id, id, TypeTerm::Variable(child_type));
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
                self.define_expression_type(tree.module_id, id, TypeTerm::Literal(TypeLiteralTerm::boolean()));
                self.walk_expression(tree, *value, tree.get(*value));
                self.walk_type_expression(tree, *target_type, tree.get(*target_type));
            }
            // value instanceof Target
            dir::Expression::InstanceOf { value, target } => {
                let value_variable = self.intern_local_type_variable(tree.module_id, *value);
                let target_variable = self.intern_local_type_variable(tree.module_id, *target);
                let instance = self.terms.push(InstanceCheckTerm {
                    source: id.into_global_any(tree.module_id),
                    value: value_variable,
                    target: target_variable,
                });
                let term = TypeTerm::InstanceCheck(instance);

                self.define_expression_type(tree.module_id, id, term);
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

                let receiver = self.intern_local_type_variable(tree.module_id, *right);
                let operator = self.terms.push(OperatorTerm {
                    source: id.into_global_any(tree.module_id),
                    kind: OperatorTermKind::Unary(*operator),
                    receiver,
                    argument: None,
                });
                let term = TypeTerm::Operator(operator);

                self.define_expression_type(tree.module_id, id, term);
                self.walk_expression(tree, *right, tree.get(*right));

                // record writes after reading the updated place
                if is_update && let Some(place) = self.resolve_place(*right, tree) {
                    self.clear_written_expression_narrowings(tree, *right);
                    self.mark_place_assigned(place);
                    self.require_writable_place(place);
                }
            }
            // ^value
            dir::Expression::MoveOf { right, .. } => {
                let form = self.terms.push(FormTerm::Owned);
                let term = TypeTerm::Form {
                    form,
                    payload: self.intern_local_type_variable(tree.module_id, *right).into(),
                };

                self.define_expression_type(tree.module_id, id, term);
                self.walk_expression(tree, *right, tree.get(*right));
            }
            // &value
            dir::Expression::BorrowOf {
                mutability, right, ..
            } => {
                let origin = ConstraintOrigin::Node(id.into_global_any(tree.module_id));
                let access = mutability
                    .map(dir::Mutability::access)
                    .unwrap_or(dir::Access::Mutable);
                let access = dir::StaticTerm::Access { access };
                let access_variable =
                    self.allocate_intermediate_variable(tree.module_id, VariableKind::Static, origin);
                self.define_static(tree.module_id, access_variable, StaticTerm::Literal(access));
                let lifetime = self.allocate_intermediate_variable(tree.module_id, VariableKind::Static, origin);
                let form = self.terms.push(FormTerm::Borrowed { lifetime: lifetime.into(), access: access_variable.into() });
                let term = TypeTerm::Form {
                    form,
                    payload: self.intern_local_type_variable(tree.module_id, *right).into(),
                };

                self.define_expression_type(tree.module_id, id, term);
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
                        let owner = self.intern_local_type_variable(tree.module_id, *left);
                        let member = self.terms.push(MemberTerm {
                            source: Some(id.into_global_any(tree.module_id)),
                            owner,
                            key: dir::StaticKey::Name(*name),
                            arguments: Vec::new().into(),
                        });

                        TypeTerm::Member(member)
                    };

                    self.define_expression_type(tree.module_id, id, term);
                }

                self.walk_expression(tree, *left, tree.get(*left));
            }
            // value[index]
            dir::Expression::Index { left, index, .. } => {
                if let Some(index) = index {
                    let term = if let Some(narrowed) = self.flow_path_narrowing(tree, id) {
                        TypeTerm::Variable(narrowed)
                    } else {
                        let index_node = *index;
                        let receiver = self.intern_local_type_variable(tree.module_id, *left);
                        let index = self.intern_local_type_variable(tree.module_id, index_node);
                        let index_term = self.terms.push(IndexTerm {
                            source: id.into_global_any(tree.module_id),
                            receiver,
                            index,
                            key: tree.get(index_node).static_key(),
                        });

                        TypeTerm::Index(index_term)
                    };

                    self.define_expression_type(tree.module_id, id, term);
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
                if let Some(symbol) = self.direct_callee_symbol(*left, tree) {
                    let source = id.into_global_any(tree.module_id);
                    let arguments =
                        self.build_generic_arguments_for_owner(symbol, generic_arguments, tree);
                    let term = TypeTerm::Reference {
                        source: Some(source),
                        symbol,
                        arguments,
                    };

                    self.define_expression_type(tree.module_id, id, term);
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
                let source = id.into_global_any(tree.module_id);
                let callee = self.intern_local_type_variable(tree.module_id, *left);

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
                    let member = MemberCallTerm {
                        receiver: self.intern_local_type_variable(tree.module_id, *left),
                        key: dir::StaticKey::Name(*name),
                        arguments: Vec::new().into(),
                        protocol: None,
                    };
                    let member = self.terms.push(member);

                    Some(member)
                } else {
                    None
                };

                // seed directly named candidates
                let candidates = match self.direct_callee_symbol(*left, tree) {
                    Some(symbol) => {
                        let ty = self.intern_symbol_type_variable(tree.module_id, symbol);

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
                let call = self.terms.push(CallTerm {
                    source,
                    callee,
                    member,
                    candidates,
                    generic_arguments: generic_arguments_term,
                    arguments: arguments_term.into_iter().map(TypeOperand::from).collect(),
                });
                let term = TypeTerm::Call(call);

                self.define_expression_type(tree.module_id, id, term);
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
                let callee = self.intern_local_type_variable(tree.module_id, *left);
                let generic_argument_terms = self.build_generic_arguments(generic_arguments, tree);
                let argument_types = self.argument_value_type_variables(arguments, tree);
                let construct = self.terms.push(ConstructTerm {
                    source: id.into_global_any(tree.module_id),
                    callee,
                    generic_arguments: generic_argument_terms,
                    arguments: argument_types.into_iter().map(TypeOperand::from).collect(),
                });
                let term = TypeTerm::Construct(construct);

                self.define_expression_type(tree.module_id, id, term);
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
                self.validate_await_context(tree.module_id, id.into_any());

                let source = id.into_global_any(tree.module_id);
                let origin = ConstraintOrigin::Node(source);
                let awaited_type = self.allocate_intermediate_variable(tree.module_id, VariableKind::Type, origin);
                let value = self.intern_local_type_variable(tree.module_id, *awaited);

                let await_term = self.terms.push(AwaitTerm { source, value });

                self.define_type(tree.module_id, awaited_type, TypeTerm::Await(await_term));

                let term = TryTerm {
                    source,
                    value: awaited_type,
                    kind: TryTermKind::Maybe,
                };

                let tried = self.terms.push(term.clone());

                self.define_expression_type(tree.module_id, id, TypeTerm::Try(tried));
                self.walk_expression(tree, *awaited, tree.get(*awaited));

                self.record_try_propagation(tree.module_id, id.into_any(), term.value);
            }
            // await! value
            dir::Expression::AwaitMust {
                expression: awaited,
            } => {
                self.validate_await_context(tree.module_id, id.into_any());

                let source = id.into_global_any(tree.module_id);
                let origin = ConstraintOrigin::Node(source);
                let awaited_type = self.allocate_intermediate_variable(tree.module_id, VariableKind::Type, origin);
                let value = self.intern_local_type_variable(tree.module_id, *awaited);

                let await_term = self.terms.push(AwaitTerm { source, value });

                self.define_type(tree.module_id, awaited_type, TypeTerm::Await(await_term));

                let term = TryTerm {
                    source,
                    value: awaited_type,
                    kind: TryTermKind::Must,
                };

                let term = self.terms.push(term);

                self.define_expression_type(tree.module_id, id, TypeTerm::Try(term));
                self.walk_expression(tree, *awaited, tree.get(*awaited));
            }
            // value?
            dir::Expression::Maybe { left, .. } => {
                let tried = TryTerm {
                    source: id.into_global_any(tree.module_id),
                    value: self.intern_local_type_variable(tree.module_id, *left),
                    kind: TryTermKind::Maybe,
                };

                let term = self.terms.push(tried.clone());

                self.define_expression_type(tree.module_id, id, TypeTerm::Try(term));
                self.walk_expression(tree, *left, tree.get(*left));

                self.record_try_propagation(tree.module_id, id.into_any(), tried.value);
            }
            // value!
            dir::Expression::Must { left, .. } => {
                let value = self.intern_local_type_variable(tree.module_id, *left);
                let term = self.terms.push(TryTerm {
                    source: id.into_global_any(tree.module_id),
                    value,
                    kind: TryTermKind::Must,
                });
                let term = TypeTerm::Try(term);

                self.define_expression_type(tree.module_id, id, term);
                self.walk_expression(tree, *left, tree.get(*left));
            }
            // left + right
            dir::Expression::Binary {
                left,
                operator,
                right,
            } => {
                let source = id.into_global_any(tree.module_id);
                let left_type = self.intern_local_type_variable(tree.module_id, *left);
                let right_type = self.intern_local_type_variable(tree.module_id, *right);
                let term = match operator {
                    // left === right
                    dir::BinaryOperator::EqualStrict | dir::BinaryOperator::NotEqualStrict => {
                        let identity = self.terms.push(IdentityTerm {
                            source,
                            operator: *operator,
                            left: left_type,
                            right: right_type,
                        });

                        TypeTerm::Identity(identity)
                    }
                    // key in receiver
                    dir::BinaryOperator::In => {
                        let membership = self.terms.push(KeyMembershipTerm {
                            source,
                            key: left_type,
                            receiver: right_type,
                            static_key: tree.get(*left).static_key(),
                        });

                        TypeTerm::KeyMembership(membership)
                    }
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
                    | dir::BinaryOperator::Coalesce => {
                        let operator = self.terms.push(OperatorTerm {
                            source,
                            kind: OperatorTermKind::Binary(*operator),
                            receiver: left_type,
                            argument: Some(right_type),
                        });

                        TypeTerm::Operator(operator)
                    }
                };

                self.define_expression_type(tree.module_id, id, term);

                match operator {
                    // left && right
                    dir::BinaryOperator::And => {
                        self.walk_expression(tree, *left, tree.get(*left));

                        let before_right = self.checkpoint_flow(tree.module_id);

                        self.restore_flow(tree.module_id, before_right);
                        self.apply_expression_narrowings(tree, *left, ConditionBranch::True);
                        self.walk_expression(tree, *right, tree.get(*right));
                        let right_flow = self
                            .expression_can_fall_through(tree, *right)
                            .then(|| self.collect_flow_branch(tree.module_id, before_right));

                        self.restore_flow(tree.module_id, before_right);
                        self.apply_expression_narrowings(tree, *left, ConditionBranch::False);
                        let skip_flow = self.collect_flow_branch(tree.module_id, before_right);

                        if let Some(right_flow) = right_flow {
                            self.merge_flow_branches(tree.module_id, before_right, &skip_flow, &right_flow);
                        } else {
                            self.apply_flow_branch(tree.module_id, before_right, &skip_flow);
                        }
                    }
                    // left || right
                    dir::BinaryOperator::Or => {
                        self.walk_expression(tree, *left, tree.get(*left));

                        let before_right = self.checkpoint_flow(tree.module_id);

                        self.restore_flow(tree.module_id, before_right);
                        self.apply_expression_narrowings(tree, *left, ConditionBranch::False);
                        self.walk_expression(tree, *right, tree.get(*right));
                        let right_flow = self
                            .expression_can_fall_through(tree, *right)
                            .then(|| self.collect_flow_branch(tree.module_id, before_right));

                        self.restore_flow(tree.module_id, before_right);
                        self.apply_expression_narrowings(tree, *left, ConditionBranch::True);
                        let skip_flow = self.collect_flow_branch(tree.module_id, before_right);

                        if let Some(right_flow) = right_flow {
                            self.merge_flow_branches(tree.module_id, before_right, &skip_flow, &right_flow);
                        } else {
                            self.apply_flow_branch(tree.module_id, before_right, &skip_flow);
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
                let value = self.intern_local_type_variable(tree.module_id, *right);

                // direct assignment uses the right side value
                let assigned_value = if *operator == dir::AssignOperator::Assign {
                    Some(value)
                }
                // compound assignment reads and writes the same place
                else {
                    match tree.get(*left) {
                        // target
                        dir::AssignPattern::Expression { value: place } => {
                            let receiver = self.intern_local_type_variable(tree.module_id, *place);
                            let source = id.into_global_any(tree.module_id);
                            let term = if let Some(binary_operator) = operator.binary_operator() {
                                let operator = self.terms.push(OperatorTerm {
                                    source,
                                    kind: OperatorTermKind::Binary(binary_operator),
                                    receiver,
                                    argument: Some(value),
                                });

                                TypeTerm::Operator(operator)
                            } else {
                                let operation = self.terms.push(TypeOperationTerm::BestCommon {
                                    elements: vec![receiver.into(), value.into()],
                                });

                                TypeTerm::Operation(operation)
                            };
                            let result = self.allocate_intermediate_variable(
                                tree.module_id,
                                VariableKind::Type,
                                ConstraintOrigin::Node(source),
                            );

                            self.define_type(tree.module_id, result, term);

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
                                let index_node = *index;
                                let receiver =
                                    self.intern_local_type_variable(tree.module_id, *left);
                                let index =
                                    self.intern_local_type_variable(tree.module_id, index_node);
                                let set = self.terms.push(IndexSetTerm {
                                    source: id.into_global_any(tree.module_id),
                                    receiver,
                                    index,
                                    value: assigned_value,
                                    key: tree.get(index_node).static_key(),
                                });

                                TypeTerm::IndexSet(set)
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

                    self.define_expression_type(tree.module_id, id, term);
                }

                if let Some(pattern) = self.build_assign_pattern_term(tree.module_id, *left, tree) {
                    let value = self.intern_local_type_variable(tree.module_id, id);

                    self.constrain_pattern(
                        tree.module_id,
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

        self.pop_static_condition(tree.module_id);
    }

    /// Walk one where clause.
    pub(in crate::check) fn walk_where_clause(
        &mut self,
        tree: &dir::Tree,
        _id: dir::LocalNodeId<dir::WhereClause>,
        where_clause: &dir::WhereClause,
    ) {
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
    fn define_expression_type(
        &mut self,
        module: ModuleId,
        id: dir::LocalNodeId<dir::Expression>,
        term: TypeTerm,
    ) {
        let variable = self.intern_local_type_variable(module, id);

        self.define_type(module, variable, term);
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

        self.declaration_symbol(tree.module_id, (*value).into_any())
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
            .map(|value| self.intern_local_type_variable(tree.module_id, value))
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
            let term = TypeTerm::Variable(self.intern_local_type_variable(tree.module_id, body));

            self.define_expression_type(tree.module_id, id, term);
            self.walk_loop_expression_with_label(tree, body, label);

            return;
        }

        // enter labeled control target
        let result = self.intern_local_type_variable(tree.module_id, id);
        let origin = ConstraintOrigin::Node(id.into_global_any(tree.module_id));
        self.enter_control_target(tree.module_id, origin, Some(label), false, result);

        // walk body with isolated flow
        let before_body = self.checkpoint_flow(tree.module_id);
        self.walk_expression(tree, body, tree.get(body));
        let fallthrough = if self.expression_can_fall_through(tree, body) {
            Some(TypeTerm::Variable(
                self.intern_local_type_variable(tree.module_id, body),
            ))
        } else {
            None
        };

        // merge fallthrough and break branches
        let body_flow = fallthrough
            .is_some()
            .then(|| self.collect_flow_branch(tree.module_id, before_body));
        let mut branches = self.leave_control_target(tree.module_id, fallthrough);
        if let Some(body_flow) = body_flow {
            branches.push(body_flow);
        }

        self.merge_flow_branches_from(tree.module_id, before_body, &branches);
    }

    /// Walk one labeled loop expression.
    fn walk_loop_expression_with_label(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        label: dir::StringId,
    ) {
        // enter static owner guard
        if !self.push_static_condition_for(tree, id.into_any(), None) {
            return;
        }
        match tree.get(id) {
            // label: while condition { body }
            dir::Expression::While {
                condition, body, ..
            } => {
                self.intern_local_type_variable(tree.module_id, id);
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
                self.intern_local_type_variable(tree.module_id, id);
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
                self.intern_local_type_variable(tree.module_id, id);
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
                self.intern_local_type_variable(tree.module_id, id);
                self.walk_loop_expression(tree, id, Some(label), *body);
            }
            // non loop labels are handled by walk_label_expression
            _ => {}
        }

        self.pop_static_condition(tree.module_id);
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
        let before = self.checkpoint_flow(tree.module_id);

        self.restore_flow(tree.module_id, before);
        self.apply_condition_narrowings(tree, condition, ConditionBranch::True);
        self.walk_expression(tree, then_expression, tree.get(then_expression));
        let then_flow = self.collect_flow_branch(tree.module_id, before);
        let then_can_fall_through = self.expression_can_fall_through(tree, then_expression);

        if let Some(else_expression) = else_expression {
            self.restore_flow(tree.module_id, before);
            self.apply_condition_narrowings(tree, condition, ConditionBranch::False);
            self.walk_expression(tree, else_expression, tree.get(else_expression));
            let else_flow = self.collect_flow_branch(tree.module_id, before);
            let else_can_fall_through = self.expression_can_fall_through(tree, else_expression);

            match (then_can_fall_through, else_can_fall_through) {
                (true, true) => {
                    self.merge_flow_branches(tree.module_id, before, &then_flow, &else_flow)
                }
                (true, false) => self.apply_flow_branch(tree.module_id, before, &then_flow),
                (false, true) => self.apply_flow_branch(tree.module_id, before, &else_flow),
                (false, false) => self.restore_flow(tree.module_id, before),
            }
        } else {
            self.restore_flow(tree.module_id, before);
            self.apply_condition_narrowings(tree, condition, ConditionBranch::False);
            let else_flow = self.collect_flow_branch(tree.module_id, before);

            if then_can_fall_through {
                self.merge_flow_branches(tree.module_id, before, &else_flow, &then_flow);
            } else {
                self.apply_flow_branch(tree.module_id, before, &else_flow);
            }
        }
    }

    /// Walk one if condition.
    fn walk_if_condition(&mut self, tree: &dir::Tree, condition: &dir::IfCondition) {
        match condition {
            // if condition
            dir::IfCondition::Expression { condition } => {
                self.walk_expression(tree, *condition, tree.get(*condition));

                let variable = self.intern_local_type_variable(tree.module_id, *condition);
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
        let target = self.intern_local_type_variable(tree.module_id, target_type);

        match branch {
            ConditionBranch::True => {
                self.narrow_flow_path(path, target);
            }
            ConditionBranch::False => {
                let original = self.intern_local_type_variable(tree.module_id, value);

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
                let original = self.intern_local_type_variable(tree.module_id, *right);

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
        let source = value.into_global_any(tree.module_id);
        let origin = ConstraintOrigin::Node(source);
        let ty = self.allocate_intermediate_variable(tree.module_id, VariableKind::Type, origin);
        self.define_type(
            tree.module_id,
            ty,
            TypeTerm::Literal(TypeLiteralTerm::Unknown),
        );
        let member = ShapeMember::Field {
            key,
            ty: ty.into(),
            is_optional: false,
            is_readonly: false,
        };
        let target =
            self.allocate_intermediate_variable(tree.module_id, VariableKind::Type, origin);
        self.define_type(
            tree.module_id,
            target,
            TypeTerm::Shape {
                members: vec![member].into(),
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
        let target = self.intern_local_type_variable(tree.module_id, target);

        match branch {
            ConditionBranch::True => {
                self.narrow_flow_path(path, target);
            }
            ConditionBranch::False => {
                let original = self.intern_local_type_variable(tree.module_id, value);

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
                let original = self.intern_local_type_variable(tree.module_id, value);

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
        let source = id.into_global_any(tree.module_id);
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

        let variable =
            self.allocate_intermediate_variable(tree.module_id, VariableKind::Type, origin);
        self.define_type(tree.module_id, variable, term);

        Some(variable)
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
        let text = self.input(tree.module_id).strings.get(*value);
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
        let source = id.into_global_any(tree.module_id);
        let origin = ConstraintOrigin::Node(source);
        let term = TypeTerm::Literal(literal);

        let variable =
            self.allocate_intermediate_variable(tree.module_id, VariableKind::Type, origin);
        self.define_type(tree.module_id, variable, term);

        Some(variable)
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
        let condition_type = self.intern_local_type_variable(tree.module_id, condition);
        self.constrain_condition(condition.into_any(), condition_type);

        // enter loop control target
        let origin = ConstraintOrigin::Node(id.into_global_any(tree.module_id));
        let result = self.intern_local_type_variable(tree.module_id, id);
        self.enter_control_target(tree.module_id, origin, label, true, result);

        // walk body under true condition facts
        let before_body = self.checkpoint_flow(tree.module_id);
        self.apply_expression_narrowings(tree, condition, ConditionBranch::True);
        self.walk_block(tree, body, tree.get(body));
        self.restore_flow(tree.module_id, before_body);

        // collect normal exit through false condition
        self.apply_expression_narrowings(tree, condition, ConditionBranch::False);
        let normal_flow = self.collect_flow_branch(tree.module_id, before_body);
        let mut branches = self.leave_control_target(
            tree.module_id,
            Some(TypeTerm::Literal(TypeLiteralTerm::Void)),
        );

        // merge break branches with normal exit
        branches.push(normal_flow);
        self.merge_flow_branches_from(tree.module_id, before_body, &branches);
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
        let origin = ConstraintOrigin::Node(id.into_global_any(tree.module_id));
        let result = self.intern_local_type_variable(tree.module_id, id);
        self.enter_control_target(tree.module_id, origin, label, true, result);

        // walk body with iteration binding assigned
        let before_body = self.checkpoint_flow(tree.module_id);
        self.mark_bindings_assigned(tree, pattern.into_any());
        self.walk_block(tree, body, tree.get(body));
        self.restore_flow(tree.module_id, before_body);

        // collect normal loop exit
        let normal_flow = self.collect_flow_branch(tree.module_id, before_body);
        let mut branches = self.leave_control_target(
            tree.module_id,
            Some(TypeTerm::Literal(TypeLiteralTerm::Void)),
        );

        // merge break branches with normal exit
        branches.push(normal_flow);
        self.merge_flow_branches_from(tree.module_id, before_body, &branches);
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

            let variable = self.intern_local_type_variable(tree.module_id, condition);
            self.constrain_condition(condition.into_any(), variable);
        }

        // enter loop control target
        let origin = ConstraintOrigin::Node(id.into_global_any(tree.module_id));
        let result = self.intern_local_type_variable(tree.module_id, id);
        self.enter_control_target(tree.module_id, origin, label, true, result);

        // walk body under true condition facts
        let before_body = self.checkpoint_flow(tree.module_id);
        if let Some(condition) = condition {
            self.apply_expression_narrowings(tree, condition, ConditionBranch::True);
        }
        self.walk_block(tree, body, tree.get(body));
        let body_flow = self
            .block_can_fall_through(tree, tree.get(body))
            .then(|| self.collect_flow_branch(tree.module_id, before_body));
        let continue_flows = self.take_current_continue_branches(tree.module_id);
        if let Some(increment) = increment {
            self.walk_for_increment_expression(
                tree,
                increment,
                before_body,
                body_flow,
                &continue_flows,
            );
        }
        self.restore_flow(tree.module_id, before_body);

        // collect normal exit through false condition
        let normal_flow = condition.map(|condition| {
            self.apply_expression_narrowings(tree, condition, ConditionBranch::False);

            self.collect_flow_branch(tree.module_id, before_body)
        });
        let fallthrough = condition.map(|_| TypeTerm::Literal(TypeLiteralTerm::Void));
        let mut branches = self.leave_control_target(tree.module_id, fallthrough);

        // merge break branches with normal exit
        if let Some(normal_flow) = normal_flow {
            branches.push(normal_flow);
        }

        self.merge_flow_branches_from(tree.module_id, before_body, &branches);
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
            self.restore_flow(tree.module_id, before_body);

            return;
        }

        // check increment from each flow that reaches the next iteration
        for flow in flows {
            self.apply_flow_branch(tree.module_id, before_body, &flow);
            self.walk_expression(tree, increment, tree.get(increment));
            self.restore_flow(tree.module_id, before_body);
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
        let source = id.into_global_any(tree.module_id);
        let origin = ConstraintOrigin::Node(source);
        let iterator_type = self.intern_local_type_variable(tree.module_id, iterator);
        let value = match operator {
            // for (const item of iterable)
            dir::ForEachOperator::Of => {
                let value =
                    self.allocate_intermediate_variable(tree.module_id, VariableKind::Type, origin);
                let unknown =
                    self.allocate_intermediate_variable(tree.module_id, VariableKind::Type, origin);
                self.define_type(
                    tree.module_id,
                    unknown,
                    TypeTerm::Literal(TypeLiteralTerm::Unknown),
                );
                let Ok(symbol) = self.language_symbol(tree.module_id, dir::LanguageItem::Iterable)
                else {
                    return;
                };
                let value_argument = GenericArgument::Type(value.into());
                let first_unknown = GenericArgument::Type(unknown.into());
                let second_unknown = GenericArgument::Type(unknown.into());
                let iterable = TypeTerm::Reference {
                    source: Some(source),
                    symbol,
                    arguments: vec![value_argument, first_unknown, second_unknown].into(),
                };
                let iterable_variable =
                    self.allocate_intermediate_variable(tree.module_id, VariableKind::Type, origin);
                self.define_type(tree.module_id, iterable_variable, iterable);

                self.constrain_type(
                    origin,
                    TypeRelation::Assignable,
                    iterator_type,
                    iterable_variable,
                );

                value
            }
            // for (const key in object)
            dir::ForEachOperator::In => {
                let operation = self.terms.push(TypeOperationTerm::KeyOf {
                    target: iterator_type,
                });
                let term = TypeTerm::Operation(operation);
                let variable =
                    self.allocate_intermediate_variable(tree.module_id, VariableKind::Type, origin);
                self.define_type(tree.module_id, variable, term);

                variable
            }
        };

        if let Some(term) = self.build_pattern_term(tree.module_id, pattern, tree) {
            self.constrain_pattern(
                tree.module_id,
                PatternRelation::Match(term),
                pattern.into_any(),
                value,
            );
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
        let origin = ConstraintOrigin::Node(id.into_global_any(tree.module_id));
        let result = self.intern_local_type_variable(tree.module_id, id);
        self.enter_control_target(tree.module_id, origin, label, true, result);

        // walk body with isolated flow
        let before_body = self.checkpoint_flow(tree.module_id);
        self.walk_block(tree, body, tree.get(body));
        self.restore_flow(tree.module_id, before_body);

        // restore only branches that leave the loop
        let branches = self.leave_control_target(tree.module_id, None);
        self.merge_flow_branches_from(tree.module_id, before_body, &branches);
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
        let before = self.checkpoint_flow(tree.module_id);

        // walk the body branch from the incoming facts
        self.restore_flow(tree.module_id, before);
        if catch.is_some() {
            self.enter_try_target(tree.module_id, id.into_any());
        }
        self.walk_expression(tree, body, tree.get(body));
        let catch_failure = catch.map(|_| self.leave_try_target(tree.module_id));
        let body_flow = self.collect_flow_branch(tree.module_id, before);
        let body_can_fall_through = self.expression_can_fall_through(tree, body);

        // merge only branches that can continue normally
        let has_normal_flow = if let Some(catch) = catch {
            let catch_can_fall_through = self.catch_can_fall_through(tree, tree.get(catch));

            self.restore_flow(tree.module_id, before);
            self.walk_catch(tree, catch, tree.get(catch), catch_failure);
            let catch_flow = self.collect_flow_branch(tree.module_id, before);

            match (body_can_fall_through, catch_can_fall_through) {
                (true, true) => {
                    self.merge_flow_branches(tree.module_id, before, &body_flow, &catch_flow);

                    true
                }
                (true, false) => {
                    self.apply_flow_branch(tree.module_id, before, &body_flow);

                    true
                }
                (false, true) => {
                    self.apply_flow_branch(tree.module_id, before, &catch_flow);

                    true
                }
                (false, false) => {
                    self.restore_flow(tree.module_id, before);

                    false
                }
            }
        } else if body_can_fall_through {
            self.apply_flow_branch(tree.module_id, before, &body_flow);

            true
        } else {
            self.restore_flow(tree.module_id, before);

            false
        };

        // finally is checked even when no normal path remains
        if let Some(finally) = finally {
            self.walk_expression(tree, finally, tree.get(finally));

            if !has_normal_flow || !self.expression_can_fall_through(tree, finally) {
                self.restore_flow(tree.module_id, before);
            }
        } else if !has_normal_flow {
            self.restore_flow(tree.module_id, before);
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
        let value_type = self.intern_local_type_variable(tree.module_id, value);
        let value_path = self.flow_path(tree, value);
        let mut active_cases = Vec::new();

        // apply static guards before exhaustiveness and case walking
        for case in cases {
            let condition = self.static_condition(tree, case.into_any(), None);
            if !condition.is_never() {
                active_cases.push((*case, condition));
            }
        }
        let active_case_ids = active_cases
            .iter()
            .map(|(case, _)| *case)
            .collect::<Vec<_>>();
        let case_terms = self.build_match_case_terms(tree, &active_case_ids);

        self.require_exhaustive_match(id.into_any(), value_type, case_terms);

        let before = self.checkpoint_flow(tree.module_id);
        let mut merged = None;

        for (case, condition) in &active_cases {
            self.restore_flow(tree.module_id, before);
            self.push_static_condition(tree.module_id, condition.clone());
            self.walk_match_case(
                tree,
                *case,
                tree.get(*case),
                Some((value_type, value_path.clone())),
            );
            self.pop_static_condition(tree.module_id);
            if !self.match_case_can_fall_through(tree, tree.get(*case)) {
                continue;
            }
            let case_flow = self.collect_flow_branch(tree.module_id, before);

            if let Some(previous) = &merged {
                self.merge_flow_branches(tree.module_id, before, previous, &case_flow);
                merged = Some(self.collect_flow_branch(tree.module_id, before));
            } else {
                merged = Some(case_flow);
            }
        }

        if let Some(merged) = merged {
            self.apply_flow_branch(tree.module_id, before, &merged);
        } else {
            self.restore_flow(tree.module_id, before);
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

    /// Walk one catch clause.
    pub(in crate::check) fn walk_catch(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Catch>,
        catch: &dir::Catch,
        failure: Option<VariableId>,
    ) {
        if !self.push_static_condition_for(tree, id.into_any(), None) {
            return;
        }
        // catch (error)
        if let Some(pattern) = catch.pattern {
            self.walk_pattern(tree, pattern, tree.get(pattern));

            if let (Some(failure), Some(ty)) = (failure, catch.ty) {
                let expected = self.intern_local_type_variable(tree.module_id, ty);
                let origin = ConstraintOrigin::Node(ty.into_global_any(tree.module_id));

                self.constrain_type(origin, TypeRelation::Assignable, failure, expected);
            }

            if let Some(value) = catch
                .ty
                .map(|ty| self.intern_local_type_variable(tree.module_id, ty))
                .or(failure)
                && let Some(pattern_term) = self.build_pattern_term(tree.module_id, pattern, tree)
            {
                self.constrain_pattern(
                    tree.module_id,
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
            let variable = self.intern_symbol_type_variable(tree.module_id, symbol);

            self.define_type(tree.module_id, variable, TypeTerm::Variable(failure));
            self.flow_mut(tree.module_id).mark_assigned(symbol);
        }

        // catch (...) { ... }
        self.walk_expression(tree, catch.body, tree.get(catch.body));

        self.pop_static_condition(tree.module_id);
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
