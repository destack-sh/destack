use destack_core::StringPool;
use destack_dir as dir;
use destack_source::{FileId, ModuleId, NodeSpanRegion, NodeSpanType, Span};

use crate::CompilerResult;
use crate::sema::CheckState;

/// Nesting depth bound guarding reified annotations.
const REIFY_DEPTH: usize = 32;

/// Synthesizes type and static expression nodes from solved check types.
pub(super) struct TypeReifier<'a, 'b> {
    /// The solved module state read for type structure.
    pub(super) check: &'a mut CheckState<'b>,
    /// The amended output tree receiving synthesized nodes.
    pub(super) tree: dir::Tree,
    /// The string pool shared with the formatted module.
    strings: &'a StringPool,
    /// The anchor span stamped on synthesized nodes.
    span: Span,
}

impl<'a, 'b> TypeReifier<'a, 'b> {
    /// Create a type reifier for one cloned module tree.
    pub(super) fn new(
        check: &'a mut CheckState<'b>,
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
    pub(super) fn anchor(&mut self, span: Span) {
        self.span = Span::new(span.file, span.end, span.end);
    }

    /// Return the current synthesized-node span.
    pub(super) fn span(&self) -> Span {
        self.span
    }

    /// Reify one solved type into a synthesized type expression.
    pub(super) fn reify(
        &mut self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::LocalNodeId<dir::TypeExpression>>> {
        self.reify_depth(id, REIFY_DEPTH)
    }

    /// Reify one parameter type, omitting undefined when `?` already implies it.
    pub(super) fn reify_parameter_type(
        &mut self,
        id: dir::GlobalTypeId,
        is_optional: bool,
    ) -> CompilerResult<Option<dir::LocalNodeId<dir::TypeExpression>>> {
        self.reify_parameter_type_depth(id, is_optional, REIFY_DEPTH)
    }

    /// Reify selected generic argument bindings into synthesized generic arguments.
    pub(super) fn reify_generic_argument_bindings(
        &mut self,
        bindings: &[dir::GenericArgumentBinding],
    ) -> CompilerResult<Option<Vec<dir::LocalNodeId<dir::GenericArgument>>>> {
        if bindings.is_empty() {
            return Ok(None);
        }
        let arguments = dir::GenericArgumentBinding::values(bindings).collect::<Vec<_>>();

        self.reify_arguments(&arguments, REIFY_DEPTH)
    }

    /// Reify one generic parameter binding into a synthesized parameter node.
    pub(super) fn reify_generic_parameter(
        &mut self,
        parameter: dir::GlobalGenericParameterId,
        binding: &dir::GenericParameterBinding,
    ) -> CompilerResult<Option<dir::LocalNodeId<dir::GenericParameter>>> {
        let Some(name) = self.generic_parameter_name(parameter)? else {
            return Ok(None);
        };

        // reify tick names as bare lifetime parameters
        if binding.memory_parameter() == Some(dir::MemoryParameter::Region)
            && self.check.strings().get(name).starts_with('\'')
        {
            let parameter = self.insert(dir::GenericParameter::Lifetime { name });

            return Ok(Some(parameter));
        }

        // reify the type parameter and stamp a side span over its constraint
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

    /// Reify one closed borrow into its named borrow expression.
    fn reify_borrowed_of(
        &mut self,
        lifetime: dir::GlobalTypeId,
        access: dir::GlobalTypeId,
        target_type: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<Option<dir::TypeExpression>> {
        // require a closed access literal for the borrow modifier
        let Some(access) = self.check.access_of(access)? else {
            return Ok(None);
        };

        // name the tick parameter or reserved lifetime literal
        let name = match self.check.resolved_ty(lifetime)? {
            // keep a region elided inside a function type annotation elided
            dir::Type::Parameter(parameter) if self.is_elided_in_annotation(parameter)? => {
                return Ok(Some(dir::TypeExpression::BorrowedOf {
                    lifetime: None,
                    access: Some(access),
                    variance: None,
                    target_type,
                }));
            }
            // read a tick parameter back through its binding
            dir::Type::Parameter(parameter) => {
                let Some(name) = self.generic_parameter_name(parameter)? else {
                    return Ok(None);
                };
                if !self.check.strings().get(name).starts_with('\'') {
                    return Ok(None);
                }

                name
            }
            // write a reserved lifetime literal back with its tick
            dir::Type::Literal(dir::Literal::String(value)) => {
                let Some(lifetime) = dir::Lifetime::parse(self.check.strings().get(value)) else {
                    return Ok(None);
                };

                self.check
                    .strings()
                    .intern(&format!("'{}", lifetime.text()))
            }
            // skip every other region head
            _ => return Ok(None),
        };

        let lifetime = self.insert(dir::TypeExpression::Lifetime { name });

        Ok(Some(dir::TypeExpression::BorrowedOf {
            lifetime: Some(lifetime),
            access: Some(access),
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
        // reify the written constraint and default, when each reifies
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

        // a variadic binding reifies in its pack form
        let parameter = if binding.is_variadic {
            dir::GenericParameter::VariadicType {
                name,
                variance: binding.variance,
                constraint,
                default,
                is_const: binding.is_const,
            }
        }
        // otherwise reify a single type parameter
        else {
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
    pub(super) fn reify_static(
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

        // read the solved head and the budget its children reify under
        let id = self.check.shallow_resolve(id)?;
        let ty = self.check.ty(id)?;
        let next = depth - 1;

        let expression = match ty {
            // skip open variables, holes, rigid and erased parameters, and errors
            dir::Type::Variable(_) | dir::Type::Erased(_) | dir::Type::Error => {
                return Ok(None);
            }
            // refinements print as their base application
            dir::Type::Refined(refined) => {
                let refined = self.check.type_refined(id.module_id, refined)?;

                return self.reify_depth(refined.base, depth);
            }

            // keyword heads reify as their own keywords
            dir::Type::Never => Self::literal(dir::TypeLiteral::Never),
            dir::Type::Unknown => Self::literal(dir::TypeLiteral::Unknown),
            dir::Type::Void => Self::literal(dir::TypeLiteral::Void),
            dir::Type::Null => Self::literal(dir::TypeLiteral::Null),
            dir::Type::Undefined => Self::literal(dir::TypeLiteral::Undefined),
            dir::Type::Intrinsic => dir::TypeExpression::Intrinsic,
            dir::Type::This => dir::TypeExpression::This,

            // scalar domains and their exact values reify by value
            dir::Type::Primitive(primitive) => Self::literal(dir::TypeLiteral::from(primitive)),
            dir::Type::Literal(literal) => dir::TypeExpression::Literal { value: literal },
            // a region reads back as its extent, its space following the referent
            dir::Type::Region(region) => return self.reify_depth(region.extent, next),
            // exact property keys reify as their scalar literal
            dir::Type::Key(key) => {
                let Some(value) = Self::static_key_literal(&key) else {
                    return Ok(None);
                };

                dir::TypeExpression::Literal { value }
            }
            // a static singleton reifies through its literal term
            dir::Type::Static(value) => match self.check.r#static(value)? {
                dir::StaticTerm::Literal { value } => {
                    dir::TypeExpression::Literal { value: *value }
                }
                _ => return Ok(None),
            },
            // intervals reify with their written end form
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

            // parameters and declaration references reify by their source name
            dir::Type::Parameter(parameter) => {
                let Some(name) = self.generic_parameter_name(parameter)? else {
                    return Ok(None);
                };

                Self::reference(name)
            }
            dir::Type::Reference(reference) => {
                let Some(name) = self.symbol_name(reference.symbol)? else {
                    return Ok(None);
                };

                if self.check.symbol_kind(reference.symbol)? == dir::SymbolKind::Class {
                    let mut value = self.insert(dir::Expression::Identifier { name });
                    if !reference.arguments.is_empty() {
                        let arguments = self
                            .check
                            .type_ids(id.module_id, reference.arguments)?
                            .to_vec();
                        let Some(generic_arguments) = self.reify_arguments(&arguments, next)?
                        else {
                            return Ok(None);
                        };
                        value = self.insert(dir::Expression::Instantiation {
                            left: value,
                            generic_arguments,
                        });
                    }

                    dir::TypeExpression::TypeOf { value }
                } else {
                    Self::reference(name)
                }
            }
            // array applications reify in their written rest form
            dir::Type::Application(_) if let Some(element) = self.check.array_element(id)? => {
                let Some(element) = self.reify_depth(element, next)? else {
                    return Ok(None);
                };

                return Ok(Some(self.insert(dir::TypeExpression::Array { element })));
            }
            // every other nominal instance reifies as a named reference with arguments
            dir::Type::Application(instance) => {
                let Some(name) = self.symbol_name(instance.symbol)? else {
                    return Ok(None);
                };

                let arguments = self
                    .check
                    .type_ids(id.module_id, instance.arguments)?
                    .to_vec();
                let arguments = self.trim_default_arguments(instance.symbol, arguments)?;
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
            // member projections reify as a qualified member access
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
            // skip precise variants, which have no written annotation form
            dir::Type::Variant(_) => return Ok(None),

            // slices reify over their element
            dir::Type::Slice(slice) => {
                let Some(element) = self.reify_depth(slice.element, next)? else {
                    return Ok(None);
                };

                dir::TypeExpression::Slice { element }
            }
            // fixed arrays reify over their element and closed length
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
            // tuples reify element by element, keeping labels and modifiers
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

                    // a rest element reifies in its spread form
                    let element = if element.is_rest {
                        dir::TupleElement::Spread {
                            label: element.label,
                            value,
                        }
                    }
                    // otherwise reify a positional element
                    else {
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
            // object types reify as their written field set
            dir::Type::Object(shape) => {
                // TODO #Incomplete: reify index and call signatures as object members
                if shape.declares_signatures() {
                    return Ok(None);
                }

                let object_properties = self
                    .check
                    .object_properties(id.module_id, shape.properties)?
                    .to_vec();
                let mut members = Vec::with_capacity(object_properties.len());
                for field in &object_properties {
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
                        visibility: None,
                        is_static: false,
                        is_optional: field.is_optional,
                        is_readonly: !field.access.is_writable(),
                    };

                    members.push(self.insert(member));
                }

                dir::TypeExpression::Object { members }
            }
            // signatures reify as a function or constructor type expression
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
            // a closure type reifies through its signature
            dir::Type::Function(function) => {
                return self.reify_depth(function.signature, next);
            }
            // a function pointer reifies through its intrinsic reference
            dir::Type::FunctionPointer(function) => {
                let Some(expression) = self.reify_function_pointer(&function, next)? else {
                    return Ok(None);
                };

                expression
            }

            // set constructors reify over their elements
            dir::Type::Union(union) => {
                let union_elements = self.check.type_ids(id.module_id, union.elements)?;
                let Some(elements) = self.reify_elements(union_elements, next)? else {
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

            // memory forms reify through their written modifier
            dir::Type::Form(form) => {
                let Some(target_type) = self.reify_depth(form.value, next)? else {
                    return Ok(None);
                };

                match &form.form {
                    // the bare forms each have their own modifier
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
                    // borrows reify through their region and access
                    dir::Form::Borrowed(borrow) => {
                        let borrow = self.check.type_borrow(id.module_id, *borrow)?;
                        let (region, access) = (borrow.region, borrow.access);
                        let region = self.check.shallow_resolve(region)?;
                        let (lifetime, spaces) = match self.check.ty(region)? {
                            dir::Type::Region(pair) => (pair.extent, Some(pair.space)),
                            _ => (region, None),
                        };

                        // read the referent space the pair closes over
                        let (space, is_induced_space) = match spaces {
                            Some(spaces) => {
                                let place = self.check.shallow_resolve(spaces)?;
                                let is_induced = match self.check.ty(place)? {
                                    dir::Type::Parameter(parameter) => {
                                        self.check.generic_parameter(parameter)?.is_some_and(
                                            |binding| binding.induced_memory_parameter().is_some(),
                                        )
                                    }
                                    _ => false,
                                };

                                (self.check.literal_space(place)?, is_induced)
                            }
                            None => (None, false),
                        };

                        // elide a closed, induced, or absent space, the referent naming it
                        let sugared = match space {
                            Some(_) => Some(target_type),
                            None if is_induced_space || spaces.is_none() => Some(target_type),
                            None => None,
                        };

                        // reify closed borrows through the borrow form
                        if let Some(sugared) = sugared
                            && let Some(borrowed) =
                                self.reify_borrowed_of(lifetime, access, sugared)?
                        {
                            borrowed
                        }
                        // reify parametric borrows through the full form algebra
                        else {
                            let Some(region) = self.reify_depth(region, next)? else {
                                return Ok(None);
                            };
                            let Some(access) = self.reify_depth(access, next)? else {
                                return Ok(None);
                            };
                            let target_type =
                                self.insert(dir::GenericArgument::Type { value: target_type });
                            let region = self.insert(dir::GenericArgument::Type { value: region });
                            let access = self.insert(dir::GenericArgument::Type { value: access });
                            let name = self.language_item_name(dir::LanguageItem::Borrowed);

                            dir::TypeExpression::Reference {
                                path: dir::Path {
                                    segments: [name].into_iter().collect(),
                                },
                                generic_arguments: vec![target_type, region, access],
                            }
                        }
                    }
                }
            }
            // erased values reify through `Dynamic` over their constraint
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

            // type operations reify through their own annotation forms
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

        // only a union carries the undefined arm the `?` already implies
        let id = self.check.shallow_resolve(id)?;
        let dir::Type::Union(union) = self.check.ty(id)? else {
            return self.reify_depth(id, depth);
        };
        let union_elements = self.check.type_ids(id.module_id, union.elements)?;

        // reify every arm apart from undefined
        let mut elements = Vec::new();
        for element in union_elements {
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
            // a bare undefined keeps its full annotation
            [] => self.reify_depth(id, depth),
            // one remaining arm annotates on its own
            [single] => Ok(Some(*single)),
            // several arms annotate as a narrower union
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
            // conditionals reify operands and both branches
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
            // keyof has its own keyword
            dir::TypeOperation::KeyOf(unary) => {
                let Some(target_type) = self.reify_depth(unary.target, depth)? else {
                    return Ok(None);
                };

                dir::TypeExpression::KeyOf { target_type }
            }
            // inference barriers reify through `NoInfer`
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
            // awaited projections reify through `Awaited`
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
            // indexed accesses reify receiver and index
            dir::TypeOperation::Index(index) => {
                let Some(left) = self.reify_depth(index.left, depth)? else {
                    return Ok(None);
                };
                let Some(index) = self.reify_depth(index.index, depth)? else {
                    return Ok(None);
                };

                dir::TypeExpression::Index { left, index }
            }
            // infer binders reify by name alone
            dir::TypeOperation::Infer(infer) => dir::TypeExpression::Infer {
                form: dir::InferForm::Infer,
                name: infer.name,
                constraint: None,
            },
            // template literals reify their strings and spans
            dir::TypeOperation::TemplateLiteral(template) => {
                let strings = self
                    .check
                    .template_strings(module, template.strings)?
                    .to_vec();
                let span_types = self.check.type_ids(module, template.spans)?;

                let mut spans = Vec::with_capacity(span_types.len());
                for span in span_types {
                    let Some(span) = self.reify_depth(*span, depth)? else {
                        return Ok(None);
                    };

                    spans.push(span);
                }

                dir::TypeExpression::TemplateLiteral { strings, spans }
            }
            // skip the remaining operations, which have no faithful annotation form
            dir::TypeOperation::StringMapping { .. }
            | dir::TypeOperation::Narrow(_)
            | dir::TypeOperation::SpaceOf(_)
            | dir::TypeOperation::TypeOf(_)
            | dir::TypeOperation::Instantiation(_)
            | dir::TypeOperation::Mapped(_)
            | dir::TypeOperation::TryOutput { .. }
            | dir::TypeOperation::TryResidual { .. }
            | dir::TypeOperation::TryFailure { .. }
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
        // reject async, generator, and generic signatures, which have no annotation form
        if function.asynchrony != dir::Asynchrony::Sync
            || function.is_generator
            || !self
                .check
                .signature_generic_parameters(module, function)?
                .is_empty()
        {
            return Ok(None);
        }

        // reify the explicit receiver parameter, when one stands there
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

        // retain declared parameter names and name anonymous parameters by position
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

            let name = match parameter.name {
                Some(name) => name,
                None => self.strings.intern(&format!("arg{index}")),
            };
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

        // an absent result annotates as void
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

        // reify the parameters as one tuple
        let signature_parameters = self
            .check
            .signature_parameters(signature_id.module_id, signature.parameters)?
            .to_vec();
        let Some(parameters) =
            self.reify_function_pointer_parameters(&signature_parameters, depth)?
        else {
            return Ok(None);
        };

        // an absent result annotates as void
        let return_type = match signature.return_type {
            Some(return_type) => match self.reify_depth(return_type, depth)? {
                Some(return_type) => return_type,
                None => return Ok(None),
            },
            None => self.insert(Self::literal(dir::TypeLiteral::Void)),
        };

        // carry both sides as generic arguments of the intrinsic reference
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

        // gather the parameters into one tuple annotation
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
            // exact values reify as their own literal
            dir::Type::Literal(value) => dir::Expression::Literal(value),
            dir::Type::Key(key) => {
                let Some(value) = Self::static_key_literal(&key) else {
                    return Ok(None);
                };

                dir::Expression::Literal(value)
            }
            dir::Type::Static(value) => match self.check.r#static(value)? {
                dir::StaticTerm::Literal { value } => dir::Expression::Literal(*value),
                _ => return Ok(None),
            },
            // a static union folds into an elementwise-or chain
            dir::Type::Union(union) => {
                let elements = self.check.type_ids(id.module_id, union.elements)?;
                let Some(expression) = self.reify_static_union(elements, depth - 1)? else {
                    return Ok(None);
                };

                return Ok(Some(expression));
            }
            // a const parameter reifies as its own identifier
            dir::Type::Parameter(parameter) => {
                let Some(name) = self.generic_parameter_name(parameter)? else {
                    return Ok(None);
                };

                dir::Expression::Identifier { name }
            }
            // skip every other head, which has no value form
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

    /// Allocate one keyword type literal at the anchor span.
    pub(super) fn insert_keyword(
        &mut self,
        keyword: dir::TypeLiteral,
    ) -> dir::LocalNodeId<dir::TypeExpression> {
        self.insert(Self::literal(keyword))
    }

    /// Drop the trailing arguments of one application standing at their parameter's default.
    fn trim_default_arguments(
        &self,
        symbol: dir::GlobalSymbolId,
        mut arguments: Vec<dir::GlobalTypeId>,
    ) -> CompilerResult<Vec<dir::GlobalTypeId>> {
        // read the parameters of the symbol's own template
        let Some(template) = self.check.symbol_template(symbol)? else {
            return Ok(arguments);
        };
        let parameters = self.check.generic_template_parameters(template)?;

        // drop each trailing argument that stands at its parameter's default
        while let Some(argument) = arguments.last()
            && let Some(parameter) = parameters.get(arguments.len() - 1)
        {
            let default = self
                .check
                .generic_parameter(*parameter)?
                .and_then(|binding| binding.default);
            if !default
                .is_some_and(|default| self.check.is_same_type(*argument, default).unwrap_or(false))
            {
                break;
            }
            arguments.pop();
        }

        Ok(arguments)
    }

    /// Return whether one generic parameter is a region elided inside a function type annotation.
    fn is_elided_in_annotation(
        &self,
        parameter: dir::GlobalGenericParameterId,
    ) -> CompilerResult<bool> {
        // require an anonymous binding, which elision mints
        let Some(binding) = self.check.generic_parameter(parameter)? else {
            return Ok(false);
        };
        if binding.key != dir::GenericParameterKey::Anonymous {
            return Ok(false);
        }

        // read the template the binding belongs to
        let template = binding.template.into_global(parameter.module_id);
        let Some(template) = self.check.generic_template(template)? else {
            return Ok(false);
        };

        Ok(template.source.local_id.ty == dir::NodeType::TypeExpression)
    }

    /// Return the source name of one generic parameter, an anonymous one by its printed name.
    pub(super) fn generic_parameter_name(
        &self,
        parameter: dir::GlobalGenericParameterId,
    ) -> CompilerResult<Option<dir::StringId>> {
        let Some(binding) = self.check.generic_parameter(parameter)? else {
            return Ok(None);
        };
        match binding.key {
            dir::GenericParameterKey::Symbol(symbol) => self.symbol_name(symbol),
            dir::GenericParameterKey::Anonymous => {
                let name = self.check.format_parameter(parameter);

                Ok(Some(self.check.strings().intern(&name)))
            }
        }
    }

    /// Allocate one synthesized node at the anchor span.
    pub(super) fn insert<T>(&mut self, node: T) -> dir::LocalNodeId<T>
    where
        T: dir::Node,
        dir::Tree: dir::TreeStore<T>,
    {
        self.tree.insert(node, self.span)
    }

    /// Reify one symbol as a value expression.
    pub(super) fn reify_symbol_expression(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::LocalNodeId<dir::Expression>>> {
        let Some(name) = self.symbol_name(symbol)? else {
            return Ok(None);
        };

        Ok(Some(self.insert(dir::Expression::Identifier { name })))
    }

    /// Return the source name of one symbol.
    fn symbol_name(&self, symbol: dir::GlobalSymbolId) -> CompilerResult<Option<dir::StringId>> {
        if let Some(item) = self.check.environment_bound.language.item(symbol) {
            return Ok(Some(self.language_item_name(item)));
        }

        // print plain name keys as identifiers
        let bindings = self.check.binding_table(symbol.module_id)?;
        Ok(match bindings.get_symbol(symbol.local_id).key {
            Some(dir::StaticKey::Name(name)) => Some(name),
            _ => None,
        })
    }

    /// Return the interned source export name of one language item.
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

    /// Return whether one signature's return annotation may be filled, a constructor's staying bare.
    pub(super) fn returns_fillable(signature: &dir::FunctionSignature) -> bool {
        signature.asynchrony == dir::Asynchrony::Sync
            && !signature.is_generator
            && !signature.is_constructor()
    }
}
