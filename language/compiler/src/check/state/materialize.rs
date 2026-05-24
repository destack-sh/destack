use destack_dir as dir;

use crate::check::{
    ArgumentTerm, FormTerm, FunctionTerm, MappedParameterTerm, ShapeMemberTerm, Solution,
    StaticTerm, TupleElementTerm, TypeLiteralTerm, TypeOperationTerm, TypeTerm, VariableId,
    VariableKind, VariableOrigin,
};

use super::CheckModuleState;

impl CheckModuleState {
    /// Materialize one visible type id as a solved variable.
    pub(in crate::check) fn materialize_type_id(&mut self, id: dir::GlobalTypeId) -> VariableId {
        if let Some(variable) = self.work.variables.type_by_id.get(&id).copied() {
            return variable;
        }

        let variable = self.push_variable(VariableKind::Type, VariableOrigin::generated());
        self.work.variables.type_by_id.insert(id, variable);

        let ty = self.get_type(id.local_id);
        let term = self.materialize_type(ty);
        self.solve_variable(variable, Solution::Type(term));

        variable
    }

    /// Materialize one visible static id as a solved variable.
    pub(in crate::check) fn materialize_static_id(
        &mut self,
        id: dir::GlobalStaticId,
    ) -> VariableId {
        if let Some(variable) = self.work.variables.static_by_id.get(&id).copied() {
            return variable;
        }

        let variable = self.push_variable(VariableKind::Static, VariableOrigin::generated());
        self.work.variables.static_by_id.insert(id, variable);

        let term = self.get_static(id.local_id);
        self.solve_variable(variable, Solution::Static(StaticTerm::Literal(term)));

        variable
    }

    /// Materialize one visible type as a solver term.
    fn materialize_type(&mut self, ty: dir::Type) -> TypeTerm {
        match ty {
            dir::Type::Named(named) => TypeTerm::Reference {
                source: None,
                symbol: named.symbol,
                arguments: self.materialize_static_arguments(named.arguments),
            },
            dir::Type::Form(form) => TypeTerm::Form {
                form: self.materialize_form(form.form),
                payload: self.materialize_type_id(form.value.into_global(self.input.module)),
            },
            dir::Type::Predicate(predicate) => TypeTerm::Predicate {
                asserts: predicate.asserts,
                subject: predicate.subject,
                target: predicate
                    .target
                    .map(|target| self.materialize_type_id(target.into_global(self.input.module))),
            },
            dir::Type::Operation(operation) => self.materialize_type_operation(operation),
            dir::Type::FixedArray(array) => TypeTerm::FixedArray {
                element: self.materialize_type_id(array.element.into_global(self.input.module)),
                length: self.materialize_static_id(array.count.into_global(self.input.module)),
                is_readonly: array.is_readonly,
            },
            dir::Type::Slice(slice) => TypeTerm::Slice {
                element: self.materialize_type_id(slice.element.into_global(self.input.module)),
                is_readonly: slice.is_readonly,
            },
            dir::Type::Tuple(tuple) => TypeTerm::Tuple {
                form: tuple.form,
                elements: self.materialize_tuple_elements(tuple.elements),
                is_readonly: tuple.is_readonly,
            },
            dir::Type::Shape(shape) => TypeTerm::Shape {
                members: self.materialize_shape_members(shape),
            },
            dir::Type::Function(function) => {
                TypeTerm::Function(self.materialize_function(function))
            }
            dir::Type::Dynamic(erased) => TypeTerm::Dynamic {
                constraint: self
                    .materialize_type_id(erased.constraint.into_global(self.input.module)),
            },
            dir::Type::Closure(closure) => TypeTerm::Closure {
                function: self.materialize_type_id(closure.function.into_global(self.input.module)),
                environment: self
                    .materialize_type_id(closure.environment.into_global(self.input.module)),
            },
            dir::Type::Union(union) => TypeTerm::Union {
                elements: union
                    .elements
                    .into_iter()
                    .map(|ty| self.materialize_type_id(ty.into_global(self.input.module)))
                    .collect(),
            },
            dir::Type::Intersection(intersection) => TypeTerm::Intersection {
                elements: intersection
                    .elements
                    .into_iter()
                    .map(|ty| self.materialize_type_id(ty.into_global(self.input.module)))
                    .collect(),
            },
            dir::Type::Range(range) => TypeTerm::Range {
                start: range.start,
                end: range.end,
                is_inclusive: range.is_inclusive,
            },
            dir::Type::Parameter(parameter) => TypeTerm::Parameter {
                symbol: parameter.symbol,
            },
            dir::Type::This => TypeTerm::This,
            ty => {
                let Some(literal) = TypeLiteralTerm::from_type(&ty) else {
                    unreachable!("recursive DIR type must materialize as a solver term")
                };

                TypeTerm::Literal(literal)
            }
        }
    }

