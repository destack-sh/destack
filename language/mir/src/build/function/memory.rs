use crate::build::{BuildError, BuildResult, FunctionBuilder};
use crate::{
    Access, Constant, Global, Instruction, Lifetime, Local, LocalNodeId, Mutability, Nullability,
    ReferenceKind, Space, Type, Value,
};

#[allow(clippy::too_many_arguments)]
impl<'a> FunctionBuilder<'a> {
    /// Return the existing or inserted pointer-sized unsigned integer type.
    pub fn ensure_usize_type(&mut self) -> LocalNodeId<Type> {
        if let Some((ty, _)) = self
            .tree
            .iter_nodes::<Type>()
            .find(|(_, ty)| matches!(ty, Type::Usize))
        {
            return ty;
        }

        self.tree.insert_type(Type::Usize)
    }

    /// Create a local variable (stack slot).
    pub fn local(&mut self, ty: LocalNodeId<Type>, mutability: Mutability) -> LocalNodeId<Local> {
        let local = self.tree.insert(Local::new(ty, mutability));
        self.locals.push(local);
        local
    }

    /// Create a reference type for inline instruction typing.
    pub fn reference_type(
        &mut self,
        kind: ReferenceKind,
        pointee: LocalNodeId<Type>,
        access: Access,
        space: Space,
        nullability: Nullability,
    ) -> LocalNodeId<Type> {
        self.tree.insert_type(Type::Reference {
            kind,
            lifetime: Lifetime::empty(),
            space,
            access,
            pointee,
            nullability,
        })
    }

