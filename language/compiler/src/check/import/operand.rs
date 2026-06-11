use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexSet;
use smallvec::SmallVec;

use crate::check::{
    CheckState, FormTerm, FunctionParameter, FunctionTerm, GenericArgument, MappedParameter,
    MemberReceiver, MemberTerm, Origin, ShapeMember, ShapeTerm, StaticOperand, StaticTerm,
    TupleElement, TypeLiteralTerm, TypeOperand, TypeOperationTerm, TypeTerm,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Import committed symbol type operands from external modules.
    pub(in crate::check) fn import_external_symbol_type_operands(&mut self) -> CompilerResult<()> {
        let modules = self.modules.keys().copied().collect::<Vec<_>>();
        let mut symbols = IndexSet::new();

        // collect committed symbol types named by component imports
        for module in modules {
            for external_module in self.module(module).external_modules.iter().copied() {
                symbols.extend(
                    self.external_module(external_module)
                        .types
                        .symbol_types()
                        .map(|(symbol, _)| (module, symbol)),
                );
            }
        }

        // import every external symbol type once
        for (module, symbol) in symbols {
            self.import_symbol_type_operand(module, symbol)?;
        }

        Ok(())
    }

    /// Import one external committed symbol type operand.
    fn import_symbol_type_operand(
        &mut self,
        requesting_module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<TypeOperand> {
        // reject component symbols
        if self.is_component_module(symbol.module_id) {
            let symbol = self.dump_in_module(requesting_module, &symbol);

            return Err(CompilerError::Internal {
                message: format!("check component symbol {symbol} reached external type import"),
            });
        }

        // validate the resolved module edge
        self.validate_import_module(requesting_module, symbol.module_id)?;
        self.import_symbol_generic_template(requesting_module, symbol)?;

        // return cached imported operand
        if let Some(operand) = self.inputs.symbol_type(symbol) {
            return Ok(operand);
        }

        let external = self.external_module(symbol.module_id);
        let Some(type_id) = external.types.get_symbol_type_id(symbol) else {
            return Err(CompilerError::Internal {
                message: format!("external symbol {symbol:?} has no committed type"),
            });
        };
        let operand = self.import_type_operand(requesting_module, type_id)?;

        self.inputs.insert_symbol_type(symbol, operand)
    }

    /// Return one committed type id as a check type operand.
    pub(in crate::check) fn import_type_operand(
        &mut self,
        module: ModuleId,
        source: dir::GlobalTypeId,
    ) -> CompilerResult<TypeOperand> {
        if let Some(operand) = self.inputs.r#type(source) {
            return Ok(operand);
        }

        // import substitutable types as check terms
        let term = match self.r#type(source) {
            dir::Type::Parameter(parameter) => {
                let parameter = self.import_generic_parameter_id(module, *parameter)?;

                Some(TypeTerm::Parameter(parameter))
            }
            dir::Type::This => Some(TypeTerm::This),
            _ => None,
        };
        if let Some(term) = term {
            let operand = TypeOperand::Term(self.inference.push_term(term));

            self.inputs.upsert_type(source, operand)?;

            return Ok(operand);
        }

        Ok(TypeOperand::Type(source))
    }

    /// Import one committed type id as a check term.
    pub(in crate::check) fn import_type_term(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
    ) -> CompilerResult<TypeTerm> {
        let module = origin.module();
        let ty = self.r#type(source).clone();

        if let Some(literal) = TypeLiteralTerm::from_type(&ty) {
            return Ok(TypeTerm::Literal(literal));
        }

        let term = match ty {
            dir::Type::Error
            | dir::Type::Never
            | dir::Type::Any
            | dir::Type::Unknown
            | dir::Type::Void
            | dir::Type::Null
            | dir::Type::Undefined
            | dir::Type::Object
            | dir::Type::Primitive(_)
            | dir::Type::Literal(_) => {
                return Err(CompilerError::Internal {
                    message: format!("literal type {source:?} reached non-literal import"),
                });
            }
            dir::Type::Intrinsic => TypeTerm::Intrinsic,
            dir::Type::Parameter(parameter) => {
                let parameter = self.import_generic_parameter_id(module, parameter)?;

                TypeTerm::Parameter(parameter)
            }
            dir::Type::Reference(reference) => TypeTerm::Reference {
                origin,
                symbol: reference.symbol,
                arguments: self.import_reference_arguments(module, reference.arguments)?,
            },
            dir::Type::This => TypeTerm::This,
            dir::Type::Member(member) => {
                let member = MemberTerm {
                    origin,
                    receiver: MemberReceiver::Value(
                        self.import_type_operand(module, member.owner)?,
                    ),
                    key: member.key,
                    arguments: self
                        .import_reference_arguments(module, member.arguments)?
                        .into(),
                };

                TypeTerm::Member(self.inference.push_term(member))
            }
            dir::Type::Form(form) => {
                let payload = self.import_type_operand(module, form.value)?;
                let form = self.import_form_term(module, form.form)?;

                TypeTerm::Form {
                    form: self.inference.push_term(form),
                    payload,
                }
            }
            dir::Type::Dynamic(dynamic) => TypeTerm::Dynamic {
                constraint: self.import_type_operand(module, dynamic.constraint)?,
            },
            dir::Type::Operation(operation) => {
                let operation = self.import_type_operation(module, operation)?;

                TypeTerm::Operation(self.inference.push_term(operation))
            }
            dir::Type::Array(array) => TypeTerm::Array {
                element: self.import_type_operand(module, array.element)?,
            },
            dir::Type::FixedArray(array) => TypeTerm::FixedArray {
                element: self.import_type_operand(module, array.element)?,
                length: self.import_static_operand(module, array.count)?,
            },
            dir::Type::Range(range) => TypeTerm::Range {
                start: range.start,
                end: range.end,
                is_inclusive: range.is_inclusive,
            },
            dir::Type::Slice(slice) => TypeTerm::Slice {
                element: self.import_type_operand(module, slice.element)?,
            },
            dir::Type::Tuple(tuple) => TypeTerm::Tuple {
                form: tuple.form,
                elements: self.import_tuple_elements(module, tuple.elements)?,
            },
            dir::Type::Shape(shape) => {
                let shape = self.import_shape_term(module, shape)?;

                TypeTerm::Shape(self.inference.push_term(shape))
            }
            dir::Type::Function(function) => {
                let function = self.import_function_type_term(module, function)?;

                TypeTerm::Function(self.inference.push_term(function))
            }
            dir::Type::Closure(closure) => TypeTerm::Closure {
                function: self.import_type_operand(module, closure.function)?,
                environment: self.import_type_operand(module, closure.environment)?,
            },
            dir::Type::Union(union) => TypeTerm::Union {
                elements: self.import_type_operands(module, union.elements)?,
            },
            dir::Type::Intersection(intersection) => TypeTerm::Intersection {
                elements: self.import_type_operands(module, intersection.elements)?,
            },
        };

        Ok(term)
    }

    /// Import one committed function type as a check function term.
    fn import_function_type_term(
        &mut self,
        module: ModuleId,
        function: dir::FunctionType,
    ) -> CompilerResult<FunctionTerm> {
        let mut generic_parameters = Vec::with_capacity(function.generic_parameters.len());

        // import generic type parameter identities
        for parameter in function.generic_parameters {
            let dir::Type::Parameter(parameter) = self.r#type(parameter) else {
                return Err(CompilerError::Internal {
                    message: "committed function generic parameter is not a parameter type"
                        .to_owned(),
                });
            };

            generic_parameters.push(self.import_generic_parameter_id(module, *parameter)?);
        }

        // import receiver and runtime parameter types
        let this_parameter = function
            .this_parameter
            .map(|parameter| self.import_type_operand(module, parameter))
            .transpose()?;
        let mut parameters = Vec::with_capacity(function.parameters.len());
        for parameter in function.parameters {
            let static_parameter = parameter
                .static_parameter
                .map(|parameter| self.import_generic_parameter_id(module, parameter))
                .transpose()?;

            parameters.push(FunctionParameter {
                ty: self.import_type_operand(module, parameter.ty)?,
                static_parameter,
                is_inferred: false,
                is_optional: parameter.is_optional,
                is_rest: parameter.is_rest,
            });
        }

        // import return type last
        let return_type = function
            .return_type
            .map(|return_type| self.import_type_operand(module, return_type))
            .transpose()?;

        Ok(FunctionTerm {
            asynchrony: function.asynchrony,
            generic_parameters: generic_parameters.into(),
            this_parameter,
            parameters: parameters.into(),
            return_type,
            is_generator: function.is_generator,
        })
    }

    /// Return one committed static id as a check static operand.
    pub(in crate::check) fn import_static_operand(
        &mut self,
        module: ModuleId,
        source: dir::GlobalStaticId,
    ) -> CompilerResult<StaticOperand> {
        if let Some(operand) = self.inputs.r#static(source) {
            return Ok(operand);
        }

        // import generic parameters as check parameters
        let parameter = match self.r#static(source) {
            dir::StaticTerm::Parameter(parameter) => Some(*parameter),
            _ => None,
        };
        if let Some(parameter) = parameter {
            let parameter = self.import_generic_parameter_id(module, parameter)?;
            let operand =
                StaticOperand::Term(self.inference.push_term(StaticTerm::Parameter(parameter)));

            self.inputs.upsert_static(source, operand)?;

            return Ok(operand);
        }

        Ok(StaticOperand::Static(source))
    }

    /// Import committed type ids as check operands.
    fn import_type_operands(
        &mut self,
        module: ModuleId,
        sources: Vec<dir::GlobalTypeId>,
    ) -> CompilerResult<Vec<TypeOperand>> {
        sources
            .into_iter()
            .map(|source| self.import_type_operand(module, source))
            .collect()
    }

    /// Import committed reference arguments as check generic arguments.
    fn import_reference_arguments(
        &mut self,
        module: ModuleId,
        arguments: Vec<dir::StaticArgument>,
    ) -> CompilerResult<SmallVec<[GenericArgument; 2]>> {
        let mut imported = SmallVec::new();

        // preserve argument order and names
        for argument in arguments {
            imported.push(self.import_reference_argument(module, argument)?);
        }

        Ok(imported)
    }

    /// Import one committed reference argument as a check generic argument.
    fn import_reference_argument(
        &mut self,
        module: ModuleId,
        argument: dir::StaticArgument,
    ) -> CompilerResult<GenericArgument> {
        let type_argument = match self.r#static(argument.value) {
            dir::StaticTerm::Type { ty } => Some(*ty),
            _ => None,
        };
        let argument = match type_argument {
            Some(ty) => GenericArgument::Type(self.import_type_operand(module, ty)?),
            None => GenericArgument::Static(self.import_static_operand(module, argument.value)?),
        };

        Ok(argument)
    }

    /// Import one committed form constructor as a check form term.
    fn import_form_term(&mut self, module: ModuleId, form: dir::Form) -> CompilerResult<FormTerm> {
        let form = match form {
            dir::Form::Managed => FormTerm::Managed,
            dir::Form::Owned => FormTerm::Owned,
            dir::Form::Borrowed { lifetime, access } => FormTerm::Borrowed {
                lifetime: self.import_static_operand(module, lifetime)?,
                access: self.import_static_operand(module, access)?,
            },
            dir::Form::Raw => FormTerm::Raw,
            dir::Form::Placed { place } => FormTerm::Placed {
                place: self.import_static_operand(module, place)?,
            },
            dir::Form::Readonly => FormTerm::Readonly,
        };

        Ok(form)
    }

    /// Import one committed type operation as a check operation term.
    fn import_type_operation(
        &mut self,
        module: ModuleId,
        operation: dir::TypeOperation,
    ) -> CompilerResult<TypeOperationTerm> {
        let operation = match operation {
            dir::TypeOperation::StringMapping { mapping, target } => {
                TypeOperationTerm::StringMapping {
                    mapping,
                    argument: self.import_type_operand(module, target)?,
                }
            }
            dir::TypeOperation::Conditional(operation) => TypeOperationTerm::Conditional {
                left: self.import_type_operand(module, operation.left)?,
                right: self.import_type_operand(module, operation.right)?,
                then_type: self.import_type_operand(module, operation.then_type)?,
                else_type: self.import_type_operand(module, operation.else_type)?,
            },
            dir::TypeOperation::Mapped(operation) => TypeOperationTerm::Mapped {
                parameter: MappedParameter {
                    name: operation.parameter.name,
                    symbol: operation.parameter.symbol,
                    constraint: self.import_type_operand(module, operation.parameter.constraint)?,
                    key_remap: operation
                        .parameter
                        .key_remap
                        .map(|key_remap| self.import_type_operand(module, key_remap))
                        .transpose()?,
                },
                modifiers: operation.modifiers,
                value: self.import_type_operand(module, operation.value)?,
            },
            dir::TypeOperation::Index(operation) => TypeOperationTerm::Index {
                left: self.import_type_operand(module, operation.left)?,
                index: self.import_type_operand(module, operation.index)?,
            },
            dir::TypeOperation::TemplateLiteral(operation) => TypeOperationTerm::TemplateLiteral {
                strings: operation.strings,
                spans: self.import_type_operands(module, operation.spans)?,
            },
            dir::TypeOperation::Infer(operation) => TypeOperationTerm::Infer {
                name: operation.name,
                constraint: operation
                    .constraint
                    .map(|constraint| self.import_type_operand(module, constraint))
                    .transpose()?,
            },
            dir::TypeOperation::KeyOf(operation) => TypeOperationTerm::KeyOf {
                target: self.import_type_operand(module, operation.target)?,
            },
        };

        Ok(operation)
    }

    /// Import committed tuple elements as check tuple elements.
    fn import_tuple_elements(
        &mut self,
        module: ModuleId,
        elements: Vec<dir::TypeElement>,
    ) -> CompilerResult<Vec<TupleElement>> {
        let mut imported = Vec::with_capacity(elements.len());

        // preserve tuple element order
        for element in elements {
            imported.push(TupleElement {
                label: element.label,
                ty: self.import_type_operand(module, element.ty)?,
                is_optional: element.is_optional,
                is_readonly: element.is_readonly,
                is_rest: element.is_rest,
            });
        }

        Ok(imported)
    }

    /// Import one committed structural shape as a check shape term.
    fn import_shape_term(
        &mut self,
        module: ModuleId,
        shape: dir::ShapeType,
    ) -> CompilerResult<ShapeTerm> {
        let mut members = SmallVec::new();

        // import fields
        for field in shape.fields {
            members.push(ShapeMember::Field {
                key: field.key,
                ty: self.import_type_operand(module, field.ty)?,
                is_optional: field.is_optional,
                is_readonly: field.is_readonly,
            });
        }

        // import call signatures
        for ty in shape.call_signatures {
            members.push(ShapeMember::CallSignature {
                ty: self.import_type_operand(module, ty)?,
            });
        }

        // import construct signatures
        for ty in shape.construct_signatures {
            members.push(ShapeMember::ConstructSignature {
                ty: self.import_type_operand(module, ty)?,
            });
        }

        // import index signatures
        for signature in shape.index_signatures {
            members.push(ShapeMember::IndexSignature {
                name: signature.name,
                key_type: self.import_type_operand(module, signature.key_type)?,
                value_type: self.import_type_operand(module, signature.value_type)?,
                is_optional: signature.is_optional,
                is_readonly: signature.is_readonly,
            });
        }

        Ok(ShapeTerm { members })
    }

    /// Validate that one module can read another module.
    fn validate_import_module(
        &self,
        requesting_module: ModuleId,
        external_module: ModuleId,
    ) -> CompilerResult<()> {
        if self
            .module(requesting_module)
            .imports_module(external_module)
        {
            return Ok(());
        }

        let module = requesting_module;
        let requesting_module = self.dump_in_module(module, &requesting_module);
        let external_module = self.dump_in_module(module, &external_module);

        Err(CompilerError::Internal {
            message: format!(
                "check module {requesting_module} cannot read module {external_module}"
            ),
        })
    }
}