    /// Materialize one memory form.
    fn materialize_form(&mut self, form: dir::Form) -> FormTerm {
        match form {
            dir::Form::Borrowed { lifetime, access } => FormTerm::Borrowed {
                lifetime: self.materialize_static_id(lifetime.into_global(self.input.module)),
                access: self.materialize_static_id(access.into_global(self.input.module)),
            },
            dir::Form::Placed { place } => FormTerm::Placed {
                place: self.materialize_static_id(place.into_global(self.input.module)),
            },
            dir::Form::Managed => FormTerm::Managed,
            dir::Form::Owned => FormTerm::Owned,
            dir::Form::Raw => FormTerm::Raw,
            dir::Form::Readonly => FormTerm::Readonly,
        }
    }

    /// Materialize static arguments.
    fn materialize_static_arguments(
        &mut self,
        arguments: Vec<dir::StaticArgument>,
    ) -> Vec<ArgumentTerm> {
        arguments
            .into_iter()
            .map(|argument| self.materialize_static_argument(argument))
            .collect()
    }

    /// Materialize one static argument.
    fn materialize_static_argument(&mut self, argument: dir::StaticArgument) -> ArgumentTerm {
        let value = self.get_static(argument.value);

        match value {
            dir::StaticTerm::Type { ty } => {
                ArgumentTerm::Type(self.materialize_type_id(ty.into_global(self.input.module)))
            }
            _ => ArgumentTerm::Static(
                self.materialize_static_id(argument.value.into_global(self.input.module)),
            ),
        }
    }

    /// Materialize tuple elements.
    fn materialize_tuple_elements(
        &mut self,
        elements: Vec<dir::TypeElement>,
    ) -> Vec<TupleElementTerm> {
        elements
            .into_iter()
            .map(|element| TupleElementTerm {
                label: element.label,
                ty: self.materialize_type_id(element.ty.into_global(self.input.module)),
                is_optional: element.is_optional,
                is_readonly: element.is_readonly,
                is_rest: element.is_rest,
            })
            .collect()
    }

    /// Materialize shape members.
    fn materialize_shape_members(&mut self, shape: dir::ShapeType) -> Vec<ShapeMemberTerm> {
        let field_count = shape.fields.len();
        let call_count = shape.call_signatures.len();
        let construct_count = shape.construct_signatures.len();
        let index_count = shape.index_signatures.len();
        let capacity = field_count + call_count + construct_count + index_count;
        let mut members = Vec::with_capacity(capacity);

        members.extend(
            shape
                .fields
                .into_iter()
                .map(|field| ShapeMemberTerm::Field {
                    key: field.key,
                    ty: self.materialize_type_id(field.ty.into_global(self.input.module)),
                    is_optional: field.is_optional,
                    is_readonly: field.is_readonly,
                }),
        );

        members.extend(shape.call_signatures.into_iter().map(|ty| {
            ShapeMemberTerm::CallSignature {
                ty: self.materialize_type_id(ty.into_global(self.input.module)),
            }
        }));

        members.extend(shape.construct_signatures.into_iter().map(|ty| {
            ShapeMemberTerm::ConstructSignature {
                ty: self.materialize_type_id(ty.into_global(self.input.module)),
            }
        }));

        members.extend(shape.index_signatures.into_iter().map(|signature| {
            ShapeMemberTerm::IndexSignature {
                name: signature.name,
                key_type: self
                    .materialize_type_id(signature.key_type.into_global(self.input.module)),
                value_type: self
                    .materialize_type_id(signature.value_type.into_global(self.input.module)),
                is_optional: signature.is_optional,
                is_readonly: signature.is_readonly,
            }
        }));

        members
    }

