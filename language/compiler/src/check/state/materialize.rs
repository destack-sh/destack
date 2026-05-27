use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{
    ConstraintOrigin, FormTerm, FunctionParameter, FunctionTerm, GenericArgument, MappedParameter,
    ShapeMember, Solution, StaticTerm, TermId, TupleElement, TypeLiteralTerm, TypeOperand,
    TypeOperationTerm, TypeTerm, VariableId, VariableKind,
};

use super::CheckState;

impl CheckState<'_> {
    /// Materialize one checked type id as a solved variable.
    pub(in crate::check) fn materialize_type_id(&mut self, id: dir::GlobalTypeId) -> VariableId {
        if let Some(variable) = self.variables.type_by_id.get(&id).copied() {
            return variable;
        }

        let source = self.local_type_source(id.module_id, id.local_id);
        let origin = ConstraintOrigin::Node(source.into_global(id.module_id));
        let variable =
            self.allocate_intermediate_variable(id.module_id, VariableKind::Type, origin);
        self.variables.type_by_id.insert(id, variable);

        let ty = self.local_type(id.module_id, id.local_id);
        let term = self.materialize_type_term(id.module_id, ty, origin);
        let term = self.terms.push(term);
        self.solve_variable(variable, Solution::Type(term));

        variable
    }

    /// Materialize one checked static id as a solved variable.
    pub(in crate::check) fn materialize_static_id(
        &mut self,
        origin: ConstraintOrigin,
        id: dir::GlobalStaticId,
    ) -> VariableId {
        if let Some(variable) = self.variables.static_by_id.get(&id).copied() {
            return variable;
        }

        let variable =
            self.allocate_intermediate_variable(id.module_id, VariableKind::Static, origin);
        self.variables.static_by_id.insert(id, variable);

        let term = self.local_static(id.module_id, id.local_id);
        let term = self.terms.push(StaticTerm::Literal(term));
        self.solve_variable(variable, Solution::Static(term));

        variable
    }

    /// Materialize one visible type as a solver term.
    fn materialize_type_term(
        &mut self,
        module: destack_source::ModuleId,
        ty: dir::Type,
        origin: ConstraintOrigin,
    ) -> TypeTerm {
        match ty {
            dir::Type::Named(named) => TypeTerm::Reference {
                source: None,
                symbol: named.symbol,
                arguments: self.materialize_static_arguments(module, origin, named.arguments),
            },
            dir::Type::Form(form) => TypeTerm::Form {
                form: self.materialize_form(module, origin, form.form),
                payload: self
                    .materialize_type_id(form.value.into_global(module))
                    .into(),
            },
            dir::Type::Predicate(predicate) => TypeTerm::Predicate {
                asserts: predicate.asserts,
                subject: predicate.subject,
                target: predicate
                    .target
                    .map(|target| self.materialize_type_id(target.into_global(module))),
            },
            dir::Type::Operation(operation) => self.materialize_type_operation(module, operation),
            dir::Type::FixedArray(array) => TypeTerm::FixedArray {
                element: self
                    .materialize_type_id(array.element.into_global(module))
                    .into(),
                length: self.materialize_static_id(origin, array.count.into_global(module)),
                is_readonly: array.is_readonly,
            },
            dir::Type::Slice(slice) => TypeTerm::Slice {
                element: self
                    .materialize_type_id(slice.element.into_global(module))
                    .into(),
                is_readonly: slice.is_readonly,
            },
            dir::Type::Tuple(tuple) => TypeTerm::Tuple {
                form: tuple.form,
                elements: self.materialize_tuple_elements(module, tuple.elements),
                is_readonly: tuple.is_readonly,
            },
            dir::Type::Shape(shape) => TypeTerm::Shape {
                members: self.materialize_shape_members(module, shape),
            },
            dir::Type::Function(function) => {
                let function = self.materialize_function(module, function);

                TypeTerm::Function(function)
            }
            dir::Type::Dynamic(erased) => TypeTerm::Dynamic {
                constraint: self.materialize_type_id(erased.constraint.into_global(module)),
            },
            dir::Type::Closure(closure) => TypeTerm::Closure {
                function: self.materialize_type_id(closure.function.into_global(module)),
                environment: self.materialize_type_id(closure.environment.into_global(module)),
            },
            dir::Type::Union(union) => TypeTerm::Union {
                elements: union
                    .elements
                    .into_iter()
                    .map(|ty| self.materialize_type_id(ty.into_global(module)))
                    .map(TypeOperand::from)
                    .collect(),
            },
            dir::Type::Intersection(intersection) => TypeTerm::Intersection {
                elements: intersection
                    .elements
                    .into_iter()
                    .map(|ty| self.materialize_type_id(ty.into_global(module)))
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
            dir::Type::Void => TypeTerm::Literal(TypeLiteralTerm::Void),
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
        module: destack_source::ModuleId,
        origin: ConstraintOrigin,
        form: dir::Form,
    ) -> TermId<FormTerm> {
        let form = match form {
            dir::Form::Borrowed { lifetime, access } => FormTerm::Borrowed {
                lifetime: self
                    .materialize_static_id(origin, lifetime.into_global(module))
                    .into(),
                access: self
                    .materialize_static_id(origin, access.into_global(module))
                    .into(),
            },
            dir::Form::Placed { place } => FormTerm::Placed {
                place: self
                    .materialize_static_id(origin, place.into_global(module))
                    .into(),
            },
            dir::Form::Managed => FormTerm::Managed,
            dir::Form::Owned => FormTerm::Owned,
            dir::Form::Raw => FormTerm::Raw,
            dir::Form::Readonly => FormTerm::Readonly,
        };

        self.terms.push(form)
    }

    /// Materialize static arguments.
    fn materialize_static_arguments(
        &mut self,
        module: destack_source::ModuleId,
        origin: ConstraintOrigin,
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
        module: destack_source::ModuleId,
        origin: ConstraintOrigin,
        argument: dir::StaticArgument,
    ) -> GenericArgument {
        let value = self.local_static(module, argument.value);
        let argument = match value {
            // type argument
            dir::StaticTerm::Type { ty } => {
                GenericArgument::Type(self.materialize_type_id(ty.into_global(module)).into())
            }
            // static argument
            _ => GenericArgument::Static(
                self.materialize_static_id(origin, argument.value.into_global(module))
                    .into(),
            ),
        };

        argument
    }

    /// Materialize tuple elements.
    fn materialize_tuple_elements(
        &mut self,
        module: destack_source::ModuleId,
        elements: Vec<dir::TypeElement>,
    ) -> SmallVec<[TupleElement; 4]> {
        elements
            .into_iter()
            .map(|element| {
                let element = TupleElement {
                    label: element.label,
                    ty: self
                        .materialize_type_id(element.ty.into_global(module))
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
        module: destack_source::ModuleId,
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
                    .materialize_type_id(field.ty.into_global(module))
                    .into(),
                is_optional: field.is_optional,
                is_readonly: field.is_readonly,
            };

            member
        }));

        members.extend(shape.call_signatures.into_iter().map(|ty| {
            let member = ShapeMember::CallSignature {
                ty: self.materialize_type_id(ty.into_global(module)).into(),
            };

            member
        }));

        members.extend(shape.construct_signatures.into_iter().map(|ty| {
            let member = ShapeMember::ConstructSignature {
                ty: self.materialize_type_id(ty.into_global(module)).into(),
            };

            member
        }));

        members.extend(shape.index_signatures.into_iter().map(|signature| {
            let member = ShapeMember::IndexSignature {
                name: signature.name,
                key_type: self
                    .materialize_type_id(signature.key_type.into_global(module))
                    .into(),
                value_type: self
                    .materialize_type_id(signature.value_type.into_global(module))
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
        module: destack_source::ModuleId,
        function: dir::FunctionType,
    ) -> TermId<FunctionTerm> {
        let parameters = function
            .parameters
            .into_iter()
            .map(|parameter| {
                let ty = self.materialize_type_id(parameter.ty.into_global(module));

                FunctionParameter {
                    ty: ty.into(),
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
                .map(|ty| self.materialize_type_id(ty.into_global(module)))
                .collect(),
            this_parameter: function
                .this_parameter
                .map(|ty| self.materialize_type_id(ty.into_global(module))),
            parameters,
            return_type: function
                .return_type
                .map(|ty| self.materialize_type_id(ty.into_global(module))),
            is_generator: function.is_generator,
        };

        self.terms.push(function)
    }

    /// Materialize one type operation.
    fn materialize_type_operation(
        &mut self,
        module: destack_source::ModuleId,
        operation: dir::TypeOperation,
    ) -> TypeTerm {
        let operation = match operation {
            dir::TypeOperation::Conditional(conditional) => TypeOperationTerm::Conditional {
                left: self.materialize_type_id(conditional.left.into_global(module)),
                right: self.materialize_type_id(conditional.right.into_global(module)),
                then_type: self.materialize_type_id(conditional.then_type.into_global(module)),
                else_type: self.materialize_type_id(conditional.else_type.into_global(module)),
            },
            dir::TypeOperation::Mapped(mapped) => {
                let constraint =
                    self.materialize_type_id(mapped.parameter.constraint.into_global(module));
                let key_remap = mapped
                    .parameter
                    .key_remap
                    .map(|ty| self.materialize_type_id(ty.into_global(module)));
                let parameter = MappedParameter {
                    name: mapped.parameter.name,
                    symbol: mapped.parameter.symbol,
                    constraint,
                    key_remap,
                };
                let value = self.materialize_type_id(mapped.value.into_global(module));

                TypeOperationTerm::Mapped {
                    parameter,
                    modifiers: mapped.modifiers,
                    value,
                }
            }
            dir::TypeOperation::Index(index) => TypeOperationTerm::Index {
                left: self.materialize_type_id(index.left.into_global(module)),
                index: self.materialize_type_id(index.index.into_global(module)),
            },
            dir::TypeOperation::TemplateLiteral(template) => TypeOperationTerm::TemplateLiteral {
                strings: template.strings,
                spans: template
                    .spans
                    .into_iter()
                    .map(|ty| self.materialize_type_id(ty.into_global(module)))
                    .collect(),
            },
            dir::TypeOperation::Infer(infer) => TypeOperationTerm::Infer {
                name: infer.name,
                constraint: infer
                    .constraint
                    .map(|ty| self.materialize_type_id(ty.into_global(module))),
            },
            dir::TypeOperation::KeyOf(key) => TypeOperationTerm::KeyOf {
                target: self.materialize_type_id(key.target.into_global(module)),
            },
            dir::TypeOperation::BuiltinTypeFunction(function) => {
                return TypeTerm::Literal(TypeLiteralTerm::BuiltinTypeFunction(function));
            }
        };

        let operation = self.terms.push(operation);

        TypeTerm::Operation(operation)
    }
}
