use crate::build::{BuildError, BuildResult, FunctionBuilder};
use crate::{
    Access, Global, Instruction, Lifetime, Local, LocalNodeId, Mutability, Nullability, Ownership,
    Place, ReferenceKind, Space, Type, TypeReference, Value, callable_signature,
    function_signature_parts,
};

#[allow(clippy::too_many_arguments)]
impl<'a> FunctionBuilder<'a> {
    /// Create a local variable (stack slot).
    pub fn local(&mut self, ty: LocalNodeId<Type>, mutability: Mutability) -> LocalNodeId<Local> {
        let local = self
            .tree
            .insert(Local::new(ty.into(), mutability, Ownership::Owned));
        let function = self.tree.get_mut(self.function_id);
        function.locals.push(local);
        local
    }

    /// Create a reference type for inline instruction typing.
    pub fn type_reference(
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
            pointee: pointee.into(),
            nullability,
        })
    }

    /// Load from a local variable.
    pub fn local_get(&mut self, local: LocalNodeId<Local>) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::LocalGet {
            destination: destination.into(),
            local: local.into(),
        });
        let local_ty = concrete_type_reference(&self.tree.get(local).ty, "local.get local type");
        let local_ty = self.expect_build(local_ty);
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
            destination: destination.into(),
            local: local.into(),
            result_type: result_type.into(),
        });
        self.define_value_with_place(destination, result_type, Place::local(local.into()));
        destination
    }

    /// Store to a local variable.
    pub fn local_set(&mut self, local: LocalNodeId<Local>, value: Value) {
        self.insert_instruction(Instruction::LocalSet {
            local: local.into(),
            value: value.into(),
        });
    }

    /// Get the address of a mutable global variable.
    pub fn global_addr(
        &mut self,
        global: LocalNodeId<Global>,
        result_type: LocalNodeId<Type>,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::GlobalAddr {
            destination: destination.into(),
            global: global.into(),
            result_type: result_type.into(),
        });
        self.define_value_with_place(destination, result_type, Place::global(global.into()));
        destination
    }

    /// Load one global value through its address.
    pub fn load_global(&mut self, global: LocalNodeId<Global>) -> Value {
        let global_ty = concrete_type_reference(&self.tree.get(global).ty, "global load type");
        let global_ty = self.expect_build(global_ty);
        let global_space = self.tree.get(global).space.clone();
        let global_pointer = self.type_reference(
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
            destination: destination.into(),
            pointer: pointer_value.into(),
            result_type: result_type.into(),
        });
        self.define_value_with_place(destination, result_type, Place::value(destination.into()));
        destination
    }

    /// Store to a pointer.
    pub fn store(&mut self, pointer_value: Value, value: Value) {
        self.insert_instruction(Instruction::Store {
            pointer: pointer_value.into(),
            value: value.into(),
        });
    }

    /// Resolve a field type for a struct or tuple aggregate.
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

                concrete_type_reference(&self.tree.get(*field_id).ty, "struct field type")
            }
            Type::Tuple { elements, .. } => {
                let Some(element) = elements.get(index as usize) else {
                    return Err(BuildError::InvalidFieldIndex {
                        aggregate: aggregate_type,
                        index,
                    });
                };

                concrete_type_reference(element, "tuple field type")
            }
            Type::Variant { tag, storage, .. } => match index {
                0 => concrete_type_reference(tag, "variant tag type"),
                1 => concrete_type_reference(storage, "variant storage type"),
                _ => Err(BuildError::InvalidFieldIndex {
                    aggregate: aggregate_type,
                    index,
                }),
            },
            Type::Closure { .. } => Err(BuildError::InvalidFieldOwner { ty: aggregate_type }),
            _ => Err(BuildError::InvalidFieldOwner { ty: aggregate_type }),
        }
    }

    /// Resolve the element type for one indexed collection.
    pub(super) fn element_type_for_array(
        &self,
        array_type: LocalNodeId<Type>,
    ) -> BuildResult<LocalNodeId<Type>> {
        let array = self.tree.get(array_type);
        match array {
            Type::Array { element, .. } | Type::Slice { element, .. } => {
                concrete_type_reference(element, "array element type")
            }
            _ => Err(BuildError::InvalidElementOwner { ty: array_type }),
        }
    }

    /// Resolve the element type for a vector type.
    pub(super) fn element_type_for_vector(
        &self,
        vector_type: LocalNodeId<Type>,
    ) -> BuildResult<LocalNodeId<Type>> {
        let vector = self.tree.get(vector_type);
        match vector {
            Type::Vector { element, .. } => concrete_type_reference(element, "vector element type"),
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
            Type::Tensor { element, .. } => concrete_type_reference(element, "tensor element type"),
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
            Type::TensorView { element, .. } => {
                concrete_type_reference(element, "tensor view element type")
            }
            _ => Err(BuildError::InvalidTensorViewOwner {
                ty: reference_type_id,
            }),
        }
    }

    /// Convert a list length into u16 for instruction metadata.
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
            Type::FunctionSignature { result, .. } => {
                concrete_type_reference(result, "function result")
            }
            Type::FunctionPointer { .. } | Type::Closure { .. } => {
                let Some(signature_id) =
                    callable_signature(signature_type).and_then(|signature| signature.ty())
                else {
                    return Err(BuildError::MissingFunctionSignature {
                        ty: signature_type_id,
                    });
                };
                let signature_type = self.tree.get(signature_id);
                let Some((_, result)) = function_signature_parts(signature_type) else {
                    return Err(BuildError::MissingFunctionSignature { ty: signature_id });
                };

                concrete_type_reference(&result, "callable function result")
            }
            _ => Err(BuildError::MissingFunctionSignature {
                ty: signature_type_id,
            }),
        }
    }

    /// Create a linear uninitialized allocation token type.
    pub fn type_uninit(&mut self, value: LocalNodeId<Type>) -> LocalNodeId<Type> {
        self.tree.insert_type(Type::Uninit {
            value: value.into(),
        })
    }

    /// Allocate zeroed heap storage.
    pub fn new_zeroed(
        &mut self,
        layout: LocalNodeId<Type>,
        result_type: LocalNodeId<Type>,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::NewZeroed {
            destination: destination.into(),
            layout: layout.into(),
            result_type: result_type.into(),
        });
        self.define_value_with_place(destination, result_type, Place::value(destination.into()));
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
            destination: destination.into(),
            layout: layout.into(),
            result_type: result_type.into(),
        });
        self.define_value_with_place(destination, result_type, Place::value(destination.into()));
        destination
    }

    /// Complete one initialized heap allocation.
    pub fn new_complete(&mut self, value: Value, result_type: LocalNodeId<Type>) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::NewComplete {
            destination: destination.into(),
            value: value.into(),
            result_type: result_type.into(),
        });
        self.define_value_with_place(destination, result_type, Place::value(destination.into()));
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
            destination: destination.into(),
            element: element.into(),
            length: length.into(),
            result_type: result_type.into(),
        });
        self.define_value_with_place(destination, result_type, Place::value(destination.into()));
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
            destination: destination.into(),
            element: element.into(),
            length: length.into(),
            result_type: result_type.into(),
        });
        self.define_value_with_place(destination, result_type, Place::value(destination.into()));
        destination
    }

    /// Free unique heap storage after drop elaboration.
    pub fn free(&mut self, value: Value) {
        self.insert_instruction(Instruction::Free {
            value: value.into(),
        });
    }

    /// Allocate zeroed frame storage.
    pub fn frame_alloc_zeroed(
        &mut self,
        layout: LocalNodeId<Type>,
        result_type: LocalNodeId<Type>,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::FrameAllocZeroed {
            destination: destination.into(),
            layout: layout.into(),
            result_type: result_type.into(),
        });
        self.define_value_with_place(destination, result_type, Place::value(destination.into()));
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
            destination: destination.into(),
            layout: layout.into(),
            result_type: result_type.into(),
        });
        self.define_value_with_place(destination, result_type, Place::value(destination.into()));
        destination
    }

    // instruction builders: assumptions

    /// Assume a condition is true (UB if false).
    pub fn assume(&mut self, condition: Value) {
        self.insert_instruction(Instruction::Assume {
            condition: condition.into(),
        });
    }
}

fn concrete_type_reference(
    reference: &TypeReference,
    context: &str,
) -> BuildResult<LocalNodeId<Type>> {
    match reference {
        TypeReference::Type { ty, .. } => Ok(*ty),
        TypeReference::Missing => Err(BuildError::MissingTypeReference {
            context: context.to_string(),
        }),
        TypeReference::Error => Err(BuildError::ErrorTypeReference {
            context: context.to_string(),
        }),
    }
}
