use destack_dir as dir;
use smallvec::SmallVec;

use crate::sema::{
    GenericArgument, GenericParameterId, MemberRole, MixedObjectSignature, Origin, Receiver,
    TypeSubstitution, VariableKind, WalkState,
};
use crate::{CompilerError, CompilerResult};

impl WalkState<'_, '_> {
    /// Walk one type annotation and commit its type.
    pub(in crate::sema) fn walk_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let ty = self.walk_type_expression_type(id)?;
        self.commit_node_type(id, ty)?;

        Ok(ty)
    }

    /// Return the type denoted by one type expression.
    pub(in crate::sema) fn walk_type_expression_type(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let source = id.into_any();

        // walk by the type expression kind
        match self.tree.get(id) {
            // "ok", 42, true
            dir::TypeExpression::Literal { value } => self.intern_type(dir::Type::Literal(*value)),
            // never, unknown, null, number, ...
            dir::TypeExpression::Keyword { value } => {
                self.intern_type(dir::Type::from(value.clone()))
            }
            // reject intrinsic markers outside declaration values
            dir::TypeExpression::Intrinsic => self.intern_type(dir::Type::Error),
            // (A, B)
            dir::TypeExpression::Tuple { form, elements } => {
                let elements = elements.iter().copied().collect::<SmallVec<[_; 4]>>();
                let mut element_types = Vec::new();
                for element in elements {
                    element_types.push(self.walk_tuple_element(element)?);
                }
                let elements = self.intern_elements(&element_types)?;

                self.intern_type(dir::Type::Tuple(dir::TupleType {
                    form: *form,
                    elements,
                }))
            }
            // T[]
            dir::TypeExpression::Array { element } => {
                let element = self.walk_type_expression(*element)?;
                self.array_type(element)
            }
            // [T]
            dir::TypeExpression::Slice { element } => {
                let element = self.walk_type_expression(*element)?;

                self.intern_type(dir::Type::Slice(dir::SliceType { element }))
            }
            // [T; N]
            dir::TypeExpression::FixedArray { element, length } => {
                let element = self.walk_type_expression(*element)?;
                let count = self.walk_static_term(*length)?;

                self.intern_type(dir::Type::FixedArray(dir::FixedArrayType {
                    element,
                    count,
                }))
            }
            // { name: string }
            dir::TypeExpression::Object { members } => {
                let members = members.iter().copied().collect::<SmallVec<[_; 4]>>();

                self.walk_object_type(id, &members)
            }
            // (value: T) => U
            dir::TypeExpression::Function(function) => {
                let function = function.clone();

                self.walk_function_type(source, &function, None)
            }
            // new (value: string) => User
            dir::TypeExpression::Constructor(constructor) => {
                let constructor = constructor.clone();

                self.walk_constructor_type(source, &constructor, None)
            }
            // 'a, 'static
            dir::TypeExpression::Lifetime { name } => {
                let name = *name;

                // read the lifetime literals from the reserved tick names
                let literal = match self.check.strings().get(name) {
                    "'static" => Some(dir::Lifetime::Static),
                    "'frame" => Some(dir::Lifetime::Frame),
                    "'managed" => Some(dir::Lifetime::Managed),
                    _ => None,
                };
                match literal {
                    Some(literal) => self.check.lifetime_literal(literal),
                    None => self.walk_reference_type(id, &dir::Path::from_segment(name), &[]),
                }
            }
            // Foo, geom.Mesh<Point>
            dir::TypeExpression::Reference {
                path,
                generic_arguments,
            } => {
                let path = path.clone();
                let generic_arguments = generic_arguments
                    .iter()
                    .copied()
                    .collect::<SmallVec<[_; 4]>>();

                self.walk_reference_type(id, &path, &generic_arguments)
            }
            // T.Item
            dir::TypeExpression::Member {
                left,
                name,
                generic_arguments,
            } => {
                let name = *name;
                let generic_arguments = generic_arguments
                    .iter()
                    .copied()
                    .collect::<SmallVec<[_; 4]>>();

                // bind this projections to their declaring scope
                let qualifier = match self.tree.get(*left) {
                    dir::TypeExpression::This => {
                        let receiver = self.flow().current_receiver();

                        self.receiver_projection_scope(receiver)?
                    }
                    _ => None,
                };
                let owner = self.walk_type_expression(*left)?;
                let arguments = self.walk_generic_arguments(&generic_arguments)?;
                let arguments: Vec<_> = arguments.into_iter().map(|argument| argument.ty).collect();
                let arguments = self.intern_type_ids(&arguments)?;

                // retain the written subject, which the write pass resolves once inference solves
                let subject = dir::MemberSubject::new(owner, owner, dir::MemberSpace::Static)
                    .with_scope(self.flow().template_scope());
                self.check
                    .module_mut(self.module)
                    .members_tail
                    .commit_subject(
                        dir::MemberSite::Node(id.into_global_any(self.module)),
                        subject,
                        Some(dir::StaticKey::Name(name)),
                    );

                self.intern_member(dir::MemberType {
                    owner,
                    key: dir::StaticKey::Name(name),
                    arguments,
                    qualifier,
                })
            }
            // 0..10
            dir::TypeExpression::Range {
                start,
                end,
                end_kind,
            } => self.walk_range_type(id, *start, *end, *end_kind),
            // const outside `as const` positions
            dir::TypeExpression::Const => {
                self.check.report_invalid_const_type(self.module, source);

                self.intern_type(dir::Type::Error)
            }
            // this, read at an extension's receiver and polymorphic in a class
            dir::TypeExpression::This => {
                let receiver = self.flow().current_receiver();
                let extension_receiver = match receiver {
                    Some(receiver) => match receiver.declaration {
                        Some(declaration)
                            if matches!(
                                self.check.symbol_kind(declaration)?,
                                dir::SymbolKind::Extension
                            ) =>
                        {
                            Some(receiver.ty)
                        }
                        _ => None,
                    },
                    None => None,
                };
                match extension_receiver {
                    Some(ty) => Ok(ty),
                    None => self.intern_type(dir::Type::This),
                }
            }
            // readonly T
            dir::TypeExpression::Readonly { target_type } => {
                let value = self.walk_type_expression(*target_type)?;

                self.intern_type(dir::Type::Form(dir::FormType {
                    form: dir::Form::Readonly,
                    value,
                }))
            }
            // keyof T
            dir::TypeExpression::KeyOf { target_type } => {
                let target = self.walk_type_expression(*target_type)?;

                self.intern_operation(dir::TypeOperation::KeyOf(dir::UnaryType { target }))
            }
            // typeof value
            dir::TypeExpression::TypeOf { value } => self.walk_typeof_type(*value),
            // static value
            dir::TypeExpression::StaticValue { expression } => self.walk_static_term(*expression),
            // T! strips nullish members distributively
            dir::TypeExpression::Must { target_type } => {
                let target = self.walk_type_expression(*target_type)?;
                let null = self.intern_type(dir::Type::Null)?;
                let undefined = self.intern_type(dir::Type::Undefined)?;
                let nullish = self.normalized_union_type([null, undefined])?;
                let never = self.intern_type(dir::Type::Never)?;

                self.intern_operation(dir::TypeOperation::Conditional(dir::ConditionalType {
                    left: target,
                    right: nullish,
                    then_type: never,
                    else_type: target,
                    is_distributive: true,
                }))
            }
            // !T
            dir::TypeExpression::Not { target_type } => {
                let target = self.walk_type_expression(*target_type)?;

                self.intern_operation(dir::TypeOperation::StaticUnary(dir::StaticUnaryType {
                    operator: dir::StaticUnaryOperator::Not,
                    target,
                }))
            }
            // ^T
            dir::TypeExpression::OwnedOf {
                mutability,
                target_type,
                ..
            } => {
                let value = self.walk_type_expression(*target_type)?;
                let value = if *mutability == Some(dir::Mutability::Immutable) {
                    self.intern_type(dir::Type::Form(dir::FormType {
                        form: dir::Form::Readonly,
                        value,
                    }))?
                } else {
                    value
                };
                let owned = self.intern_type(dir::Type::Form(dir::FormType {
                    form: dir::Form::Owned,
                    value,
                }))?;

                // interpret a family-default owned form as its bare payload
                let origin = Origin::Node(id.into_global_any(self.module), None);
                self.check.reduce_default_ownership_chain(origin, owned)
            }
            // walk the written borrow lifetime, or open the elided hole
            dir::TypeExpression::BorrowedOf {
                lifetime,
                access,
                target_type,
                ..
            } => self.walk_borrowed_type(id, *lifetime, *access, *target_type),
            // *T
            dir::TypeExpression::PointerOf { target_type, .. } => {
                let value = self.walk_type_expression(*target_type)?;

                self.intern_type(dir::Type::Form(dir::FormType {
                    form: dir::Form::Raw,
                    value,
                }))
            }
            // A | B
            dir::TypeExpression::Union { elements } => {
                let elements = elements.iter().copied().collect::<SmallVec<[_; 4]>>();
                let mut element_types = Vec::new();
                for element in elements {
                    element_types.push(self.walk_type_expression(element)?);
                }

                self.normalized_union_type(element_types)
            }
            // A & B
            dir::TypeExpression::Intersection { elements } => {
                let elements = elements.iter().copied().collect::<SmallVec<[_; 4]>>();
                let mut element_types = Vec::new();
                for element in elements {
                    element_types.push(self.walk_type_expression(element)?);
                }

                let elements = self.intern_type_ids(&element_types)?;
                let written =
                    self.intern_type(dir::Type::Intersection(dir::IntersectionType { elements }))?;

                // interpret the written intersection as its reduced form
                let origin = Origin::Node(
                    id.into_global_any(self.module),
                    self.flow().template_scope(),
                );
                self.check
                    .reduce_intersection(origin, written, &element_types)
            }
            // T extends U ? X : Y
            dir::TypeExpression::Conditional {
                left,
                extends_type,
                then_type,
                else_type,
            } => {
                let (extends_type, then_type, else_type) = (*extends_type, *then_type, *else_type);
                let left = self.walk_type_expression(*left)?;

                // distribute only naked parameter scrutinees over unions
                let is_distributive = matches!(self.check.ty(left)?, dir::Type::Parameter(_));
                self.extends_clauses += 1;
                let right = self.walk_type_expression(extends_type)?;
                self.extends_clauses -= 1;

                // assume the parameter's extension inside the true branch, as a where clause would
                if is_distributive {
                    let source = source.into_global(self.module);
                    let template = self
                        .check
                        .open_generic_template(source, self.flow().template_scope())?;
                    self.check.push_template_predicate(
                        template,
                        dir::WherePredicate {
                            source,
                            relation: dir::WhereRelation::Satisfies,
                            left,
                            right,
                        },
                    )?;
                }

                // expose each capture's constraint within the conditional scope
                for binder in self.check.collect_infer_binders(right)? {
                    if let (Some(symbol), Some(constraint)) = (binder.symbol, binder.constraint) {
                        let source = source.into_global(self.module);
                        let template = self
                            .check
                            .open_generic_template(source, self.flow().template_scope())?;
                        let arguments = self.intern_type_ids(&[])?;
                        let capture =
                            self.intern_type(dir::Type::Application(dir::GenericApplication {
                                symbol,
                                arguments,
                            }))?;
                        self.check.push_template_predicate(
                            template,
                            dir::WherePredicate {
                                source,
                                relation: dir::WhereRelation::Satisfies,
                                left: capture,
                                right: constraint,
                            },
                        )?;
                    }
                }

                let then_type = self.walk_type_expression(then_type)?;
                let else_type = self.walk_type_expression(else_type)?;

                self.intern_operation(dir::TypeOperation::Conditional(dir::ConditionalType {
                    left,
                    right,
                    then_type,
                    else_type,
                    is_distributive,
                }))
            }
            // T extends U, T implements U
            dir::TypeExpression::Extends { left, right }
            | dir::TypeExpression::Implements { left, right } => {
                let left = self.walk_type_expression(*left)?;
                let right = self.walk_type_expression(*right)?;
                let then_type =
                    self.intern_type(dir::Type::Literal(dir::Literal::Boolean(true)))?;
                let else_type =
                    self.intern_type(dir::Type::Literal(dir::Literal::Boolean(false)))?;

                self.intern_operation(dir::TypeOperation::Conditional(dir::ConditionalType {
                    left,
                    right,
                    then_type,
                    else_type,
                    is_distributive: false,
                }))
            }
            // { [K in keyof T]: T[K] }
            dir::TypeExpression::Mapped {
                parameter,
                readonly,
                optional,
                value,
            } => self.walk_mapped_type(id, *parameter, *readonly, *optional, *value),
            // T[K]
            dir::TypeExpression::Index { left, index } => {
                let left = self.walk_type_expression(*left)?;
                let index = self.walk_type_expression(*index)?;
                let ty = self
                    .intern_operation(dir::TypeOperation::Index(dir::IndexType { left, index }))?;

                Ok(ty)
            }
            // `get${Name}`
            dir::TypeExpression::TemplateLiteral { strings, spans } => {
                let strings = strings.clone();
                let spans = spans.iter().copied().collect::<SmallVec<[_; 4]>>();
                let mut span_types = Vec::new();
                for span in spans {
                    span_types.push(self.walk_type_expression(span)?);
                }
                let strings = self.intern_strings(&strings)?;
                let spans = self.intern_type_ids(&span_types)?;

                self.intern_operation(dir::TypeOperation::TemplateLiteral(
                    dir::TemplateLiteralType { strings, spans },
                ))
            }
            // _, infer T
            dir::TypeExpression::Infer {
                form,
                name,
                constraint,
            } => {
                let (form, name, constraint) = (*form, *name, *constraint);
                let constraint = constraint
                    .map(|constraint| self.walk_type_expression(constraint))
                    .transpose()?;

                self.walk_infer_type_expression(id, form, name, constraint)
            }
            // produce the error type for damaged children
            dir::TypeExpression::Missing | dir::TypeExpression::Error => {
                self.intern_type(dir::Type::Error)
            }
        }
    }

    /// Walk a rest annotation and declare an unconstrained capture as a parameter sequence.
    pub(in crate::sema) fn walk_rest_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // read an ordinary annotation as the collection
        let dir::TypeExpression::Infer {
            form: dir::InferForm::Infer,
            name,
            constraint: None,
        } = *self.tree.get(id)
        else {
            return self.walk_value_type(id);
        };

        // infer the complete tuple of arguments, including optional and rest elements
        let unknown = self.intern_type(dir::Type::Unknown)?;
        let symbol = self.language_symbol(dir::LanguageItem::ReadonlyArray)?;
        let arguments = self.intern_type_ids(&[unknown])?;
        let array = self.intern_type(dir::Type::Application(dir::GenericApplication {
            symbol,
            arguments,
        }))?;
        let constraint = self.intern_tuple(&[dir::TypeElement {
            is_rest: true,
            is_readonly: true,
            ..dir::TypeElement::new(array)
        }])?;
        let ty =
            self.walk_infer_type_expression(id, dir::InferForm::Infer, name, Some(constraint))?;
        self.commit_node_type(id, ty)?;

        Ok(ty)
    }

    /// Walk one `_` or `infer T` type expression.
    fn walk_infer_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        form: dir::InferForm,
        name: Option<dir::StringId>,
        constraint: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let source = id.into_any();

        // walk by the infer form kind
        match form {
            // open anonymous holes for ordinary inference
            dir::InferForm::Hole => {
                if let Some(rejected) = self.report_declaration_hole(source)? {
                    return Ok(rejected);
                }

                self.open_type_hole(source, VariableKind::Type)
            }

            // preserve named infer bindings for conditional matching
            dir::InferForm::Infer => {
                if self.extends_clauses == 0 {
                    self.check
                        .report_infer_outside_conditional(self.module, source);

                    return self.intern_type(dir::Type::Error);
                }
                let symbol = match name {
                    Some(_) => Some(self.declared_symbol(source).ok_or_else(|| {
                        CompilerError::Internal {
                            message: format!(
                                "named infer binding {id:?} has no declaration symbol"
                            ),
                        }
                    })?),
                    None => None,
                };

                self.intern_operation(dir::TypeOperation::Infer(dir::InferType {
                    name,
                    symbol,
                    constraint,
                }))
            }
        }
    }

    /// Require a value declaration as the operand of a type query.
    fn walk_typeof_type(
        &mut self,
        value: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // resolve the complete type query
        let ty = self.walk_typeof_reference(value)?;

        // check the declaration to which the query applies generic arguments
        let mut reference = value;
        while let dir::Expression::Instantiation { left, .. } = self.tree.get(reference) {
            reference = *left;
        }
        let source = reference.into_global_any(self.module);
        if let Some(resolution) = self.check.name_decision(source).cloned()
            && !self.check.check_value_reference(source, &resolution)?
        {
            return self.intern_type(dir::Type::Error);
        }

        Ok(ty)
    }

    /// Resolve a type query to declarations and member type operations.
    fn walk_typeof_reference(
        &mut self,
        value: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let source = value.into_global_any(self.module);

        // apply generic arguments to the queried type without evaluating its value
        if let dir::Expression::Instantiation {
            left,
            generic_arguments,
        } = self.tree.get(value).clone()
        {
            let target = self.walk_typeof_reference(left)?;
            let arguments = self.walk_generic_arguments(&generic_arguments)?;
            let arguments = arguments
                .iter()
                .map(|argument| argument.ty)
                .collect::<SmallVec<[_; 4]>>();
            let arguments = self.intern_type_ids(&arguments)?;

            return self.intern_operation(dir::TypeOperation::Instantiation(
                dir::InstantiationType { target, arguments },
            ));
        }

        // preserve the parser's diagnosis of a missing operand
        if matches!(
            self.tree.get(value),
            dir::Expression::Missing | dir::Expression::Error
        ) {
            self.check
                .commit_decision(source, dir::Decision::Poisoned)?;

            return self.intern_type(dir::Type::Error);
        }

        // require a name or a named member path
        let Some(path) = self.tree.reference_path(value) else {
            self.check
                .report_invalid_type_query(self.module, value.into_any());
            self.check
                .commit_decision(source, dir::Decision::Rejected)?;

            return self.intern_type(dir::Type::Error);
        };
        let reference = self
            .check
            .module(self.module)
            .resolved
            .references
            .get(source)
            .cloned();

        // resolve complete declaration paths before projecting runtime members
        match reference {
            Some(dir::Reference::Bound(symbols)) => {
                let symbols = self.check.present_symbols(&symbols);
                if symbols.is_empty() {
                    self.check
                        .report_unresolved_reference(self.module, value.into_any(), &path)?;
                    self.check
                        .commit_decision(source, dir::Decision::Rejected)?;

                    return self.intern_type(dir::Type::Error);
                }
                let resolution = dir::NameResolution::from_symbols(symbols.to_vec());
                self.check.commit_name(source, resolution.clone())?;

                // query each overload without evaluating its value
                let mut types = SmallVec::<[_; 2]>::new();
                for symbol in symbols {
                    self.capture_symbol_reference(source, symbol)?;
                    let ty =
                        match self.check.reduce_typeof(symbol)? {
                            Some(ty) => ty,
                            None => self.intern_operation(dir::TypeOperation::TypeOf(
                                dir::TypeOfType { symbol },
                            ))?,
                        };
                    types.push(ty);
                }
                if let [ty] = types.as_slice() {
                    return Ok(*ty);
                }
                let elements = self.check.intern_type_ids(&types)?;

                self.intern_type(dir::Type::Intersection(dir::IntersectionType { elements }))
            }
            Some(dir::Reference::Projected { .. }) => {
                let dir::Expression::Member {
                    left,
                    name: Some(name),
                    ..
                } = self.tree.get(value).clone()
                else {
                    return Err(CompilerError::Internal {
                        message: format!("projected type query {source:?} has no member"),
                    });
                };
                let left = self.walk_typeof_reference(left)?;
                let index = self.intern_type(dir::Type::Literal(dir::Literal::String(name)))?;

                self.intern_operation(dir::TypeOperation::Index(dir::IndexType { left, index }))
            }
            Some(dir::Reference::TypeLiteral(literal)) => {
                let denoted = self.intern_type(dir::Type::from(literal))?;
                let resolution = dir::NameResolution::new_type(denoted);
                self.check.commit_name(source, resolution.clone())?;
                Ok(denoted)
            }
            Some(dir::Reference::Ambiguous(_)) => {
                self.check
                    .report_ambiguous_reference(self.module, value.into_any(), &path)?;
                self.check
                    .commit_decision(source, dir::Decision::Rejected)?;

                self.intern_type(dir::Type::Error)
            }
            Some(dir::Reference::Missing | dir::Reference::Namespace { .. }) => {
                self.check
                    .report_unresolved_reference(self.module, value.into_any(), &path)?;
                self.check
                    .commit_decision(source, dir::Decision::Rejected)?;

                self.intern_type(dir::Type::Error)
            }
            None => Err(CompilerError::Internal {
                message: format!("type query {source:?} has no resolved reference"),
            }),
        }
    }

    /// Walk one construct target, such as the `Wrap` in `Wrap { value }`.
    pub(in crate::sema) fn walk_construct_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<()> {
        self.check.visit_site(id.into_global_any(self.module))?;

        // skip an inferred hole, which names no construct target
        if matches!(
            self.tree.get(id),
            dir::TypeExpression::Infer {
                form: dir::InferForm::Hole,
                ..
            }
        ) {
            return Ok(());
        }

        // capture reference heads without requiring a complete annotation
        if let dir::TypeExpression::Reference {
            path,
            generic_arguments,
        } = self.tree.get(id)
        {
            let path = path.clone();
            let generic_arguments = generic_arguments
                .iter()
                .copied()
                .collect::<SmallVec<[_; 4]>>();
            self.walk_construct_reference_type(id, &path, &generic_arguments)?;

            return Ok(());
        }

        self.walk_type_expression(id)?;

        Ok(())
    }

    /// Walk one construct reference head without applying omitted arguments.
    fn walk_construct_reference_type(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        path: &dir::Path,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    ) -> CompilerResult<()> {
        let source = id.into_global_any(self.module);

        // leave omitted arguments on direct nominal types for construction inference
        if let Some(dir::Reference::Bound(symbols)) = self.resolved_type_reference(id) {
            let symbols = self.check.present_symbols(&symbols);
            if let [symbol] = symbols.as_slice()
                && self.check.symbol_kind(*symbol)?.is_nominal()
            {
                self.commit_reference_name(source, *symbol)?;
                self.walk_generic_arguments(generic_arguments)?;

                return Ok(());
            }
        }

        // resolve aliases and projected paths through ordinary type checking
        let ty = self.walk_reference_type(id, path, generic_arguments)?;
        self.commit_node_type(id, ty)?;

        Ok(())
    }

    /// Return the type for one reference annotation, such as `Foo<T>`.
    fn walk_reference_type(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        path: &dir::Path,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    ) -> CompilerResult<dir::GlobalTypeId> {
        // read the resolver output for this lexical reference
        let reference = self.resolved_type_reference(id);

        // denote by the reference the resolver bound
        match reference {
            // a type literal name denotes its builtin type
            Some(dir::Reference::TypeLiteral(literal)) => {
                self.intern_type(dir::Type::from(literal))
            }
            // use one resolved type declaration directly
            Some(dir::Reference::Bound(symbols)) => {
                self.walk_bound_reference_type(id, path, generic_arguments, &symbols)
            }

            // project a type-member path from the resolved base declaration
            Some(dir::Reference::Projected {
                base: dir::ReferenceTarget::Symbol(base),
                from,
            }) => self.walk_member_path_type(
                id,
                base,
                from,
                &path.segments[from as usize..],
                generic_arguments,
            ),

            // reject resolve conflicts in type position
            Some(dir::Reference::Ambiguous(_)) => {
                self.check
                    .report_ambiguous_reference(self.module, id.into_any(), path)?;

                self.intern_type(dir::Type::Error)
            }

            // reject namespaces and missing references in type position
            Some(dir::Reference::Namespace { .. })
            | Some(dir::Reference::Projected {
                base: dir::ReferenceTarget::Namespace(_),
                ..
            })
            | Some(dir::Reference::Missing) => {
                self.check
                    .report_unresolved_reference(self.module, id.into_any(), path)?;

                self.intern_type(dir::Type::Error)
            }

            // require resolve to write every lexical reference
            None => Err(CompilerError::Internal {
                message: format!(
                    "type reference {:?} has no resolved name",
                    id.into_global_any(self.module)
                ),
            }),
        }
    }

    /// Walk one generic argument, keeping a bare value binding symbolic for its parameter slot.
    pub(in crate::sema) fn walk_argument_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // recognize a bare reference naming one value binding
        if let dir::TypeExpression::Reference {
            generic_arguments, ..
        } = self.tree.get(id)
            && generic_arguments.is_empty()
            && let Some(dir::Reference::Bound(symbols)) = self.resolved_type_reference(id)
            && let [symbol] = self.check.present_symbols(&symbols).as_slice()
            && self.check.symbol_kind(*symbol)?.is_binding()
        {
            let symbol = *symbol;
            let source = id.into_global_any(self.module);
            self.commit_reference_name(source, symbol)?;
            let symbolic = dir::Type::Reference(dir::TypeReference::new(symbol));

            return self.intern_type(symbolic);
        }

        self.walk_type_expression(id)
    }

    /// Return the resolver output for one type reference.
    fn resolved_type_reference(
        &self,
        id: dir::LocalNodeId<dir::TypeExpression>,
    ) -> Option<dir::Reference> {
        let source = id.into_global_any(self.module);

        // read the resolver output for this reference
        self.check
            .module(self.module)
            .resolved
            .references
            .get(source)
            .cloned()
    }

    /// Commit one resolved reference name.
    fn commit_reference_name(
        &mut self,
        source: dir::GlobalNodeIdAny,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        self.capture_symbol_reference(source, symbol)?;
        self.check
            .commit_name(source, dir::NameResolution::new(symbol))
    }

    /// Return the type for one resolver-bound reference annotation.
    fn walk_bound_reference_type(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        path: &dir::Path,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
        symbols: &[dir::GlobalSymbolId],
    ) -> CompilerResult<dir::GlobalTypeId> {
        let source = id.into_global_any(self.module);
        let symbols = self.check.present_symbols(symbols);

        // denote by the symbols the reference selects
        match symbols.as_slice() {
            // reject malformed resolver state
            [] => Err(CompilerError::Internal {
                message: format!("type reference {source:?} resolved to no symbols"),
            }),

            // use the single resolved type declaration
            [symbol] => self.walk_symbol_reference_type(id, *symbol, generic_arguments),

            // reject annotations that name more than one declaration
            _ => {
                self.check
                    .report_ambiguous_reference(self.module, id.into_any(), path)?;

                self.intern_type(dir::Type::Error)
            }
        }
    }

    /// Return the type for one declaration symbol with written type arguments.
    fn walk_symbol_reference_type(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        symbol: dir::GlobalSymbolId,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    ) -> CompilerResult<dir::GlobalTypeId> {
        let source = id.into_global_any(self.module);

        // commit the resolved name for snapshots and downstream selection
        self.commit_reference_name(source, symbol)?;

        // apply written type arguments and open omitted slots
        let applied = self.walk_generic_arguments(generic_arguments)?;

        self.apply_written_reference(id.into_any(), symbol, &applied)
    }

    /// Return the type identity of one receiver's declaring scope.
    fn receiver_projection_scope(
        &mut self,
        receiver: Option<Receiver>,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let Some(receiver) = receiver else {
            return Ok(None);
        };
        let Some(declaration) = receiver.declaration else {
            return Ok(None);
        };

        // extensions project by their declaration reference
        if matches!(
            self.check.symbol_kind(declaration)?,
            dir::SymbolKind::Extension
        ) {
            let reference = dir::Type::Reference(dir::TypeReference::new(declaration));

            return Ok(Some(self.intern_type(reference)?));
        }

        Ok(Some(receiver.ty))
    }

    /// Apply one written declaration reference and return its type.
    fn apply_written_reference(
        &mut self,
        source: dir::LocalNodeIdAny,
        symbol: dir::GlobalSymbolId,
        applied: &[GenericArgument],
    ) -> CompilerResult<dir::GlobalTypeId> {
        // return the parameter type for generic parameter names
        if let Some(parameter) = self.check.parameter_by_symbol(symbol)? {
            if !applied.is_empty() {
                let name = self.check.format_symbol(symbol);
                self.check
                    .report_wrong_generic_arity(self.module, source, name, 0, applied.len());

                return self.intern_type(dir::Type::Error);
            }

            return self.intern_type(dir::Type::Parameter(parameter));
        }

        // reject a value binding written in type position
        if self.check.symbol_kind(symbol)?.is_binding() {
            let name = self.check.format_symbol(symbol);
            self.check
                .report_value_used_as_type(self.module, source, name);

            return self.intern_type(dir::Type::Error);
        }

        // bind foreign references by their declared parameter kinds
        if self.check.is_declaring() && !self.check.is_own_module(symbol.module_id) {
            let positional = applied
                .iter()
                .filter(|argument| argument.name.is_none())
                .map(|argument| argument.ty)
                .collect::<Vec<_>>();
            let arguments = self.bind_foreign_arguments(source, symbol, &positional)?;

            return self.build_application_type(source, symbol, &arguments, applied);
        }

        // bind written arguments to the declaration's parameter slots
        let Some(arguments) = self.bind_written_arguments(source, symbol, applied)? else {
            return self.intern_type(dir::Type::Error);
        };

        self.build_application_type(source, symbol, &arguments, applied)
    }

    /// Return the value binding one symbolic argument reference names.
    fn argument_binding_symbol(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalSymbolId>> {
        let dir::Type::Reference(reference) = self.check.ty(ty)? else {
            return Ok(None);
        };
        let is_symbolic = self.check.symbol_kind(reference.symbol)?.is_binding();

        Ok(is_symbolic.then_some(reference.symbol))
    }

    /// Settle one const slot's value binding to its static value.
    fn static_binding_argument(
        &mut self,
        source: dir::LocalNodeIdAny,
        symbol: dir::GlobalSymbolId,
        symbolic: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // read the committed term's singleton type
        if let Some(value) = self.check.static_value(symbol)? {
            return Ok(value);
        }

        // keep the reference symbolic while declaring
        if self.check.is_declaring() {
            return Ok(symbolic);
        }

        // report values the static evaluator cannot decide
        self.check
            .report_undecidable_static_value(self.module, source);

        self.intern_type(dir::Type::Error)
    }

    /// Report one written application whose arguments cannot bind the slots.
    fn report_binding_arity(
        &mut self,
        source: dir::LocalNodeIdAny,
        symbol: dir::GlobalSymbolId,
        parameters: &[GenericParameterId],
        written: usize,
    ) -> CompilerResult<()> {
        let name = self.check.format_symbol(symbol);
        let expected = self.check.writable_parameter_count(parameters)?;
        self.check
            .report_wrong_generic_arity(self.module, source, name, expected, written);

        Ok(())
    }

    /// Bind written arguments to a declaration's parameter slots in order.
    fn bind_written_arguments(
        &mut self,
        source: dir::LocalNodeIdAny,
        symbol: dir::GlobalSymbolId,
        applied: &[GenericArgument],
    ) -> CompilerResult<Option<Vec<dir::GlobalTypeId>>> {
        let parameters = match self.check.symbol_template(symbol)? {
            Some(template) => self.check.generic_template_parameters(template)?,
            None => SmallVec::new(),
        };
        let written = applied
            .iter()
            .filter(|argument| argument.name.is_none())
            .collect::<SmallVec<[_; 4]>>();
        let origin = Origin::Node(
            source.into_global(self.module),
            self.flow().template_scope(),
        );

        // bind each declared parameter to its written argument
        let mut substitution = TypeSubstitution::default();
        let mut cursor = 0;
        for parameter in parameters.iter().copied() {
            let Some(binding) = self.check.generic_parameter(parameter)?.cloned() else {
                return Err(CompilerError::Internal {
                    message: "written type application names a missing generic parameter"
                        .to_string(),
                });
            };

            // memory parameters consume only written arguments of their own kind
            let kind_matches = cursor < written.len()
                && self
                    .check
                    .argument_fills_parameter(&binding, written[cursor].ty)?;

            // bind the next written argument to the next writable slot
            let argument = if binding.is_writable() && kind_matches {
                let argument = written[cursor].ty;
                cursor += 1;

                // interpret a value binding argument by the slot's kind
                match self.argument_binding_symbol(argument)? {
                    // resolve a const slot's binding to its static value
                    Some(value) if binding.is_const => {
                        self.static_binding_argument(source, value, argument)?
                    }
                    // reject a value binding filling a type slot
                    Some(value) => {
                        let name = self.check.format_symbol(value);
                        self.check
                            .report_value_used_as_type(self.module, source, name);

                        self.intern_type(dir::Type::Error)?
                    }
                    None => argument,
                }
            }
            // evaluate defaults against the application built so far
            else if let Some(default) = binding.default {
                self.check.substitute_type(default, &substitution)?
            }
            // elide omitted memory parameters like unwritten borrow lifetimes
            else if let Some(kind) = binding.memory_parameter() {
                match kind {
                    dir::MemoryParameter::Region => self.elided_borrow_region(source, None)?,
                    kind => self.elided_memory_component(source, kind)?,
                }
            }
            // reject unbound parameters
            else {
                self.report_binding_arity(source, symbol, &parameters, written.len())?;

                return Ok(None);
            };

            // store memory arguments canonically
            let argument = match binding.memory_parameter().is_some() {
                true => self.check.normalize_memory_component(origin, argument)?,
                false => argument,
            };

            substitution
                .bindings
                .push(dir::GenericArgumentBinding::new(parameter, argument));
        }

        // reject written arguments that no slot consumed
        if cursor < written.len() {
            self.report_binding_arity(source, symbol, &parameters, written.len())?;

            return Ok(None);
        }

        Ok(Some(substitution.arguments().collect()))
    }

    /// Build one application type and apply its written refinements.
    fn build_application_type(
        &mut self,
        source: dir::LocalNodeIdAny,
        symbol: dir::GlobalSymbolId,
        arguments: &[dir::GlobalTypeId],
        applied: &[GenericArgument],
    ) -> CompilerResult<dir::GlobalTypeId> {
        // build memory form applications as their canonical forms
        let ty = if let Some(item) = self.check.language_item(symbol)?
            && item.is_memory_form()
        {
            self.build_memory_form(source, item, symbol, arguments)?
        } else {
            let arguments = self.intern_type_ids(arguments)?;
            self.intern_type(dir::Type::Application(dir::GenericApplication {
                symbol,
                arguments,
            }))?
        };

        self.apply_named_refinements(ty, applied)
    }

    /// Bind arguments to a foreign template's parameters by kind, eliding omitted memory ones.
    fn bind_foreign_arguments(
        &mut self,
        source: dir::LocalNodeIdAny,
        symbol: dir::GlobalSymbolId,
        written: &[dir::GlobalTypeId],
    ) -> CompilerResult<Vec<dir::GlobalTypeId>> {
        // memory forms build their canonical form from the written arguments alone
        if self
            .check
            .language_item(symbol)?
            .is_some_and(|item| item.is_memory_form())
        {
            return Ok(written.to_vec());
        }
        let parameters = self.check.template_parameters(symbol)?;

        // bind a complete argument list positionally, blind to unresolved argument shapes
        if written.len() == parameters.len() {
            return Ok(written.to_vec());
        }

        let mut arguments = Vec::with_capacity(parameters.len());
        let mut cursor = 0;
        for parameter in parameters {
            // take the next written argument the parameter's kind admits
            let fills = cursor < written.len()
                && self
                    .check
                    .argument_fills_kind(parameter.kind, written[cursor])?;
            if fills {
                arguments.push(written[cursor]);
                cursor += 1;

                continue;
            }

            // leave omitted parameters with defaults to their declared defaults
            if parameter.has_default {
                break;
            }

            // elide omitted memory parameters like unwritten borrow lifetimes
            match parameter.kind {
                dir::GenericParameterKind::Memory(dir::MemoryParameter::Region) => {
                    arguments.push(self.elided_borrow_region(source, None)?);
                }
                dir::GenericParameterKind::Memory(memory) => {
                    arguments.push(self.elided_memory_component(source, memory)?);
                }
                dir::GenericParameterKind::Type => break,
            }
        }
        arguments.extend(written[cursor..].iter().copied());

        Ok(arguments)
    }

    /// Build one written memory form application as its canonical form.
    fn build_memory_form(
        &mut self,
        source: dir::LocalNodeIdAny,
        item: dir::LanguageItem,
        symbol: dir::GlobalSymbolId,
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<dir::GlobalTypeId> {
        // require the payload the form stores
        let Some(value) = arguments.first().copied() else {
            let name = self.check.format_symbol(symbol);
            self.check
                .report_wrong_generic_arity(self.module, source, name, 1, 0);

            return self.intern_type(dir::Type::Error);
        };

        // anchor the written components at the application node
        let origin = Origin::Node(
            source.into_global(self.module),
            self.flow().template_scope(),
        );

        // build the form the item constructs
        let form = match item {
            // take the plain forms the item names
            dir::LanguageItem::Owned => dir::Form::Owned,
            dir::LanguageItem::Raw => dir::Form::Raw,
            dir::LanguageItem::Readonly => dir::Form::Readonly,
            // keep the written borrow components, eliding like the `&T` sugar
            dir::LanguageItem::Borrowed => {
                // slide an access-kinded second argument into the access slot
                let second = arguments.get(1).copied();
                let third = arguments.get(2).copied();
                let (region_argument, access_argument) = match (second, third) {
                    (Some(second), None)
                        if self.check.memory_kind(second)?
                            == Some(dir::MemoryParameter::Access) =>
                    {
                        (None, Some(second))
                    }
                    slots => slots,
                };

                let region = match region_argument {
                    Some(region) => {
                        let region = self.check.normalize_memory_component(origin, region)?;

                        self.borrow_region(source, region, value)?
                    }
                    None => self.elided_borrow_region(source, Some(value))?,
                };
                let access = match access_argument {
                    Some(access) => self.check.normalize_memory_component(origin, access)?,
                    None => self.access_literal(dir::Access::BARE)?,
                };

                return self.check.borrow_value(region, access, value);
            }
            // fail on every other head
            _ => {
                return Err(CompilerError::Internal {
                    message: "memory form build entered another language item".to_string(),
                });
            }
        };

        // interpret a form redundantly repeating its payload's family default as the payload
        let written = self.intern_type(dir::Type::Form(dir::FormType { form, value }))?;
        self.check.reduce_default_ownership_chain(origin, written)
    }

    /// Wrap one application with its named refinements in canonical key order.
    fn apply_named_refinements(
        &mut self,
        ty: dir::GlobalTypeId,
        applied: &[GenericArgument],
    ) -> CompilerResult<dir::GlobalTypeId> {
        // retain each named argument's subject for the write pass to resolve after inference
        for argument in applied {
            let Some(name) = argument.name else {
                continue;
            };
            let site = dir::MemberSite::Node(argument.id.into_global_any(self.module));
            let subject = dir::MemberSubject::new(ty, ty, dir::MemberSpace::Static)
                .with_scope(self.flow().template_scope());
            self.check
                .module_mut(self.module)
                .members_tail
                .commit_subject(site, subject, Some(dir::StaticKey::Name(name)));
        }

        // refine the application by its named arguments
        let refinements = applied
            .iter()
            .filter_map(|argument| {
                argument
                    .name
                    .map(|name| (dir::StaticKey::Name(name), argument.ty))
            })
            .collect::<SmallVec<[_; 2]>>();

        self.check.intern_refinements(ty, &refinements)
    }

    /// Return a type-member path from one resolved base symbol.
    fn walk_member_path_type(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        base: dir::GlobalSymbolId,
        from: u32,
        tail: &[dir::StringId],
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    ) -> CompilerResult<dir::GlobalTypeId> {
        let source = id.into_global_any(self.module);
        self.commit_reference_name(source, base)?;

        // start from the resolved base symbol
        let mut ty = self.apply_written_reference(id.into_any(), base, &[])?;

        // append each remaining path segment as a type member
        for (index, segment) in tail.iter().copied().enumerate() {
            let is_last = index + 1 == tail.len();
            let arguments: Vec<dir::GlobalTypeId> = if is_last && !generic_arguments.is_empty() {
                self.walk_generic_arguments(generic_arguments)?
                    .into_iter()
                    .map(|argument| argument.ty)
                    .collect()
            } else {
                Vec::new()
            };

            let arguments = self.intern_type_ids(&arguments)?;

            // commit the lookup subject for this path segment
            let path_segment = from
                .checked_add(index as u32)
                .and_then(|index| u16::try_from(index).ok())
                .ok_or_else(|| CompilerError::Internal {
                    message: format!("type reference {source:?} overflows its segment index"),
                })?;
            let site = dir::MemberSite::Path {
                node: source,
                segment: path_segment,
            };

            // retain the written subject, which the write pass resolves once inference solves
            let subject = dir::MemberSubject::new(ty, ty, dir::MemberSpace::Static)
                .with_scope(self.flow().template_scope());
            self.check
                .module_mut(self.module)
                .members_tail
                .commit_subject(site, subject, Some(dir::StaticKey::Name(segment)));

            ty = self.intern_member(dir::MemberType {
                owner: ty,
                key: dir::StaticKey::Name(segment),
                arguments,
                qualifier: None,
            })?;
        }

        Ok(ty)
    }

    /// Return the concrete type one object type expression writes.
    fn walk_object_type(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        members: &[dir::LocalNodeId<dir::TypeMember>],
    ) -> CompilerResult<dir::GlobalTypeId> {
        // collect the shape entries the written members declare
        let mut properties = Vec::new();
        let mut call_signatures = Vec::new();
        let mut construct_signatures = Vec::new();
        let mut index_signatures = Vec::new();

        // walk each written member into the shape
        for member in members {
            let member = *member;
            match self.tree.get(member) {
                // name: T
                dir::TypeMember::Field {
                    name,
                    declared_type,
                    is_optional,
                    is_readonly,
                    ..
                } => {
                    let (declared_type, is_optional, is_readonly) =
                        (*declared_type, *is_optional, *is_readonly);
                    let key = (*name).into();
                    let ty = match declared_type {
                        Some(declared_type) => self.walk_type_expression(declared_type)?,
                        None => self.intern_type(dir::Type::Unknown)?,
                    };

                    // type the field symbol
                    if let Some(symbol) = self.declared_symbol(member.into_any()) {
                        self.commit_symbol_type(symbol, ty)?;
                    }

                    let access = if is_readonly {
                        dir::PropertyAccess::Read(ty)
                    } else {
                        dir::PropertyAccess::ReadWrite {
                            read: ty,
                            write: ty,
                        }
                    };

                    push_shape_property(
                        &mut properties,
                        dir::TypeProperty {
                            key,
                            access,
                            is_optional,
                        },
                    );
                }
                // method(): T
                dir::TypeMember::Method {
                    name,
                    signature,
                    is_optional,
                    ..
                } => {
                    let (signature, is_optional) = (signature.clone(), *is_optional);
                    let key = (*name).into();
                    let template = self
                        .open_signature_template(member.into_global_any(self.module), &signature)?;

                    let (header, result, tracked) = self.walk_signature_header(
                        member.into_any(),
                        template,
                        &signature,
                        None,
                        false,
                    )?;
                    let ty = self.walk_function_signature_type(
                        member.into_any(),
                        &signature,
                        header,
                        None,
                        None,
                        result,
                        tracked,
                    )?;

                    let role = MemberRole::from(signature.role);
                    let ty = self.check.shallow_resolve(ty)?;
                    let access = self
                        .check
                        .property_access(role, ty, false)?
                        .ok_or_else(|| CompilerError::Internal {
                            message: format!("structural property {member:?} has no access"),
                        })?;
                    push_shape_property(
                        &mut properties,
                        dir::TypeProperty {
                            key,
                            access,
                            is_optional,
                        },
                    );
                }
                // (value: T): U
                dir::TypeMember::CallSignature { signature } => {
                    let signature = signature.clone();
                    let ty = self.walk_function_type(member.into_any(), &signature, None)?;
                    call_signatures.push(ty);
                }
                // new (value: T): U
                dir::TypeMember::ConstructSignature { signature } => {
                    let signature = signature.clone();
                    let ty = self.walk_constructor_type(member.into_any(), &signature, None)?;
                    construct_signatures.push(ty);
                }
                // [key: K]: V
                dir::TypeMember::IndexSignature {
                    name,
                    key_type,
                    value_type,
                    is_optional,
                    is_readonly,
                } => {
                    let (name, is_optional, is_readonly) = (*name, *is_optional, *is_readonly);
                    let key_type = self.walk_type_expression(*key_type)?;
                    let value_type = self.walk_type_expression(*value_type)?;

                    index_signatures.push(dir::TypeIndexSignature {
                        name,
                        key_type,
                        value_type,
                        is_optional,
                        is_readonly,
                    });
                }
                // skip associated members in structural shapes
                _ => {}
            }
        }

        // name the signature the named properties sit beside
        let conflict = match (
            properties.is_empty(),
            index_signatures.is_empty(),
            call_signatures.is_empty(),
            construct_signatures.is_empty(),
        ) {
            (false, false, _, _) => Some(MixedObjectSignature::IndexSignature),
            (false, _, false, _) => Some(MixedObjectSignature::CallSignature),
            (false, _, _, false) => Some(MixedObjectSignature::ConstructSignature),
            _ => None,
        };

        // reject the mixed object type the written members declare
        if let Some(conflict) = conflict {
            self.check
                .report_mixed_object_type(self.module, id.into_any(), conflict);
        }

        // an inline callable object type is the signature it declares
        match (
            properties.as_slice(),
            call_signatures.as_slice(),
            construct_signatures.as_slice(),
            index_signatures.as_slice(),
        ) {
            ([], [call], [], []) => Ok(*call),
            ([], [], [construct], []) => Ok(*construct),

            // declare one anonymous class for every other written member set
            _ => {
                let properties = self.intern_properties(&properties)?;
                let call_signatures = self.intern_type_ids(&call_signatures)?;
                let construct_signatures = self.intern_type_ids(&construct_signatures)?;
                let index_signatures = self.intern_index_signatures(&index_signatures)?;

                self.intern_type(dir::Type::Object(dir::ObjectType {
                    properties,
                    call_signatures,
                    construct_signatures,
                    index_signatures,
                }))
            }
        }
    }

    /// Return one tuple element type.
    fn walk_tuple_element(
        &mut self,
        id: dir::LocalNodeId<dir::TupleElement>,
    ) -> CompilerResult<dir::TypeElement> {
        // walk by the element kind
        match self.tree.get(id) {
            // label: T
            dir::TupleElement::Element {
                label,
                value,
                is_optional,
                is_readonly,
            } => {
                let (label, is_optional, is_readonly) = (*label, *is_optional, *is_readonly);
                let ty = self.walk_type_expression(*value)?;

                Ok(dir::TypeElement {
                    label,
                    ty,
                    is_optional,
                    is_readonly,
                    is_rest: false,
                })
            }
            // damaged element
            dir::TupleElement::Error => {
                let ty = self.intern_type(dir::Type::Error)?;

                Ok(dir::TypeElement {
                    label: None,
                    ty,
                    is_optional: false,
                    is_readonly: false,
                    is_rest: false,
                })
            }
            // ...T
            dir::TupleElement::Spread { label, value } => {
                let label = *label;
                let ty = self.walk_rest_type_expression(*value)?;

                Ok(dir::TypeElement {
                    label,
                    ty,
                    is_optional: false,
                    is_readonly: false,
                    is_rest: true,
                })
            }
        }
    }

    /// Return one borrowed type expression.
    fn walk_borrowed_type(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        lifetime: Option<dir::LocalNodeId<dir::TypeExpression>>,
        access: Option<dir::Access>,
        target_type: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // walk the borrowed value and the written access
        let source = id.into_any();
        let value = self.walk_type_expression(target_type)?;
        let access = self.access_literal(access.unwrap_or(dir::Access::BARE))?;

        // take an annotated region term whole, elide the region otherwise
        let region = match lifetime {
            Some(lifetime) => {
                let region = self.walk_type_expression(lifetime)?;

                self.borrow_region(source, region, value)?
            }
            None => self.elided_borrow_region(source, Some(value))?,
        };

        self.check.borrow_value(region, access, value)
    }

    /// Return one range type expression from its literal bounds.
    fn walk_range_type(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        start: Option<dir::LocalNodeId<dir::TypeExpression>>,
        end: Option<dir::LocalNodeId<dir::TypeExpression>>,
        end_kind: dir::RangeEnd,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // require both interval bounds
        let (Some(start), Some(end)) = (start, end) else {
            self.check
                .report_unbounded_interval_type(self.module, id.into_any());

            return self.intern_type(dir::Type::Error);
        };

        // bounds are integer, bigint, or char literals of one discrete domain
        let start = self.range_literal_bound(start)?;
        let end = self.range_literal_bound(end)?;
        let domains = (
            start.and_then(Self::interval_bound_domain),
            end.and_then(Self::interval_bound_domain),
        );
        let ((Some(start_domain), Some(end_domain)), Some(start), Some(end)) =
            (domains, start, end)
        else {
            self.check
                .report_invalid_interval_domain(self.module, id.into_any());

            return self.intern_type(dir::Type::Error);
        };

        // require both bounds in one domain
        if start_domain != end_domain {
            self.check
                .report_invalid_interval_domain(self.module, id.into_any());

            return self.intern_type(dir::Type::Error);
        }

        // intern the written interval
        self.intern_type(dir::Type::Range(dir::RangeType {
            start: Some(start),
            end: Some(end),
            is_inclusive: end_kind == dir::RangeEnd::Inclusive,
        }))
    }

    /// Return the discrete scalar domain of one interval bound literal.
    fn interval_bound_domain(literal: dir::Literal) -> Option<dir::ScalarDomain> {
        match literal {
            dir::Literal::Integer(_) => Some(dir::ScalarDomain::Integer),
            dir::Literal::Bigint(_) => Some(dir::ScalarDomain::Bigint),
            dir::Literal::Character(_) => Some(dir::ScalarDomain::Character),
            _ => None,
        }
    }

    /// Read one range bound's scalar literal value.
    fn range_literal_bound(
        &mut self,
        bound: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<Option<dir::Literal>> {
        match self.tree.get(bound) {
            dir::TypeExpression::Literal { value } => Ok(Some(*value)),
            _ => Ok(None),
        }
    }

    /// Return one mapped type expression.
    fn walk_mapped_type(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        parameter: dir::LocalNodeId<dir::TypeMappedParameter>,
        readonly: dir::MappedTypeModifier,
        optional: dir::MappedTypeModifier,
        value: Option<dir::LocalNodeId<dir::TypeExpression>>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // read the written binder and its source constraint
        let mapped = self.tree.get(parameter);
        let (name, source_type, key_remap) = (mapped.name, mapped.source_type, mapped.key_remap);
        let Some(symbol) = self.declared_symbol(parameter.into_any()) else {
            return self.intern_type(dir::Type::Error);
        };

        // walk the key source the mapped type reads
        let constraint = self.walk_type_expression(source_type)?;

        // create a local generic parameter for the mapped key
        let binder = match self.check.parameter_by_symbol(symbol)? {
            Some(binder) => binder,
            None => {
                // the mapped type introduces the scope its key parameter lives in
                let template = self.check.open_generic_template(
                    id.into_global_any(self.module),
                    self.flow().template_scope(),
                )?;
                self.check.push_generic_parameter(
                    template,
                    parameter.into_global_any(self.module),
                    Some(symbol),
                    dir::GenericParameterKey::Symbol(symbol),
                    None,
                    Some(constraint),
                    None,
                    dir::GenericParameterOrigin::Explicit,
                    dir::GenericParameterKind::Type,
                    false,
                    false,
                )?
            }
        };

        // bind the key symbol to its parameter type
        let ty = self.check.generic_parameter_type(binder)?;
        self.commit_symbol_type(symbol, ty)?;

        // walk the remap and value under that binder
        let key_remap = match key_remap {
            Some(key_remap) => Some(self.walk_type_expression(key_remap)?),
            None => None,
        };
        let value = match value {
            Some(value) => self.walk_type_expression(value)?,
            None => self.intern_type(dir::Type::Unknown)?,
        };

        // find the source behind a keyof constraint, directly or through the key parameter's bound
        let modifiers_type = match self.check.ty(constraint)? {
            dir::Type::Operation(operation)
                if let dir::TypeOperation::KeyOf(unary) =
                    self.check.type_operation(constraint.module_id, operation)? =>
            {
                Some(unary.target)
            }
            dir::Type::Parameter(parameter) => self
                .check
                .generic_parameter(parameter)?
                .and_then(|binding| binding.constraint)
                .and_then(|bound| match self.check.operation_head(bound) {
                    Ok(Some(dir::TypeOperation::KeyOf(unary))) => Some(unary.target),
                    _ => None,
                }),
            _ => None,
        };

        // intern the mapped operation
        self.intern_operation(dir::TypeOperation::Mapped(dir::MappedType {
            parameter: dir::MappedTypeParameter {
                name,
                parameter: binder,
                constraint,
                key_remap,
                modifiers_type,
            },
            modifiers: dir::MappedTypeModifiers { readonly, optional },
            value,
        }))
    }
}

/// Push one shape property, merging accessor pairs sharing a key.
fn push_shape_property(properties: &mut Vec<dir::TypeProperty>, property: dir::TypeProperty) {
    if let Some(existing) = properties
        .iter_mut()
        .find(|existing| existing.key == property.key)
    {
        existing.access = existing.access.merged(property.access);
        existing.is_optional &= property.is_optional;

        return;
    }

    properties.push(property);
}
