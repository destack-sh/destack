use destack_core::StringPool;
use destack_dir as dir;
use destack_source::{FileId, ModuleId, NodeSpanRegion, NodeSpanType, Span};

use crate::CompilerResult;
use crate::sema::CheckState;

/// Nesting depth bound guarding reified annotations.
const REIFY_DEPTH: usize = 32;

/// Synthesizes type and static expression nodes from solved check types.
pub(in crate::sema) struct TypeReifier<'a, 'b> {
    /// The solved module state read for type structure.
    check: &'a CheckState<'b>,
    /// The amended output tree receiving synthesized nodes.
    pub(super) tree: dir::Tree,
    /// The string pool shared with the rendered module.
    strings: &'a StringPool,
    /// The anchor span stamped on synthesized nodes.
    span: Span,
}

impl<'a, 'b> TypeReifier<'a, 'b> {
    /// Create a type reifier for one cloned module tree.
    pub(in crate::sema) fn new(
        check: &'a CheckState<'b>,
        tree: dir::Tree,
        strings: &'a StringPool,
    ) -> Self {
        Self {
            check,
            tree,
            strings,
            span: Span::empty(FileId::new(0)),
        }
    }

    /// Anchor synthesized nodes at one source position.
    pub(in crate::sema) fn anchor(&mut self, span: Span) {
        self.span = Span::new(span.file, span.end, span.end);
    }

    /// Return the current synthesized-node span.
    pub(in crate::sema) fn span(&self) -> Span {
        self.span
    }

