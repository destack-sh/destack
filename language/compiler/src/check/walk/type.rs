use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{
    Decision, GenericArgument, GenericPosition, Origin, Receiver, Relation, TypeSubstitution,
    WalkState, Widening,
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
            // (T)
            dir::TypeExpression::Parenthesized { expression } => {
                self.walk_type_expression(*expression)
            }
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
            // (A, B) and [A, B]
            dir::TypeExpression::Tuple { elements }
            | dir::TypeExpression::ArrayTuple { elements } => {
                let elements = elements.iter().copied().collect::<SmallVec<[_; 4]>>();
                let form = match self.tree.get(id) {
                    dir::TypeExpression::ArrayTuple { .. } => dir::TupleForm::Array,
                    _ => dir::TupleForm::Tuple,
                };
                let mut element_types = Vec::new();
                for element in elements {
                    element_types.push(self.walk_tuple_element(element)?);
                }
                let elements = self.intern_elements(&element_types)?;

                self.intern_type(dir::Type::Tuple(dir::TupleType { form, elements }))
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

                self.walk_function_type(source, &function, None, None)
            }
            // new (value: string) => User
            dir::TypeExpression::Constructor(constructor) => {
                let constructor = constructor.clone();

                self.walk_constructor_type(source, &constructor, None, None)
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

                self.walk_reference_type(id, &path, &generic_arguments, GenericPosition::Annotation)
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

                self.intern_type(dir::Type::Member(dir::MemberType {
                    owner,
                    key: dir::StaticKey::Name(name),
                    arguments,
                    qualifier,
                }))
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

                self.intern_type(dir::Type::Operation(dir::TypeOperation::KeyOf(
                    dir::UnaryType { target },
                )))
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

                self.intern_type(dir::Type::Operation(dir::TypeOperation::Conditional(
                    dir::ConditionalType {
                        left: target,
                        right: nullish,
                        then_type: never,
                        else_type: target,
                        is_distributive: true,
                    },
                )))
            }
            // !T
            dir::TypeExpression::Not { target_type } => {
                let target = self.walk_type_expression(*target_type)?;

                self.intern_type(dir::Type::Operation(dir::TypeOperation::StaticUnary(
                    dir::StaticUnaryType {
                        operator: dir::StaticUnaryOperator::Not,
                        target,
                    },
                )))
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
            // open the elided borrow lifetime for induction
            dir::TypeExpression::BorrowedOf {
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
                let lifetime = self.elided_borrow_lifetime(source)?;

                self.intern_type(dir::Type::Form(dir::FormType {
                    form: dir::Form::Borrowed { lifetime, access },
                    value,
                }))
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

                self.intern_type(dir::Type::Operation(dir::TypeOperation::Conditional(
                    dir::ConditionalType {
                        left,
                        right,
                        then_type,
                        else_type,
                        is_distributive,
                    },
                )))
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

                self.intern_type(dir::Type::Operation(dir::TypeOperation::Conditional(
                    dir::ConditionalType {
                        left,
                        right,
                        then_type,
                        else_type,
                        is_distributive: false,
                    },
                )))
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

                self.intern_type(dir::Type::Operation(dir::TypeOperation::Index(
                    dir::IndexType { left, index },
                )))
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

                self.intern_type(dir::Type::Operation(dir::TypeOperation::TemplateLiteral(
                    dir::TemplateLiteralType { strings, spans },
                )))
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
            dir::InferForm::Hole => self.open_type_hole(source, Widening::Widen),

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

                self.intern_type(dir::Type::Operation(dir::TypeOperation::Infer(
                    dir::InferType {
                        name,
                        symbol,
                        constraint,
                    },
                )))
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

        self.intern_type(dir::Type::Operation(dir::TypeOperation::TypeOf(
            dir::TypeOfType {
                value: value.into_global_any(self.module),
            },
        )))
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
                    .report_unresolved_reference(self.module, id.into_any(), &path);

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
    ///
    /// Construction infers omitted head arguments from its inputs, so
    /// the head application opens where an annotation would reject.
    pub(in crate::check) fn walk_construct_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // infer at reference heads, walk every other form strictly
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
            let ty = self.walk_reference_type(
                id,
                &path,
                &generic_arguments,
                GenericPosition::Inference,
            )?;
            self.commit_node_type(id, ty)?;

            return Ok(ty);
        }

        self.walk_type_expression(id)
    }

    /// Return the type for one reference annotation, such as `Foo<T>`.
    fn walk_reference_type(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        path: &dir::Path,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
        position: GenericPosition,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let source = id.into_global_any(self.module);

        // read the resolver output for this lexical reference
        let reference = self
            .check
            .module(self.module)
            .resolved
            .references
            .get(source)
            .cloned();

        match reference {
            // use one resolved type declaration directly
            Some(dir::Reference::Bound(symbols)) => {
                self.walk_bound_reference_type(id, path, generic_arguments, &symbols, position)
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
                    .report_unresolved_reference(self.module, id.into_any(), path);

                self.intern_type(dir::Type::Error)
            }

            // require resolve to write every lexical reference
            None => Err(CompilerError::Internal {
                message: format!("type reference {source:?} has no resolved name"),
            }),
        }
    }

    /// Return the type for one resolver-bound reference annotation.
    fn walk_bound_reference_type(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        path: &dir::Path,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
        symbols: &[dir::GlobalSymbolId],
        position: GenericPosition,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let source = id.into_global_any(self.module);
        let symbols = self.check.present_symbols(symbols);

        match symbols.as_slice() {
            // reject malformed resolver state
            [] => Err(CompilerError::Internal {
                message: format!("type reference {source:?} resolved to no symbols"),
            }),

            // use the single resolved type declaration
            [symbol] => self.walk_symbol_reference_type(id, *symbol, generic_arguments, position),

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
        position: GenericPosition,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let source = id.into_global_any(self.module);

        // record the resolved name for snapshots and downstream selection
        self.capture_symbol_reference(symbol);
        self.check
            .commit_decision(source, Decision::Name(dir::NameResolution::new(symbol)))?;

        // apply written type arguments and open omitted slots
        let applied = self.walk_generic_arguments(generic_arguments)?;
        let ty = self.referenced_symbol_type(id.into_any(), symbol, &applied, position)?;

        Ok(ty)
    }

    /// Return the type identity of one receiver's declaring scope.
    /// Extensions project by declaration reference, every other scope
    /// by its self application, which substitutes with the receiver.
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
            self.check.symbol_kind(declaration),
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
        position: GenericPosition,
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

        // reject written arguments on non-generic declarations
        let Some(template) = self.check.symbol_template(symbol) else {
            if !applied.is_empty() {
                let name = self.check.format_symbol(symbol);
                self.check
                    .report_wrong_generic_arity(self.module, source, name, 0, applied.len());

                return self.intern_type(dir::Type::Error);
            }

            let arguments = self.intern_type_ids(&[])?;

            return self.intern_type(dir::Type::Instance(dir::GenericInstance {
                symbol,
                arguments,
            }));
        };

        // reject impossible arities before instantiating
        let parameters = self.check.generic_template_parameters(template);
        let written_count = self.check.written_parameter_count(&parameters);
        if applied.len() > written_count {
            let name = self.check.format_symbol(symbol);
            self.check.report_wrong_generic_arity(
                self.module,
                source,
                name,
                written_count,
                applied.len(),
            );

            return self.intern_type(dir::Type::Error);
        }

        // bind written arguments and open every omitted slot
        let written = applied
            .iter()
            .map(|argument| argument.ty)
            .collect::<SmallVec<[_; 4]>>();
        let origin = Origin::Node(source.into_global(self.module));
        let Some(substitution) =
            self.check
                .instantiate_generic_parameters(origin, &parameters, &written, position)?
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
        let arguments = substitution.arguments;

        // constrain type arguments by declared parameter bounds
        self.constrain_applied_symbol_arguments(source, symbol, &parameters, &arguments)?;

        let arguments = self.intern_type_ids(&arguments)?;

        self.intern_type(dir::Type::Instance(dir::GenericInstance {
            symbol,
            arguments,
        }))
    }

    /// Constrain applied type arguments by their declared parameter bounds.
    fn constrain_applied_symbol_arguments(
        &mut self,
        source: dir::LocalNodeIdAny,
        symbol: dir::GlobalSymbolId,
        parameters: &[dir::GlobalGenericParameterId],
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<()> {
        let substitution = TypeSubstitution {
            parameters: parameters.iter().copied().collect(),
            arguments: arguments.iter().copied().collect(),
            receiver: None,
        };
        let origin = Origin::Node(source.into_global(self.module));

        // enqueue parameter bounds as ordinary type relations
        for (parameter, argument) in parameters.iter().copied().zip(arguments.iter().copied()) {
            let Some(constraint) = self
                .check
                .generic_parameter(parameter)
                .and_then(|binding| binding.constraint)
            else {
                continue;
            };
            let constraint = self
                .check
                .substitute_type(self.module, constraint, &substitution)?;

            self.relate_generic_bound(
                origin,
                source.into_global(self.module),
                argument,
                constraint,
            );
        }

        // enqueue declared where predicates as satisfaction relations
        let template = self.check.symbol_template(symbol);
        for predicate in self.check.template_predicates(template) {
            let left = self
                .check
                .substitute_type(self.module, predicate.left, &substitution)?;
            let right = self
                .check
                .substitute_type(self.module, predicate.right, &substitution)?;

            self.relate_type(origin, Relation::Satisfies, left, right);
        }

        Ok(())
    }

    /// Return a type-member path from one resolved base symbol.
    fn walk_member_path_type(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        base: dir::GlobalSymbolId,
        tail: &[dir::StringId],
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.capture_symbol_reference(base);
        self.check.commit_decision(
            id.into_global_any(self.module),
            Decision::Name(dir::NameResolution::new(base)),
        )?;

        // start from the resolved base symbol
        let mut ty =
            self.referenced_symbol_type(id.into_any(), base, &[], GenericPosition::Annotation)?;

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

            ty = self.intern_type(dir::Type::Member(dir::MemberType {
                owner: ty,
                key: dir::StaticKey::Name(segment),
                arguments,
                qualifier: None,
            }))?;
        }

        Ok(ty)
    }

    /// Return a structural shape from one object type expression.
    fn walk_object_type(
        &mut self,
        _id: dir::LocalNodeId<dir::TypeExpression>,
        members: &[dir::LocalNodeId<dir::TypeMember>],
    ) -> CompilerResult<dir::GlobalTypeId> {
        let mut fields = Vec::new();
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

                    fields.push(dir::TypeField {
                        key,
                        ty,
                        is_optional,
                        is_readonly,
                    });
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
                    let template = self.open_signature_template(
                        member.into_global_any(self.module),
                        None,
                        None,
                        &signature,
                    )?;
                    let header = self.walk_function_signature(template, &signature)?;
                    let result =
                        self.walk_function_result_type(member.into_any(), &signature, None)?;
                    let ty =
                        self.walk_function_signature_type(&signature, header, None, None, result)?;

                    fields.push(dir::TypeField {
                        key,
                        ty,
                        is_optional,
                        is_readonly: false,
                    });
                }
                dir::TypeMember::CallSignature { signature } => {
                    let signature = signature.clone();
                    let ty = self.walk_function_type(member.into_any(), &signature, None, None)?;
                    call_signatures.push(ty);
                }
                dir::TypeMember::ConstructSignature { signature } => {
                    let signature = signature.clone();
                    let ty =
                        self.walk_constructor_type(member.into_any(), &signature, None, None)?;
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

        let fields = self.intern_fields(&fields)?;
        let call_signatures = self.intern_type_ids(&call_signatures)?;
        let construct_signatures = self.intern_type_ids(&construct_signatures)?;
        let index_signatures = self.intern_index_signatures(&index_signatures)?;

        self.intern_type(dir::Type::Shape(dir::ShapeType {
            fields,
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

        self.intern_type(dir::Type::Form(dir::FormType {
            form: dir::Form::Placed { place },
            value,
        }))
    }

    /// Return one range type expression from its literal bounds.
    fn walk_range_type(
        &mut self,
        _id: dir::LocalNodeId<dir::TypeExpression>,
        start: Option<dir::LocalNodeId<dir::TypeExpression>>,
        end: Option<dir::LocalNodeId<dir::TypeExpression>>,
        end_kind: dir::RangeEnd,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let start = match start {
            Some(start) => self.range_literal_bound(start)?,
            None => None,
        };
        let end = match end {
            Some(end) => self.range_literal_bound(end)?,
            None => None,
        };

        self.intern_type(dir::Type::Range(dir::RangeType {
            start,
            end,
            is_inclusive: end_kind == dir::RangeEnd::Inclusive,
        }))
    }

    /// Read one range bound's scalar literal value.
    /// Leave symbolic bounds uninterpreted in interval types.
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
                let template = self.check.open_generic_template(
                    parameter.into_global_any(self.module),
                    None,
                    None,
                )?;
                let binding = dir::GenericParameterBinding {
                    template: template.local_id,
                    key: dir::GenericParameterKey::Symbol(symbol),
                    variance: None,
                    constraint: Some(constraint),
                    default: None,
                    origin: dir::GenericParameterOrigin::Explicit,
                    is_variadic: false,
                    is_const: false,
                    is_comptime: false,
                };

                self.check
                    .push_generic_parameter(binding, template, Some(symbol))?
            }
        };
        let ty = self.intern_type(dir::Type::Parameter(binder))?;
        self.bind_symbol_type(symbol, ty)?;

        let key_remap = match key_remap {
            Some(key_remap) => Some(self.walk_type_expression(key_remap)?),
            None => None,
        };
        let value = match value {
            Some(value) => self.walk_type_expression(value)?,
            None => self.intern_type(dir::Type::Unknown)?,
        };

        self.intern_type(dir::Type::Operation(dir::TypeOperation::Mapped(
            dir::MappedType {
                parameter: dir::MappedTypeParameter {
                    name,
                    parameter: binder,
                    constraint,
                    key_remap,
                },
                modifiers: dir::MappedTypeModifiers { readonly, optional },
                value,
            },
        )))
    }
}
