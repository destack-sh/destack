use crate::build::FunctionBuilder;
use crate::{
    AddressSpace, Global, Instruction, Local, LocalNodeId, Mutability, Ownership, ReferenceKind,
    Type, TypeReference, Value, callable_signature, function_signature_parts,
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
        mutability: Mutability,
        address_space: AddressSpace,
        is_nullable: bool,
    ) -> LocalNodeId<Type> {
        self.tree.insert_type(Type::Reference {
            kind,
            address_space,
            mutability,
            pointee: pointee.into(),
            is_nullable,
        })
    }

    /// Load from a local variable.
    pub fn local_get(&mut self, local: LocalNodeId<Local>) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::LocalGet {
            destination: destination.into(),
            local: local.into(),
        });
        let local_ty = concrete_type_reference(self.tree.get(local).ty, "local.get local type");
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
        self.define_value(destination, result_type);
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
        self.define_value(destination, result_type);
        destination
    }

    /// Load the value of an immutable global constant.
    pub fn global_const(&mut self, global: LocalNodeId<Global>) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::GlobalConst {
            destination: destination.into(),
            global: global.into(),
        });
        let global_ty =
            concrete_type_reference(self.tree.get(global).ty, "global.const global type");
        self.define_value(destination, global_ty);
        destination
    }

    /// Load from a pointer.
    pub fn load(&mut self, pointer_value: Value, result_type: LocalNodeId<Type>) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::Load {
            destination: destination.into(),
            pointer: pointer_value.into(),
            result_type: result_type.into(),
        });
        self.define_value(destination, result_type);
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
    ) -> LocalNodeId<Type> {
        let aggregate = self.tree.get(aggregate_type);
        match aggregate {
            Type::Struct { fields, .. } => fields
                .get(index as usize)
                .map(|field_id| {
                    concrete_type_reference(self.tree.get(*field_id).ty, "struct field type")
                })
                .unwrap_or_else(|| panic!("field index out of bounds")),
            Type::Tuple { elements, .. } => elements
                .get(index as usize)
                .copied()
                .map(|element| concrete_type_reference(element, "tuple field type"))
                .unwrap_or_else(|| panic!("field index out of bounds")),
            Type::Closure { .. } => panic!("field access does not support callable"),
            _ => panic!("field access expects struct or tuple"),
        }
    }

    /// Resolve the element type for one indexed collection.
    pub(super) fn element_type_for_array(
        &self,
        array_type: LocalNodeId<Type>,
    ) -> LocalNodeId<Type> {
        let array = self.tree.get(array_type);
        match array {
            Type::Array { element, .. } | Type::Slice { element, .. } => {
                concrete_type_reference(*element, "array element type")
            }
            _ => panic!("element access expects one indexed collection type"),
        }
    }

    /// Resolve the element type for a vector type.
    pub(super) fn element_type_for_vector(
        &self,
        vector_type: LocalNodeId<Type>,
    ) -> LocalNodeId<Type> {
        let vector = self.tree.get(vector_type);
        match vector {
            Type::Vector { element, .. } => {
                concrete_type_reference(*element, "vector element type")
            }
            _ => panic!("vector access expects vector type"),
        }
    }

    /// Resolve the element type for a tensor view.
    pub(super) fn element_type_for_tensor_view(
        &self,
        reference_type: LocalNodeId<Type>,
    ) -> LocalNodeId<Type> {
        let reference_type = self.tree.get(reference_type);
        match reference_type {
            Type::TensorView { element, .. } => {
                concrete_type_reference(*element, "tensor view element type")
            }
            _ => panic!("tensor access expects tensor view type"),
        }
    }

    /// Convert a list length into u16 for instruction metadata.
    pub(super) fn to_u16_count(&self, count: usize, context: &str) -> u16 {
        u16::try_from(count).unwrap_or_else(|_| panic!("{context} is too large"))
    }

    /// Resolve the return type for a function signature.
    pub(super) fn signature_result_type(&self, signature: LocalNodeId<Type>) -> LocalNodeId<Type> {
        let signature_type = self.tree.get(signature);
        match signature_type {
            Type::FunctionSignature { result, .. } => {
                concrete_type_reference(*result, "function result")
            }
            Type::FunctionPointer { .. } | Type::Closure { .. } => {
                let signature = callable_signature(signature_type)
                    .and_then(TypeReference::ty)
                    .unwrap_or_else(|| panic!("callable must carry a function signature"));
                let signature_type = self.tree.get(signature);
                let (_, result) = function_signature_parts(signature_type)
                    .unwrap_or_else(|| panic!("callable must carry a function signature"));

                concrete_type_reference(result, "callable function result")
            }
            _ => panic!("call expects function signature"),
        }
    }

    /// Allocate heap storage.
    /// Returns a managed or owned reference type.
    pub fn new_(&mut self, layout: LocalNodeId<Type>, result_type: LocalNodeId<Type>) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::New {
            destination: destination.into(),
            layout: layout.into(),
            result_type: result_type.into(),
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Allocate repeated heap storage.
    /// Returns a slice value.
    pub fn new_slice(
        &mut self,
        element: LocalNodeId<Type>,
        length: Value,
        result_type: LocalNodeId<Type>,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::NewSlice {
            destination: destination.into(),
            element: element.into(),
            length: length.into(),
            result_type: result_type.into(),
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Allocate raw memory on the heap.
    /// Returns a raw reference type. Caller must free with `raw.free`.
    pub fn raw_alloc(
        &mut self,
        layout: LocalNodeId<Type>,
        result_type: LocalNodeId<Type>,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::RawAlloc {
            destination: destination.into(),
            layout: layout.into(),
            result_type: result_type.into(),
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Free raw heap memory previously allocated with `raw.alloc`.
    pub fn raw_free(&mut self, pointer: Value) {
        self.insert_instruction(Instruction::RawFree {
            pointer: pointer.into(),
        });
    }

    /// Allocate on the stack (lives until function returns).
    /// Returns a raw stack reference type.
    pub fn stack_alloc(
        &mut self,
        layout: LocalNodeId<Type>,
        result_type: LocalNodeId<Type>,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::StackAlloc {
            destination: destination.into(),
            layout: layout.into(),
            result_type: result_type.into(),
        });
        self.define_value(destination, result_type);
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

fn concrete_type_reference(reference: TypeReference, context: &str) -> LocalNodeId<Type> {
    match reference {
        TypeReference::Type(ty) => ty,
        TypeReference::Missing => panic!("missing type reference for {context}"),
        TypeReference::Error => panic!("malformed type reference for {context}"),
    }
}