    /// Reify one solved type into a synthesized type expression.
    pub(in crate::sema) fn reify(
        &mut self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::LocalNodeId<dir::TypeExpression>>> {
        self.reify_depth(id, REIFY_DEPTH)
    }

    /// Reify one parameter type, omitting undefined when `?` already implies it.
    pub(in crate::sema) fn reify_parameter_type(
        &mut self,
        id: dir::GlobalTypeId,
        is_optional: bool,
    ) -> CompilerResult<Option<dir::LocalNodeId<dir::TypeExpression>>> {
        self.reify_parameter_type_depth(id, is_optional, REIFY_DEPTH)
    }

    /// Reify selected generic argument bindings into synthesized generic arguments.
    pub(in crate::sema) fn reify_generic_argument_bindings(
        &mut self,
        bindings: &[dir::GenericArgumentBinding],
    ) -> CompilerResult<Option<Vec<dir::LocalNodeId<dir::GenericArgument>>>> {
        let arguments = dir::GenericArgumentBinding::values(bindings).collect::<Vec<_>>();

        self.reify_arguments(&arguments, REIFY_DEPTH)
    }

    /// Fill selected generic arguments on one reified type reference.
    pub(super) fn fill_type_arguments(
        &mut self,
        ty: dir::LocalNodeId<dir::TypeExpression>,
        bindings: &[dir::GenericArgumentBinding],
    ) -> CompilerResult<()> {
        match self.tree.get(ty) {
            dir::TypeExpression::Reference {
                generic_arguments, ..
            }
            | dir::TypeExpression::Member {
                generic_arguments, ..
            } if generic_arguments.is_empty() => {}
            _ => return Ok(()),
        }

        let Some(generic_arguments) = self.reify_generic_argument_bindings(bindings)? else {
            return Ok(());
        };

        match self.tree.get_mut(ty) {
            dir::TypeExpression::Reference {
                generic_arguments: target,
                ..
            }
            | dir::TypeExpression::Member {
                generic_arguments: target,
                ..
            } => {
                *target = generic_arguments;
            }
            _ => unreachable!("generic type arguments are filtered above"),
        }

        Ok(())
    }

    /// Reify one generic parameter binding into a synthesized parameter node.
    pub(super) fn reify_generic_parameter(
        &mut self,
        binding: &dir::GenericParameterBinding,
    ) -> CompilerResult<Option<dir::LocalNodeId<dir::GenericParameter>>> {
        let Some(name) = self.generic_parameter_name(binding) else {
            return Ok(None);
        };

        // reify tick names as bare lifetime parameters
        if binding.memory_parameter() == Some(dir::MemoryParameter::Lifetime)
            && self.check.strings().get(name).starts_with('\'')
        {
            let parameter = self.insert(dir::GenericParameter::Lifetime { name });

            return Ok(Some(parameter));
        }

        let parameter = self.reify_type_parameter(binding, name)?;
        let parameter = self.insert(parameter);
        if binding.constraint.is_some() {
            self.tree.set_side_span(
                parameter,
                NodeSpanType::Region(NodeSpanRegion::Type),
                self.span(),
            );
        }

        Ok(Some(parameter))
    }

    /// Reify one settled borrow into its named borrow expression.
    fn reify_borrowed_of(
        &mut self,
        lifetime: dir::GlobalTypeId,
        access: dir::GlobalTypeId,
        target_type: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<Option<dir::TypeExpression>> {
        // require a settled access literal for the borrow modifier
        let access = match self.check.ty(self.check.shallow_resolve(access)?)? {
            dir::Type::Memory(dir::MemoryLiteral::Access(access)) => access,
            _ => return Ok(None),
        };
        let mutability = match access {
            dir::Access::Mutable => dir::Mutability::Mutable,
            dir::Access::Readonly => dir::Mutability::Immutable,
            dir::Access::Exclusive => dir::Mutability::Exclusive,
        };

        // name the tick parameter or reserved lifetime literal
        let name = match self.check.ty(self.check.shallow_resolve(lifetime)?)? {
            dir::Type::Parameter(parameter) => {
                let Some(binding) = self.check.generic_parameter(parameter) else {
                    return Ok(None);
                };
                let name = match binding.key {
                    dir::GenericParameterKey::Symbol(symbol) => {
                        let Some(name) = self.symbol_name(symbol) else {
                            return Ok(None);
                        };

                        name
                    }
                    dir::GenericParameterKey::Generated(name) => name,
                };
                if !self.check.strings().get(name).starts_with('\'') {
                    return Ok(None);
                }

                name
            }
            dir::Type::Memory(dir::MemoryLiteral::Lifetime(dir::Lifetime::Static)) => {
                self.check.strings().intern("'static")
            }
            dir::Type::Memory(dir::MemoryLiteral::Lifetime(dir::Lifetime::Frame)) => {
                self.check.strings().intern("'frame")
            }
            _ => return Ok(None),
        };
        let lifetime = self.insert(dir::TypeExpression::Lifetime { name });

        Ok(Some(dir::TypeExpression::BorrowedOf {
            lifetime: Some(lifetime),
            mutability: Some(mutability),
            variance: None,
            target_type,
        }))
    }

    /// Reify one type generic parameter binding.
    fn reify_type_parameter(
        &mut self,
        binding: &dir::GenericParameterBinding,
        name: dir::StringId,
    ) -> CompilerResult<dir::GenericParameter> {
        let constraint = binding
            .constraint
            .map(|constraint| self.reify(constraint))
            .transpose()?
            .flatten();
        let default = binding
            .default
            .map(|default| self.reify(default))
            .transpose()?
            .flatten();

        let parameter = if binding.is_variadic {
            dir::GenericParameter::VariadicType {
                name,
                variance: binding.variance,
                constraint,
                default,
                is_const: binding.is_const,
            }
        } else {
            dir::GenericParameter::Type {
                name,
                variance: binding.variance,
                constraint,
                default,
                is_const: binding.is_const,
            }
        };

        Ok(parameter)
    }

    /// Reify one solved static singleton into a synthesized expression.
    pub(in crate::sema) fn reify_static(
        &mut self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::LocalNodeId<dir::Expression>>> {
        self.reify_static_depth(id, REIFY_DEPTH)
    }

    /// Reify one solved type up to a nesting depth.
    fn reify_depth(
        &mut self,
        id: dir::GlobalTypeId,
        depth: usize,
    ) -> CompilerResult<Option<dir::LocalNodeId<dir::TypeExpression>>> {
        if depth == 0 {
            return Ok(None);
        }
        let id = self.check.shallow_resolve(id)?;
        let ty = self.check.ty(id)?;
        let next = depth - 1;

        let expression = match ty {
            // skip open variables, holes, rigid and erased parameters, and errors
            dir::Type::Variable(_)
            | dir::Type::Hole(_)
            | dir::Type::Rigid(_)
            | dir::Type::Erased(_)
            | dir::Type::Error => {
                return Ok(None);
            }
            // refinements print as their base application
            dir::Type::Refined(refined) => {
                let refined = self.check.type_refined(id.module_id, refined)?;

                return self.reify_depth(refined.base, depth);
            }

            dir::Type::Never => Self::literal(dir::TypeLiteral::Never),
            dir::Type::Unknown => Self::literal(dir::TypeLiteral::Unknown),
            dir::Type::Void => Self::literal(dir::TypeLiteral::Void),
            dir::Type::Null => Self::literal(dir::TypeLiteral::Null),
            dir::Type::Undefined => Self::literal(dir::TypeLiteral::Undefined),
            dir::Type::Intrinsic => dir::TypeExpression::Intrinsic,
            dir::Type::This => dir::TypeExpression::This,

            dir::Type::Primitive(primitive) => Self::literal(dir::TypeLiteral::from(primitive)),
            dir::Type::Literal(literal) => dir::TypeExpression::Literal { value: literal },
            dir::Type::Key(key) => {
                let Some(value) = Self::static_key_literal(&key) else {
                    return Ok(None);
                };

                dir::TypeExpression::Literal { value }
            }
            dir::Type::Memory(literal) => dir::TypeExpression::Literal {
                value: dir::Literal::String(self.strings.intern(literal.text())),
            },
            dir::Type::Static(value) => match self.check.r#static(value) {
                dir::StaticTerm::Literal { value } => {
                    dir::TypeExpression::Literal { value: *value }
                }
                _ => return Ok(None),
            },
            dir::Type::Range(range) => {
                let start = range
                    .start
                    .map(|value| self.insert(dir::TypeExpression::Literal { value }));
                let end = range
                    .end
                    .map(|value| self.insert(dir::TypeExpression::Literal { value }));
                let end_kind = if range.is_inclusive {
                    dir::RangeEnd::Inclusive
                } else {
                    dir::RangeEnd::Open
                };

                dir::TypeExpression::Range {
                    start,
                    end,
                    end_kind,
                }
            }

            dir::Type::Parameter(parameter) => {
                let Some(name) = self.generic_parameter_name_by_id(parameter) else {
                    return Ok(None);
                };

                Self::reference(name)
            }
            dir::Type::Reference(reference) => {
                let Some(name) = self.symbol_name(reference.symbol) else {
                    return Ok(None);
                };

                Self::reference(name)
            }
            // array applications reify in their written rest form
            dir::Type::Application(_) if let Some(element) = self.check.array_element(id)? => {
                let Some(element) = self.reify_depth(element, next)? else {
                    return Ok(None);
                };

                return Ok(Some(self.insert(dir::TypeExpression::Array { element })));
            }
            dir::Type::Application(instance) => {
                let Some(name) = self.symbol_name(instance.symbol) else {
                    return Ok(None);
                };
                let arguments = self
                    .check
                    .type_ids(id.module_id, instance.arguments)?
                    .to_vec();
                let Some(arguments) = self.reify_arguments(&arguments, next)? else {
                    return Ok(None);
                };

                dir::TypeExpression::Reference {
                    path: dir::Path {
                        segments: [name].into_iter().collect(),
                    },
                    generic_arguments: arguments,
                }
            }
            dir::Type::Member(member) => {
                let member = self.check.type_member(id.module_id, member)?;
                let dir::StaticKey::Name(key) = member.key else {
                    return Ok(None);
                };
                let Some(left) = self.reify_depth(member.owner, next)? else {
                    return Ok(None);
                };

                // keep member generic arguments in the type-argument form
                let member_arguments = self
                    .check
                    .type_ids(id.module_id, member.arguments)?
                    .to_vec();
                let Some(arguments) = self.reify_arguments(&member_arguments, next)? else {
                    return Ok(None);
                };

                dir::TypeExpression::Member {
                    left,
                    name: key,
                    generic_arguments: arguments,
                }
            }
            dir::Type::Variant(_) => return Ok(None),

            dir::Type::Slice(slice) => {
                let Some(element) = self.reify_depth(slice.element, next)? else {
                    return Ok(None);
                };

                dir::TypeExpression::Slice { element }
            }
            dir::Type::FixedArray(array) => {
                let Some(element) = self.reify_depth(array.element, next)? else {
                    return Ok(None);
                };
                let count = self.check.shallow_resolve(array.count)?;
                let dir::Type::Literal(value) = self.check.ty(count)? else {
                    return Ok(None);
                };
                let length = self.insert(dir::Expression::Literal(value));

                dir::TypeExpression::FixedArray { element, length }
            }
            dir::Type::Tuple(tuple) => {
                let tuple_elements = self
                    .check
                    .tuple_elements(id.module_id, tuple.elements)?
                    .to_vec();
                let mut elements = Vec::with_capacity(tuple_elements.len());
                for element in &tuple_elements {
                    let Some(value) = self.reify_depth(element.ty, next)? else {
                        return Ok(None);
                    };
                    let element = if element.is_rest {
                        dir::TupleElement::Spread {
                            label: element.label,
                            value,
                        }
                    } else {
                        dir::TupleElement::Element {
                            label: element.label,
                            value,
                            is_optional: element.is_optional,
                            is_readonly: element.is_readonly,
                        }
                    };

                    elements.push(self.insert(element));
                }

                dir::TypeExpression::Tuple {
                    form: tuple.form,
                    elements,
                }
            }
            dir::Type::Object(shape) => {
                // TODO #Incomplete: reify index and call signatures as object members
                if shape.declares_signatures() {
                    return Ok(None);
                }

                let shape_properties = self
                    .check
                    .shape_properties(id.module_id, shape.properties)?
                    .to_vec();
                let mut members = Vec::with_capacity(shape_properties.len());
                for field in &shape_properties {
                    let name = match field.key {
                        dir::StaticKey::Name(name) => dir::Name::Identifier(name),
                        dir::StaticKey::Index(index) => dir::Name::Index(index),
                    };
                    let Some(declared_type) = self.reify_depth(field.access.store(), next)? else {
                        return Ok(None);
                    };
                    let member = dir::TypeMember::Field {
                        name,
                        declared_type: Some(declared_type),
                        is_static: false,
                        is_optional: field.is_optional,
                        is_readonly: !field.access.is_writable(),
                    };

                    members.push(self.insert(member));
                }

                dir::TypeExpression::Object { members }
            }
            dir::Type::FunctionSignature(function) => {
                let signature = self.check.type_signature(id.module_id, function)?;
                let Some(function) = self.reify_function(id.module_id, &signature, next)? else {
                    return Ok(None);
                };

                // construct signatures annotate in their `new` form
                match signature.is_construct {
                    true => dir::TypeExpression::Constructor(dir::ConstructorType {
                        generic_parameters: function.generic_parameters,
                        where_clauses: function.where_clauses,
                        parameters: function.parameters,
                        return_type: function.return_type,
                        is_abstract: false,
                    }),
                    false => dir::TypeExpression::Function(function),
                }
            }
            dir::Type::Function(function) => {
                return self.reify_depth(function.signature, next);
            }
            dir::Type::FunctionPointer(function) => {
                let Some(expression) = self.reify_function_pointer(&function, next)? else {
                    return Ok(None);
                };

                expression
            }

            dir::Type::Union(union) => {
                let union_elements = self.check.type_ids(id.module_id, union.elements)?.to_vec();
                let Some(elements) = self.reify_elements(&union_elements, next)? else {
                    return Ok(None);
                };

                dir::TypeExpression::Union { elements }
            }
            dir::Type::Intersection(intersection) => {
                let intersection_elements = self
                    .check
                    .type_ids(id.module_id, intersection.elements)?
                    .to_vec();
                let Some(elements) = self.reify_elements(&intersection_elements, next)? else {
                    return Ok(None);
                };

                dir::TypeExpression::Intersection { elements }
            }

            dir::Type::Form(form) => {
                let Some(target_type) = self.reify_depth(form.value, next)? else {
                    return Ok(None);
                };

                match &form.form {
                    // reduction leaves only meaningful boxes, spell them
                    dir::Form::Managed => {
                        let target_type =
                            self.insert(dir::GenericArgument::Type { value: target_type });
                        let name = self.language_item_name(dir::LanguageItem::Managed);

                        dir::TypeExpression::Reference {
                            path: dir::Path {
                                segments: [name].into_iter().collect(),
                            },
                            generic_arguments: vec![target_type],
                        }
                    }
                    dir::Form::Owned => dir::TypeExpression::OwnedOf {
                        mutability: None,
                        variance: None,
                        target_type,
                    },
                    dir::Form::Raw => dir::TypeExpression::PointerOf {
                        mutability: None,
                        target_type,
                    },
                    dir::Form::Readonly => dir::TypeExpression::Readonly { target_type },
                    dir::Form::Borrowed(borrow) => {
                        let borrow = self.check.type_borrow(id.module_id, *borrow)?;
                        let (lifetime, access) = (borrow.lifetime, borrow.access);

                        // render settled borrows through the borrow form
                        if let Some(borrowed) =
                            self.reify_borrowed_of(lifetime, access, target_type)?
                        {
                            borrowed
                        }
                        // render parametric slots through the full algebra
                        else {
                            let Some(lifetime) = self.reify_depth(lifetime, next)? else {
                                return Ok(None);
                            };
                            let Some(access) = self.reify_depth(access, next)? else {
                                return Ok(None);
                            };
                            let target_type =
                                self.insert(dir::GenericArgument::Type { value: target_type });
                            let lifetime =
                                self.insert(dir::GenericArgument::Type { value: lifetime });
                            let access = self.insert(dir::GenericArgument::Type { value: access });
                            let name = self.language_item_name(dir::LanguageItem::Borrowed);

                            dir::TypeExpression::Reference {
                                path: dir::Path {
                                    segments: [name].into_iter().collect(),
                                },
                                generic_arguments: vec![target_type, lifetime, access],
                            }
                        }
                    }
                    dir::Form::Placed { place } => {
                        let place = self.check.shallow_resolve(*place)?;
                        let concrete = match self.check.ty(place)? {
                            dir::Type::Memory(dir::MemoryLiteral::Place(dir::Place::Space(
                                space,
                            ))) => Some(space),
                            _ => None,
                        };

                        match concrete {
                            Some(dir::Space::Local) => dir::TypeExpression::Local { target_type },
                            Some(dir::Space::Shared) => dir::TypeExpression::Shared { target_type },
                            None => {
                                let Some(place) = self.reify_depth(place, next)? else {
                                    return Ok(None);
                                };
                                let target =
                                    self.insert(dir::GenericArgument::Type { value: target_type });
                                let place =
                                    self.insert(dir::GenericArgument::Type { value: place });
                                let name = self.language_item_name(dir::LanguageItem::Placed);

                                dir::TypeExpression::Reference {
                                    path: dir::Path {
                                        segments: [name].into_iter().collect(),
                                    },
                                    generic_arguments: vec![target, place],
                                }
                            }
                        }
                    }
                }
            }
            dir::Type::Dynamic(dynamic) => {
                let Some(constraint) = self.reify_depth(dynamic.constraint, next)? else {
                    return Ok(None);
                };
                let argument = self.insert(dir::GenericArgument::Type { value: constraint });

                dir::TypeExpression::Reference {
                    path: dir::Path {
                        segments: [self.language_item_name(dir::LanguageItem::Dynamic)]
                            .into_iter()
                            .collect(),
                    },
                    generic_arguments: vec![argument],
                }
            }

            dir::Type::Operation(operation) => {
                let operation = self.check.type_operation(id.module_id, operation)?;
                let Some(expression) = self.reify_operation(id.module_id, &operation, next)? else {
                    return Ok(None);
                };

                expression
            }
        };

        Ok(Some(self.insert(expression)))
    }

    /// Reify one parameter type up to a nesting depth.
    fn reify_parameter_type_depth(
        &mut self,
        id: dir::GlobalTypeId,
        is_optional: bool,
        depth: usize,
    ) -> CompilerResult<Option<dir::LocalNodeId<dir::TypeExpression>>> {
        if !is_optional {
            return self.reify_depth(id, depth);
        }
        if depth == 0 {
            return Ok(None);
        }

        let id = self.check.shallow_resolve(id)?;
        let dir::Type::Union(union) = self.check.ty(id)? else {
            return self.reify_depth(id, depth);
        };
        let union_elements = self.check.type_ids(id.module_id, union.elements)?.to_vec();

        let mut elements = Vec::new();
        for element in &union_elements {
            let element = self.check.shallow_resolve(*element)?;
            if self.check.ty(element)?.is_undefined() {
                continue;
            }
            let Some(element) = self.reify_depth(element, depth - 1)? else {
                return Ok(None);
            };

            elements.push(element);
        }

        match elements.as_slice() {
            [] => self.reify_depth(id, depth),
            [single] => Ok(Some(*single)),
            _ => Ok(Some(self.insert(dir::TypeExpression::Union { elements }))),
        }
    }

    /// Reify one preserved type operation.
    fn reify_operation(
        &mut self,
        module: ModuleId,
        operation: &dir::TypeOperation,
        depth: usize,
    ) -> CompilerResult<Option<dir::TypeExpression>> {
        let expression = match operation {
            dir::TypeOperation::Conditional(conditional) => {
                let Some(left) = self.reify_depth(conditional.left, depth)? else {
                    return Ok(None);
                };
                let Some(extends_type) = self.reify_depth(conditional.right, depth)? else {
                    return Ok(None);
                };
                let Some(then_type) = self.reify_depth(conditional.then_type, depth)? else {
                    return Ok(None);
                };
                let Some(else_type) = self.reify_depth(conditional.else_type, depth)? else {
                    return Ok(None);
                };

                dir::TypeExpression::Conditional {
                    left,
                    extends_type,
                    then_type,
                    else_type,
                }
            }
            dir::TypeOperation::KeyOf(unary) => {
                let Some(target_type) = self.reify_depth(unary.target, depth)? else {
                    return Ok(None);
                };

                dir::TypeExpression::KeyOf { target_type }
            }
            dir::TypeOperation::NoInfer(unary) => {
                let Some(target) = self.reify_depth(unary.target, depth)? else {
                    return Ok(None);
                };
                let argument = self.insert(dir::GenericArgument::Type { value: target });

                dir::TypeExpression::Reference {
                    path: dir::Path {
                        segments: [self.language_item_name(dir::LanguageItem::NoInfer)]
                            .into_iter()
                            .collect(),
                    },
                    generic_arguments: vec![argument],
                }
            }
            dir::TypeOperation::Awaited(unary) => {
                let Some(target) = self.reify_depth(unary.target, depth)? else {
                    return Ok(None);
                };
                let argument = self.insert(dir::GenericArgument::Type { value: target });

                dir::TypeExpression::Reference {
                    path: dir::Path {
                        segments: [self.language_item_name(dir::LanguageItem::Awaited)]
                            .into_iter()
                            .collect(),
                    },
                    generic_arguments: vec![argument],
                }
            }
            dir::TypeOperation::Index(index) => {
                let Some(left) = self.reify_depth(index.left, depth)? else {
                    return Ok(None);
                };
                let Some(index) = self.reify_depth(index.index, depth)? else {
                    return Ok(None);
                };

                dir::TypeExpression::Index { left, index }
            }
            dir::TypeOperation::Infer(infer) => dir::TypeExpression::Infer {
                form: dir::InferForm::Infer,
                name: infer.name,
                constraint: None,
            },
            dir::TypeOperation::TemplateLiteral(template) => {
                let strings = self
                    .check
                    .template_strings(module, template.strings)?
                    .to_vec();
                let span_types = self.check.type_ids(module, template.spans)?.to_vec();
                let mut spans = Vec::with_capacity(span_types.len());
                for span in span_types {
                    let Some(span) = self.reify_depth(span, depth)? else {
                        return Ok(None);
                    };
                    spans.push(span);
                }

                dir::TypeExpression::TemplateLiteral { strings, spans }
            }
            // skip the remaining operations, which have no faithful annotation form
            dir::TypeOperation::StringMapping { .. }
            | dir::TypeOperation::Narrow(_)
            | dir::TypeOperation::TypeOf(_)
            | dir::TypeOperation::Mapped(_)
            | dir::TypeOperation::TryOutput { .. }
            | dir::TypeOperation::TryResidual { .. }
            | dir::TypeOperation::StaticBinary(_)
            | dir::TypeOperation::StaticUnary(_) => return Ok(None),
        };

        Ok(Some(expression))
    }

    /// Reify one function type into a function type expression.
    fn reify_function(
        &mut self,
        module: destack_source::ModuleId,
        function: &dir::FunctionSignatureType,
        depth: usize,
    ) -> CompilerResult<Option<dir::FunctionTypeExpression>> {
        // async, generator, and generic signatures have no annotation form
        if function.asynchrony != dir::Asynchrony::Sync
            || function.is_generator
            || !self
                .check
                .signature_generic_parameters(function)?
                .is_empty()
        {
            return Ok(None);
        }

        let this_parameter = match function.this_parameter {
            Some(this) => {
                let Some(declared_type) = self.reify_depth(this, depth)? else {
                    return Ok(None);
                };
                let parameter = dir::Parameter::Named {
                    name: self.strings.intern("this"),
                    declared_type: Some(declared_type),
                    default: None,
                    is_optional: false,
                };

                Some(self.insert(parameter))
            }
            None => None,
        };

        let function_parameters = self
            .check
            .signature_parameters(module, function.parameters)?
            .to_vec();
        let mut parameters = Vec::with_capacity(function_parameters.len());
        for (index, parameter) in function_parameters.iter().enumerate() {
            let Some(declared_type) =
                self.reify_parameter_type_depth(parameter.ty, parameter.is_optional, depth)?
            else {
                return Ok(None);
            };
            let name = self.strings.intern(&format!("arg{index}"));
            let parameter = if parameter.is_rest {
                dir::Parameter::VariadicNamed {
                    name,
                    declared_type: Some(declared_type),
                }
            } else {
                dir::Parameter::Named {
                    name,
                    declared_type: Some(declared_type),
                    default: None,
                    is_optional: parameter.is_optional,
                }
            };

            parameters.push(self.insert(parameter));
        }

        let return_type = match function.return_type {
            Some(return_type) => match self.reify_depth(return_type, depth)? {
                Some(return_type) => return_type,
                None => return Ok(None),
            },
            None => self.insert(Self::literal(dir::TypeLiteral::Void)),
        };

        Ok(Some(dir::FunctionTypeExpression {
            generic_parameters: Vec::new(),
            where_clauses: Vec::new(),
            this_form: this_parameter.map(|_| dir::ThisForm::Explicit),
            this_parameter,
            parameters,
            return_type: Some(return_type),
        }))
    }

    /// Reify one function pointer type into its intrinsic type expression.
    fn reify_function_pointer(
        &mut self,
        function: &dir::FunctionPointerType,
        depth: usize,
    ) -> CompilerResult<Option<dir::TypeExpression>> {
        let signature_id = self.check.shallow_resolve(function.signature)?;
        let Some(signature) = self.check.signature_head(signature_id)? else {
            return Ok(None);
        };
        let signature_parameters = self
            .check
            .signature_parameters(signature_id.module_id, signature.parameters)?
            .to_vec();
        let Some(parameters) =
            self.reify_function_pointer_parameters(&signature_parameters, depth)?
        else {
            return Ok(None);
        };
        let return_type = match signature.return_type {
            Some(return_type) => match self.reify_depth(return_type, depth)? {
                Some(return_type) => return_type,
                None => return Ok(None),
            },
            None => self.insert(Self::literal(dir::TypeLiteral::Void)),
        };
        let parameters = self.insert(dir::GenericArgument::Type { value: parameters });
        let return_type = self.insert(dir::GenericArgument::Type { value: return_type });

        Ok(Some(dir::TypeExpression::Reference {
            path: dir::Path {
                segments: [self.strings.intern("FunctionPointer")]
                    .into_iter()
                    .collect(),
            },
            generic_arguments: vec![parameters, return_type],
        }))
    }

    /// Reify one function pointer parameter tuple.
    fn reify_function_pointer_parameters(
        &mut self,
        parameters: &[dir::FunctionParameterType],
        depth: usize,
    ) -> CompilerResult<Option<dir::LocalNodeId<dir::TypeExpression>>> {
        let mut elements = Vec::with_capacity(parameters.len());
        for parameter in parameters {
            let Some(value) =
                self.reify_parameter_type_depth(parameter.ty, parameter.is_optional, depth)?
            else {
                return Ok(None);
            };
            let element = if parameter.is_rest {
                dir::TupleElement::Spread { label: None, value }
            } else {
                dir::TupleElement::Element {
                    label: None,
                    value,
                    is_optional: parameter.is_optional,
                    is_readonly: false,
                }
            };

            elements.push(self.insert(element));
        }
        let expression = dir::TypeExpression::Tuple {
            form: dir::TupleForm::Tuple,
            elements,
        };
        let expression = self.insert(expression);

        Ok(Some(expression))
    }

    /// Reify one type list into type generic arguments.
    fn reify_arguments(
        &mut self,
        ids: &[dir::GlobalTypeId],
        depth: usize,
    ) -> CompilerResult<Option<Vec<dir::LocalNodeId<dir::GenericArgument>>>> {
        let mut arguments = Vec::with_capacity(ids.len());
        for id in ids {
            let Some(value) = self.reify_depth(*id, depth)? else {
                return Ok(None);
            };

            let argument = self.insert(dir::GenericArgument::Type { value });
            arguments.push(argument);
        }

        Ok(Some(arguments))
    }

    /// Reify one static singleton into a synthesized expression.
    fn reify_static_depth(
        &mut self,
        id: dir::GlobalTypeId,
        depth: usize,
    ) -> CompilerResult<Option<dir::LocalNodeId<dir::Expression>>> {
        if depth == 0 {
            return Ok(None);
        }
        let id = self.check.shallow_resolve(id)?;

        let expression = match self.check.ty(id)? {
            dir::Type::Literal(value) => dir::Expression::Literal(value),
            dir::Type::Key(key) => {
                let Some(value) = Self::static_key_literal(&key) else {
                    return Ok(None);
                };

                dir::Expression::Literal(value)
            }
            dir::Type::Memory(literal) => {
                dir::Expression::Literal(dir::Literal::String(self.strings.intern(literal.text())))
            }
            dir::Type::Static(value) => match self.check.r#static(value) {
                dir::StaticTerm::Literal { value } => dir::Expression::Literal(*value),
                _ => return Ok(None),
            },
            dir::Type::Union(union) => {
                let elements = self.check.type_ids(id.module_id, union.elements)?.to_vec();
                let Some(expression) = self.reify_static_union(&elements, depth - 1)? else {
                    return Ok(None);
                };

                return Ok(Some(expression));
            }
            dir::Type::Parameter(parameter) => {
                let Some(name) = self.generic_parameter_name_by_id(parameter) else {
                    return Ok(None);
                };

                dir::Expression::Identifier { name }
            }
            _ => return Ok(None),
        };

        Ok(Some(self.insert(expression)))
    }

    /// Reify one static union as a value expression.
    fn reify_static_union(
        &mut self,
        elements: &[dir::GlobalTypeId],
        depth: usize,
    ) -> CompilerResult<Option<dir::LocalNodeId<dir::Expression>>> {
        let mut elements = elements.iter().copied();
        let Some(first) = elements.next() else {
            return Ok(None);
        };
        let Some(mut expression) = self.reify_static_depth(first, depth)? else {
            return Ok(None);
        };

        // fold remaining elements into a binary value expression
        for element in elements {
            let Some(right) = self.reify_static_depth(element, depth)? else {
                return Ok(None);
            };
            expression = self.insert(dir::Expression::Binary {
                left: expression,
                operator: dir::BinaryOperator::ElementwiseOr,
                right,
            });
        }

        Ok(Some(expression))
    }

    /// Return the scalar literal text of one exact key type.
    fn static_key_literal(key: &dir::StaticKey) -> Option<dir::Literal> {
        match key {
            dir::StaticKey::Name(name) => Some(dir::Literal::String(*name)),
            dir::StaticKey::Index(index) => {
                let index = i64::try_from(*index).ok()?;

                Some(dir::Literal::Integer(index))
            }
        }
    }

    /// Reify one type element list.
    fn reify_elements(
        &mut self,
        ids: &[dir::GlobalTypeId],
        depth: usize,
    ) -> CompilerResult<Option<Vec<dir::LocalNodeId<dir::TypeExpression>>>> {
        let mut elements = Vec::with_capacity(ids.len());
        for id in ids {
            let Some(element) = self.reify_depth(*id, depth)? else {
                return Ok(None);
            };

            elements.push(element);
        }

        Ok(Some(elements))
    }

    /// Return the source name of one generic parameter.
    fn generic_parameter_name_by_id(
        &self,
        parameter: dir::GlobalGenericParameterId,
    ) -> Option<dir::StringId> {
        let binding = self.check.generic_parameter(parameter)?;

        match binding.key {
            dir::GenericParameterKey::Symbol(symbol) => self.symbol_name(symbol),
            dir::GenericParameterKey::Generated(name) => Some(name),
        }
    }

    /// Allocate one keyword type literal at the anchor span.
    pub(super) fn insert_keyword(
        &mut self,
        keyword: dir::TypeLiteral,
    ) -> dir::LocalNodeId<dir::TypeExpression> {
        self.insert(Self::literal(keyword))
    }

    /// Return the source name of one generic parameter binding.
    pub(super) fn generic_parameter_name(
        &self,
        binding: &dir::GenericParameterBinding,
    ) -> Option<dir::StringId> {
        match binding.key {
            dir::GenericParameterKey::Symbol(symbol) => self.symbol_name(symbol),
            dir::GenericParameterKey::Generated(name) => Some(name),
        }
    }

    /// Allocate one synthesized node at the anchor span.
    pub(in crate::sema) fn insert<T>(&mut self, node: T) -> dir::LocalNodeId<T>
    where
        T: dir::Node,
        dir::Tree: dir::TreeStore<T>,
    {
        self.tree.insert(node, self.span)
    }

    /// Reify one symbol as a value expression.
    pub(in crate::sema) fn reify_symbol_expression(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<dir::LocalNodeId<dir::Expression>> {
        let name = self.symbol_name(symbol)?;
        Some(self.insert(dir::Expression::Identifier { name }))
    }

    /// Write one symbol name into the render pool.
    fn symbol_name(&self, symbol: dir::GlobalSymbolId) -> Option<dir::StringId> {
        if let Some(item) = self.check.environment_bound.language.item(symbol) {
            return Some(self.language_item_name(item));
        }

        // print plain name keys as identifiers
        let bindings = self.check.binding_table(symbol.module_id);
        match bindings.get_symbol(symbol.local_id).key {
            Some(dir::StaticKey::Name(name)) => Some(name),
            _ => None,
        }
    }

    /// Write one language item by its source export name.
    fn language_item_name(&self, item: dir::LanguageItem) -> dir::StringId {
        self.strings.intern(item.export_name())
    }

    /// Return one keyword type literal expression.
    fn literal(literal: dir::TypeLiteral) -> dir::TypeExpression {
        dir::TypeExpression::Keyword { value: literal }
    }

    /// Return one bare type reference expression.
    fn reference(name: dir::StringId) -> dir::TypeExpression {
        dir::TypeExpression::Reference {
            path: dir::Path {
                segments: [name].into_iter().collect(),
            },
            generic_arguments: Vec::new(),
        }
    }

    /// Return whether one signature's return annotation may be filled.
    pub(super) fn returns_fillable(signature: &dir::FunctionSignature) -> bool {
        signature.asynchrony == dir::Asynchrony::Sync && !signature.is_generator
    }
}
