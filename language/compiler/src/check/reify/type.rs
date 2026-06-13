use destack_core::StringPool;
use destack_dir as dir;
use destack_source::{FileId, Span};

use crate::CompilerResult;
use crate::check::CheckState;

/// Nesting depth bound guarding reified annotation spellings.
const REIFY_DEPTH: usize = 32;

/// Reifies solved working types into synthesized type expressions.
/// Every reified node lands in the amended output tree; types without
/// a faithful source spelling leave their annotation slot empty.
pub(in crate::check) struct Reifier<'a, 'b> {
    /// The solved component state read for type structure.
    check: &'a CheckState<'b>,
    /// The amended output tree receiving synthesized nodes.
    pub(super) tree: dir::Tree,
    /// The string pool shared with the rendered module.
    strings: &'a StringPool,
    /// The anchor span stamped on synthesized nodes.
    span: Span,
}

impl<'a, 'b> Reifier<'a, 'b> {
    /// Create one reifier amending one cloned module tree.
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

    /// Reify one solved type into a synthesized type expression.
    /// Returns None when the type has no faithful source spelling.
    pub(in crate::check) fn reify(
        &mut self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::LocalNodeId<dir::TypeExpression>>> {
        self.reify_depth(id, REIFY_DEPTH)
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
        let id = self.check.resolve_root(id)?;
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
                let Some(binding) = self.check.generic_parameter(*parameter) else {
                    return Ok(None);
                };
                let name = match binding.key {
                    dir::GenericParameterKey::Symbol(symbol) => {
                        let Some(name) = self.symbol_name(symbol) else {
                            return Ok(None);
                        };

                        name
                    }
                    dir::GenericParameterKey::Generated(name) => {
                        self.strings.intern(&self.check_text(name))
                    }
                };

                Self::reference(name)
            }
            dir::Type::Reference(instance) => {
                let Some(name) = self.symbol_name(instance.symbol) else {
                    return Ok(None);
                };
                let Some(arguments) = self.reify_arguments(&instance.arguments, next)? else {
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
                let Some(arguments) = self.reify_arguments(&member.arguments, next)? else {
                    return Ok(None);
                };

                dir::TypeExpression::Member {
                    left,
                    name: self.strings.intern(&self.check_text(key)),
                    generic_arguments: arguments,
                }
            }

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
                let count = self.check.resolve_root(array.count)?;
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
            dir::Type::Function(function) => {
                let Some(function) = self.reify_function(function, next)? else {
                    return Ok(None);
                };

                dir::TypeExpression::Function(function)
            }
            dir::Type::Closure(closure) => {
                return self.reify_depth(closure.function, next);
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
                    dir::Form::Borrowed { access, .. } => {
                        let mutability = match self.check.ty(self.check.resolve_root(*access)?)? {
                            dir::Type::Memory(dir::MemoryLiteral::Access(
                                dir::Access::Readonly,
                            )) => Some(dir::Mutability::Immutable),
                            dir::Type::Memory(dir::MemoryLiteral::Access(
                                dir::Access::Exclusive,
                            )) => Some(dir::Mutability::Exclusive),
                            _ => None,
                        };

                        dir::TypeExpression::BorrowedOf {
                            mutability,
                            variance: None,
                            target_type,
                        }
                    }
                    dir::Form::Placed { place } => {
                        let place = match self.check.ty(self.check.resolve_root(*place)?)? {
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
        function: &dir::FunctionType,
        depth: usize,
    ) -> CompilerResult<Option<dir::FunctionTypeExpression>> {
        // async, generator, and generic contracts have no annotation spelling
        if function.asynchrony != dir::Asynchrony::Sync
            || function.is_generator
            || !function.generic_parameters.is_empty()
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
            let Some(declared_type) = self.reify_depth(parameter.ty, depth)? else {
                return Ok(None);
            };
            let name = self.strings.intern(&format!("p{index}"));
            let parameter = if parameter.is_rest {
                dir::Parameter::VariadicNamed {
                    name,
                    declared_type: Some(declared_type),
                    is_comptime: false,
                }
            } else {
                dir::Parameter::Named {
                    name,
                    declared_type: Some(declared_type),
                    default: None,
                    is_optional: parameter.is_optional,
                    is_comptime: false,
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

    /// Allocate one synthesized node at the anchor span.
    fn insert<T>(&mut self, node: T) -> dir::LocalNodeId<T>
    where
        T: dir::Node,
        dir::Tree: dir::TreeStore<T>,
    {
        self.tree.insert(node, self.span)
    }

    /// Spell one symbol name into the render pool.
    fn symbol_name(&self, symbol: dir::GlobalSymbolId) -> Option<dir::StringId> {
        let name = self.check.format_symbol(symbol);
        if name == "<anonymous>" || name == "[symbol]" {
            return None;
        }

        Some(self.strings.intern(&name))
    }

    /// Resolve one interned string through the component pools.
    fn check_text(&self, id: dir::StringId) -> String {
        self.check.format_static_key(&dir::StaticKey::Name(id))
    }

    /// Build one keyword type literal expression.
    fn literal(literal: dir::TypeLiteral) -> dir::TypeExpression {
        dir::TypeExpression::Literal { value: literal }
    }

    /// Build one bare type reference expression.
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
