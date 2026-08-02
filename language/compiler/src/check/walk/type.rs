use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{
    Decision, GenericArgument, Origin, Receiver, TypeSubstitution, VariableRole, WalkState,
    Widening,
};
use crate::{CompilerError, CompilerResult};

impl WalkState<'_, '_> {
    /// Walk one type annotation and record its type.
    ///
    /// Resolve reference annotations to symbols, allocate memory forms, and
    /// allocate type operations.
    ///
    /// Example:
    /// ```ds
    /// readonly Box<T>
    /// ```
    pub(in crate::check) fn walk_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let ty = self.walk_type_expression_type(id)?;
        self.commit_node_type(id, ty)?;

        Ok(ty)
    }

    /// Return the type denoted by one type expression.
    fn walk_type_expression_type(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let source = id.into_any();

        match self.tree.get(id) {
            // "ok", 42, true
            dir::TypeExpression::ScalarLiteral { value } => {
                self.intern_type(dir::Type::Literal(*value))
            }
            // never, any, null, number, ...
            dir::TypeExpression::Literal { value } => {
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
                self.intern_type(dir::Type::Array(dir::ArrayType { element }))
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

                // the reserved tick names spell the lifetime literals
                let literal = match self.check.strings().get(name) {
                    "'static" => Some(dir::Lifetime::Static),
                    "'frame" => Some(dir::Lifetime::Frame),
                    _ => None,
                };
                match literal {
                    Some(literal) => {
                        self.intern_type(dir::Type::Memory(dir::MemoryLiteral::Lifetime(literal)))
                    }
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

                // this projections bind to their declaring scope
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
            // this
            dir::TypeExpression::This => self.intern_type(dir::Type::This),
            // readonly T
            dir::TypeExpression::Readonly { target_type } => {
                let value = self.walk_type_expression(*target_type)?;

                self.intern_type(dir::Type::Form(dir::FormType {
                    form: dir::Form::Readonly,
                    value,
                }))
            }
            // local T, shared T
            dir::TypeExpression::Local { target_type } => {
                self.walk_placed_type(id, *target_type, dir::Space::Local)
            }
            dir::TypeExpression::Shared { target_type } => {
                self.walk_placed_type(id, *target_type, dir::Space::Shared)
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

                self.intern_type(dir::Type::Form(dir::FormType {
                    form: dir::Form::Owned,
                    value,
                }))
            }
            // walk the written borrow lifetime, or open the elided hole
            dir::TypeExpression::BorrowedOf {
                lifetime,
                mutability,
                target_type,
                ..
            } => {
                let value = self.walk_type_expression(*target_type)?;
                let access = mutability
                    .map(dir::Mutability::access)
                    .unwrap_or(dir::Access::Mutable);
                let access =
                    self.intern_type(dir::Type::Memory(dir::MemoryLiteral::Access(access)))?;
                let lifetime = match lifetime {
                    Some(lifetime) => self.walk_type_expression(*lifetime)?,
                    None => self.elided_borrow_lifetime(source)?,
                };

                let form = self.intern_borrow(lifetime, access)?;

                self.intern_type(dir::Type::Form(dir::FormType { form, value }))
            }
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

                self.intern_type(dir::Type::Intersection(dir::IntersectionType { elements }))
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
                let right = self.walk_type_expression(extends_type)?;
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
                    self.intern_type(dir::Type::Literal(dir::ScalarLiteral::Boolean(true)))?;
                let else_type =
                    self.intern_type(dir::Type::Literal(dir::ScalarLiteral::Boolean(false)))?;

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
            } => self.walk_infer_type_expression(id, *form, *name, *constraint),
            // produce the error type for damaged children
            dir::TypeExpression::Missing | dir::TypeExpression::Error => {
                self.intern_type(dir::Type::Error)
            }
        }
    }

    /// Walk one `_` or `infer T` type expression.
    fn walk_infer_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        form: dir::InferForm,
        name: Option<dir::StringId>,
        constraint: Option<dir::LocalNodeId<dir::TypeExpression>>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let source = id.into_any();

        match form {
            // open anonymous holes for ordinary inference
            dir::InferForm::Hole => {
                self.open_type_hole(source, Widening::Always, VariableRole::Regular)
            }

            // preserve named infer bindings for conditional matching
            dir::InferForm::Infer => {
                let constraint = match constraint {
                    Some(constraint) => Some(self.walk_type_expression(constraint)?),
                    None => None,
                };
                let symbol = match name {
                    Some(_) => Some(
                        self.check
                            .module(self.module)
                            .declaration_symbol(source)
                            .ok_or_else(|| CompilerError::Internal {
                                message: format!(
                                    "named infer binding {id:?} has no declaration symbol"
                                ),
                            })?,
                    ),
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

    /// Return a deferred type query over one stable value reference.
    fn walk_typeof_type(
        &mut self,
        value: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // reject expression forms that cannot name a type-query target
        if self.tree.reference_path(value).is_none() {
            self.check
                .report_invalid_type_query(self.module, value.into_any());

            return self.intern_type(dir::Type::Error);
        }

        // record lexical decisions without performing a runtime read
        self.walk_type_query_value(value)?;

        self.intern_operation(dir::TypeOperation::TypeOf(dir::TypeOfType {
            value: value.into_global_any(self.module),
        }))
    }

    /// Walk the reference path named by one type query operand.
    fn walk_type_query_value(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        match self.tree.get(id) {
            // record the name binding at the path root
            dir::Expression::Identifier { name } => {
                let mut segments = SmallVec::new();
                segments.push(*name);
                let path = dir::Path { segments };

                self.walk_type_query_reference(id, path).map(|_| ())
            }

            // record a fully bound member path or keep walking its owner
            dir::Expression::Member {
                left,
                name: Some(_),
                ..
            } => {
                if self.walk_type_query_reference_path(id)? {
                    Ok(())
                } else {
                    self.walk_type_query_value(*left)
                }
            }

            // reject dynamic member segments before reduction
            dir::Expression::Member { name: None, .. } => {
                self.check
                    .report_invalid_type_query(self.module, id.into_any());

                Ok(())
            }

            // reject any expression that escaped the static-path guard
            _ => Err(CompilerError::Internal {
                message: format!(
                    "type query operand {} is not a static reference path",
                    self.check.node_label(id.into_global_any(self.module))
                ),
            }),
        }
    }

    /// Return true when one type-query path resolved to a complete value declaration.
    fn walk_type_query_reference_path(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<bool> {
        let Some(path) = self.tree.reference_path(id) else {
            self.check
                .report_invalid_type_query(self.module, id.into_any());

            return Ok(true);
        };

        self.walk_type_query_reference(id, path)
    }

    /// Record the resolver decision for one type-query path.
    fn walk_type_query_reference(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        path: dir::Path,
    ) -> CompilerResult<bool> {
        let source = id.into_global_any(self.module);
        let reference = self
            .check
            .module(self.module)
            .resolved
            .references
            .get(source)
            .cloned();

        match reference {
            // complete name paths reduce through their bound declaration
            Some(dir::Reference::Bound(symbols)) => {
                let symbols = self.check.present_symbols(&symbols);
                for symbol in symbols.iter().copied() {
                    self.capture_symbol_reference(symbol);
                }
                self.check.commit_decision(
                    source,
                    Decision::Name(dir::NameResolution::from_symbols(symbols.to_vec())),
                )?;

                Ok(true)
            }

            // member paths off a resolved declaration reduce from the owner path
            Some(dir::Reference::Projected { .. }) | None => Ok(false),

            // conflicting paths fail at the type query site
            Some(dir::Reference::Ambiguous(_)) => {
                self.check
                    .report_ambiguous_reference(self.module, id.into_any(), &path);

                Ok(true)
            }

            // missing paths fail at the type query site
            Some(dir::Reference::Missing) => {
                self.check
                    .reject_unresolved_reference(self.module, id.into_any(), &path);

                Ok(true)
            }

            // namespaces alone are not values
            Some(dir::Reference::Namespace(_)) => {
                self.check
                    .report_invalid_type_query(self.module, id.into_any());

                Ok(true)
            }
        }
    }

    /// Walk one construct target, such as the `Wrap` in `Wrap { value }`.
    pub(in crate::check) fn walk_construct_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<()> {
        self.commit_node_scope(id)?;

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
        let reference = self.resolved_type_reference(id);

        // commit the construct declaration name
        match reference {
            Some(dir::Reference::Bound(symbols)) => {
                let symbols = self.check.present_symbols(&symbols);
                match symbols.as_slice() {
                    [] => {
                        return Err(CompilerError::Internal {
                            message: format!(
                                "construct reference {source:?} resolved to no symbols"
                            ),
                        });
                    }
                    [symbol] => {
                        self.commit_reference_name(source, *symbol)?;
                    }
                    _ => {
                        self.check
                            .report_ambiguous_reference(self.module, id.into_any(), path);
                    }
                }
            }
            Some(dir::Reference::Projected { base, .. }) => {
                self.commit_reference_name(source, base)?;
            }
            Some(dir::Reference::Ambiguous(_)) => {
                self.check
                    .report_ambiguous_reference(self.module, id.into_any(), path);
            }
            Some(dir::Reference::Namespace(_)) | Some(dir::Reference::Missing) => {
                self.check
                    .reject_unresolved_reference(self.module, id.into_any(), path);
            }
            None => {
                return Err(CompilerError::Internal {
                    message: format!("construct reference {source:?} has no resolved name"),
                });
            }
        }

        // walk written arguments so selection can bind them later
        self.walk_generic_arguments(generic_arguments)?;

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

        match reference {
            // use one resolved type declaration directly
            Some(dir::Reference::Bound(symbols)) => {
                self.walk_bound_reference_type(id, path, generic_arguments, &symbols)
            }

            // project a type-member path from the resolved base declaration
            Some(dir::Reference::Projected { base, from }) => self.walk_member_path_type(
                id,
                base,
                &path.segments[from as usize..],
                generic_arguments,
            ),

            // reject resolve conflicts in type position
            Some(dir::Reference::Ambiguous(_)) => {
                self.check
                    .report_ambiguous_reference(self.module, id.into_any(), path);

                self.intern_type(dir::Type::Error)
            }

            // reject namespaces and missing references in type position
            Some(dir::Reference::Namespace(_)) | Some(dir::Reference::Missing) => {
                self.check
                    .reject_unresolved_reference(self.module, id.into_any(), path);

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

    /// Return the resolver output for one type reference.
    fn resolved_type_reference(
        &self,
        id: dir::LocalNodeId<dir::TypeExpression>,
    ) -> Option<dir::Reference> {
        let source = id.into_global_any(self.module);

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
        self.capture_symbol_reference(symbol);
        self.check
            .commit_decision(source, Decision::Name(dir::NameResolution::new(symbol)))
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
                    .report_ambiguous_reference(self.module, id.into_any(), path);

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

        // record the resolved name for snapshots and downstream selection
        self.commit_reference_name(source, symbol)?;

        // apply written type arguments and open omitted slots
        let applied = self.walk_generic_arguments(generic_arguments)?;
        let ty = self.referenced_symbol_type(id.into_any(), symbol, &applied)?;

        Ok(ty)
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

        // extensions have no application, so they project by reference
        if matches!(
            self.check.symbol_kind(declaration)?,
            dir::SymbolKind::Extension
        ) {
            let reference = dir::Type::Reference(dir::TypeReference {
                symbol: declaration,
            });

            return Ok(Some(self.intern_type(reference)?));
        }

        Ok(Some(receiver.ty))
    }

    /// Return the type for one declaration symbol application.
    fn referenced_symbol_type(
        &mut self,
        source: dir::LocalNodeIdAny,
        symbol: dir::GlobalSymbolId,
        applied: &[GenericArgument],
    ) -> CompilerResult<dir::GlobalTypeId> {
        // return the parameter type for generic parameter names
        if let Some(parameter) = self.check.generics.parameter_by_symbol(symbol) {
            if !applied.is_empty() {
                let name = self.check.format_symbol(symbol);
                self.check
                    .report_wrong_generic_arity(self.module, source, name, 0, applied.len());

                return self.intern_type(dir::Type::Error);
            }

            return self.intern_type(dir::Type::Parameter(parameter));
        }

        // keep foreign references symbolic while declaring
        if self.check.is_declaration() && !self.check.is_own_module(symbol.module_id) {
            let positional = applied
                .iter()
                .filter(|argument| argument.name.is_none())
                .map(|argument| argument.ty)
                .collect::<Vec<_>>();
            let arguments = self.intern_type_ids(&positional)?;
            let ty = self.intern_type(dir::Type::Application(dir::GenericApplication {
                symbol,
                arguments,
            }))?;

            return self.apply_named_refinements(ty, applied);
        }

        // load foreign declarations before reading their templates
        if !self.check.is_own_module(symbol.module_id) {
            self.check.import_external_module(symbol.module_id)?;
        }

        // reject positional arguments on non-generic declarations
        let Some(template) = self.check.symbol_template(symbol)? else {
            let positional = applied
                .iter()
                .filter(|argument| argument.name.is_none())
                .count();
            if positional > 0 {
                let name = self.check.format_symbol(symbol);
                self.check
                    .report_wrong_generic_arity(self.module, source, name, 0, positional);

                return self.intern_type(dir::Type::Error);
            }

            let arguments = self.intern_type_ids(&[])?;
            let ty = self.intern_type(dir::Type::Application(dir::GenericApplication {
                symbol,
                arguments,
            }))?;

            return self.apply_named_refinements(ty, applied);
        };

        // reject impossible arities before instantiating
        let parameters = self.check.generic_template_parameters(template)?;
        let written_count = self.check.writable_parameter_count(&parameters);
        let positional = applied
            .iter()
            .filter(|argument| argument.name.is_none())
            .count();
        if positional > written_count {
            let name = self.check.format_symbol(symbol);
            self.check.report_wrong_generic_arity(
                self.module,
                source,
                name,
                written_count,
                positional,
            );

            return self.intern_type(dir::Type::Error);
        }

        // bind every parameter in declaration order
        let written = applied
            .iter()
            .filter(|argument| argument.name.is_none())
            .collect::<SmallVec<[_; 4]>>();
        let origin = Origin::Node(
            source.into_global(self.module),
            self.flow().template_scope(),
        );
        let mut substitution = TypeSubstitution::default();
        let mut cursor = 0;
        for parameter in parameters.iter().copied() {
            let Some(binding) = self.check.generic_parameter(parameter).copied() else {
                return Err(CompilerError::Internal {
                    message: "written type application names a missing generic parameter"
                        .to_string(),
                });
            };

            // slot only written lifetimes into lifetime parameters,
            //  other arguments skip the whole lifetime group
            let wants_lifetime = binding.memory_parameter() == Some(dir::MemoryParameter::Lifetime);
            let next_is_lifetime = cursor < written.len()
                && self
                    .check
                    .written_argument_is_lifetime(written[cursor].ty)?;

            // bind the next written argument to the next writable slot
            let argument = if binding.is_writable()
                && cursor < written.len()
                && (!wants_lifetime || next_is_lifetime)
            {
                let argument = written[cursor].ty;
                cursor += 1;

                argument
            }
            // evaluate defaults against the application built so far
            else if let Some(default) = binding.default {
                self.check.substitute_type(default, &substitution)?
            }
            // elide omitted memory parameters like unwritten borrow lifetimes
            else if let Some(kind) = binding.memory_parameter() {
                match kind {
                    dir::MemoryParameter::Lifetime => self.elided_borrow_lifetime(source)?,
                    kind => self.open_memory_hole(source, kind)?,
                }
            }
            // reject unbound parameters
            else {
                let name = self.check.format_symbol(symbol);
                self.check.report_wrong_generic_arity(
                    self.module,
                    source,
                    name,
                    written_count,
                    applied.len(),
                );

                return self.intern_type(dir::Type::Error);
            };

            // store memory arguments canonically
            let argument = match binding.memory_parameter() {
                Some(kind) => self
                    .check
                    .normalize_memory_component(origin, argument, kind)?,
                None => argument,
            };
            substitution
                .bindings
                .push(dir::GenericArgumentBinding::new(parameter, argument));
        }
        let arguments = substitution.arguments().collect::<Vec<_>>();

        // build the application before attaching its argument checks
        let argument_list = self.intern_type_ids(&arguments)?;
        let ty = self.intern_type(dir::Type::Application(dir::GenericApplication {
            symbol,
            arguments: argument_list,
        }))?;
        let ty = self.apply_named_refinements(ty, applied)?;

        Ok(ty)
    }

    /// Wrap one application with its named refinements in canonical key order.
    fn apply_named_refinements(
        &mut self,
        ty: dir::GlobalTypeId,
        applied: &[GenericArgument],
    ) -> CompilerResult<dir::GlobalTypeId> {
        let mut refinements = applied
            .iter()
            .filter_map(|argument| argument.name.map(|name| (name, argument.ty)))
            .collect::<SmallVec<[_; 2]>>();
        refinements.sort_by_key(|(name, _)| *name);

        let mut ty = ty;
        for (name, value) in refinements {
            ty = self.intern_refined(dir::RefinedType {
                base: ty,
                key: dir::StaticKey::Name(name),
                value,
            })?;
        }

        Ok(ty)
    }

    /// Return a type-member path from one resolved base symbol.
    fn walk_member_path_type(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        base: dir::GlobalSymbolId,
        tail: &[dir::StringId],
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.commit_reference_name(id.into_global_any(self.module), base)?;

        // start from the resolved base symbol
        let mut ty = self.referenced_symbol_type(id.into_any(), base, &[])?;

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

            ty = self.intern_member(dir::MemberType {
                owner: ty,
                key: dir::StaticKey::Name(segment),
                arguments,
                qualifier: None,
            })?;
        }

        Ok(ty)
    }

    /// Return a structural shape from one object type expression.
    fn walk_object_type(
        &mut self,
        _id: dir::LocalNodeId<dir::TypeExpression>,
        members: &[dir::LocalNodeId<dir::TypeMember>],
    ) -> CompilerResult<dir::GlobalTypeId> {
        let mut properties = Vec::new();
        let mut call_signatures = Vec::new();
        let mut construct_signatures = Vec::new();
        let mut index_signatures = Vec::new();

        for member in members {
            let member = *member;
            match self.tree.get(member) {
                dir::TypeMember::Field {
                    key,
                    declared_type,
                    is_optional,
                    is_readonly,
                    ..
                } => {
                    let (declared_type, is_optional, is_readonly) =
                        (*declared_type, *is_optional, *is_readonly);
                    let Some(key) = self.static_key(*key)? else {
                        continue;
                    };
                    let ty = match declared_type {
                        Some(declared_type) => self.walk_type_expression(declared_type)?,
                        None => self.intern_type(dir::Type::Unknown)?,
                    };

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
                dir::TypeMember::Method {
                    key,
                    signature,
                    is_optional,
                    ..
                } => {
                    let (signature, is_optional) = (signature.clone(), *is_optional);
                    let Some(key) = self.static_key(*key)? else {
                        continue;
                    };
                    let template = self
                        .open_signature_template(member.into_global_any(self.module), &signature)?;

                    let (header, result, tracked) =
                        self.walk_signature_header(member.into_any(), template, &signature, None)?;
                    let ty = self.walk_function_signature_type(
                        member.into_any(),
                        &signature,
                        header,
                        None,
                        None,
                        result,
                        tracked,
                    )?;

                    let access = match signature.role {
                        Some(dir::FunctionRole::Getter) => {
                            dir::PropertyAccess::Read(self.type_member_getter_type(ty)?)
                        }
                        Some(dir::FunctionRole::Setter) => {
                            dir::PropertyAccess::Write(self.type_member_setter_type(ty)?)
                        }
                        _ => dir::PropertyAccess::ReadWrite {
                            read: ty,
                            write: ty,
                        },
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
                dir::TypeMember::CallSignature { signature } => {
                    let signature = signature.clone();
                    let ty = self.walk_function_type(member.into_any(), &signature, None)?;
                    call_signatures.push(ty);
                }
                dir::TypeMember::ConstructSignature { signature } => {
                    let signature = signature.clone();
                    let ty = self.walk_constructor_type(member.into_any(), &signature, None)?;
                    construct_signatures.push(ty);
                }
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

        let properties = self.intern_properties(&properties)?;
        let call_signatures = self.intern_type_ids(&call_signatures)?;
        let construct_signatures = self.intern_type_ids(&construct_signatures)?;
        let index_signatures = self.intern_index_signatures(&index_signatures)?;

        self.intern_type(dir::Type::from(dir::ShapeType {
            properties,
            call_signatures,
            construct_signatures,
            index_signatures,
        }))
    }

    /// Return one tuple element type.
    fn walk_tuple_element(
        &mut self,
        id: dir::LocalNodeId<dir::TupleElement>,
    ) -> CompilerResult<dir::TypeElement> {
        match self.tree.get(id) {
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
            dir::TupleElement::Spread { label, value } => {
                let label = *label;
                let ty = self.walk_type_expression(*value)?;

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

    /// Return one placed type expression.
    fn walk_placed_type(
        &mut self,
        _id: dir::LocalNodeId<dir::TypeExpression>,
        target_type: dir::LocalNodeId<dir::TypeExpression>,
        space: dir::Space,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let value = self.walk_type_expression(target_type)?;
        let place = self.intern_type(dir::Type::Memory(dir::MemoryLiteral::Place(
            dir::Place::Space(space),
        )))?;

        let ty = self.intern_type(dir::Type::Form(dir::FormType {
            form: dir::Form::Placed { place },
            value,
        }))?;
        Ok(ty)
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
        if start_domain != end_domain {
            self.check
                .report_invalid_interval_domain(self.module, id.into_any());

            return self.intern_type(dir::Type::Error);
        }

        self.intern_type(dir::Type::Range(dir::RangeType {
            start: Some(start),
            end: Some(end),
            is_inclusive: end_kind == dir::RangeEnd::Inclusive,
        }))
    }

    /// Return the discrete scalar domain of one interval bound literal.
    fn interval_bound_domain(literal: dir::ScalarLiteral) -> Option<dir::ScalarDomain> {
        match literal {
            dir::ScalarLiteral::Integer(_) => Some(dir::ScalarDomain::Integer),
            dir::ScalarLiteral::Bigint(_) => Some(dir::ScalarDomain::Bigint),
            dir::ScalarLiteral::Character(_) => Some(dir::ScalarDomain::Character),
            _ => None,
        }
    }

    /// Read one range bound's scalar literal value.
    fn range_literal_bound(
        &mut self,
        bound: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<Option<dir::ScalarLiteral>> {
        match self.tree.get(bound) {
            dir::TypeExpression::ScalarLiteral { value } => Ok(Some(*value)),
            _ => Ok(None),
        }
    }

    /// Return one mapped type expression.
    fn walk_mapped_type(
        &mut self,
        _id: dir::LocalNodeId<dir::TypeExpression>,
        parameter: dir::LocalNodeId<dir::TypeMappedParameter>,
        readonly: dir::MappedTypeModifier,
        optional: dir::MappedTypeModifier,
        value: Option<dir::LocalNodeId<dir::TypeExpression>>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let mapped = self.tree.get(parameter);
        let (name, source_type, key_remap) = (mapped.name, mapped.source_type, mapped.key_remap);

        let Some(symbol) = self
            .check
            .module(self.module)
            .declaration_symbol(parameter.into_any())
        else {
            return self.intern_type(dir::Type::Error);
        };
        let constraint = self.walk_type_expression(source_type)?;

        // create a local generic parameter for the mapped key
        let binder = match self.check.generics.parameter_by_symbol(symbol) {
            Some(binder) => binder,
            None => {
                let template = self
                    .check
                    .open_generic_template(parameter.into_global_any(self.module))?;
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
        let ty = self.check.generic_parameter_type(binder)?;
        self.bind_symbol_type(symbol, ty)?;

        let key_remap = match key_remap {
            Some(key_remap) => Some(self.walk_type_expression(key_remap)?),
            None => None,
        };
        let value = match value {
            Some(value) => self.walk_type_expression(value)?,
            None => self.intern_type(dir::Type::Unknown)?,
        };

        // find the source behind a keyof constraint, written directly
        //  or reached through the key parameter's bound
        let modifiers_type = match self.check.ty(constraint)? {
            dir::Type::Operation(operation)
                if let dir::TypeOperation::KeyOf(unary) =
                    self.check.type_operation(constraint.module_id, operation)? =>
            {
                Some(unary.target)
            }
            dir::Type::Parameter(parameter) => self
                .check
                .generic_parameter(parameter)
                .and_then(|binding| binding.constraint)
                .and_then(|bound| match self.check.operation_head(bound) {
                    Ok(Some(dir::TypeOperation::KeyOf(unary))) => Some(unary.target),
                    _ => None,
                }),
            _ => None,
        };

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
    /// Return the result type of one structural getter.
    fn type_member_getter_type(&self, ty: dir::GlobalTypeId) -> CompilerResult<dir::GlobalTypeId> {
        let dir::Type::FunctionSignature(function) = self.check.ty(ty)? else {
            return Err(CompilerError::Internal {
                message: format!("structural getter has non-signature type {ty:?}"),
            });
        };
        let function = self.check.type_signature(ty.module_id, function)?;
        let Some(result) = function.return_type else {
            return Err(CompilerError::Internal {
                message: "structural getter has no result type".into(),
            });
        };

        Ok(result)
    }

    /// Return the parameter type of one structural setter.
    fn type_member_setter_type(&self, ty: dir::GlobalTypeId) -> CompilerResult<dir::GlobalTypeId> {
        let dir::Type::FunctionSignature(function) = self.check.ty(ty)? else {
            return Err(CompilerError::Internal {
                message: format!("structural setter has non-signature type {ty:?}"),
            });
        };
        let function = self.check.type_signature(ty.module_id, function)?;
        let parameters = self
            .check
            .signature_parameters(ty.module_id, function.parameters)?;
        let [parameter] = parameters else {
            return Err(CompilerError::Internal {
                message: format!(
                    "structural setter has {} value parameters",
                    parameters.len()
                ),
            });
        };

        Ok(parameter.ty)
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
