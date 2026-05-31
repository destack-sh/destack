use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    FormTerm, FunctionParameter, FunctionTerm, GenericArgument, MappedParameter, MemberTerm,
    Origin, ShapeMember, Solution, StaticOperand, StaticTerm, TermId, TupleElement,
    TypeLiteralTerm, TypeOperand, TypeOperationTerm, TypeTerm, VariableId, VariableKind,
};

use super::CheckState;

impl CheckState<'_> {
    /// Materialize one checked type id as a solved variable.
    pub(in crate::check) fn materialize_type_by_id(&mut self, id: dir::GlobalTypeId) -> VariableId {
        if let Some(variable) = self.inference.variables.type_by_id.get(&id).copied() {
            return variable;
        }

        let source = self.visible_type_source(id);
        let origin = Origin::Node(source.into_global(id.module_id));
        let variable = self.allocate_variable(id.module_id, VariableKind::Type, origin);
        self.inference.variables.type_by_id.insert(id, variable);

        let ty = self.visible_type(id);
        let term = self.materialize_type_term(id.module_id, ty, origin);
        let term = self.inference.terms.push(term);
        self.insert_known_solution(variable, Solution::Type(term));

        variable
    }

    /// Materialize one checked static id as a solved variable.
    pub(in crate::check) fn materialize_static_by_id(
        &mut self,
        origin: Origin,
        id: dir::GlobalStaticId,
    ) -> VariableId {
        if let Some(variable) = self.inference.variables.static_by_id.get(&id).copied() {
            return variable;
        }

        let variable = self.allocate_variable(id.module_id, VariableKind::Static, origin);
        self.inference.variables.static_by_id.insert(id, variable);

        let term = self.visible_static(id);
        let term = self.materialize_static_term(id.module_id, origin, term);
        let term = self.inference.terms.push(term);
        self.insert_known_solution(variable, Solution::Static(term));

        variable
    }

    /// Materialize one visible static value as a solver term.
    fn materialize_static_term(
        &mut self,
        module: ModuleId,
        origin: Origin,
        term: dir::StaticTerm,
    ) -> StaticTerm {
        match term {
            dir::StaticTerm::Union { elements } => StaticTerm::Union {
                elements: elements
                    .into_iter()
                    .map(|element| {
                        self.materialize_static_by_id(origin, element.into_global(module))
                    })
                    .map(StaticOperand::from)
                    .collect(),
            },
            term => StaticTerm::Literal(term),
        }
    }

    /// Materialize one visible type as a solver term.
    fn materialize_type_term(
        &mut self,
        module: ModuleId,
        ty: dir::Type,
        origin: Origin,
    ) -> TypeTerm {
        match ty {
            dir::Type::Reference(named) => TypeTerm::Reference {
                origin: Origin::Symbol(named.symbol),
                symbol: named.symbol,
                arguments: self.materialize_static_arguments(module, origin, named.arguments),
            },
            dir::Type::Member(member) => {
                let owner = self
                    .materialize_type_by_id(member.owner.into_global(module))
                    .into();
                let arguments = self.materialize_static_arguments(module, origin, member.arguments);
                let member = MemberTerm {
                    origin,
                    owner,
                    key: member.key,
                    arguments,
                };
                let member = self.inference.terms.push(member);

                TypeTerm::Member(member)
            }
            dir::Type::Form(form) => TypeTerm::Form {
                form: self.materialize_form(module, origin, form.form),
                payload: self
                    .materialize_type_by_id(form.value.into_global(module))
                    .into(),
            },
            dir::Type::Predicate(predicate) => TypeTerm::Predicate {
                asserts: predicate.asserts,
                subject: predicate.subject,
                target: predicate.target.map(|target| {
                    self.materialize_type_by_id(target.into_global(module))
                        .into()
                }),
            },
            dir::Type::Operation(operation) => self.materialize_type_operation(module, operation),
            dir::Type::Array(array) => TypeTerm::Array {
                element: self
                    .materialize_type_by_id(array.element.into_global(module))
                    .into(),
            },
            dir::Type::FixedArray(array) => TypeTerm::FixedArray {
                element: self
                    .materialize_type_by_id(array.element.into_global(module))
                    .into(),
                length: self
                    .materialize_static_by_id(origin, array.count.into_global(module))
                    .into(),
            },
            dir::Type::Slice(slice) => TypeTerm::Slice {
                element: self
                    .materialize_type_by_id(slice.element.into_global(module))
                    .into(),
            },
            dir::Type::Tuple(tuple) => TypeTerm::Tuple {
                form: tuple.form,
                elements: self.materialize_tuple_elements(module, tuple.elements),
            },
            dir::Type::Shape(shape) => TypeTerm::Shape {
                members: self.materialize_shape_members(module, shape),
            },
            dir::Type::Function(function) => {
                let function = self.materialize_function(module, function);

                TypeTerm::Function(function)
            }
            dir::Type::Dynamic(erased) => TypeTerm::Dynamic {
                constraint: self
                    .materialize_type_by_id(erased.constraint.into_global(module))
                    .into(),
            },
            dir::Type::Closure(closure) => TypeTerm::Closure {
                function: self
                    .materialize_type_by_id(closure.function.into_global(module))
                    .into(),
                environment: self
                    .materialize_type_by_id(closure.environment.into_global(module))
                    .into(),
            },
            dir::Type::Union(union) => TypeTerm::Union {
                elements: union
                    .elements
                    .into_iter()
                    .map(|ty| self.materialize_type_by_id(ty.into_global(module)))
                    .map(TypeOperand::from)
                    .collect(),
            },
            dir::Type::Intersection(intersection) => TypeTerm::Intersection {
                elements: intersection
                    .elements
                    .into_iter()
                    .map(|ty| self.materialize_type_by_id(ty.into_global(module)))
                    .map(TypeOperand::from)
                    .collect(),
            },
            dir::Type::Range(range) => TypeTerm::Range {
                start: range.start,
                end: range.end,
                is_inclusive: range.is_inclusive,
            },
            dir::Type::Parameter(parameter) => TypeTerm::Parameter(parameter.into()),
            dir::Type::This => TypeTerm::This,
            dir::Type::Error => TypeTerm::Literal(TypeLiteralTerm::Error),
            dir::Type::Never => TypeTerm::Literal(TypeLiteralTerm::Never),
            dir::Type::Any => TypeTerm::Literal(TypeLiteralTerm::Any),
            dir::Type::Unknown => TypeTerm::Literal(TypeLiteralTerm::Unknown),
            dir::Type::Void => TypeTerm::unit(),
            dir::Type::Null => TypeTerm::Literal(TypeLiteralTerm::Null),
            dir::Type::Undefined => TypeTerm::Literal(TypeLiteralTerm::Undefined),
            dir::Type::Object => TypeTerm::Literal(TypeLiteralTerm::Object),
            dir::Type::Primitive(primitive) => {
                TypeTerm::Literal(TypeLiteralTerm::Primitive(primitive))
            }
            dir::Type::Literal(literal) => TypeTerm::Literal(TypeLiteralTerm::Scalar(literal)),
        }
    }

    /// Materialize one memory form.
    fn materialize_form(
        &mut self,
        module: ModuleId,
        origin: Origin,
        form: dir::Form,
    ) -> TermId<FormTerm> {
        let form = match form {
            dir::Form::Borrowed { lifetime, access } => FormTerm::Borrowed {
                lifetime: self
                    .materialize_static_by_id(origin, lifetime.into_global(module))
                    .into(),
                access: self
                    .materialize_static_by_id(origin, access.into_global(module))
                    .into(),
            },
            dir::Form::Placed { place } => FormTerm::Placed {
                place: self
                    .materialize_static_by_id(origin, place.into_global(module))
                    .into(),
            },
            dir::Form::Managed => FormTerm::Managed,
            dir::Form::Owned => FormTerm::Owned,
            dir::Form::Raw => FormTerm::Raw,
            dir::Form::Readonly => FormTerm::Readonly,
        };

        self.inference.terms.push(form)
    }

    /// Materialize static arguments.
    fn materialize_static_arguments(
        &mut self,
        module: ModuleId,
        origin: Origin,
        arguments: Vec<dir::StaticArgument>,
    ) -> SmallVec<[GenericArgument; 4]> {
        arguments
            .into_iter()
            .map(|argument| self.materialize_static_argument(module, origin, argument))
            .collect()
    }

    /// Materialize one static argument.
    fn materialize_static_argument(
        &mut self,
        module: ModuleId,
        origin: Origin,
        argument: dir::StaticArgument,
    ) -> GenericArgument {
        let value = self.visible_static(argument.value.into_global(module));
        let argument = match value {
            // type argument
            dir::StaticTerm::Type { ty } => {
                GenericArgument::Type(self.materialize_type_by_id(ty.into_global(module)).into())
            }
            // static argument
            _ => GenericArgument::Static(
                self.materialize_static_by_id(origin, argument.value.into_global(module))
                    .into(),
            ),
        };

        argument
    }

    /// Materialize tuple elements.
    fn materialize_tuple_elements(
        &mut self,
        module: ModuleId,
        elements: Vec<dir::TypeElement>,
    ) -> SmallVec<[TupleElement; 4]> {
        elements
            .into_iter()
            .map(|element| {
                let element = TupleElement {
                    label: element.label,
                    ty: self
                        .materialize_type_by_id(element.ty.into_global(module))
                        .into(),
                    is_optional: element.is_optional,
                    is_readonly: element.is_readonly,
                    is_rest: element.is_rest,
                };

                element
            })
            .collect()
    }

    /// Materialize shape members.
    fn materialize_shape_members(
        &mut self,
        module: ModuleId,
        shape: dir::ShapeType,
    ) -> SmallVec<[ShapeMember; 8]> {
        let field_count = shape.fields.len();
        let call_count = shape.call_signatures.len();
        let construct_count = shape.construct_signatures.len();
        let index_count = shape.index_signatures.len();
        let capacity = field_count + call_count + construct_count + index_count;
        let mut members = SmallVec::with_capacity(capacity);

        members.extend(shape.fields.into_iter().map(|field| {
            let member = ShapeMember::Field {
                key: field.key,
                ty: self
                    .materialize_type_by_id(field.ty.into_global(module))
                    .into(),
                is_optional: field.is_optional,
                is_readonly: field.is_readonly,
            };

            member
        }));

        members.extend(shape.call_signatures.into_iter().map(|ty| {
            let member = ShapeMember::CallSignature {
                ty: self.materialize_type_by_id(ty.into_global(module)).into(),
            };

            member
        }));

        members.extend(shape.construct_signatures.into_iter().map(|ty| {
            let member = ShapeMember::ConstructSignature {
                ty: self.materialize_type_by_id(ty.into_global(module)).into(),
            };

            member
        }));

        members.extend(shape.index_signatures.into_iter().map(|signature| {
            let member = ShapeMember::IndexSignature {
                name: signature.name,
                key_type: self
                    .materialize_type_by_id(signature.key_type.into_global(module))
                    .into(),
                value_type: self
                    .materialize_type_by_id(signature.value_type.into_global(module))
                    .into(),
                is_optional: signature.is_optional,
                is_readonly: signature.is_readonly,
            };

            member
        }));

        members
    }

    /// Materialize one function type.
    fn materialize_function(
        &mut self,
        module: ModuleId,
        function: dir::FunctionTypeShape,
    ) -> TermId<FunctionTerm> {
        let parameters = function
            .parameters
            .into_iter()
            .map(|parameter| {
                let ty = self.materialize_type_by_id(parameter.ty.into_global(module));
                let static_slot = parameter
                    .static_slot
                    .and_then(|slot| slot.symbol())
                    .and_then(|symbol| self.generic_static_variable_for_symbol(module, symbol));

                FunctionParameter {
                    ty: ty.into(),
                    static_slot,
                    is_optional: parameter.is_optional,
                    is_rest: parameter.is_rest,
                }
            })
            .collect();

        let function = FunctionTerm {
            asynchrony: function.asynchrony,
            generic_parameters: function
                .generic_parameters
                .into_iter()
                .map(|ty| self.materialize_type_by_id(ty.into_global(module)))
                .collect(),
            this_parameter: function
                .this_parameter
                .map(|ty| self.materialize_type_by_id(ty.into_global(module)).into()),
            parameters,
            return_type: function
                .return_type
                .map(|ty| self.materialize_type_by_id(ty.into_global(module)).into()),
            is_generator: function.is_generator,
        };

        self.inference.terms.push(function)
    }

    /// Materialize one type operation.
    fn materialize_type_operation(
        &mut self,
        module: ModuleId,
        operation: dir::TypeOperation,
    ) -> TypeTerm {
        let operation = match operation {
            dir::TypeOperation::Conditional(conditional) => TypeOperationTerm::Conditional {
                left: self
                    .materialize_type_by_id(conditional.left.into_global(module))
                    .into(),
                right: self
                    .materialize_type_by_id(conditional.right.into_global(module))
                    .into(),
                then_type: self
                    .materialize_type_by_id(conditional.then_type.into_global(module))
                    .into(),
                else_type: self
                    .materialize_type_by_id(conditional.else_type.into_global(module))
                    .into(),
            },
            dir::TypeOperation::Mapped(mapped) => {
                let constraint =
                    self.materialize_type_by_id(mapped.parameter.constraint.into_global(module));
                let key_remap = mapped
                    .parameter
                    .key_remap
                    .map(|ty| self.materialize_type_by_id(ty.into_global(module)).into());
                let parameter = MappedParameter {
                    name: mapped.parameter.name,
                    symbol: mapped.parameter.symbol,
                    constraint: constraint.into(),
                    key_remap,
                };
                let value = self
                    .materialize_type_by_id(mapped.value.into_global(module))
                    .into();

                TypeOperationTerm::Mapped {
                    parameter,
                    modifiers: mapped.modifiers,
                    value,
                }
            }
            dir::TypeOperation::Index(index) => TypeOperationTerm::Index {
                left: self
                    .materialize_type_by_id(index.left.into_global(module))
                    .into(),
                index: self
                    .materialize_type_by_id(index.index.into_global(module))
                    .into(),
            },
            dir::TypeOperation::TemplateLiteral(template) => TypeOperationTerm::TemplateLiteral {
                strings: template.strings,
                spans: template
                    .spans
                    .into_iter()
                    .map(|ty| self.materialize_type_by_id(ty.into_global(module)).into())
                    .collect(),
            },
            dir::TypeOperation::Infer(infer) => TypeOperationTerm::Infer {
                name: infer.name,
                constraint: infer
                    .constraint
                    .map(|ty| self.materialize_type_by_id(ty.into_global(module)).into()),
            },
            dir::TypeOperation::KeyOf(key) => TypeOperationTerm::KeyOf {
                target: self
                    .materialize_type_by_id(key.target.into_global(module))
                    .into(),
            },
            dir::TypeOperation::StringMapping { mapping, target } => {
                TypeOperationTerm::StringMapping {
                    mapping,
                    argument: self
                        .materialize_type_by_id(target.into_global(module))
                        .into(),
                }
            }
        };

        let operation = self.inference.terms.push(operation);

        TypeTerm::Operation(operation)
    }

    /// Return a visible checked type source.
    fn visible_type_source(&self, id: dir::GlobalTypeId) -> dir::LocalNodeIdAny {
        if let Some(module) = self.modules.get(&id.module_id) {
            return module.type_table().get_type_source(id.local_id);
        }

        self.dependency(id.module_id)
            .types
            .get_type_source(id.local_id)
    }

    /// Return a visible checked type.
    fn visible_type(&self, id: dir::GlobalTypeId) -> dir::Type {
        if let Some(module) = self.modules.get(&id.module_id) {
            return module.type_table().get_type(id.local_id).clone();
        }

        self.dependency(id.module_id)
            .types
            .get_type(id.local_id)
            .clone()
    }

    /// Return a visible checked static value.
    fn visible_static(&self, id: dir::GlobalStaticId) -> dir::StaticTerm {
        if let Some(module) = self.modules.get(&id.module_id) {
            return module.static_table().get_static(id.local_id).clone();
        }

        self.dependency(id.module_id)
            .statics
            .get_static(id.local_id)
            .clone()
    }
}