    /// Materialize one function type.
    fn materialize_function(&mut self, function: dir::FunctionType) -> FunctionTerm {
        FunctionTerm {
            asynchrony: function.asynchrony,
            generic_parameters: function
                .generic_parameters
                .into_iter()
                .map(|ty| self.materialize_type_id(ty.into_global(self.input.module)))
                .collect(),
            this_parameter: function
                .this_parameter
                .map(|ty| self.materialize_type_id(ty.into_global(self.input.module))),
            parameters: function
                .parameters
                .into_iter()
                .map(|ty| self.materialize_type_id(ty.into_global(self.input.module)))
                .collect(),
            return_type: function
                .return_type
                .map(|ty| self.materialize_type_id(ty.into_global(self.input.module))),
            is_generator: function.is_generator,
        }
    }

    /// Materialize one type operation.
    fn materialize_type_operation(&mut self, operation: dir::TypeOperation) -> TypeTerm {
        let operation = match operation {
            dir::TypeOperation::Conditional(conditional) => TypeOperationTerm::Conditional {
                left: self.materialize_type_id(conditional.left.into_global(self.input.module)),
                right: self.materialize_type_id(conditional.right.into_global(self.input.module)),
                then_type: self
                    .materialize_type_id(conditional.then_type.into_global(self.input.module)),
                else_type: self
                    .materialize_type_id(conditional.else_type.into_global(self.input.module)),
            },
            dir::TypeOperation::Mapped(mapped) => TypeOperationTerm::Mapped {
                parameter: MappedParameterTerm {
                    name: mapped.parameter.name,
                    symbol: mapped.parameter.symbol,
                    constraint: self.materialize_type_id(
                        mapped.parameter.constraint.into_global(self.input.module),
                    ),
                    key_remap: mapped
                        .parameter
                        .key_remap
                        .map(|ty| self.materialize_type_id(ty.into_global(self.input.module))),
                },
                modifiers: mapped.modifiers,
                value: self.materialize_type_id(mapped.value.into_global(self.input.module)),
            },
            dir::TypeOperation::Index(index) => TypeOperationTerm::Index {
                left: self.materialize_type_id(index.left.into_global(self.input.module)),
                index: self.materialize_type_id(index.index.into_global(self.input.module)),
            },
            dir::TypeOperation::TemplateLiteral(template) => TypeOperationTerm::TemplateLiteral {
                strings: template.strings,
                spans: template
                    .spans
                    .into_iter()
                    .map(|ty| self.materialize_type_id(ty.into_global(self.input.module)))
                    .collect(),
            },
            dir::TypeOperation::Infer(infer) => TypeOperationTerm::Infer {
                name: infer.name,
                constraint: infer
                    .constraint
                    .map(|ty| self.materialize_type_id(ty.into_global(self.input.module))),
            },
            dir::TypeOperation::KeyOf(key) => TypeOperationTerm::KeyOf {
                target: self.materialize_type_id(key.target.into_global(self.input.module)),
            },
            dir::TypeOperation::BuiltinTypeFunction(function) => {
                return TypeTerm::Literal(TypeLiteralTerm::BuiltinTypeFunction(function));
            }
        };

        TypeTerm::Operation(operation)
    }
}