    /// Load from a local variable.
    pub fn local_get(&mut self, local: LocalNodeId<Local>) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::LocalGet { destination, local });
        let local_ty = self.tree.get(local).ty;
        self.define_value(destination, local_ty);
        destination
    }

    /// Get the address of a local variable.
    pub fn local_addr(
        &mut self,
        local: LocalNodeId<Local>,
        result_type: LocalNodeId<Type>,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::LocalAddr {
            destination,
            local,
            result_type,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Store to a local variable.
    pub fn local_set(&mut self, local: LocalNodeId<Local>, value: Value) {
        self.insert_instruction(Instruction::LocalSet { local, value });
    }

    /// Get the address of a mutable global variable.
    pub fn global_addr(
        &mut self,
        global: LocalNodeId<Global>,
        result_type: LocalNodeId<Type>,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::GlobalAddr {
            destination,
            global,
            result_type,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Load one global value through its address.
    pub fn load_global(&mut self, global: LocalNodeId<Global>) -> Value {
        let global_ty = self.tree.get(global).ty;
        let global_space = self.tree.get(global).space.clone();
        let global_pointer = self.reference_type(
            ReferenceKind::Raw,
            global_ty,
            Access::Readonly,
            global_space,
            Nullability::None,
        );
        let pointer = self.global_addr(global, global_pointer);

        self.load(pointer, global_ty)
    }

    /// Load from a pointer.
    pub fn load(&mut self, pointer_value: Value, result_type: LocalNodeId<Type>) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::Load {
            destination,
            pointer: pointer_value,
            result_type,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Store to a pointer.
    pub fn store(&mut self, pointer_value: Value, value: Value) {
        self.insert_instruction(Instruction::Store {
            pointer: pointer_value,
            value,
        });
    }

    /// Resolve a static layout slot type for an aggregate.
    pub(super) fn field_type_for_aggregate(
        &self,
        aggregate_type: LocalNodeId<Type>,
        index: u32,
    ) -> BuildResult<LocalNodeId<Type>> {
        let aggregate = self.tree.get(aggregate_type);
        match aggregate {
            Type::Struct { fields, .. } => {
                let Some(field_id) = fields.get(index as usize) else {
                    return Err(BuildError::InvalidFieldIndex {
                        aggregate: aggregate_type,
                        index,
                    });
                };

                Ok(self.tree.get(*field_id).ty)
            }
            Type::Tuple { elements, .. } => {
                let Some(element) = elements.get(index as usize) else {
                    return Err(BuildError::InvalidFieldIndex {
                        aggregate: aggregate_type,
                        index,
                    });
                };

                Ok(*element)
            }
            Type::Variant { tag, storage, .. } => match index {
                0 => Ok(*tag),
                1 => Ok(*storage),
                _ => Err(BuildError::InvalidFieldIndex {
                    aggregate: aggregate_type,
                    index,
                }),
            },
            Type::FixedArray {
                element, length, ..
            } => {
                if u64::from(index) < *length {
                    Ok(*element)
                } else {
                    Err(BuildError::InvalidFieldIndex {
                        aggregate: aggregate_type,
                        index,
                    })
                }
            }
            Type::Function { .. } => Err(BuildError::InvalidFieldOwner { ty: aggregate_type }),
            _ => Err(BuildError::InvalidFieldOwner { ty: aggregate_type }),
        }
    }

    /// Resolve the tag type for one variant aggregate.
    pub(super) fn variant_tag_type(
        &self,
        variant_type: LocalNodeId<Type>,
    ) -> BuildResult<LocalNodeId<Type>> {
        match self.tree.get(variant_type) {
            Type::Variant { tag, .. } => Ok(*tag),
            _ => Err(BuildError::InvalidFieldOwner { ty: variant_type }),
        }
    }

    /// Resolve the payload type for one variant case.
    pub(super) fn variant_payload_type(
        &self,
        variant_type: LocalNodeId<Type>,
        tag: &Constant,
    ) -> BuildResult<LocalNodeId<Type>> {
        match self.tree.get(variant_type) {
            Type::Variant { cases, .. } => cases
                .iter()
                .find(|case| case.tag == *tag)
                .map(|case| case.ty)
                .ok_or(BuildError::InvalidFieldOwner { ty: variant_type }),
            _ => Err(BuildError::InvalidFieldOwner { ty: variant_type }),
        }
    }

    /// Resolve the element type for a vector type.
    pub(super) fn element_type_for_vector(
        &self,
        vector_type: LocalNodeId<Type>,
    ) -> BuildResult<LocalNodeId<Type>> {
        let vector = self.tree.get(vector_type);
        match vector {
            Type::Vector { element, .. } => Ok(*element),
            _ => Err(BuildError::InvalidVectorOwner { ty: vector_type }),
        }
    }

    /// Resolve the element type for a tensor view.
    pub(super) fn element_type_for_tensor(
        &self,
        tensor_type_id: LocalNodeId<Type>,
    ) -> BuildResult<LocalNodeId<Type>> {
        let tensor_type = self.tree.get(tensor_type_id);
        match tensor_type {
            Type::Tensor { element, .. } => Ok(*element),
            _ => Err(BuildError::InvalidTensorOwner { ty: tensor_type_id }),
        }
    }

    /// Resolve the element type for a tensor view.
    pub(super) fn element_type_for_tensor_view(
        &self,
        reference_type_id: LocalNodeId<Type>,
    ) -> BuildResult<LocalNodeId<Type>> {
        let reference_type = self.tree.get(reference_type_id);
        match reference_type {
            Type::TensorView { element, .. } => Ok(*element),
            _ => Err(BuildError::InvalidTensorViewOwner {
                ty: reference_type_id,
            }),
        }
    }

    /// Convert a list length into u16 for instruction tables.
    pub(super) fn to_u16_count(&self, count: usize, context: &str) -> BuildResult<u16> {
        u16::try_from(count).map_err(|_| BuildError::CountTooLarge {
            count,
            context: context.to_string(),
        })
    }

    /// Resolve the return type for a function signature.
    pub(super) fn signature_result_type(
        &self,
        signature_type_id: LocalNodeId<Type>,
    ) -> BuildResult<LocalNodeId<Type>> {
        let signature_type = self.tree.get(signature_type_id);
        match signature_type {
            Type::FunctionSignature { result, .. } => Ok(*result),
            Type::FunctionPointer { .. } | Type::Function { .. } => {
                let Some(signature_id) = signature_type.callable_signature() else {
                    return Err(BuildError::MissingFunctionSignature {
                        ty: signature_type_id,
                    });
                };
                let signature_type = self.tree.get(signature_id);
                let Some((_, _, result)) = signature_type.function_signature_parts() else {
                    return Err(BuildError::MissingFunctionSignature { ty: signature_id });
                };

                Ok(result)
            }
            _ => Err(BuildError::MissingFunctionSignature {
                ty: signature_type_id,
            }),
        }
    }

    /// Create a linear uninitialized allocation token type.
    pub fn type_uninit(&mut self, value: LocalNodeId<Type>) -> LocalNodeId<Type> {
        self.tree.insert_type(Type::Uninit { value })
    }

    /// Allocate zeroed heap storage.
    pub fn new_zeroed(
        &mut self,
        layout: LocalNodeId<Type>,
        result_type: LocalNodeId<Type>,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::NewZeroed {
            destination,
            layout,
            result_type,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Allocate uninitialized heap storage.
    pub fn new_uninit(
        &mut self,
        layout: LocalNodeId<Type>,
        result_type: LocalNodeId<Type>,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::NewUninit {
            destination,
            layout,
            result_type,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Complete one initialized heap allocation.
    pub fn new_complete(&mut self, value: Value, result_type: LocalNodeId<Type>) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::NewComplete {
            destination,
            value,
            result_type,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Allocate zeroed repeated heap storage.
    pub fn new_slice_zeroed(
        &mut self,
        element: LocalNodeId<Type>,
        length: Value,
        result_type: LocalNodeId<Type>,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::NewSliceZeroed {
            destination,
            element,
            length,
            result_type,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Allocate uninitialized repeated heap storage.
    pub fn new_slice_uninit(
        &mut self,
        element: LocalNodeId<Type>,
        length: Value,
        result_type: LocalNodeId<Type>,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::NewSliceUninit {
            destination,
            element,
            length,
            result_type,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Free unique heap storage after drop elaboration.
    pub fn free(&mut self, value: Value) {
        self.insert_instruction(Instruction::Free { value });
    }

    /// Allocate zeroed frame storage.
    pub fn frame_alloc_zeroed(
        &mut self,
        layout: LocalNodeId<Type>,
        result_type: LocalNodeId<Type>,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::FrameAllocZeroed {
            destination,
            layout,
            result_type,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Allocate uninitialized frame storage.
    pub fn frame_alloc_uninit(
        &mut self,
        layout: LocalNodeId<Type>,
        result_type: LocalNodeId<Type>,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::FrameAllocUninit {
            destination,
            layout,
            result_type,
        });
        self.define_value(destination, result_type);
        destination
    }

    // instruction builders: assumptions

    /// Assume a condition is true (UB if false).
    pub fn assume(&mut self, condition: Value) {
        self.insert_instruction(Instruction::Assume { condition });
    }
}
