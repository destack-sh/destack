use tspp_dir as dir;
use tspp_mir as mir;

use crate::lower::FunctionLowerer;
use crate::lower::function::operand::Operand;
use crate::lower::function::slice::SliceBuilder;
use crate::{CompilerError, CompilerResult};

/// One argument supplied by the enclosing operation.
#[derive(Clone, Copy)]
pub(in crate::lower) enum Argument {
    /// An expression evaluated at its argument position.
    Expression(dir::LocalNodeId<dir::Expression>),
    /// A value the enclosing operation has already evaluated.
    Value(mir::Value),
}

impl FunctionLowerer<'_, '_, '_> {
    /// Lower one argument list against its declared parameter representations.
    pub(in crate::lower) fn lower_call_arguments(
        &mut self,
        arguments: &[dir::ArgumentBinding],
        parameters: &[mir::TypeId],
        supplied: &[Argument],
    ) -> CompilerResult<Vec<mir::Value>> {
        // lower each argument at its parameter representation
        let mut values = Vec::with_capacity(arguments.len());
        for (index, binding) in arguments.iter().enumerate() {
            let parameter = parameters.get(index).copied();
            values.push(self.lower_call_argument(binding, parameter, supplied)?);
        }

        Ok(values)
    }

    /// Lower one selected argument and its conversion.
    fn lower_call_argument(
        &mut self,
        binding: &dir::ArgumentBinding,
        parameter: Option<mir::TypeId>,
        supplied: &[Argument],
    ) -> CompilerResult<mir::Value> {
        let value = match binding.source {
            // lower a provided argument
            dir::ArgumentSource::Provided(source) => {
                self.lower_argument_at(source, binding.parameter_type)?
            }
            // store omission at the parameter representation
            dir::ArgumentSource::Omitted => {
                self.lower_omitted_argument(binding.parameter_type, parameter)?
            }
            // read the argument supplied by the enclosing operation
            dir::ArgumentSource::Supplied(index) => {
                let Some(argument) = supplied.get(index as usize) else {
                    return Err(CompilerError::Internal {
                        message: "a call without its supplied argument".to_string(),
                    });
                };

                match *argument {
                    Argument::Expression(expression) => self.lower_value(expression)?,
                    Argument::Value(value) => value,
                }
            }
            // materialize a static argument as its literal constant
            dir::ArgumentSource::Static(argument) => {
                let dir::Type::Literal(literal) = self.lower.ty(argument)? else {
                    return Err(self.internal("a non-literal static argument"));
                };
                let representation = match parameter {
                    Some(parameter) => parameter,
                    None => self.lower_type(argument)?,
                };

                self.lower_constant(literal, representation)?
            }
            // pack the trailing arguments into the rest collection
            dir::ArgumentSource::Rest {
                ref elements,
                ref pack,
            } => self.lower_rest_pack(
                elements,
                binding.argument_type,
                pack.as_ref(),
                binding.parameter_type,
                supplied,
            )?,
            dir::ArgumentSource::Spread(_) => {
                return Err(self.internal("an unexpanded spread argument"));
            }
            dir::ArgumentSource::Error => {
                return Err(self.internal("a rejected argument reached lowering"));
            }
        };

        // execute the conversion selected for this argument
        self.convert_value(value, binding.coercion.as_deref())
    }

    /// Lower one provided argument as its value expression.
    pub(in crate::lower) fn lower_argument(
        &mut self,
        source: dir::GlobalNodeIdAny,
    ) -> CompilerResult<mir::Value> {
        let expression = self.provided_expression(source)?;
        self.lower_value(expression)
    }

    /// Lower one provided argument at its parameter type, a literal widening to it.
    pub(in crate::lower) fn lower_argument_at(
        &mut self,
        source: dir::GlobalNodeIdAny,
        parameter: dir::GlobalTypeId,
    ) -> CompilerResult<mir::Value> {
        let expression = self.provided_expression(source)?;
        let operand = self.lower_operand(expression)?;

        self.lower_anchored(expression, |lower| lower.as_value(operand, parameter))
    }

    /// Return the value expression of one provided argument node.
    fn provided_expression(
        &mut self,
        source: dir::GlobalNodeIdAny,
    ) -> CompilerResult<dir::LocalNodeId<dir::Expression>> {
        // unwrap the provided value from argument nodes
        if let Ok(argument) = source.local_id.try_into_typed::<dir::Argument>() {
            return match self.source().tree().get(argument) {
                dir::Argument::Positional { value } | dir::Argument::Spread { value } => Ok(*value),
                // reject an empty argument slot
                dir::Argument::Elision | dir::Argument::Error => Err(CompilerError::Internal {
                    message: "an empty argument".to_string(),
                }),
            };
        }

        // read the source as the value expression itself
        source
            .local_id
            .try_into_typed::<dir::Expression>()
            .map_err(|_| CompilerError::Internal {
                message: format!("a non-expression argument node {}", source.local_id.id),
            })
    }

    /// Pack the rest elements into their parameter's collection, an owned slice in fresh storage.
    pub(in crate::lower) fn lower_rest_pack(
        &mut self,
        elements: &[dir::ArgumentBinding],
        element_type: dir::GlobalTypeId,
        pack: Option<&dir::InstanceKey>,
        target: dir::GlobalTypeId,
        supplied: &[Argument],
    ) -> CompilerResult<mir::Value> {
        // evaluate the fixed prefix before opening any spread iterator
        let element = self.lower_type(element_type)?;
        let is_owned = self.lower.ownership(target)? == dir::Ownership::Owned;
        let target = self.lower_type(target)?;
        let prefix = elements
            .iter()
            .position(|binding| matches!(binding.source, dir::ArgumentSource::Spread(_)))
            .unwrap_or(elements.len());
        let (prefix, elements) = elements.split_at(prefix);
        let mut values = Vec::with_capacity(prefix.len());
        for binding in prefix {
            values.push(self.lower_call_argument(binding, Some(element), supplied)?);
        }

        // allocate in the storage selected by the slice parameter
        let constructor = match pack {
            Some(pack) => Some(self.resolve_callee(pack)?),
            None => None,
        };

        // append the remaining elements in source order
        let completed = if !elements.is_empty() {
            let builder = SliceBuilder::new(element, values, self)?;
            for binding in elements {
                match &binding.source {
                    // consume each iterator before evaluating the next argument
                    dir::ArgumentSource::Spread(spread) => {
                        let coercion = match spread.value.source {
                            dir::ArgumentSource::Provided(source) => self.coercion(source.local_id),
                            _ => binding.coercion.as_deref().cloned(),
                        };
                        let receiver = self.lower_call_argument(&spread.value, None, supplied)?;
                        let opened = self.lower_target_call(
                            Operand::Value(receiver),
                            &spread.iteration.iterator,
                        )?;
                        let Some(opened) = opened else {
                            return Err(self.internal("a spread iterator call without a result"));
                        };
                        self.lower_iteration(&spread.iteration, opened, |lower, value, _, _| {
                            let value = lower.convert_value(value, coercion.as_ref())?;
                            builder.push(value, lower);

                            Ok(false)
                        })?;
                    }
                    // append individual arguments at the same element representation
                    _ => {
                        let value = self.lower_call_argument(binding, Some(element), supplied)?;
                        builder.push(value, self);
                    }
                }
            }
            builder.finish(self)
        } else {
            // allocate fixed storage when every argument contributes one element
            if pack.is_none() && !is_owned {
                let view = self.temporary_slice(target)?;

                return self.frame_slice(values, view);
            }

            self.owned_slice(element, values)?
        };

        // construct the selected collection from completed storage
        if let Some(function) = constructor {
            let parameters = self.signature_parameters(function.signature)?;
            let Some(parameter) = parameters.first().copied() else {
                return Err(self.internal("a pack constructor without its slice parameter"));
            };
            let completed = self.adopt(completed, parameter)?;

            let value = self
                .call(&function, vec![completed])
                .ok_or_else(|| self.internal("a pack constructor call without a value"))?;

            return self.adopt(value, target);
        }
        if is_owned {
            return Ok(completed);
        }

        // retain the allocation while the borrowed rest argument is in use
        let storage = self.value_representation(completed)?;
        let length = self.builder.slice_length(completed);
        let local = self.builder.local(storage, mir::Mutability::Immutable);
        self.builder.local_set(local, completed);
        let slice = self.temporary_slice(target)?;
        let start = self.builder.usize_const(0);
        let place = mir::Place::local(local)
            .with_projection(mir::Projection::Deref)
            .with_projection(mir::Projection::Slice { start, length });

        Ok(self.builder.address(place, slice))
    }

    /// Return one slice type viewed from a temporary's storage for the frame's extent.
    pub(in crate::lower) fn temporary_slice(
        &mut self,
        slice: mir::TypeId,
    ) -> CompilerResult<mir::TypeId> {
        let mir::Type::Slice {
            kind,
            element,
            access,
            ..
        } = *self.builder.tree().type_definition(slice)
        else {
            return Err(self.internal("a slice view outside slice storage"));
        };

        Ok(self.builder.tree_mut().intern_type(mir::Type::Slice {
            kind,
            lifetime: mir::Lifetime::frame(),
            element,
            access,
        }))
    }

    /// Store one value sequence in a frame slot, viewed at one frame slice type.
    pub(in crate::lower) fn frame_slice(
        &mut self,
        values: Vec<mir::Value>,
        slice: mir::TypeId,
    ) -> CompilerResult<mir::Value> {
        let mir::Type::Slice { element, .. } = *self.builder.tree().type_definition(slice) else {
            return Err(self.internal("a frame slice outside slice storage"));
        };

        // complete each value at the element representation
        let values = values
            .into_iter()
            .map(|value| self.adopt(value, element))
            .collect::<CompilerResult<Vec<_>>>()?;

        // store the aggregate in one frame slot
        let length = values.len();
        let count = self
            .builder
            .tree_mut()
            .intern_static(mir::Static::Integer(length as i64));
        let storage = self.builder.tree_mut().intern_type(mir::Type::FixedArray {
            element,
            length: count,
        });
        let aggregate = self.builder.aggregate(storage, values);
        let slot = self.builder.local(storage, mir::Mutability::Immutable);
        self.builder.local_set(slot, aggregate);

        // borrow the complete element range from frame storage
        let start = self.builder.usize_const(0);
        let length = self.builder.usize_const(length as u128);

        let place =
            mir::Place::local(slot).with_projection(mir::Projection::Slice { start, length });

        Ok(self.builder.address(place, slice))
    }

    /// Move one value sequence into a fresh unique slice.
    fn owned_slice(
        &mut self,
        element: mir::TypeId,
        values: Vec<mir::Value>,
    ) -> CompilerResult<mir::Value> {
        let storage = self.builder.tree_mut().intern_type(mir::Type::Slice {
            kind: mir::Reference::Unique,
            lifetime: mir::Lifetime::empty(),
            element,
            access: mir::Access::Mutable,
        });

        // allocate the elements' storage uninitialized
        let uninit = self
            .builder
            .tree_mut()
            .intern_type(mir::Type::Uninit { value: element });
        let allocation = self.builder.tree_mut().intern_type(mir::Type::Slice {
            kind: mir::Reference::Unique,
            lifetime: mir::Lifetime::empty(),
            element: uninit,
            access: mir::Access::Mutable,
        });
        let space = self.allocation_space(allocation)?;
        let length = self.builder.usize_const(values.len() as u128);
        let allocated = self
            .builder
            .new_slice_uninit(uninit, length, allocation, space);

        // initialize each element in order
        for (index, value) in values.into_iter().enumerate() {
            let index = self.builder.usize_const(index as u128);
            let place = mir::Place::value(allocated)
                .with_projection(mir::Projection::Deref)
                .with_projection(mir::Projection::Index { index });
            self.builder.store(place, value);
        }

        // complete the storage
        Ok(self.builder.new_complete(allocated, storage))
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

        self.absent_value(representation)
            .ok_or_else(|| CompilerError::Internal {
                message: "an omitted argument at a representation without an undefined case"
                    .to_string(),
            })
    }
}
