use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Decision, GenericInductionParameter, NameLookup, Origin, WalkState, Widening};

impl WalkState<'_, '_> {
    /// Lower one annotation node to its working type.
    ///
    /// Type expressions lower directly into working types here: named
    /// references resolve through the resolve-phase tables, forms spell
    /// their memory wrappers, and operations stay symbolic for reduce.
    /// Each node records its input so guards, patterns, and selections
    /// read annotation types uniformly.
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

        // reuse already-lowered annotation inputs
        if let Some(ty) = self.check.inputs.node_type(node) {
            return Ok(ty);
        }

        let ty = self.lower_type_expression(id)?;
        self.check.inputs.set_node_type(node, ty)?;

        Ok(ty)
    }

    /// Lower one type expression node by its syntactic shape.
    fn lower_type_expression(
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
            // intrinsic markers validate at their declarations
            dir::TypeExpression::Intrinsic => self.push_type(dir::Type::Error, source),
            // (A, B) and [A, B]
            dir::TypeExpression::Tuple { elements }
            | dir::TypeExpression::ArrayTuple { elements } => {
                let elements = elements.iter().copied().collect::<SmallVec<[_; 4]>>();
                let form = match self.tree.get(id) {
                    dir::TypeExpression::ArrayTuple { .. } => dir::TupleForm::Array,
                    _ => dir::TupleForm::Tuple,
                };
                let mut lowered = Vec::new();
                for element in elements {
                    lowered.push(self.lower_tuple_element(element)?);
                }

                self.push_type(
                    dir::Type::Tuple(dir::TupleType {
                        form,
                        elements: lowered,
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
                let count = self.lower_static_predicate(*length)?;

                self.push_type(
                    dir::Type::FixedArray(dir::FixedArrayType { element, count }),
                    source,
                )
            }
            // { name: string }
            dir::TypeExpression::Object { members } => {
                let members = members.iter().copied().collect::<SmallVec<[_; 4]>>();

                self.lower_object_type(id, &members)
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

                self.lower_reference_type(id, &path, &generic_arguments)
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
            } => self.lower_range_type(id, *start, *end, *end_kind),
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
                self.lower_placed_type(id, *target_type, dir::Space::Local)
            }
            dir::TypeExpression::Shared { target_type } => {
                self.lower_placed_type(id, *target_type, dir::Space::Shared)
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
            // &T opens its elided lifetime for induction
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
                // the open lifetime induces a hidden comptime parameter
                //  through the surrounding declaration's induction sites
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
                    let recipe = GenericInductionParameter {
                        prefix: "L",
                        constraint,
                        is_comptime: true,
                        induction: dir::GenericParameterInduction::Form,
                    };
                    self.check.generics.insert_induction(variable, recipe)?;
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
                let mut lowered = Vec::new();
                for element in elements {
                    lowered.push(self.walk_type_expression(element)?);
                }

                self.push_type(
                    dir::Type::Union(dir::UnionType { elements: lowered }),
                    source,
                )
            }
            // A & B
            dir::TypeExpression::Intersection { elements } => {
                let elements = elements.iter().copied().collect::<SmallVec<[_; 4]>>();
                let mut lowered = Vec::new();
                for element in elements {
                    lowered.push(self.walk_type_expression(element)?);
                }

                self.push_type(
                    dir::Type::Intersection(dir::IntersectionType { elements: lowered }),
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
                // only naked parameter scrutinees distribute over unions
                let is_distributive = match self.check.ty(left)? {
                    dir::Type::Parameter(_) => true,
                    dir::Type::Reference(instance) => {
                        instance.arguments.is_empty()
                            && self
                                .check
                                .generics
                                .parameter_by_symbol(instance.symbol)
                                .is_some()
                    }
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
            } => self.lower_mapped_type(id, *parameter, *readonly, *optional, *value),
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
                let mut lowered = Vec::new();
                for span in spans {
                    lowered.push(self.walk_type_expression(span)?);
                }

                self.push_type(
                    dir::Type::Operation(dir::TypeOperation::TemplateLiteral(
                        dir::TemplateLiteralType {
                            strings,
                            spans: lowered,
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
                    // anonymous holes open fresh variables that widen
                    dir::InferForm::Hole => {
                        let node = id.into_global_any(self.module);
                        if let Some(ty) = self.check.inputs.node_type(node) {
                            return Ok(ty);
                        }
                        let variable = self.check.allocate_variable(
                            self.module,
                            Origin::Node(node),
                            Widening::Widen,
                        );
                        let ty = self.check.push_variable_type(variable, source)?;
                        self.check.inputs.set_node_type(node, ty)?;

                        Ok(ty)
                    }
                    // infer bindings stay symbolic for conditional probes
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
            // missing and malformed children poison silently
            dir::TypeExpression::Missing | dir::TypeExpression::Error => {
                self.push_type(dir::Type::Error, source)
            }
        }
    }

    /// Lower one named type reference through the resolve tables.
    fn lower_reference_type(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        path: &dir::Path,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    ) -> CompilerResult<dir::GlobalTypeId> {
        let source = id.into_global_any(self.module);

        // resolve the path through the resolve-phase table
        let key = dir::PathKey::new(source, path.segments.len() as u32);
        let resolution = self
            .check
            .module(self.module)
            .resolved
            .paths
            .get(key)
            .cloned();
        let symbol = match resolution {
            Some(dir::PathResolution::Found(dir::PathTarget::Symbol(symbol))) => Some(symbol),
            Some(dir::PathResolution::Found(dir::PathTarget::Namespace(_))) => None,
            Some(dir::PathResolution::Ambiguous(_)) => {
                self.check
                    .report_ambiguous_reference(self.module, id.into_any(), path);

                None
            }
            Some(dir::PathResolution::Missing) | None => {
                // single-segment paths resolve lexically in type space
                let symbol = if let [name] = path.segments.as_slice() {
                    let lookup = self.check.lookup_name(
                        self.module,
                        id.into_any(),
                        *name,
                        dir::SymbolSpace::Type,
                    );

                    match lookup {
                        NameLookup::Found(candidate) => candidate.symbol(),
                        NameLookup::Missing | NameLookup::Ambiguous(_) => None,
                    }
                } else {
                    None
                };
                if symbol.is_none() {
                    self.check
                        .report_unresolved_reference(self.module, id.into_any(), path);
                }

                symbol
            }
        };

        let Some(symbol) = symbol else {
            return self.push_type(dir::Type::Error, id.into_any());
        };

        self.capture_symbol_reference(symbol);
        self.check
            .record_decision(source, Decision::Name(dir::NameResolution::new(symbol)))?;

        if generic_arguments.is_empty() {
            return self.reference_symbol_type(symbol);
        }

        // apply written arguments over the declared parameters
        let applied = self.walk_generic_arguments(generic_arguments)?;
        let arguments = self.canonical_generic_arguments(id.into_any(), symbol, &applied)?;

        self.push_type(
            dir::Type::Reference(dir::GenericInstance { symbol, arguments }),
            id.into_any(),
        )
    }

    /// Lower one object type expression to a structural shape.
    fn lower_object_type(
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
                // associated members live on declarations, not shapes
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

    /// Lower one tuple element to its checked row.
    fn lower_tuple_element(
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

    /// Lower one placed type expression.
    fn lower_placed_type(
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

    /// Lower one range type expression from its literal bounds.
    fn lower_range_type(
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

    /// Return one range bound's scalar literal value.
    /// Symbolic bounds stay uninterpreted in interval types.
    fn range_bound_literal(
        &mut self,
        bound: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<Option<dir::ScalarLiteral>> {
        match self.tree.get(bound) {
            dir::TypeExpression::ScalarLiteral { value } => Ok(Some(*value)),
            _ => Ok(None),
        }
    }

    /// Lower one mapped type expression.
    fn lower_mapped_type(
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

        // the binder is a generic parameter on its own template, so
        // its uses stay symbolic and expansion substitutes per key
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
