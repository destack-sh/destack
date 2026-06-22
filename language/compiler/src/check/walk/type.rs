use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{
    Decision, DynamicSafetyObligation, GenericInductionParameter, Obligation, Origin, WalkState,
    Widening,
};
use crate::{CompilerError, CompilerResult};

impl WalkState<'_, '_> {
    /// Walk one type annotation and record its type.
    ///
    /// Build a `Type` row for the annotation node.
    /// Resolve reference annotations to symbols, allocate `Form` rows for
    /// memory forms, and allocate `TypeOperation` rows for type operators.
    ///
    /// Example:
    /// ```ds
    /// readonly Box<T>
    /// ```
    pub(in crate::check) fn walk_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let node = id.into_global_any(self.module);

        // reuse the recorded type for repeated annotation visits
        if let Some(ty) = self.check.node_type(node) {
            return Ok(ty);
        }

        let ty = self.build_type_expression(id)?;
        self.check.set_node_type(node, ty)?;

        Ok(ty)
    }

    /// Build the type for one `TypeExpression` node.
    fn build_type_expression(
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
                self.push_type(dir::Type::Literal(*value), source)
            }
            // never, any, null, number, ...
            dir::TypeExpression::Literal { value } => {
                self.push_type(dir::Type::from(value.clone()), source)
            }
            // validate intrinsic markers at their declarations
            dir::TypeExpression::Intrinsic => self.push_type(dir::Type::Error, source),
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
                    element_types.push(self.build_tuple_element(element)?);
                }

                self.push_type(
                    dir::Type::Tuple(dir::TupleType {
                        form,
                        elements: element_types,
                    }),
                    source,
                )
            }
            // T[]
            dir::TypeExpression::Array { element } => {
                let element = self.walk_type_expression(*element)?;
                self.push_type(dir::Type::Array(dir::ArrayType { element }), source)
            }
            // [T]
            dir::TypeExpression::Slice { element } => {
                let element = self.walk_type_expression(*element)?;

                self.push_type(dir::Type::Slice(dir::SliceType { element }), source)
            }
            // [T; N]
            dir::TypeExpression::FixedArray { element, length } => {
                let element = self.walk_type_expression(*element)?;
                let count = self.static_expression_type(*length)?;

                self.push_type(
                    dir::Type::FixedArray(dir::FixedArrayType { element, count }),
                    source,
                )
            }
            // { name: string }
            dir::TypeExpression::Object { members } => {
                let members = members.iter().copied().collect::<SmallVec<[_; 4]>>();

                self.build_shape_type(id, &members)
            }
            // (value: T) => U
            dir::TypeExpression::Function(function) => {
                let function = function.clone();

                self.function_type(source, &function, None, None)
            }
            // new (value: string) => User
            dir::TypeExpression::Constructor(constructor) => {
                let constructor = constructor.clone();

                self.constructor_type(source, &constructor, None, None)
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

                self.build_reference_type(id, &path, &generic_arguments)
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
                let owner = self.walk_type_expression(*left)?;
                let arguments = self.walk_generic_arguments(&generic_arguments)?;
                let arguments = arguments.into_iter().map(|(_, ty)| ty).collect();

                self.push_type(
                    dir::Type::Member(dir::MemberType {
                        owner,
                        key: dir::StaticKey::Name(name),
                        arguments,
                    }),
                    source,
                )
            }
            // 0..10
            dir::TypeExpression::Range {
                start,
                end,
                end_kind,
            } => self.build_range_type(id, *start, *end, *end_kind),
            // const outside `as const` positions
            dir::TypeExpression::Const => {
                self.check.report_invalid_const_type(self.module, source);

                self.push_type(dir::Type::Error, source)
            }
            // this
            dir::TypeExpression::This => self.push_type(dir::Type::This, source),
            // readonly T
            dir::TypeExpression::Readonly { target_type } => {
                let value = self.walk_type_expression(*target_type)?;

                self.push_type(
                    dir::Type::Form(dir::FormType {
                        form: dir::Form::Readonly,
                        value,
                    }),
                    source,
                )
            }
            // local T, shared T
            dir::TypeExpression::Local { target_type } => {
                self.build_placed_type(id, *target_type, dir::Space::Local)
            }
            dir::TypeExpression::Shared { target_type } => {
                self.build_placed_type(id, *target_type, dir::Space::Shared)
            }
            // keyof T
            dir::TypeExpression::KeyOf { target_type } => {
                let target = self.walk_type_expression(*target_type)?;

                self.push_type(
                    dir::Type::Operation(dir::TypeOperation::KeyOf(dir::UnaryType { target })),
                    source,
                )
            }
            // typeof value
            dir::TypeExpression::TypeOf { value } => {
                // check the queried expression in declaration context
                let value = *value;
                let before_value = self.fork_flow();
                self.walk_expression(value, self.tree.get(value))?;
                self.restore_flow(before_value);

                self.node_type(value)
            }
            // T! strips nullish members distributively
            dir::TypeExpression::Must { target_type } => {
                let target = self.walk_type_expression(*target_type)?;
                let null = self.push_type(dir::Type::Null, source)?;
                let undefined = self.push_type(dir::Type::Undefined, source)?;
                let nullish = self.push_type(
                    dir::Type::Union(dir::UnionType {
                        elements: vec![null, undefined],
                    }),
                    source,
                )?;
                let never = self.push_type(dir::Type::Never, source)?;

                self.push_type(
                    dir::Type::Operation(dir::TypeOperation::Conditional(dir::ConditionalType {
                        left: target,
                        right: nullish,
                        then_type: never,
                        else_type: target,
                        is_distributive: true,
                    })),
                    source,
                )
            }
            // !T
            dir::TypeExpression::Not { target_type } => {
                let target = self.walk_type_expression(*target_type)?;

                self.push_type(
                    dir::Type::Operation(dir::TypeOperation::StaticUnary(dir::StaticUnaryType {
                        operator: dir::StaticUnaryOperator::Not,
                        target,
                    })),
                    source,
                )
            }
            // ^T
            dir::TypeExpression::OwnedOf { target_type, .. } => {
                let value = self.walk_type_expression(*target_type)?;

                self.push_type(
                    dir::Type::Form(dir::FormType {
                        form: dir::Form::Owned,
                        value,
                    }),
                    source,
                )
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
                let access = self.push_type(
                    dir::Type::Memory(dir::MemoryLiteral::Access(access)),
                    source,
                )?;
                // induce a hidden comptime parameter through declaration sites
                let lifetime = self.open_type(source)?;
                if let Some(variable) = self.check.root_variable(lifetime)? {
                    // constrain the induced parameter to the lifetime kind
                    let constraint = match self
                        .check
                        .environment
                        .language
                        .symbol(dir::LanguageItem::Lifetime)
                    {
                        Some(symbol) => Some(self.push_type(
                            dir::Type::Reference(dir::GenericInstance {
                                symbol,
                                arguments: Vec::new(),
                            }),
                            source,
                        )?),
                        None => None,
                    };
                    let induction = GenericInductionParameter {
                        prefix: "L",
                        constraint,
                        is_comptime: true,
                        induction: dir::GenericParameterInduction::Form,
                    };
                    self.check.generics.insert_induction(variable, induction)?;
                }

                self.push_type(
                    dir::Type::Form(dir::FormType {
                        form: dir::Form::Borrowed { lifetime, access },
                        value,
                    }),
                    source,
                )
            }
            // *T
            dir::TypeExpression::PointerOf { target_type, .. } => {
                let value = self.walk_type_expression(*target_type)?;

                self.push_type(
                    dir::Type::Form(dir::FormType {
                        form: dir::Form::Raw,
                        value,
                    }),
                    source,
                )
            }
            // A | B
            dir::TypeExpression::Union { elements } => {
                let elements = elements.iter().copied().collect::<SmallVec<[_; 4]>>();
                let mut element_types = Vec::new();
                for element in elements {
                    element_types.push(self.walk_type_expression(element)?);
                }

                self.push_type(
                    dir::Type::Union(dir::UnionType {
                        elements: element_types,
                    }),
                    source,
                )
            }
            // A & B
            dir::TypeExpression::Intersection { elements } => {
                let elements = elements.iter().copied().collect::<SmallVec<[_; 4]>>();
                let mut element_types = Vec::new();
                for element in elements {
                    element_types.push(self.walk_type_expression(element)?);
                }

                self.push_type(
                    dir::Type::Intersection(dir::IntersectionType {
                        elements: element_types,
                    }),
                    source,
                )
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
                let is_distributive = match self.check.ty(left)? {
                    dir::Type::Parameter(_) => true,
                    _ => false,
                };
                let right = self.walk_type_expression(extends_type)?;
                let then_type = self.walk_type_expression(then_type)?;
                let else_type = self.walk_type_expression(else_type)?;

                self.push_type(
                    dir::Type::Operation(dir::TypeOperation::Conditional(dir::ConditionalType {
                        left,
                        right,
                        then_type,
                        else_type,
                        is_distributive,
                    })),
                    source,
                )
            }
            // T extends U, T implements U
            dir::TypeExpression::Extends { left, right }
            | dir::TypeExpression::Implements { left, right } => {
                let left = self.walk_type_expression(*left)?;
                let right = self.walk_type_expression(*right)?;
                let then_type = self.push_type(
                    dir::Type::Literal(dir::ScalarLiteral::Boolean(true)),
                    source,
                )?;
                let else_type = self.push_type(
                    dir::Type::Literal(dir::ScalarLiteral::Boolean(false)),
                    source,
                )?;

                self.push_type(
                    dir::Type::Operation(dir::TypeOperation::Conditional(dir::ConditionalType {
                        left,
                        right,
                        then_type,
                        else_type,
                        is_distributive: false,
                    })),
                    source,
                )
            }
            // { [K in keyof T]: T[K] }
            dir::TypeExpression::Mapped {
                parameter,
                readonly,
                optional,
                value,
            } => self.build_mapped_type(id, *parameter, *readonly, *optional, *value),
            // T[K]
            dir::TypeExpression::Index { left, index } => {
                let left = self.walk_type_expression(*left)?;
                let index = self.walk_type_expression(*index)?;

                self.push_type(
                    dir::Type::Operation(dir::TypeOperation::Index(dir::IndexType { left, index })),
                    source,
                )
            }
            // `get${Name}`
            dir::TypeExpression::TemplateLiteral { strings, spans } => {
                let strings = strings.clone();
                let spans = spans.iter().copied().collect::<SmallVec<[_; 4]>>();
                let mut span_types = Vec::new();
                for span in spans {
                    span_types.push(self.walk_type_expression(span)?);
                }

                self.push_type(
                    dir::Type::Operation(dir::TypeOperation::TemplateLiteral(
                        dir::TemplateLiteralType {
                            strings,
                            spans: span_types,
                        },
                    )),
                    source,
                )
            }
            // _, infer T
            dir::TypeExpression::Infer {
                form,
                name,
                constraint,
            } => {
                let (form, name, constraint) = (*form, *name, *constraint);

                match form {
                    // open a widening variable for anonymous holes
                    dir::InferForm::Hole => {
                        let node = id.into_global_any(self.module);
                        if let Some(ty) = self.check.node_type(node) {
                            return Ok(ty);
                        }
                        let variable = self.check.allocate_variable(
                            self.module,
                            Origin::Node(node),
                            Widening::Widen,
                        );
                        let ty = self.check.push_variable_type(variable, source)?;
                        self.check.set_node_type(node, ty)?;

                        Ok(ty)
                    }
                    // keep infer bindings symbolic for conditional probes
                    dir::InferForm::Infer => {
                        let constraint = match constraint {
                            Some(constraint) => Some(self.walk_type_expression(constraint)?),
                            None => None,
                        };

                        self.push_type(
                            dir::Type::Operation(dir::TypeOperation::Infer(dir::InferType {
                                name,
                                constraint,
                            })),
                            source,
                        )
                    }
                }
            }
            // produce the error type for damaged children
            dir::TypeExpression::Missing | dir::TypeExpression::Error => {
                self.push_type(dir::Type::Error, source)
            }
        }
    }

    /// Build the type for one reference annotation, such as `Foo<T>`.
    fn build_reference_type(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        path: &dir::Path,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    ) -> CompilerResult<dir::GlobalTypeId> {
        let source = id.into_global_any(self.module);

        // read resolver output when available
        let reference = self
            .check
            .module(self.module)
            .resolved
            .references
            .get(source)
            .cloned();

        match reference {
            // build a direct reference from one resolved type declaration
            Some(dir::Reference::Bound(symbols)) => {
                self.build_bound_reference_type(id, path, generic_arguments, &symbols)
            }

            // build a type-member path from the resolved base declaration
            Some(dir::Reference::Projected { base, from }) => self.build_member_path_type(
                id,
                base,
                &path.segments[from as usize..],
                generic_arguments,
            ),

            // reject resolve conflicts in type position
            Some(dir::Reference::Ambiguous(_)) => {
                self.check
                    .report_ambiguous_reference(self.module, id.into_any(), path);

                self.push_type(dir::Type::Error, id.into_any())
            }

            // reject namespaces and missing references in type position
            Some(dir::Reference::Namespace(_)) | Some(dir::Reference::Missing) => {
                self.check
                    .report_unresolved_reference(self.module, id.into_any(), path);

                self.push_type(dir::Type::Error, id.into_any())
            }

            // require resolve to write every lexical reference row
            None => Err(CompilerError::Internal {
                message: format!("type reference {source:?} has no resolved name"),
            }),
        }
    }

    /// Build the type for one resolver-bound reference annotation.
    fn build_bound_reference_type(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        path: &dir::Path,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
        symbols: &[dir::GlobalSymbolId],
    ) -> CompilerResult<dir::GlobalTypeId> {
        let source = id.into_global_any(self.module);
        let symbols = self.check.available_symbols(symbols);

        match symbols.as_slice() {
            // reject malformed resolver state
            [] => Err(CompilerError::Internal {
                message: format!("type reference {source:?} resolved to no symbols"),
            }),

            // build the single resolved type declaration
            [symbol] => self.build_symbol_reference_type(id, *symbol, generic_arguments),

            // reject annotations that name more than one declaration
            _ => {
                self.check
                    .report_ambiguous_reference(self.module, id.into_any(), path);

                self.push_type(dir::Type::Error, id.into_any())
            }
        }
    }

    /// Build the type for one declaration symbol with written type arguments.
    fn build_symbol_reference_type(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        symbol: dir::GlobalSymbolId,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    ) -> CompilerResult<dir::GlobalTypeId> {
        let source = id.into_global_any(self.module);

        // record the resolved name for snapshots and downstream selection
        self.capture_symbol_reference(symbol);
        self.check
            .record_decision(source, Decision::Name(dir::NameResolution::new(symbol)))?;

        // apply written type arguments and declaration defaults
        let applied = self.walk_generic_arguments(generic_arguments)?;
        let ty = self.build_applied_symbol_type(id.into_any(), symbol, &applied)?;

        // require Dynamic<T> to carry a dynamically safe constraint
        if self.is_dynamic_symbol(symbol) {
            self.oblige_dynamic_reference(source, ty)?;
        }

        Ok(ty)
    }

    /// Build the type for one declaration symbol application.
    fn build_applied_symbol_type(
        &mut self,
        source: dir::LocalNodeIdAny,
        symbol: dir::GlobalSymbolId,
        applied: &[(Option<dir::StringId>, dir::GlobalTypeId)],
    ) -> CompilerResult<dir::GlobalTypeId> {
        // return the parameter type for generic parameter names
        if let Some(parameter) = self.check.generics.parameter_by_symbol(symbol) {
            if !applied.is_empty() {
                let name = self.check.format_symbol(symbol);
                self.check
                    .report_wrong_generic_arity(self.module, source, name, 0, applied.len());

                return self.push_type(dir::Type::Error, source);
            }

            return self.push_type(dir::Type::Parameter(parameter), source);
        }

        // reject written arguments on non-generic declarations
        let Some(template) = self.check.symbol_template(symbol) else {
            if !applied.is_empty() {
                let name = self.check.format_symbol(symbol);
                self.check
                    .report_wrong_generic_arity(self.module, source, name, 0, applied.len());

                return self.push_type(dir::Type::Error, source);
            }

            return self.push_type(
                dir::Type::Reference(dir::GenericInstance {
                    symbol,
                    arguments: Vec::new(),
                }),
                source,
            );
        };

        // reject impossible arities before applying defaults
        let parameters = self.check.generic_template_parameters(template);
        if applied.len() > parameters.len() {
            let name = self.check.format_symbol(symbol);
            self.check.report_wrong_generic_arity(
                self.module,
                source,
                name,
                parameters.len(),
                applied.len(),
            );

            return self.push_type(dir::Type::Error, source);
        }

        // apply written arguments and declared defaults in order
        let mut arguments = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        let mut written = applied.iter().map(|(_, argument)| *argument);
        for (index, parameter) in parameters.iter().copied().enumerate() {
            let argument = match written.next() {
                Some(argument) => argument,
                None => {
                    match self.check.generic_parameter_default(
                        self.module,
                        source,
                        parameter,
                        &parameters[..index],
                        &arguments,
                    )? {
                        Some(default) => default,
                        None => {
                            let name = self.check.format_symbol(symbol);
                            self.check.report_wrong_generic_arity(
                                self.module,
                                source,
                                name,
                                parameters.len(),
                                applied.len(),
                            );

                            return self.push_type(dir::Type::Error, source);
                        }
                    }
                }
            };
            arguments.push(argument);
        }

        self.push_type(
            dir::Type::Reference(dir::GenericInstance {
                symbol,
                arguments: arguments.into_vec(),
            }),
            source,
        )
    }

    /// Queue one dynamic-safety obligation under the active guard.
    fn oblige_dynamic_safe(&mut self, source: dir::GlobalNodeIdAny, ty: dir::GlobalTypeId) {
        let condition = self.active_static_guard();
        self.check
            .push_obligation(Obligation::DynamicSafety(DynamicSafetyObligation {
                source,
                condition,
                ty,
            }));
    }

    /// Return whether one symbol names the `Dynamic` language item.
    fn is_dynamic_symbol(&self, symbol: dir::GlobalSymbolId) -> bool {
        self.check
            .environment
            .language
            .item(symbol)
            .is_some_and(|item| item == dir::LanguageItem::Dynamic)
    }

    /// Queue the dynamic-safety obligation for one known `Dynamic<T>` reference.
    fn oblige_dynamic_reference(
        &mut self,
        source: dir::GlobalNodeIdAny,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        match self.check.ty(ty)? {
            // read the single constraint argument from Dynamic<T>
            dir::Type::Reference(instance) => match instance.arguments.as_slice() {
                [constraint] => self.oblige_dynamic_safe(source, *constraint),
                arguments => {
                    return Err(CompilerError::Internal {
                        message: format!("Dynamic reference carried {} arguments", arguments.len()),
                    });
                }
            },

            // skip failed references: arity checking already emitted the diagnostic
            dir::Type::Error => {}

            // reject broken internal construction
            other => {
                return Err(CompilerError::Internal {
                    message: format!("Dynamic reference produced {other:?}"),
                });
            }
        }

        Ok(())
    }

    /// Build a type-member path from one resolved base symbol.
    fn build_member_path_type(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        base: dir::GlobalSymbolId,
        tail: &[dir::StringId],
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.capture_symbol_reference(base);
        self.check.record_decision(
            id.into_global_any(self.module),
            Decision::Name(dir::NameResolution::new(base)),
        )?;

        // start from the resolved base symbol
        let mut ty = self.build_applied_symbol_type(id.into_any(), base, &[])?;

        // append each remaining path segment as a type member
        for (index, segment) in tail.iter().copied().enumerate() {
            let is_last = index + 1 == tail.len();
            let arguments = if is_last && !generic_arguments.is_empty() {
                self.walk_generic_arguments(generic_arguments)?
                    .into_iter()
                    .map(|(_, ty)| ty)
                    .collect()
            } else {
                Vec::new()
            };

            ty = self.push_type(
                dir::Type::Member(dir::MemberType {
                    owner: ty,
                    key: dir::StaticKey::Name(segment),
                    arguments,
                }),
                id.into_any(),
            )?;
        }

        Ok(ty)
    }

    /// Build a structural shape from one object type expression.
    fn build_shape_type(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
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
                    let Some(key) = key.direct_static_key() else {
                        continue;
                    };
                    let ty = match declared_type {
                        Some(declared_type) => self.walk_type_expression(declared_type)?,
                        None => self.push_type(dir::Type::Unknown, member.into_any())?,
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
                    let Some(key) = key.direct_static_key() else {
                        continue;
                    };
                    let result = self.function_result_type(member.into_any(), &signature, None)?;
                    let ty = self.function_signature_type(
                        member.into_any(),
                        &signature,
                        None,
                        None,
                        result,
                    )?;

                    fields.push(dir::TypeField {
                        key,
                        ty,
                        is_optional,
                        is_readonly: false,
                    });
                }
                dir::TypeMember::CallSignature { signature } => {
                    let signature = signature.clone();
                    let ty = self.function_type(member.into_any(), &signature, None, None)?;
                    call_signatures.push(ty);
                }
                dir::TypeMember::ConstructSignature { signature } => {
                    let signature = signature.clone();
                    let ty = self.constructor_type(member.into_any(), &signature, None, None)?;
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

        self.push_type(
            dir::Type::Shape(dir::ShapeType {
                fields,
                call_signatures,
                construct_signatures,
                index_signatures,
            }),
            id.into_any(),
        )
    }

    /// Build one tuple element row.
    fn build_tuple_element(
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
                let ty = self.push_type(dir::Type::Error, id.into_any())?;

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

    /// Build one placed type expression.
    fn build_placed_type(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        target_type: dir::LocalNodeId<dir::TypeExpression>,
        space: dir::Space,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let value = self.walk_type_expression(target_type)?;
        let place = self.push_type(
            dir::Type::Memory(dir::MemoryLiteral::Place(dir::Place::Space(space))),
            id.into_any(),
        )?;

        self.push_type(
            dir::Type::Form(dir::FormType {
                form: dir::Form::Placed { place },
                value,
            }),
            id.into_any(),
        )
    }

    /// Build one range type expression from its literal bounds.
    fn build_range_type(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        start: Option<dir::LocalNodeId<dir::TypeExpression>>,
        end: Option<dir::LocalNodeId<dir::TypeExpression>>,
        end_kind: dir::RangeEnd,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let start = match start {
            Some(start) => self.range_bound_literal(start)?,
            None => None,
        };
        let end = match end {
            Some(end) => self.range_bound_literal(end)?,
            None => None,
        };

        self.push_type(
            dir::Type::Range(dir::RangeType {
                start,
                end,
                is_inclusive: end_kind == dir::RangeEnd::Inclusive,
            }),
            id.into_any(),
        )
    }

    /// Read one range bound's scalar literal value.
    /// Leave symbolic bounds uninterpreted in interval types.
    fn range_bound_literal(
        &mut self,
        bound: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<Option<dir::ScalarLiteral>> {
        match self.tree.get(bound) {
            dir::TypeExpression::ScalarLiteral { value } => Ok(Some(*value)),
            _ => Ok(None),
        }
    }

    /// Build one mapped type expression.
    fn build_mapped_type(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
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
            return self.push_type(dir::Type::Error, id.into_any());
        };
        let constraint = self.walk_type_expression(source_type)?;

        // create a local generic parameter for the mapped key
        let binder = match self.check.generics.parameter_by_symbol(symbol) {
            Some(binder) => binder,
            None => {
                let template = self.check.declare_generic_template(
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
                    .declare_generic_parameter(binding, template, Some(symbol))?
            }
        };
        let ty = self.push_type(dir::Type::Parameter(binder), parameter.into_any())?;
        self.declare_symbol_type(symbol, ty)?;

        let key_remap = match key_remap {
            Some(key_remap) => Some(self.walk_type_expression(key_remap)?),
            None => None,
        };
        let value = match value {
            Some(value) => self.walk_type_expression(value)?,
            None => self.push_type(dir::Type::Unknown, id.into_any())?,
        };

        self.push_type(
            dir::Type::Operation(dir::TypeOperation::Mapped(dir::MappedType {
                parameter: dir::MappedTypeParameter {
                    name,
                    parameter: binder,
                    constraint,
                    key_remap,
                },
                modifiers: dir::MappedTypeModifiers { readonly, optional },
                value,
            })),
            id.into_any(),
        )
    }
}
