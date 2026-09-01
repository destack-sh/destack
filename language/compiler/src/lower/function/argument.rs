use destack_dir as dir;
use destack_mir as mir;

use crate::lower::FunctionLowerer;
use crate::{CompilerError, CompilerResult, LowerError};

impl FunctionLowerer<'_, '_, '_> {
    /// Lower one argument list against its declared parameter representations.
    pub(in crate::lower) fn lower_call_arguments(
        &mut self,
        arguments: &[dir::ArgumentBinding],
        parameters: &[mir::TypeId],
        write: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> CompilerResult<Vec<mir::Value>> {
        // lower each argument at its parameter representation
        let mut values = Vec::with_capacity(arguments.len());
        for (index, binding) in arguments.iter().enumerate() {
            let parameter = parameters.get(index).copied();
            let value = match binding.source {
                // lower a written argument
                dir::ArgumentSource::Provided(source) => self.lower_argument(source)?,
                // store omission at the parameter representation
                dir::ArgumentSource::Omitted => {
                    values.push(self.lower_omitted_argument(binding.parameter_type, parameter)?);

                    continue;
                }
                // fill the write argument with the assigned value for setter calls
                dir::ArgumentSource::Write => {
                    let Some(expression) = write else {
                        return Err(CompilerError::Internal {
                            message: "an implicit write argument outside a setter call".to_string(),
                        });
                    };

                    self.lower_expression(expression)?
                }
                // materialize a static argument as its literal constant
                dir::ArgumentSource::Static(argument) => {
                    let dir::Type::Literal(literal) = self.lower.ty(argument)? else {
                        return Err(LowerError::Unsupported {
                            anchor: self.lower.module.into(),
                            construct: "a non-literal static argument".to_string(),
                        }
                        .into());
                    };
                    let representation = self.lower_type(argument)?;
                    let representation = self.builder.tree().get(representation).clone();

                    self.lower_constant(literal, representation)?
                }
                // pack the trailing arguments into the rest collection
                dir::ArgumentSource::Rest {
                    ref elements,
                    ref pack,
                } => {
                    let elements = elements.clone();
                    let pack = pack.clone();

                    self.lower_rest_pack(&elements, binding.argument_type, pack.as_ref())?
                }
            };

            // adapt the value to its declared parameter representation
            match parameter {
                Some(parameter) => values.push(self.adapt_to_representation(value, parameter)?),
                None => values.push(value),
            }
        }

        Ok(values)
    }

    /// Lower one provided argument source to its value.
    pub(in crate::lower) fn lower_argument(
        &mut self,
        source: dir::GlobalNodeIdAny,
    ) -> CompilerResult<mir::Value> {
        // unwrap the provided value from argument nodes
        if let Ok(argument) = source.local_id.try_into_typed::<dir::Argument>() {
            let value = match self.source().tree().get(argument) {
                dir::Argument::Positional { value } => *value,
                // reject a spread argument
                dir::Argument::Spread { .. } => {
                    return Err(LowerError::Unsupported {
                        anchor: self.lower.module.into(),
                        construct: "a spread argument".to_string(),
                    }
                    .into());
                }
                // reject an empty argument slot
                dir::Argument::Elision | dir::Argument::Error => {
                    return Err(CompilerError::Internal {
                        message: "an empty argument".to_string(),
                    });
                }
            };

            return self.lower_expression(value);
        }

        // lower the source as the value expression itself
        let Ok(expression) = source.local_id.try_into_typed::<dir::Expression>() else {
            return Err(CompilerError::Internal {
                message: format!("a non-expression argument node {}", source.local_id.id),
            });
        };

        self.lower_expression(expression)
    }

    /// Pack the rest elements into their parameter's collection.
    pub(in crate::lower) fn lower_rest_pack(
        &mut self,
        elements: &[dir::GlobalNodeIdAny],
        element_type: dir::GlobalTypeId,
        pack: Option<&dir::InstanceKey>,
    ) -> CompilerResult<mir::Value> {
        // forward a sole spread argument whole
        if let [source] = elements
            && let Ok(argument) = source.local_id.try_into_typed::<dir::Argument>()
            && let dir::Argument::Spread { value, .. } = self.source().tree().get(argument)
        {
            let value = *value;
            return self.lower_expression(value);
        }

        // materialize the elements into fixed stack storage
        let element = self.lower_type(element_type)?;
        let mut values = Vec::with_capacity(elements.len());
        for source in elements {
            values.push(self.lower_argument(*source)?);
        }

        // store the aggregate in one frame slot
        let storage = self.builder.tree_mut().intern_type(mir::Type::FixedArray {
            element: mir::TypeId::from(element),
            length: values.len() as u64,
            copy: mir::Copy::No,
        });
        let aggregate = self.builder.aggregate(storage, values);
        let slot = self.builder.local(storage, mir::Mutability::Immutable);
        self.builder.local_set(slot, aggregate);

        // view the storage as a borrowed slice of the elements
        let address = self.insert_reference(
            mir::ReferenceKind::Borrowed,
            mir::Access::Readonly,
            mir::Storage::Frame,
            storage,
        );
        let address = self.builder.local_addr(slot, address);
        let slice = self.builder.tree_mut().intern_type(mir::Type::Slice {
            kind: mir::ReferenceKind::Borrowed,
            lifetime: mir::Lifetime::empty(),
            element: mir::TypeId::from(element),
            storage: mir::Storage::Frame,
            access: mir::Access::Readonly,
            nullability: mir::Nullability::None,
        });
        let start = self.builder.iconst(0, 64, false);
        let length = self.builder.usize_const(elements.len() as u128);
        let view = self.builder.slice_view(address, start, length, slice);

        // take the view directly for a slice parameter
        let Some(pack) = pack else {
            return Ok(view);
        };

        // build the collection by calling its pack constructor over the view
        let function = self.selection_function(pack)?;
        let parameters = self.function_parameters(function);
        let Some(parameter) = parameters.first() else {
            return Err(CompilerError::Internal {
                message: "a pack constructor without a declared slice slot".to_string(),
            });
        };
        let view = self.adapt_to_representation(view, *parameter)?;
        let Some(packed) = self.builder.call_function(function, vec![view]) else {
            return Err(CompilerError::Internal {
                message: "a pack constructor call without a value".to_string(),
            });
        };

        Ok(packed)
    }

    /// Lower one omitted argument at its parameter representation.
    fn lower_omitted_argument(
        &mut self,
        ty: dir::GlobalTypeId,
        parameter: Option<mir::TypeId>,
    ) -> CompilerResult<mir::Value> {
        // take the parameter representation, else the declared type
        let representation = match parameter {
            Some(parameter) => parameter,
            None => self.lower_type(ty)?,
        };

        Ok(self.absent_argument_value(representation))
    }
}
