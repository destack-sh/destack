use destack_core::StringId;

use crate::build::ModuleBuilder;
use crate::{
    AddressSpace, Copyability, Field, LocalNodeId, Mutability, ReferenceKind, TensorDimension,
    TensorLayout, Type, TypeReference,
};

#[allow(clippy::too_many_arguments)]
impl ModuleBuilder {
    /// Create a void type.
    pub fn type_void(&mut self) -> LocalNodeId<Type> {
        self.tree.insert_type(Type::Void)
    }

    /// Create a boolean type.
    pub fn type_boolean(&mut self) -> LocalNodeId<Type> {
        self.tree.insert_type(Type::Boolean)
    }

    /// Create an integer type.
    pub fn type_int(&mut self, width: u16, signed: bool) -> LocalNodeId<Type> {
        self.tree.insert_type(Type::Int {
            width,
            is_signed: signed,
        })
    }

    /// Create a pointer-sized signed integer type.
    pub fn type_isize(&mut self) -> LocalNodeId<Type> {
        self.tree.insert_type(Type::Isize)
    }

    /// Create a pointer-sized unsigned integer type.
    pub fn type_usize(&mut self) -> LocalNodeId<Type> {
        self.tree.insert_type(Type::Usize)
    }

    /// Create a 32-bit signed integer type.
    pub fn type_i32(&mut self) -> LocalNodeId<Type> {
        self.type_int(32, true)
    }

    /// Create a 64-bit signed integer type.
    pub fn type_i64(&mut self) -> LocalNodeId<Type> {
        self.type_int(64, true)
    }

    /// Create a 32-bit unsigned integer type.
    pub fn type_u32(&mut self) -> LocalNodeId<Type> {
        self.type_int(32, false)
    }

    /// Create a 64-bit unsigned integer type.
    pub fn type_u64(&mut self) -> LocalNodeId<Type> {
        self.type_int(64, false)
    }

    /// Create a float type.
    pub fn type_float(&mut self, width: u16) -> LocalNodeId<Type> {
        self.tree.insert_type(Type::Float { width })
    }

    /// Create a 32-bit float type.
    pub fn type_f32(&mut self) -> LocalNodeId<Type> {
        self.type_float(32)
    }

    /// Create a 64-bit float type.
    pub fn type_f64(&mut self) -> LocalNodeId<Type> {
        self.type_float(64)
    }

    /// Create a type descriptor handle type.
    pub fn type_type_descriptor(&mut self) -> LocalNodeId<Type> {
        self.tree.insert_type(Type::TypeDescriptor)
    }

    /// Create a type id value type.
    pub fn type_type_id(&mut self) -> LocalNodeId<Type> {
        self.tree.insert_type(Type::TypeId)
    }

    /// Create a reference type.
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

    /// Create a borrowed reference type.
    pub fn type_borrowed_reference(
        &mut self,
        pointee: LocalNodeId<Type>,
        mutability: Mutability,
    ) -> LocalNodeId<Type> {
        self.type_reference(
            ReferenceKind::Borrowed,
            pointee,
            mutability,
            AddressSpace::Local,
            false,
        )
    }

    /// Create an owning handle type.
    pub fn type_owned_reference(
        &mut self,
        pointee: LocalNodeId<Type>,
        mutability: Mutability,
    ) -> LocalNodeId<Type> {
        self.type_reference(
            ReferenceKind::Owned,
            pointee,
            mutability,
            AddressSpace::Local,
            false,
        )
    }

    /// Create an owning mutable handle type.
    pub fn type_owned_reference_mutable(
        &mut self,
        pointee: LocalNodeId<Type>,
    ) -> LocalNodeId<Type> {
        self.type_owned_reference(pointee, Mutability::Mutable)
    }

    /// Create an owning readonly handle type.
    pub fn type_owned_reference_readonly(
        &mut self,
        pointee: LocalNodeId<Type>,
    ) -> LocalNodeId<Type> {
        self.type_owned_reference(pointee, Mutability::Immutable)
    }

    /// Create a raw pointer type (manual memory management).
    pub fn type_raw_pointer(&mut self, pointee: LocalNodeId<Type>) -> LocalNodeId<Type> {
        self.type_raw_pointer_with_mutability(pointee, Mutability::Immutable)
    }

    /// Create a raw pointer type with explicit mutability.
    pub fn type_raw_pointer_with_mutability(
        &mut self,
        pointee: LocalNodeId<Type>,
        mutability: Mutability,
    ) -> LocalNodeId<Type> {
        self.type_reference(
            ReferenceKind::Raw,
            pointee,
            mutability,
            AddressSpace::Local,
            false,
        )
    }

    /// Create a mutable raw pointer type.
    pub fn type_raw_pointer_mutable(&mut self, pointee: LocalNodeId<Type>) -> LocalNodeId<Type> {
        self.type_raw_pointer_with_mutability(pointee, Mutability::Mutable)
    }

    /// Create a managed reference type (runtime-tracked).
    pub fn type_managed_reference(&mut self, pointee: LocalNodeId<Type>) -> LocalNodeId<Type> {
        self.type_managed_reference_with_mutability(pointee, Mutability::Immutable)
    }

