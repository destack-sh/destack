use destack_core::StringPool;
use destack_dir as dir;
use destack_source::{FileId, NodeSpanRegion, NodeSpanType, Span};

use crate::CompilerResult;
use crate::check::CheckState;

/// Nesting depth bound guarding reified annotation spellings.
const REIFY_DEPTH: usize = 32;

/// Synthesizes type and static expression nodes from solved check types.
/// Every reified node lands in the amended output tree; types without
/// a faithful source spelling leave their annotation slot empty.
pub(in crate::check) struct TypeReifier<'a, 'b> {
    /// The solved component state read for type structure.
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
    pub(in crate::check) fn new(
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
    pub(in crate::check) fn anchor(&mut self, span: Span) {
        self.span = Span::new(span.file, span.end, span.end);
    }

    /// Return the current synthesized-node span.
    pub(in crate::check) fn span(&self) -> Span {
        self.span
    }

    /// Reify one solved type into a synthesized type expression.
    /// Returns None when the type has no faithful source spelling.
    pub(in crate::check) fn reify(
        &mut self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::LocalNodeId<dir::TypeExpression>>> {
        self.reify_depth(id, REIFY_DEPTH)
    }

    /// Reify one parameter type, omitting undefined when `?` already carries it.
    pub(in crate::check) fn reify_parameter_type(
        &mut self,
        id: dir::GlobalTypeId,
        is_optional: bool,
    ) -> CompilerResult<Option<dir::LocalNodeId<dir::TypeExpression>>> {
        self.reify_parameter_type_depth(id, is_optional, REIFY_DEPTH)
    }

    /// Reify selected generic argument bindings into synthesized generic arguments.
    pub(in crate::check) fn reify_generic_argument_bindings(
        &mut self,
        bindings: &[dir::GenericArgumentBinding],
    ) -> CompilerResult<Option<Vec<dir::LocalNodeId<dir::GenericArgument>>>> {
        self.reify_generic_argument_bindings_depth(bindings, REIFY_DEPTH)
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
        let parameter = if binding.is_comptime {
            self.reify_value_parameter(binding, name)?
        } else {
            self.reify_type_parameter(binding, name)?
        };
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

    /// Reify one value generic parameter binding.
    fn reify_value_parameter(
        &mut self,
        binding: &dir::GenericParameterBinding,
        name: dir::StringId,
    ) -> CompilerResult<dir::GenericParameter> {
        let declared_type = binding
            .constraint
            .map(|constraint| self.reify(constraint))
            .transpose()?
            .flatten();
        let default = binding
            .default
            .map(|default| self.reify_static(default))
            .transpose()?
            .flatten();

        let parameter = if binding.is_variadic {
            dir::GenericParameter::VariadicValue {
                name,
                declared_type,
                default,
                is_comptime: true,
            }
        } else {
            dir::GenericParameter::Value {
                name,
                declared_type,
                default,
                is_comptime: true,
            }
        };

        Ok(parameter)
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
    pub(in crate::check) fn reify_static(
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
        let id = self.check.settled_root(id)?;
        let ty = self.check.ty(id)?.clone();
        let next = depth - 1;

        let expression = match &ty {
            // open variables and errors have no honest spelling
            dir::Type::Variable(_) | dir::Type::Error => return Ok(None),

            dir::Type::Never => Self::literal(dir::TypeLiteral::Never),
            dir::Type::Any => Self::literal(dir::TypeLiteral::Any),
            dir::Type::Unknown => Self::literal(dir::TypeLiteral::Unknown),
            dir::Type::Void => Self::literal(dir::TypeLiteral::Void),
            dir::Type::Null => Self::literal(dir::TypeLiteral::Null),
            dir::Type::Undefined => Self::literal(dir::TypeLiteral::Undefined),
            dir::Type::Object => Self::literal(dir::TypeLiteral::Object),
            dir::Type::Intrinsic => dir::TypeExpression::Intrinsic,
            dir::Type::This => dir::TypeExpression::This,

            dir::Type::Primitive(primitive) => Self::literal(dir::TypeLiteral::from(*primitive)),
            dir::Type::Literal(literal) => dir::TypeExpression::ScalarLiteral { value: *literal },
            dir::Type::Key(key) => {
                let Some(value) = Self::static_key_literal(key) else {
                    return Ok(None);
                };

                dir::TypeExpression::ScalarLiteral { value }
            }
            dir::Type::Memory(literal) => dir::TypeExpression::ScalarLiteral {
                value: dir::ScalarLiteral::String(self.strings.intern(literal.text())),
            },
            dir::Type::Static(value) => match self.check.r#static(*value) {
                dir::StaticTerm::ScalarLiteral { value } => {
                    dir::TypeExpression::ScalarLiteral { value: *value }
                }
                _ => return Ok(None),
            },
            dir::Type::Range(range) => {
                let start = range
                    .start
                    .map(|value| self.insert(dir::TypeExpression::ScalarLiteral { value }));
                let end = range
                    .end
                    .map(|value| self.insert(dir::TypeExpression::ScalarLiteral { value }));
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
                let Some(name) = self.generic_parameter_name_by_id(*parameter) else {
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
            dir::Type::Instance(instance) => {
                let Some(name) = self.symbol_name(instance.symbol) else {
                    return Ok(None);
                };
                let template = self.check.symbol_template(instance.symbol);
                let Some(arguments) =
                    self.reify_template_arguments_depth(template, &instance.arguments, next)?
                else {
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
                let dir::StaticKey::Name(key) = member.key else {
                    return Ok(None);
                };
                let Some(left) = self.reify_depth(member.owner, next)? else {
                    return Ok(None);
                };
                // member types do not carry the selected declaration symbol,
                // so their generic arguments keep the type-argument form
                let Some(arguments) = self.reify_arguments(&member.arguments, next)? else {
                    return Ok(None);
                };

                dir::TypeExpression::Member {
                    left,
                    name: self.strings.intern(&self.check_text(key)),
                    generic_arguments: arguments,
                }
            }
            dir::Type::EnumMember(_) => return Ok(None),

            dir::Type::Array(array) => {
                let Some(element) = self.reify_depth(array.element, next)? else {
                    return Ok(None);
                };

                dir::TypeExpression::Array { element }
            }
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
                let count = self.check.settled_root(array.count)?;
                let dir::Type::Literal(value) = self.check.ty(count)? else {
                    return Ok(None);
                };
                let length = self.insert(dir::Expression::ScalarLiteral(*value));

                dir::TypeExpression::FixedArray { element, length }
            }
            dir::Type::Tuple(tuple) => {
                let mut elements = Vec::with_capacity(tuple.elements.len());
                for element in &tuple.elements {
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

                match tuple.form {
                    dir::TupleForm::Tuple => dir::TypeExpression::Tuple { elements },
                    dir::TupleForm::Array => dir::TypeExpression::ArrayTuple { elements },
                }
            }
            dir::Type::Shape(shape) => {
                // signatures have no member spelling here yet
                if !shape.call_signatures.is_empty()
                    || !shape.construct_signatures.is_empty()
                    || !shape.index_signatures.is_empty()
                {
                    return Ok(None);
                }

                let mut members = Vec::with_capacity(shape.fields.len());
                for field in &shape.fields {
                    let key = match field.key {
                        dir::StaticKey::Name(name) => dir::Key::Name(dir::Name::Identifier(
                            self.strings.intern(&self.check_text(name)),
                        )),
                        dir::StaticKey::Index(index) => dir::Key::Name(dir::Name::Index(index)),
                        dir::StaticKey::Symbol(_) => return Ok(None),
                    };
                    let Some(declared_type) = self.reify_depth(field.ty, next)? else {
                        return Ok(None);
                    };
                    let member = dir::TypeMember::Field {
                        key,
                        declared_type: Some(declared_type),
                        is_static: false,
                        is_optional: field.is_optional,
                        is_readonly: field.is_readonly,
                    };

                    members.push(self.insert(member));
                }

                dir::TypeExpression::Object { members }
            }
            dir::Type::FunctionSignature(function) => {
                let Some(function) = self.reify_function(function, next)? else {
                    return Ok(None);
                };

                dir::TypeExpression::Function(function)
            }
            dir::Type::Function(function) => {
                return self.reify_depth(function.signature, next);
            }
            dir::Type::FunctionPointer(function) => {
                let Some(expression) = self.reify_function_pointer(function, next)? else {
                    return Ok(None);
                };

                expression
            }

            dir::Type::Union(union) => {
                let Some(elements) = self.reify_elements(&union.elements, next)? else {
                    return Ok(None);
                };

                dir::TypeExpression::Union { elements }
            }
            dir::Type::Intersection(intersection) => {
                let Some(elements) = self.reify_elements(&intersection.elements, next)? else {
                    return Ok(None);
                };

                dir::TypeExpression::Intersection { elements }
            }

            dir::Type::Form(form) => {
                // the managed default reads transparently as its payload
                if form.form == dir::Form::Managed {
                    return self.reify_depth(form.value, next);
                }
                let Some(target_type) = self.reify_depth(form.value, next)? else {
                    return Ok(None);
                };

                match &form.form {
                    dir::Form::Managed => unreachable!("managed forms reify transparently"),
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
                    dir::Form::Borrowed { lifetime, access } => {
                        let Some(lifetime) = self.reify_static_depth(*lifetime, next)? else {
                            return Ok(None);
                        };
                        let Some(access) = self.reify_static_depth(*access, next)? else {
                            return Ok(None);
                        };
                        let target_type =
                            self.insert(dir::GenericArgument::Type { value: target_type });
                        let lifetime = self.insert(dir::GenericArgument::Value { value: lifetime });
                        let access = self.insert(dir::GenericArgument::Value { value: access });
                        let name = self.language_item_name(dir::LanguageItem::Borrowed);

                        dir::TypeExpression::Reference {
                            path: dir::Path {
                                segments: [name].into_iter().collect(),
                            },
                            generic_arguments: vec![target_type, lifetime, access],
                        }
                    }
                    dir::Form::Placed { place } => {
                        let place = match self.check.ty(self.check.settled_root(*place)?)? {
                            dir::Type::Memory(dir::MemoryLiteral::Place(dir::Place::Space(
                                space,
                            ))) => *space,
                            _ => return Ok(None),
                        };

                        match place {
                            dir::Space::Local => dir::TypeExpression::Local { target_type },
                            dir::Space::Shared => dir::TypeExpression::Shared { target_type },
                            dir::Space::Static | dir::Space::Frame => return Ok(None),
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
                        segments: [self.strings.intern("Dynamic")].into_iter().collect(),
                    },
                    generic_arguments: vec![argument],
                }
            }

            dir::Type::Operation(operation) => {
                let Some(expression) = self.reify_operation(operation, next)? else {
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

        let id = self.check.settled_root(id)?;
        let dir::Type::Union(union) = self.check.ty(id)? else {
            return self.reify_depth(id, depth);
        };

        let mut elements = Vec::new();
        for element in &union.elements {
            let element = self.check.settled_root(*element)?;
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
                name: infer
                    .name
                    .map(|name| self.strings.intern(&self.check_text(name))),
                constraint: None,
            },
            // the remaining operations have no faithful annotation spelling
            dir::TypeOperation::StringMapping { .. }
            | dir::TypeOperation::Narrow(_)
            | dir::TypeOperation::TypeOf(_)
            | dir::TypeOperation::Mapped(_)
            | dir::TypeOperation::TemplateLiteral(_)
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
        function: &dir::FunctionSignatureType,
        depth: usize,
    ) -> CompilerResult<Option<dir::FunctionTypeExpression>> {
        // async, generator, and generic signatures have no annotation spelling
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
                    is_comptime: false,
                };

                Some(self.insert(parameter))
            }
            None => None,
        };

        let mut parameters = Vec::with_capacity(function.parameters.len());
        for (index, parameter) in function.parameters.iter().enumerate() {
            let Some(declared_type) =
                self.reify_parameter_type_depth(parameter.ty, parameter.is_optional, depth)?
            else {
                return Ok(None);
            };
            let name = match parameter.static_parameter {
                Some(parameter) => match self.generic_parameter_name_by_id(parameter) {
                    Some(name) => name,
                    None => return Ok(None),
                },
                None => self.strings.intern(&format!("arg{index}")),
            };
            let is_comptime = parameter.static_parameter.is_some();
            let parameter = if parameter.is_rest {
                dir::Parameter::VariadicNamed {
                    name,
                    declared_type: Some(declared_type),
                    is_comptime,
                }
            } else {
                dir::Parameter::Named {
                    name,
                    declared_type: Some(declared_type),
                    default: None,
                    is_optional: parameter.is_optional,
                    is_comptime,
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
        let signature = self.check.settled_root(function.signature)?;
        let dir::Type::FunctionSignature(signature) = self.check.ty(signature)?.clone() else {
            return Ok(None);
        };
        let Some(parameters) =
            self.reify_function_pointer_parameters(&signature.parameters, depth)?
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
        let expression = dir::TypeExpression::Tuple { elements };
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

    /// Reify solved arguments against their declaration parameter forms.
    fn reify_template_arguments_depth(
        &mut self,
        template: Option<dir::GlobalGenericTemplateId>,
        ids: &[dir::GlobalTypeId],
        depth: usize,
    ) -> CompilerResult<Option<Vec<dir::LocalNodeId<dir::GenericArgument>>>> {
        let parameters = template
            .map(|template| self.check.generic_template_parameters(template))
            .unwrap_or_default();
        let mut arguments = Vec::with_capacity(ids.len());
        for (index, id) in ids.iter().copied().enumerate() {
            let parameter = parameters
                .get(index)
                .and_then(|parameter| self.check.generic_parameter(*parameter));
            let argument = if parameter.is_some_and(|parameter| parameter.is_comptime) {
                let Some(value) = self.reify_static_depth(id, depth)? else {
                    return Ok(None);
                };

                dir::GenericArgument::Value { value }
            } else {
                let Some(value) = self.reify_depth(id, depth)? else {
                    return Ok(None);
                };

                dir::GenericArgument::Type { value }
            };

            arguments.push(self.insert(argument));
        }

        Ok(Some(arguments))
    }

    /// Reify selected generic argument bindings up to a nesting depth.
    fn reify_generic_argument_bindings_depth(
        &mut self,
        bindings: &[dir::GenericArgumentBinding],
        depth: usize,
    ) -> CompilerResult<Option<Vec<dir::LocalNodeId<dir::GenericArgument>>>> {
        let mut arguments = Vec::with_capacity(bindings.len());
        for binding in bindings {
            let Some(parameter) = self.check.generic_parameter(binding.parameter) else {
                return Ok(None);
            };
            let argument = if parameter.is_comptime {
                let Some(value) = self.reify_static_depth(binding.argument, depth)? else {
                    return Ok(None);
                };

                dir::GenericArgument::Value { value }
            } else {
                let Some(value) = self.reify_depth(binding.argument, depth)? else {
                    return Ok(None);
                };

                dir::GenericArgument::Type { value }
            };

            arguments.push(self.insert(argument));
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
        let id = self.check.settled_root(id)?;

        let expression = match self.check.ty(id)? {
            dir::Type::Literal(value) => dir::Expression::ScalarLiteral(*value),
            dir::Type::Key(key) => {
                let Some(value) = Self::static_key_literal(key) else {
                    return Ok(None);
                };

                dir::Expression::ScalarLiteral(value)
            }
            dir::Type::Memory(literal) => dir::Expression::ScalarLiteral(
                dir::ScalarLiteral::String(self.strings.intern(literal.text())),
            ),
            dir::Type::Static(value) => match self.check.r#static(*value) {
                dir::StaticTerm::ScalarLiteral { value } => dir::Expression::ScalarLiteral(*value),
                _ => return Ok(None),
            },
            dir::Type::Union(union) => {
                let elements = union.elements.iter().copied().collect::<Vec<_>>();
                let Some(expression) = self.reify_static_union(&elements, depth - 1)? else {
                    return Ok(None);
                };

                return Ok(Some(expression));
            }
            dir::Type::Parameter(parameter) => {
                let Some(name) = self.generic_parameter_name_by_id(*parameter) else {
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

    /// Return the scalar literal spelling of one exact key type.
    fn static_key_literal(key: &dir::StaticKey) -> Option<dir::ScalarLiteral> {
        match key {
            dir::StaticKey::Name(name) => Some(dir::ScalarLiteral::String(*name)),
            dir::StaticKey::Index(index) => {
                let index = i64::try_from(*index).ok()?;

                Some(dir::ScalarLiteral::Integer(index))
            }
            dir::StaticKey::Symbol(_) => None,
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
            dir::GenericParameterKey::Generated(name) => {
                Some(self.strings.intern(&self.check_text(name)))
            }
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
            dir::GenericParameterKey::Generated(name) => {
                Some(self.strings.intern(&self.check_text(name)))
            }
        }
    }

    /// Allocate one synthesized node at the anchor span.
    pub(in crate::check) fn insert<T>(&mut self, node: T) -> dir::LocalNodeId<T>
    where
        T: dir::Node,
        dir::Tree: dir::TreeStore<T>,
    {
        self.tree.insert(node, self.span)
    }

    /// Spell one symbol name into the render pool.
    fn symbol_name(&self, symbol: dir::GlobalSymbolId) -> Option<dir::StringId> {
        if let Some(item) = self.check.environment.language.item(symbol) {
            return Some(self.language_item_name(item));
        }

        let name = self.check.format_symbol(symbol);
        if name == "<anonymous>" || name == "[symbol]" {
            return None;
        }

        Some(self.strings.intern(&name))
    }

    /// Spell one language item by its source export name.
    fn language_item_name(&self, item: dir::LanguageItem) -> dir::StringId {
        self.strings.intern(item.export_name())
    }

    /// Resolve one interned string through the component pools.
    fn check_text(&self, id: dir::StringId) -> String {
        self.check.format_static_key(&dir::StaticKey::Name(id))
    }

    /// Return one keyword type literal expression.
    fn literal(literal: dir::TypeLiteral) -> dir::TypeExpression {
        dir::TypeExpression::Literal { value: literal }
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
    ///
    /// Async and generator returns spell carrier types and wait for
    /// their reified carrier wiring.
    pub(super) fn returns_fillable(signature: &dir::FunctionSignature) -> bool {
        signature.asynchrony == dir::Asynchrony::Sync && !signature.is_generator
    }
}
