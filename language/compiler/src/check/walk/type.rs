use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    CheckState, ConstraintOrigin, FormTerm, MappedParameter, MemberTerm, ReceiverCapture,
    ShapeMember, StaticTerm, TupleElement, TypeLiteralTerm, TypeOperand, TypeOperationTerm,
    TypeTerm, VariableId, VariableKind,
};

impl CheckState<'_> {
    /// Walk one type expression.
    pub(in crate::check) fn walk_type_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::TypeExpression>,
        type_expression: &dir::TypeExpression,
    ) {
        // enter static owner guard
        if !self.push_static_condition_for(tree, id.into_any(), None) {
            return;
        }
        match type_expression {
            // (T)
            dir::TypeExpression::Parenthesized { expression } => {
                let ty = self.intern_local_type_variable(tree.module_id, *expression);

                self.define_type_expression_type(tree.module_id, id, TypeTerm::Variable(ty));
                self.walk_type_expression(tree, *expression, tree.get(*expression));
            }
            // 1
            dir::TypeExpression::ScalarLiteral { value } => {
                let term = TypeTerm::Literal(TypeLiteralTerm::Scalar(value.clone()));

                self.define_type_expression_type(tree.module_id, id, term);
            }
            // null
            dir::TypeExpression::Literal { value } => {
                let term = if *value == dir::TypeLiteral::Void {
                    TypeTerm::unit()
                } else if *value == dir::TypeLiteral::Object {
                    self.report_unsupported_type(tree.module_id, id.into_any(), "object");
                    TypeTerm::Literal(TypeLiteralTerm::Error)
                } else {
                    let ty = dir::Type::from(value.clone());
                    let literal = TypeLiteralTerm::from_type(&ty).unwrap_or(TypeLiteralTerm::Error);
                    TypeTerm::Literal(literal)
                };

                self.define_type_expression_type(tree.module_id, id, term);
            }
            // intrinsic
            dir::TypeExpression::Intrinsic => {
                self.define_type_expression_type(tree.module_id, id, TypeTerm::Intrinsic);
            }
            // const
            dir::TypeExpression::Const => {
                self.define_type_expression_type(tree.module_id, id, TypeTerm::ConstAssertion);
            }
            // this
            dir::TypeExpression::This => {
                self.define_type_expression_type(tree.module_id, id, TypeTerm::This);
            }
            // (T, U)
            dir::TypeExpression::Tuple { elements } => {
                let term = self.build_tuple_type_term(elements, dir::TupleForm::Tuple, tree);

                self.define_type_expression_type(tree.module_id, id, term);

                for element in elements {
                    self.walk_tuple_element(tree, *element, tree.get(*element));
                }
            }
            // [T, U]
            dir::TypeExpression::ArrayTuple { elements } => {
                let term = self.build_tuple_type_term(elements, dir::TupleForm::Array, tree);

                self.define_type_expression_type(tree.module_id, id, term);

                for element in elements {
                    self.walk_tuple_element(tree, *element, tree.get(*element));
                }
            }
            // T[]
            dir::TypeExpression::Array { element } => {
                let term = TypeTerm::Array {
                    element: self.intern_local_type_variable(tree.module_id, *element).into(),
                };

                self.define_type_expression_type(tree.module_id, id, term);
                self.walk_type_expression(tree, *element, tree.get(*element));
            }
            // [T]
            dir::TypeExpression::Slice { element } => {
                let term = TypeTerm::Slice {
                    element: self.intern_local_type_variable(tree.module_id, *element).into(),
                    is_readonly: false,
                };

                self.define_type_expression_type(tree.module_id, id, term);
                self.walk_type_expression(tree, *element, tree.get(*element));
            }
            // [T; N]
            dir::TypeExpression::FixedArray { element, length } => {
                let term = TypeTerm::FixedArray {
                    element: self.intern_local_type_variable(tree.module_id, *element).into(),
                    length: self.define_static_expression_variable(tree.module_id, *length),
                    is_readonly: false,
                };

                self.define_type_expression_type(tree.module_id, id, term);
                self.walk_type_expression(tree, *element, tree.get(*element));

                // check fixed array length in type context
                let before_length = self.checkpoint_flow(tree.module_id);

                self.walk_expression(tree, *length, tree.get(*length));
                self.restore_flow(tree.module_id, before_length);
            }
            // { name: T }
            dir::TypeExpression::Object { members } => {
                let term = TypeTerm::Shape {
                    members: self.build_shape_member_terms(members, tree),
                };

                self.define_type_expression_type(tree.module_id, id, term);

                for member in members {
                    self.walk_type_member(tree, *member, tree.get(*member));
                }
            }
            // (value: T) => U
            dir::TypeExpression::Function(declaration) => {
                let return_type = declaration
                    .return_type
                    .map(|return_type| self.intern_local_type_variable(tree.module_id, return_type));
                let term = self.build_function_type_term(declaration, return_type, tree);

                self.define_type_expression_type(tree.module_id, id, TypeTerm::Function(term));
                self.walk_function_type(tree, declaration);
            }
            // new (value: T) => U
            dir::TypeExpression::Constructor(declaration) => {
                let return_type = declaration
                    .return_type
                    .map(|return_type| self.intern_local_type_variable(tree.module_id, return_type));
                let term = self.build_constructor_type_term(declaration, return_type, tree);

                self.define_type_expression_type(tree.module_id, id, TypeTerm::Function(term));
                self.walk_constructor_type(tree, declaration);
            }
            // T
            dir::TypeExpression::Reference {
                path,
                generic_arguments,
            } => {
                if let Some(term) =
                    self.resolve_reference_type_expression(id, path, generic_arguments, tree)
                {
                    self.define_type_expression_type(tree.module_id, id, term);
                }

                for argument in generic_arguments {
                    self.walk_generic_argument(tree, *argument, tree.get(*argument));
                }
            }
            // T.Item
            dir::TypeExpression::Member {
                left,
                name,
                generic_arguments,
            } => {
                let arguments = self.build_generic_arguments(generic_arguments, tree);
                let owner = self.intern_local_type_variable(tree.module_id, *left);
                let member = self.terms.push(MemberTerm {
                    source: Some(id.into_global_any(tree.module_id)),
                    owner,
                    key: dir::StaticKey::Name(*name),
                    arguments,
                });
                let term = TypeTerm::Member(member);

                self.define_type_expression_type(tree.module_id, id, term);
                self.walk_type_expression(tree, *left, tree.get(*left));

                for argument in generic_arguments {
                    self.walk_generic_argument(tree, *argument, tree.get(*argument));
                }
            }
            // 0..10
            dir::TypeExpression::Range {
                start,
                end,
                end_kind,
            } => {
                if let Some(term) = self.build_range_type_term(*start, *end, *end_kind, tree) {
                    self.define_type_expression_type(tree.module_id, id, term);
                }

                if let Some(start) = start {
                    self.walk_type_expression(tree, *start, tree.get(*start));
                }
                if let Some(end) = end {
                    self.walk_type_expression(tree, *end, tree.get(*end));
                }
            }
            // readonly T
            dir::TypeExpression::Readonly { target_type } => {
                let form = self.terms.push(FormTerm::Readonly);
                let term = TypeTerm::Form {
                    form,
                    payload: self.intern_local_type_variable(tree.module_id, *target_type).into(),
                };

                self.define_type_expression_type(tree.module_id, id, term);
                self.walk_type_expression(tree, *target_type, tree.get(*target_type));
            }
            // ^T
            dir::TypeExpression::OwnedOf { target_type, .. } => {
                let form = self.terms.push(FormTerm::Owned);
                let term = TypeTerm::Form {
                    form,
                    payload: self.intern_local_type_variable(tree.module_id, *target_type).into(),
                };

                self.define_type_expression_type(tree.module_id, id, term);
                self.walk_type_expression(tree, *target_type, tree.get(*target_type));
            }
            // &T
            dir::TypeExpression::BorrowedOf {
                mutability,
                target_type,
                ..
            } => {
                let origin = ConstraintOrigin::Node(id.into_global_any(tree.module_id));
                let access = mutability
                    .map(dir::Mutability::access)
                    .unwrap_or(dir::Access::Mutable);
                let access_variable = self.allocate_intermediate_variable(
                    tree.module_id,
                    VariableKind::Static,
                    origin,
                );
                self.define_static(
                    tree.module_id,
                    access_variable,
                    StaticTerm::Literal(dir::StaticTerm::Access { access }),
                );
                let lifetime =
                    self.allocate_intermediate_variable(tree.module_id, VariableKind::Static, origin);
                let form = self.terms.push(FormTerm::Borrowed {
                    lifetime: lifetime.into(),
                    access: access_variable.into(),
                });
                let term = TypeTerm::Form {
                    form,
                    payload: self.intern_local_type_variable(tree.module_id, *target_type).into(),
                };

                self.define_type_expression_type(tree.module_id, id, term);
                self.walk_type_expression(tree, *target_type, tree.get(*target_type));
            }
            // *T
            dir::TypeExpression::PointerOf { target_type, .. } => {
                let form = self.terms.push(FormTerm::Raw);
                let term = TypeTerm::Form {
                    form,
                    payload: self.intern_local_type_variable(tree.module_id, *target_type).into(),
                };

                self.define_type_expression_type(tree.module_id, id, term);
                self.walk_type_expression(tree, *target_type, tree.get(*target_type));
            }
            // local T
            dir::TypeExpression::Local { target_type } => {
                let origin = ConstraintOrigin::Node(id.into_global_any(tree.module_id));
                let place = dir::StaticTerm::Place {
                    place: dir::Place::Space(dir::Space::Local),
                };
                let place_variable =
                    self.allocate_intermediate_variable(tree.module_id, VariableKind::Static, origin);
                self.define_static(tree.module_id, place_variable, StaticTerm::Literal(place));
                let form = self.terms.push(FormTerm::Placed {
                    place: place_variable.into(),
                });
                let term = TypeTerm::Form {
                    form,
                    payload: self.intern_local_type_variable(tree.module_id, *target_type).into(),
                };

                self.define_type_expression_type(tree.module_id, id, term);
                self.walk_type_expression(tree, *target_type, tree.get(*target_type));
            }
            // shared T
            dir::TypeExpression::Shared { target_type } => {
                let origin = ConstraintOrigin::Node(id.into_global_any(tree.module_id));
                let place = dir::StaticTerm::Place {
                    place: dir::Place::Space(dir::Space::Shared),
                };
                let place_variable =
                    self.allocate_intermediate_variable(tree.module_id, VariableKind::Static, origin);
                self.define_static(tree.module_id, place_variable, StaticTerm::Literal(place));
                let form = self.terms.push(FormTerm::Placed {
                    place: place_variable.into(),
                });
                let term = TypeTerm::Form {
                    form,
                    payload: self.intern_local_type_variable(tree.module_id, *target_type).into(),
                };

                self.define_type_expression_type(tree.module_id, id, term);
                self.walk_type_expression(tree, *target_type, tree.get(*target_type));
            }
            // T!
            dir::TypeExpression::Must { target_type }
            // !T
            | dir::TypeExpression::Not { target_type } => {
                self.define_materialized_type_expression(tree.module_id, id);
                self.walk_type_expression(tree, *target_type, tree.get(*target_type));
            }
            // keyof T
            dir::TypeExpression::KeyOf { target_type } => {
                let target = self.intern_local_type_variable(tree.module_id, *target_type);
                let operation = self.terms.push(TypeOperationTerm::KeyOf {
                    target,
                });
                let term = TypeTerm::Operation(operation);

                self.define_type_expression_type(tree.module_id, id, term);
                self.walk_type_expression(tree, *target_type, tree.get(*target_type));
            }
            // typeof value
            dir::TypeExpression::TypeOfValue { value } => {
                let ty = self.intern_local_type_variable(tree.module_id, *value);

                self.define_type_expression_type(tree.module_id, id, TypeTerm::Variable(ty));

                // check type query operand in type context
                let before_value = self.checkpoint_flow(tree.module_id);

                self.walk_expression(tree, *value, tree.get(*value));
                self.restore_flow(tree.module_id, before_value);
            }
            // T | U
            dir::TypeExpression::Union { elements } => {
                let term = TypeTerm::Union {
                    elements: elements
                        .iter()
                        .map(|element| self.intern_local_type_variable(tree.module_id, *element))
                        .map(TypeOperand::from)
                        .collect(),
                };

                self.define_type_expression_type(tree.module_id, id, term);

                for element in elements {
                    self.walk_type_expression(tree, *element, tree.get(*element));
                }
            }
            // T & U
            dir::TypeExpression::Intersection { elements } => {
                let term = TypeTerm::Intersection {
                    elements: elements
                        .iter()
                        .map(|element| self.intern_local_type_variable(tree.module_id, *element))
                        .map(TypeOperand::from)
                        .collect(),
                };

                self.define_type_expression_type(tree.module_id, id, term);

                for element in elements {
                    self.walk_type_expression(tree, *element, tree.get(*element));
                }
            }
            // T extends U ? X : Y
            dir::TypeExpression::Conditional {
                left,
                extends_type,
                then_type,
                else_type,
            } => {
                let left_variable = self.intern_local_type_variable(tree.module_id, *left);
                let right_variable =
                    self.intern_local_type_variable(tree.module_id, *extends_type);
                let then_variable = self.intern_local_type_variable(tree.module_id, *then_type);
                let else_variable = self.intern_local_type_variable(tree.module_id, *else_type);
                let operation = self.terms.push(TypeOperationTerm::Conditional {
                    left: left_variable,
                    right: right_variable,
                    then_type: then_variable,
                    else_type: else_variable,
                });
                let term = TypeTerm::Operation(operation);

                self.define_type_expression_type(tree.module_id, id, term);
                self.walk_type_expression(tree, *left, tree.get(*left));
                self.walk_type_expression(tree, *extends_type, tree.get(*extends_type));
                self.walk_type_expression(tree, *then_type, tree.get(*then_type));
                self.walk_type_expression(tree, *else_type, tree.get(*else_type));
            }
            // T extends U
            dir::TypeExpression::Extends { left, right }
            // T implements U
            | dir::TypeExpression::Implements { left, right } => {
                self.define_materialized_type_expression(tree.module_id, id);
                self.walk_type_expression(tree, *left, tree.get(*left));
                self.walk_type_expression(tree, *right, tree.get(*right));
            }
            // { [K in keyof T]: T[K] }
            dir::TypeExpression::Mapped {
                parameter,
                readonly,
                optional,
                value,
            } => {
                if let Some(term) =
                    self.build_mapped_type_term(*parameter, *readonly, *optional, *value, tree)
                {
                    self.define_type_expression_type(tree.module_id, id, term);
                }

                self.walk_type_mapped_parameter(tree, *parameter, tree.get(*parameter));

                if let Some(value) = value {
                    self.walk_type_expression(tree, *value, tree.get(*value));
                }
            }
            // T[K]
            dir::TypeExpression::Index { left, index } => {
                let left_variable = self.intern_local_type_variable(tree.module_id, *left);
                let index_variable = self.intern_local_type_variable(tree.module_id, *index);
                let operation = self.terms.push(TypeOperationTerm::Index {
                    left: left_variable,
                    index: index_variable,
                });
                let term = TypeTerm::Operation(operation);

                self.define_type_expression_type(tree.module_id, id, term);
                self.walk_type_expression(tree, *left, tree.get(*left));
                self.walk_type_expression(tree, *index, tree.get(*index));
            }
            // `get${Name}`
            dir::TypeExpression::TemplateLiteral { strings, spans } => {
                let span_variables = spans
                    .iter()
                    .map(|span| self.intern_local_type_variable(tree.module_id, *span))
                    .collect();
                let operation = self.terms.push(TypeOperationTerm::TemplateLiteral {
                    strings: strings.clone(),
                    spans: span_variables,
                });
                let term = TypeTerm::Operation(operation);

                self.define_type_expression_type(tree.module_id, id, term);

                for span in spans {
                    self.walk_type_expression(tree, *span, tree.get(*span));
                }
            }
            // infer T extends U
            dir::TypeExpression::Infer {
                name, constraint, ..
            } => {
                let constraint_variable = constraint
                    .map(|constraint| self.intern_local_type_variable(tree.module_id, constraint));
                let operation = self.terms.push(TypeOperationTerm::Infer {
                    name: *name,
                    constraint: constraint_variable,
                });
                let term = TypeTerm::Operation(operation);

                self.define_type_expression_type(tree.module_id, id, term);

                if let Some(constraint) = constraint {
                    self.walk_type_expression(tree, *constraint, tree.get(*constraint));
                }
            }
            // value is T
            dir::TypeExpression::Predicate {
                asserts,
                subject,
                target: Some(target),
            } => {
                if let Some(subject) = self.predicate_subject(tree.module_id, id, *subject) {
                    let term = TypeTerm::Predicate {
                        asserts: *asserts,
                        subject,
                        target: Some(self.intern_local_type_variable(tree.module_id, *target)),
                    };

                    self.define_type_expression_type(tree.module_id, id, term);
                }

                self.walk_type_expression(tree, *target, tree.get(*target));
            }
            // asserts value
            dir::TypeExpression::Predicate {
                asserts,
                subject,
                target: None,
            } => {
                if let Some(subject) = self.predicate_subject(tree.module_id, id, *subject) {
                    let term = TypeTerm::Predicate {
                        asserts: *asserts,
                        subject,
                        target: None,
                    };

                    self.define_type_expression_type(tree.module_id, id, term);
                }
            }
            // ignore damaged syntax
            dir::TypeExpression::Missing => {}
            // ignore damaged syntax
            dir::TypeExpression::Error => {}
        }

        self.pop_static_condition(tree.module_id);
    }

    /// Walk one type member.
    pub(in crate::check) fn walk_type_member(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::TypeMember>,
        type_member: &dir::TypeMember,
    ) {
        // enter static owner guard
        let static_receiver = self.type_member_static_receiver(tree.module_id, id);
        if !self.push_static_condition_for(tree, id.into_any(), static_receiver) {
            return;
        }

        match type_member {
            // field: T
            dir::TypeMember::Field {
                key, declared_type, ..
            } => {
                if declared_type.is_none() {
                    self.report_missing_type_annotation(tree.module_id, id.into_any());
                }

                if let Some(declared_type) = declared_type {
                    if key.direct_static_key().is_some() {
                        if let Some(symbol) = self.declaration_symbol(tree.module_id, id.into_any())
                        {
                            let variable = self.intern_symbol_type_variable(tree.module_id, symbol);
                            let declared_type =
                                self.intern_local_type_variable(tree.module_id, *declared_type);

                            self.define_type(
                                tree.module_id,
                                variable,
                                TypeTerm::Variable(declared_type),
                            );
                        }
                    }
                }

                // check computed type member key in declaration context
                if let dir::Key::Expression(key) = key {
                    let before_key = self.checkpoint_flow(tree.module_id);

                    self.walk_expression(tree, *key, tree.get(*key));
                    self.restore_flow(tree.module_id, before_key);
                }

                if let Some(declared_type) = declared_type {
                    self.walk_type_expression(tree, *declared_type, tree.get(*declared_type));
                }
            }
            // method(): T
            dir::TypeMember::Method {
                key,
                signature,
                body,
                is_static,
                ..
            } => {
                // check computed type member key in declaration context
                if let dir::Key::Expression(key) = key {
                    let before_key = self.checkpoint_flow(tree.module_id);

                    self.walk_expression(tree, *key, tree.get(*key));
                    self.restore_flow(tree.module_id, before_key);
                }

                let receiver = self.type_member_receiver(tree, id, signature, *is_static);
                let symbol = self.declaration_symbol(tree.module_id, id.into_any());
                if let Some(symbol) = symbol {
                    let variable = self.intern_symbol_type_variable(tree.module_id, symbol);
                    let return_type = self.intern_signature_return_type_variable(
                        tree.module_id,
                        id.into_any(),
                        signature,
                        *body,
                    );
                    let term = self.build_function_signature_term(signature, return_type, tree);
                    let function = self.terms.get_mut(term);
                    if function.this_parameter.is_none() && !*is_static {
                        function.this_parameter = receiver.map(|receiver| receiver.ty);
                    }

                    self.define_type(tree.module_id, variable, TypeTerm::Function(term));
                }

                self.walk_function_signature(tree, signature);

                // walk method body when bind provided the required symbols
                if let Some(body) = body
                    && let Some(symbol) = symbol
                    && let Some(return_type) = self.intern_signature_return_type_variable(
                        tree.module_id,
                        id.into_any(),
                        signature,
                        Some(*body),
                    )
                {
                    self.walk_function_body(tree, symbol, signature, *body, return_type, receiver);
                }
            }
            // (...): T
            dir::TypeMember::CallSignature { signature } => {
                self.walk_function_type(tree, signature);

                let return_type = signature.return_type.map(|return_type| {
                    self.intern_local_type_variable(tree.module_id, return_type)
                });
                let term = self.build_function_type_term(signature, return_type, tree);
                let variable = self.intern_local_type_variable(tree.module_id, id);

                self.define_type(tree.module_id, variable, TypeTerm::Function(term));
            }
            // new (...): T
            dir::TypeMember::ConstructSignature { signature } => {
                self.walk_constructor_type(tree, signature);

                let return_type = signature.return_type.map(|return_type| {
                    self.intern_local_type_variable(tree.module_id, return_type)
                });
                let term = self.build_constructor_type_term(signature, return_type, tree);
                let variable = self.intern_local_type_variable(tree.module_id, id);

                self.define_type(tree.module_id, variable, TypeTerm::Function(term));
            }
            // [key: K]: V
            dir::TypeMember::IndexSignature {
                key_type,
                value_type,
                ..
            } => {
                self.walk_type_expression(tree, *key_type, tree.get(*key_type));
                self.walk_type_expression(tree, *value_type, tree.get(*value_type));
            }
            // type Item = T
            dir::TypeMember::AssociatedType {
                generic_parameters,
                where_clauses,
                constraint,
                value,
                ..
            } => {
                if let Some(value) = value {
                    if let Some(symbol) = self.declaration_symbol(tree.module_id, id.into_any()) {
                        let variable = self.intern_symbol_type_variable(tree.module_id, symbol);
                        let value = self.intern_local_type_variable(tree.module_id, *value);

                        self.define_type(tree.module_id, variable, TypeTerm::Variable(value));
                    }
                }

                for generic_parameter in generic_parameters {
                    self.walk_generic_parameter(
                        tree,
                        *generic_parameter,
                        tree.get(*generic_parameter),
                    );
                }

                for where_clause in where_clauses {
                    self.walk_where_clause(tree, *where_clause, tree.get(*where_clause));
                }

                if let Some(constraint) = constraint {
                    self.walk_type_expression(tree, *constraint, tree.get(*constraint));
                }

                if let Some(value) = value {
                    self.walk_type_expression(tree, *value, tree.get(*value));
                }
            }
            // const item: T = value
            dir::TypeMember::AssociatedConst {
                declared_type,
                value,
                ..
            } => {
                if value.is_some() && declared_type.is_none() {
                    self.report_missing_type_annotation(tree.module_id, id.into_any());
                }

                if let Some(symbol) = self.declaration_symbol(tree.module_id, id.into_any()) {
                    // associated const type lives in type space
                    if let Some(declared_type) = declared_type {
                        let variable = self.intern_symbol_type_variable(tree.module_id, symbol);
                        let declared_type =
                            self.intern_local_type_variable(tree.module_id, *declared_type);

                        self.define_type(
                            tree.module_id,
                            variable,
                            TypeTerm::Variable(declared_type),
                        );
                    }

                    // associated const value lives in static space
                    if let Some(value) = value {
                        let variable = self.intern_symbol_static_variable(tree.module_id, symbol);
                        let value = self.define_static_expression_variable(tree.module_id, *value);

                        self.define_static(tree.module_id, variable, StaticTerm::Variable(value));
                    }
                }

                if let Some(declared_type) = declared_type {
                    self.walk_type_expression(tree, *declared_type, tree.get(*declared_type));
                }

                if let Some(value) = value {
                    self.walk_static_expression(tree, *value);
                }
            }
            // ignore damaged syntax
            dir::TypeMember::Error => {}
        }

        self.pop_static_condition(tree.module_id);
    }

    /// Return the receiver visible to decorators on one type member.
    fn type_member_static_receiver(
        &mut self,
        module: ModuleId,
        id: dir::LocalNodeId<dir::TypeMember>,
    ) -> Option<ReceiverCapture> {
        let symbol = self.implicit_receiver_symbol(module, id.into_any())?;
        let (owner, ty) = self.type_member_receiver_type(module, id)?;

        Some(ReceiverCapture { symbol, owner, ty })
    }

    /// Walk one mapped type parameter.
    pub(in crate::check) fn walk_type_mapped_parameter(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::TypeMappedParameter>,
        type_mapped_parameter: &dir::TypeMappedParameter,
    ) {
        // enter static owner guard
        if !self.push_static_condition_for(tree, id.into_any(), None) {
            return;
        }
        self.walk_type_expression(
            tree,
            type_mapped_parameter.source_type,
            tree.get(type_mapped_parameter.source_type),
        );

        if let Some(key_remap) = type_mapped_parameter.key_remap {
            self.walk_type_expression(tree, key_remap, tree.get(key_remap));
        }

        self.pop_static_condition(tree.module_id);
    }

    /// Walk one tuple element.
    pub(in crate::check) fn walk_tuple_element(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::TupleElement>,
        tuple_element: &dir::TupleElement,
    ) {
        // enter static owner guard
        if !self.push_static_condition_for(tree, id.into_any(), None) {
            return;
        }
        match tuple_element {
            // [T]
            dir::TupleElement::Element { value, .. } => {
                self.walk_type_expression(tree, *value, tree.get(*value));
            }
            // [...T]
            dir::TupleElement::Spread { value, .. } => {
                self.walk_type_expression(tree, *value, tree.get(*value));
            }
            // ignore damaged syntax
            dir::TupleElement::Error => {}
        }

        self.pop_static_condition(tree.module_id);
    }

    /// Walk one type-space function declaration.
    fn walk_function_type(&mut self, tree: &dir::Tree, declaration: &dir::FunctionType) {
        for parameter in &declaration.generic_parameters {
            self.walk_generic_parameter(tree, *parameter, tree.get(*parameter));
        }

        if let Some(parameter) = declaration.this_parameter {
            self.walk_parameter(tree, parameter, tree.get(parameter), true);
        }

        for parameter in &declaration.parameters {
            self.walk_parameter(tree, *parameter, tree.get(*parameter), true);
        }

        if let Some(return_type) = declaration.return_type {
            self.walk_type_expression(tree, return_type, tree.get(return_type));
        }

        for where_clause in &declaration.where_clauses {
            self.walk_where_clause(tree, *where_clause, tree.get(*where_clause));
        }
    }

    /// Walk one type-space constructor declaration.
    fn walk_constructor_type(&mut self, tree: &dir::Tree, declaration: &dir::ConstructorType) {
        for parameter in &declaration.generic_parameters {
            self.walk_generic_parameter(tree, *parameter, tree.get(*parameter));
        }

        for parameter in &declaration.parameters {
            self.walk_parameter(tree, *parameter, tree.get(*parameter), true);
        }

        if let Some(return_type) = declaration.return_type {
            self.walk_type_expression(tree, return_type, tree.get(return_type));
        }

        for where_clause in &declaration.where_clauses {
            self.walk_where_clause(tree, *where_clause, tree.get(*where_clause));
        }
    }

    /// Return the receiver visible inside one type-surface method.
    fn type_member_receiver(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::TypeMember>,
        signature: &dir::FunctionSignature,
        is_static: bool,
    ) -> Option<ReceiverCapture> {
        let owner = self.type_member_receiver_owner(tree.module_id, id);
        if let Some(receiver) = self.function_receiver(signature.this_parameter, owner, tree) {
            return Some(receiver);
        }
        if is_static {
            return None;
        }
        let Some(symbol) = self.implicit_receiver_symbol(tree.module_id, id.into_any()) else {
            return None;
        };
        let Some((owner, ty)) = self.type_member_receiver_type(tree.module_id, id) else {
            return None;
        };

        Some(ReceiverCapture { symbol, owner, ty })
    }

    /// Return the contextual type used for an implicit type member receiver.
    fn type_member_receiver_type(
        &mut self,
        module: ModuleId,
        id: dir::LocalNodeId<dir::TypeMember>,
    ) -> Option<(Option<dir::GlobalSymbolId>, VariableId)> {
        if let Some(object) = self.enclosing_object_type_expression(module, id) {
            let ty = self.intern_local_type_variable(module, object);

            return Some((None, ty));
        }

        let declaration = self.enclosing_declaration(module, id.into_any())?;
        let Some(owner) = self.declaration_symbol(module, declaration.into_any()) else {
            return None;
        };
        let ty = self.intern_symbol_type_variable(module, owner);

        Some((Some(owner), ty))
    }

    /// Return the nominal owner that provides one type member receiver.
    fn type_member_receiver_owner(
        &self,
        module: ModuleId,
        id: dir::LocalNodeId<dir::TypeMember>,
    ) -> Option<dir::GlobalSymbolId> {
        if self.enclosing_object_type_expression(module, id).is_some() {
            return None;
        }

        let declaration = self.enclosing_declaration(module, id.into_any())?;

        self.declaration_symbol(module, declaration.into_any())
    }

    /// Return the enclosing object type expression.
    fn enclosing_object_type_expression(
        &self,
        module: ModuleId,
        id: dir::LocalNodeId<dir::TypeMember>,
    ) -> Option<dir::LocalNodeId<dir::TypeExpression>> {
        let view = self.input(module).view();
        let parent = view.get_parent_for(id)?;
        if parent.ty != dir::NodeType::TypeExpression {
            return None;
        }

        let expression = dir::LocalNodeId::<dir::TypeExpression>::new(parent.id);
        if matches!(view.get(expression), dir::TypeExpression::Object { .. }) {
            Some(expression)
        } else {
            None
        }
    }

    /// Return the nearest enclosing declaration.
    fn enclosing_declaration(
        &self,
        module: ModuleId,
        id: dir::LocalNodeIdAny,
    ) -> Option<dir::LocalNodeId<dir::Declaration>> {
        let view = self.input(module).view();
        let mut current = view.get_parent(id.id);

        while let Some(node) = current {
            if node.ty == dir::NodeType::Declaration {
                return Some(dir::LocalNodeId::<dir::Declaration>::new(node.id));
            }

            current = view.get_parent(node.id);
        }

        None
    }

    /// Define one type expression output type.
    fn define_type_expression_type(
        &mut self,
        module: ModuleId,
        id: dir::LocalNodeId<dir::TypeExpression>,
        term: TypeTerm,
    ) {
        let variable = self.intern_local_type_variable(module, id);

        self.define_type(module, variable, term);
    }

    /// Define one type expression from the imported or prechecked type table.
    fn define_materialized_type_expression(
        &mut self,
        module: ModuleId,
        id: dir::LocalNodeId<dir::TypeExpression>,
    ) {
        let source = id.into_global_any(module);
        let type_id = match self.input(module).type_table().get_node_type_id(source) {
            Some(type_id) => type_id,
            None => return,
        };
        let materialized = self.materialize_type_id(type_id.into_global(module));

        self.define_type_expression_type(module, id, TypeTerm::Variable(materialized));
    }

    /// Return one compact scalar range type term.
    fn build_range_type_term(
        &self,
        start: Option<dir::LocalNodeId<dir::TypeExpression>>,
        end: Option<dir::LocalNodeId<dir::TypeExpression>>,
        end_kind: dir::RangeEnd,
        tree: &dir::Tree,
    ) -> Option<TypeTerm> {
        let start = match start {
            Some(start) => Some(self.scalar_range_bound(start, tree)?),
            None => None,
        };
        let end = match end {
            Some(end) => Some(self.scalar_range_bound(end, tree)?),
            None => None,
        };

        Some(TypeTerm::Range {
            start,
            end,
            is_inclusive: end_kind == dir::RangeEnd::Inclusive,
        })
    }

    /// Return one scalar literal range bound.
    fn scalar_range_bound(
        &self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        tree: &dir::Tree,
    ) -> Option<dir::ScalarLiteral> {
        match tree.get(id) {
            // 0
            dir::TypeExpression::ScalarLiteral { value } => Some(value.clone()),
            // not a compact scalar range bound
            _ => None,
        }
    }

    /// Return one semantic predicate subject.
    fn predicate_subject(
        &mut self,
        module: ModuleId,
        id: dir::LocalNodeId<dir::TypeExpression>,
        subject: dir::TypePredicateSubject,
    ) -> Option<dir::PredicateSubject> {
        match subject {
            // value is T
            dir::TypePredicateSubject::Identifier(name) => {
                let symbol = self.require_symbol_by_name(
                    module,
                    id.into_any(),
                    name,
                    dir::SymbolSpace::Value,
                )?;
                let source = id.into_global_any(module);

                self.record_value_reference(source, symbol);

                Some(dir::PredicateSubject::Symbol(symbol))
            }
            // this is T
            dir::TypePredicateSubject::This => Some(dir::PredicateSubject::This),
        }
    }

    /// Return one tuple type term.
    fn build_tuple_type_term(
        &mut self,
        elements: &[dir::LocalNodeId<dir::TupleElement>],
        form: dir::TupleForm,
        tree: &dir::Tree,
    ) -> TypeTerm {
        let elements = elements
            .iter()
            .filter_map(|element| self.build_tuple_element_term(*element, tree))
            .collect();

        TypeTerm::Tuple {
            form,
            elements,
            is_readonly: false,
        }
    }

    /// Return one tuple element term from tuple type syntax.
    fn build_tuple_element_term(
        &mut self,
        id: dir::LocalNodeId<dir::TupleElement>,
        tree: &dir::Tree,
    ) -> Option<TupleElement> {
        let element = match tree.get(id) {
            // [T]
            dir::TupleElement::Element {
                label,
                value,
                is_optional,
                is_readonly,
            } => TupleElement {
                label: *label,
                ty: self
                    .intern_local_type_variable(tree.module_id, *value)
                    .into(),
                is_optional: *is_optional,
                is_readonly: *is_readonly,
                is_rest: false,
            },
            // [...T]
            dir::TupleElement::Spread { label, value } => TupleElement {
                label: *label,
                ty: self
                    .intern_local_type_variable(tree.module_id, *value)
                    .into(),
                is_optional: false,
                is_readonly: false,
                is_rest: true,
            },
            // ignore damaged syntax
            dir::TupleElement::Error => return None,
        };

        Some(element)
    }

    /// Return shape member terms from object type syntax.
    fn build_shape_member_terms(
        &mut self,
        members: &[dir::LocalNodeId<dir::TypeMember>],
        tree: &dir::Tree,
    ) -> SmallVec<[ShapeMember; 8]> {
        members
            .iter()
            .filter_map(|member| self.build_shape_member_term(*member, tree))
            .collect()
    }

    /// Return one shape member term from object type syntax.
    fn build_shape_member_term(
        &mut self,
        id: dir::LocalNodeId<dir::TypeMember>,
        tree: &dir::Tree,
    ) -> Option<ShapeMember> {
        let member = match tree.get(id) {
            // field: T
            dir::TypeMember::Field {
                key,
                declared_type,
                is_static: false,
                is_optional,
                is_readonly,
            } => {
                let key = key.static_key(tree)?;
                let ty = if let Some(declared_type) = declared_type {
                    self.intern_local_type_variable(tree.module_id, *declared_type)
                } else {
                    self.intern_local_type_variable(tree.module_id, id)
                };

                ShapeMember::Field {
                    key,
                    ty: ty.into(),
                    is_optional: *is_optional,
                    is_readonly: *is_readonly,
                }
            }
            // method(): T
            dir::TypeMember::Method {
                key,
                is_static: false,
                is_optional,
                ..
            } => {
                let key = key.static_key(tree)?;
                let Some(symbol) = self.declaration_symbol(tree.module_id, id.into_any()) else {
                    return None;
                };
                let ty = self.intern_symbol_type_variable(tree.module_id, symbol);

                ShapeMember::Field {
                    key,
                    ty: ty.into(),
                    is_optional: *is_optional,
                    is_readonly: false,
                }
            }
            // (...): T
            dir::TypeMember::CallSignature { .. } => ShapeMember::CallSignature {
                ty: self.intern_local_type_variable(tree.module_id, id).into(),
            },
            // new (...): T
            dir::TypeMember::ConstructSignature { .. } => ShapeMember::ConstructSignature {
                ty: self.intern_local_type_variable(tree.module_id, id).into(),
            },
            // [key: K]: V
            dir::TypeMember::IndexSignature {
                name,
                key_type,
                value_type,
                is_optional,
                is_readonly,
            } => ShapeMember::IndexSignature {
                name: *name,
                key_type: self.intern_local_type_variable(tree.module_id, *key_type).into(),
                value_type: self.intern_local_type_variable(tree.module_id, *value_type).into(),
                is_optional: *is_optional,
                is_readonly: *is_readonly,
            },
            // static field: T
            dir::TypeMember::Field {
                is_static: true, ..
            }
            // static method(): T
            | dir::TypeMember::Method {
                is_static: true, ..
            }
            // type Item = T
            | dir::TypeMember::AssociatedType { .. }
            // const item: T = value
            | dir::TypeMember::AssociatedConst { .. }
            // ignore damaged syntax
            | dir::TypeMember::Error => return None,
        };

        Some(member)
    }

    /// Return one mapped type operation term.
    fn build_mapped_type_term(
        &mut self,
        parameter: dir::LocalNodeId<dir::TypeMappedParameter>,
        readonly: dir::MappedTypeModifier,
        optional: dir::MappedTypeModifier,
        value: Option<dir::LocalNodeId<dir::TypeExpression>>,
        tree: &dir::Tree,
    ) -> Option<TypeTerm> {
        let mapped_parameter = tree.get(parameter);
        let symbol = self.declaration_symbol(tree.module_id, parameter.into_any())?;
        let value = value?;
        let parameter = MappedParameter {
            name: mapped_parameter.name,
            symbol,
            constraint: self
                .intern_local_type_variable(tree.module_id, mapped_parameter.source_type),
            key_remap: mapped_parameter
                .key_remap
                .map(|key_remap| self.intern_local_type_variable(tree.module_id, key_remap)),
        };
        let modifiers = dir::MappedTypeModifiers { readonly, optional };

        let value = self.intern_local_type_variable(tree.module_id, value);
        let operation = self.terms.push(TypeOperationTerm::Mapped {
            parameter,
            modifiers,
            value,
        });

        Some(TypeTerm::Operation(operation))
    }
}
