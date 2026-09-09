use destack_dir as dir;
use destack_mir as mir;

use crate::lower::FunctionLowerer;
use crate::{CompilerError, CompilerResult};

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
                dir::ArgumentSource::Provided(source) => {
                    self.lower_argument_at(source, binding.parameter_type)?
                }
                // store omission at the parameter representation
                dir::ArgumentSource::Omitted => {
                    values.push(self.lower_omitted_argument(binding.parameter_type, parameter)?);

                    continue;
                }
                // fill the write argument with the assigned value for setter calls
                dir::ArgumentSource::Supplied => {
                    let Some(expression) = write else {
                        return Err(CompilerError::Internal {
                            message: "an implicit write argument outside a setter call".to_string(),
                        });
                    };

                    self.lower_value(expression)?
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
                } => {
                    let elements = elements.clone();
                    let pack = pack.clone();
                    let owned =
                        self.lower.ownership(binding.parameter_type)? == dir::Ownership::Owned;

                    self.lower_rest_pack(&elements, binding.argument_type, pack.as_ref(), owned)?
                }
            };

            values.push(value);
        }

        Ok(values)
    }

    /// Lower one provided argument as the value expression it carries.
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

    /// Return the value expression one provided argument node carries.
    fn provided_expression(
        &mut self,
        source: dir::GlobalNodeIdAny,
    ) -> CompilerResult<dir::LocalNodeId<dir::Expression>> {
        // unwrap the provided value from argument nodes
        if let Ok(argument) = source.local_id.try_into_typed::<dir::Argument>() {
            return match self.source().tree().get(argument) {
                dir::Argument::Positional { value } => Ok(*value),
                // reject a spread argument
                dir::Argument::Spread { .. } => Err(self.unsupported("a spread argument")),
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
        elements: &[dir::GlobalNodeIdAny],
        element_type: dir::GlobalTypeId,
        pack: Option<&dir::InstanceKey>,
        owned: bool,
    ) -> CompilerResult<mir::Value> {
        // forward a sole spread argument whole
        if let [source] = elements
            && let Ok(argument) = source.local_id.try_into_typed::<dir::Argument>()
            && let dir::Argument::Spread { value, .. } = self.source().tree().get(argument)
        {
            let value = *value;
            return self.lower_value(value);
        }

        let element = self.lower_type(element_type)?;
        // lower each element in order
        let mut values = Vec::with_capacity(elements.len());
        for source in elements {
            values.push(self.lower_argument(*source)?);
        }

        // hand a collection its elements in owned storage
        if let Some(pack) = pack {
            return self.lower_owned_pack(element, values, pack);
        }
        if owned {
            return self.owned_slice(element, values);
        }

        self.frame_slice(element, values)
    }

    /// Store one value sequence in a frame slot, viewed as a borrowed slice.
    pub(in crate::lower) fn frame_slice(
        &mut self,
        element: mir::LocalNodeId<mir::Type>,
        values: Vec<mir::Value>,
    ) -> CompilerResult<mir::Value> {
        // store the aggregate in one frame slot
        let length = values.len();
        let count = self
            .builder
            .tree_mut()
            .intern_static(mir::Static::Integer(length as i64));
        let storage = self.builder.tree_mut().intern_type(mir::Type::FixedArray {
            element: mir::TypeId::from(element),
            length: count,
        });
        let aggregate = self.builder.aggregate(storage, values);
        let slot = self.builder.local(storage, mir::Mutability::Immutable);
        self.builder.local_set(slot, aggregate);

        // view the storage as a borrowed slice of the elements
        let address = self.insert_reference(
            mir::ReferenceKind::Borrowed,
            mir::Lifetime::frame(),
            mir::Access::Readonly,
            mir::Storage::Frame,
            storage,
        );
        let address = self
            .builder
            .local_addr(slot, address, mir::AddressKind::Borrow);
        let slice = self.builder.tree_mut().intern_type(mir::Type::Slice {
            kind: mir::ReferenceKind::Borrowed,
            lifetime: mir::Lifetime::frame(),
            element: mir::TypeId::from(element),
            storage: mir::Storage::Frame,
            access: mir::Access::Readonly,
        });
        let start = self.builder.iconst(0, 64, false);
        let length = self.builder.usize_const(length as u128);

        Ok(self.builder.slice_view(address, start, length, slice))
    }

    /// Lower one collection over its elements moved into owned slice storage.
    fn lower_owned_pack(
        &mut self,
        element: mir::LocalNodeId<mir::Type>,
        values: Vec<mir::Value>,
        pack: &dir::InstanceKey,
    ) -> CompilerResult<mir::Value> {
        // read the owned slice the pack constructor takes
        let function = self.resolve_callee(pack)?;
        let parameters = self.signature_parameters(function.signature)?;
        let Some(storage) = parameters.first().copied() else {
            return Err(CompilerError::Internal {
                message: "a pack constructor without a declared slice slot".to_string(),
            });
        };

        // call the constructor over the packed elements
        let completed = self.owned_slice_at(element, values, storage);
        let Some(packed) = self.call(&function, vec![completed]) else {
            return Err(CompilerError::Internal {
                message: "a pack constructor call without a value".to_string(),
            });
        };

        Ok(packed)
    }

    /// Move one value sequence into a fresh unique slice.
    fn owned_slice(
        &mut self,
        element: mir::LocalNodeId<mir::Type>,
        values: Vec<mir::Value>,
    ) -> CompilerResult<mir::Value> {
        let storage = self.builder.tree_mut().intern_type(mir::Type::Slice {
            kind: mir::ReferenceKind::Unique,
            lifetime: mir::Lifetime::empty(),
            element: mir::TypeId::from(element),
            storage: mir::Storage::Heap(mir::Space::Local),
            access: mir::Access::Mutable,
        });

        Ok(self.owned_slice_at(element, values, mir::TypeId::from(storage)))
    }

    /// Move one value sequence into fresh storage completed at one unique slice type.
    fn owned_slice_at(
        &mut self,
        element: mir::LocalNodeId<mir::Type>,
        values: Vec<mir::Value>,
        storage: mir::TypeId,
    ) -> mir::Value {
        // allocate the elements' storage uninitialized
        let uninit = self.builder.tree_mut().intern_type(mir::Type::Uninit {
            value: mir::TypeId::from(element),
        });
        let allocation = self.builder.tree_mut().intern_type(mir::Type::Slice {
            kind: mir::ReferenceKind::Unique,
            lifetime: mir::Lifetime::empty(),
            element: mir::TypeId::from(uninit),
            storage: mir::Storage::Heap(mir::Space::Local),
            access: mir::Access::Mutable,
        });
        let length = self.builder.usize_const(values.len() as u128);
        let allocated = self.builder.new_slice_uninit(uninit, length, allocation);

        // initialize each element in order
        let slot = self.insert_reference(
            mir::ReferenceKind::Borrowed,
            mir::Lifetime::frame(),
            mir::Access::Mutable,
            mir::Storage::Heap(mir::Space::Local),
            uninit,
        );
        for (index, value) in values.into_iter().enumerate() {
            let index = self.builder.usize_const(index as u128);
            let pointer =
                self.builder
                    .element_addr(allocated, index, slot, mir::AddressKind::Borrow);
            self.builder.store(pointer, value);
        }

        // complete the storage
        self.builder.new_complete(allocated, storage)
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