    /// Create a managed reference type with explicit mutability.
    pub fn type_managed_reference_with_mutability(
        &mut self,
        pointee: LocalNodeId<Type>,
        mutability: Mutability,
    ) -> LocalNodeId<Type> {
        self.type_reference(
            ReferenceKind::Managed,
            pointee,
            mutability,
            AddressSpace::Local,
            false,
        )
    }

    /// Create a mutable managed reference type.
    pub fn type_managed_reference_mutable(
        &mut self,
        pointee: LocalNodeId<Type>,
    ) -> LocalNodeId<Type> {
        self.type_managed_reference_with_mutability(pointee, Mutability::Mutable)
    }

    /// Create a nullable managed reference type.
    pub fn type_managed_reference_nullable(
        &mut self,
        pointee: LocalNodeId<Type>,
    ) -> LocalNodeId<Type> {
        self.type_managed_reference_nullable_with_mutability(pointee, Mutability::Immutable)
    }

    /// Create a nullable managed reference type with explicit mutability.
    pub fn type_managed_reference_nullable_with_mutability(
        &mut self,
        pointee: LocalNodeId<Type>,
        mutability: Mutability,
    ) -> LocalNodeId<Type> {
        self.type_reference(
            ReferenceKind::Managed,
            pointee,
            mutability,
            AddressSpace::Local,
            true,
        )
    }

    /// Create a mutable nullable managed reference type.
    pub fn type_managed_reference_nullable_mutable(
        &mut self,
        pointee: LocalNodeId<Type>,
    ) -> LocalNodeId<Type> {
        self.type_managed_reference_nullable_with_mutability(pointee, Mutability::Mutable)
    }

    /// Create a vector type.
    pub fn type_vector(
        &mut self,
        element: LocalNodeId<Type>,
        lanes: u32,
        copyability: Copyability,
    ) -> LocalNodeId<Type> {
        self.tree.insert_type(Type::Vector {
            element: element.into(),
            lanes,
            copyability,
        })
    }

    /// Create a tensor type.
    pub fn type_tensor(
        &mut self,
        element: LocalNodeId<Type>,
        shape: Vec<TensorDimension>,
        layout: TensorLayout,
        copyability: Copyability,
    ) -> LocalNodeId<Type> {
        self.tree.insert_type(Type::Tensor {
            element: element.into(),
            shape,
            layout,
            copyability,
        })
    }

    /// Create a tensor reference type.
    pub fn type_tensor_reference(
        &mut self,
        kind: ReferenceKind,
        element: LocalNodeId<Type>,
        mutability: Mutability,
        address_space: AddressSpace,
        shape: Vec<TensorDimension>,
        layout: TensorLayout,
        is_nullable: bool,
    ) -> LocalNodeId<Type> {
        self.tree.insert_type(Type::TensorReference {
            kind,
            address_space,
            mutability,
            element: element.into(),
            shape,
            layout,
            is_nullable,
        })
    }

    /// Create an array type with explicit copyability.
    pub fn type_array(
        &mut self,
        element: LocalNodeId<Type>,
        length: u64,
        copyability: Copyability,
    ) -> LocalNodeId<Type> {
        self.tree.insert_type(Type::Array {
            element: element.into(),
            length,
            copyability,
        })
    }

    /// Create a dynamic array type with explicit copyability.
    pub fn type_dynamic_array(
        &mut self,
        element: LocalNodeId<Type>,
        copyability: Copyability,
    ) -> LocalNodeId<Type> {
        self.tree.insert_type(Type::DynamicArray {
            element: element.into(),
            copyability,
        })
    }

    /// Create a tuple type with explicit copyability.
    pub fn type_tuple(
        &mut self,
        elements: Vec<LocalNodeId<Type>>,
        copyability: Copyability,
    ) -> LocalNodeId<Type> {
        let elements = elements.into_iter().map(TypeReference::from).collect();

        self.tree.insert_type(Type::Tuple {
            elements,
            copyability,
        })
    }

    /// Create a struct type with explicit copyability.
    pub fn type_struct(
        &mut self,
        fields: Vec<LocalNodeId<Field>>,
        copyability: Copyability,
    ) -> LocalNodeId<Type> {
        self.tree.insert_type(Type::Struct {
            fields,
            copyability,
        })
    }

    /// Create a field definition for a struct type.
    pub fn field(&mut self, name: Option<StringId>, ty: LocalNodeId<Type>) -> LocalNodeId<Field> {
        self.tree.insert(Field {
            name,
            ty: ty.into(),
        })
    }

    /// Create a function pointer type.
    pub fn type_function_pointer(
        &mut self,
        parameters: Vec<LocalNodeId<Type>>,
        result: LocalNodeId<Type>,
    ) -> LocalNodeId<Type> {
        let parameters = parameters.into_iter().map(TypeReference::from).collect();

        self.tree.insert_type(Type::FunctionPointer {
            parameters,
            result: result.into(),
        })
    }

    /// Create a callable function value type.
    pub fn type_function_value(&mut self, signature: LocalNodeId<Type>) -> LocalNodeId<Type> {
        self.tree.ensure_function_value_environment_type();
        self.tree.insert_type(Type::Closure {
            signature: signature.into(),
        })
    }
}
