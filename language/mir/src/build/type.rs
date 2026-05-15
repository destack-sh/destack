use destack_core::StringId;

use crate::build::ModuleBuilder;
use crate::{
    Access, AddressSpace, Constant, Copy, Field, FloatType, Lifetime, LocalNodeId, Nullability,
    ReferenceKind, TensorDimension, TensorLayout, TensorViewLayout, Type, TypeReference,
    VariantCase,
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
        let float_type = match width {
            32 => FloatType::Float32,
            64 => FloatType::Float64,
            _ => panic!("unsupported float width: {width}"),
        };

        self.tree.insert_type(Type::Float(float_type))
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

    /// Create an erased interface value type.
    pub fn type_any(&mut self, interface: LocalNodeId<Type>) -> LocalNodeId<Type> {
        self.tree.insert_type(Type::Any {
            interface: interface.into(),
        })
    }

    /// Create a reference type.
    pub fn type_reference(
        &mut self,
        kind: ReferenceKind,
        pointee: LocalNodeId<Type>,
        access: Access,
        address_space: AddressSpace,
        nullability: Nullability,
    ) -> LocalNodeId<Type> {
        self.type_reference_with_lifetime(
            kind,
            Lifetime::empty(),
            pointee,
            access,
            address_space,
            nullability,
        )
    }

    /// Create a reference type with an explicit lifetime.
    pub fn type_reference_with_lifetime(
        &mut self,
        kind: ReferenceKind,
        lifetime: Lifetime,
        pointee: LocalNodeId<Type>,
        access: Access,
        address_space: AddressSpace,
        nullability: Nullability,
    ) -> LocalNodeId<Type> {
        self.tree.insert_type(Type::Reference {
            kind,
            lifetime,
            address_space,
            access,
            pointee: pointee.into(),
            nullability,
        })
    }

    /// Create a borrowed reference type.
    pub fn type_borrowed_reference(
        &mut self,
        pointee: LocalNodeId<Type>,
        access: Access,
    ) -> LocalNodeId<Type> {
        self.type_reference(
            ReferenceKind::Borrowed,
            pointee,
            access,
            AddressSpace::Local,
            Nullability::None,
        )
    }

    /// Create a raw pointer type (manual memory management).
    pub fn type_raw_pointer(&mut self, pointee: LocalNodeId<Type>) -> LocalNodeId<Type> {
        self.type_raw_pointer_with_access(pointee, Access::Readonly)
    }

    /// Create a raw pointer type with explicit access.
    pub fn type_raw_pointer_with_access(
        &mut self,
        pointee: LocalNodeId<Type>,
        access: Access,
    ) -> LocalNodeId<Type> {
        self.type_reference(
            ReferenceKind::Raw,
            pointee,
            access,
            AddressSpace::Local,
            Nullability::None,
        )
    }

    /// Create a mutable raw pointer type.
    pub fn type_raw_pointer_mutable(&mut self, pointee: LocalNodeId<Type>) -> LocalNodeId<Type> {
        self.type_raw_pointer_with_access(pointee, Access::Mutable)
    }

    /// Create a managed reference type (runtime-tracked).
    pub fn type_managed_reference(&mut self, pointee: LocalNodeId<Type>) -> LocalNodeId<Type> {
        self.type_managed_reference_with_access(pointee, Access::Readonly)
    }

    /// Create a managed reference type with explicit access.
    pub fn type_managed_reference_with_access(
        &mut self,
        pointee: LocalNodeId<Type>,
        access: Access,
    ) -> LocalNodeId<Type> {
        self.type_reference(
            ReferenceKind::Managed,
            pointee,
            access,
            AddressSpace::Local,
            Nullability::None,
        )
    }

    /// Create a mutable managed reference type.
    pub fn type_managed_reference_mutable(
        &mut self,
        pointee: LocalNodeId<Type>,
    ) -> LocalNodeId<Type> {
        self.type_managed_reference_with_access(pointee, Access::Mutable)
    }

    /// Create a nullable managed reference type.
    pub fn type_managed_reference_nullable(
        &mut self,
        pointee: LocalNodeId<Type>,
    ) -> LocalNodeId<Type> {
        self.type_managed_reference_nullable_with_access(pointee, Access::Readonly)
    }

    /// Create a nullable managed reference type with explicit access.
    pub fn type_managed_reference_nullable_with_access(
        &mut self,
        pointee: LocalNodeId<Type>,
        access: Access,
    ) -> LocalNodeId<Type> {
        self.type_reference(
            ReferenceKind::Managed,
            pointee,
            access,
            AddressSpace::Local,
            Nullability::Null,
        )
    }

    /// Create a mutable nullable managed reference type.
    pub fn type_managed_reference_nullable_mutable(
        &mut self,
        pointee: LocalNodeId<Type>,
    ) -> LocalNodeId<Type> {
        self.type_managed_reference_nullable_with_access(pointee, Access::Mutable)
    }

    /// Create a unique typed heap reference.
    pub fn type_unique_reference(&mut self, pointee: LocalNodeId<Type>) -> LocalNodeId<Type> {
        self.type_unique_reference_with_access(pointee, Access::Mutable)
    }

    /// Create a unique typed heap reference with explicit access.
    pub fn type_unique_reference_with_access(
        &mut self,
        pointee: LocalNodeId<Type>,
        access: Access,
    ) -> LocalNodeId<Type> {
        self.type_reference(
            ReferenceKind::Unique,
            pointee,
            access,
            AddressSpace::Local,
            Nullability::None,
        )
    }

    /// Create a vector type.
    pub fn type_vector(
        &mut self,
        element: LocalNodeId<Type>,
        lanes: u32,
        copy: Copy,
    ) -> LocalNodeId<Type> {
        self.tree.insert_type(Type::Vector {
            element: element.into(),
            lanes,
            copy,
        })
    }

    /// Create a tensor type.
    pub fn type_tensor(
        &mut self,
        element: LocalNodeId<Type>,
        shape: Vec<TensorDimension>,
        layout: TensorLayout,
        copy: Copy,
    ) -> LocalNodeId<Type> {
        self.tree.insert_type(Type::Tensor {
            element: element.into(),
            shape,
            layout,
            copy,
        })
    }

    /// Create a tensor view type.
    pub fn type_tensor_view(
        &mut self,
        kind: ReferenceKind,
        element: LocalNodeId<Type>,
        access: Access,
        address_space: AddressSpace,
        shape: Vec<TensorDimension>,
        layout: TensorViewLayout,
        nullability: Nullability,
    ) -> LocalNodeId<Type> {
        self.tree.insert_type(Type::TensorView {
            kind,
            lifetime: Lifetime::empty(),
            address_space,
            access,
            element: element.into(),
            shape,
            layout,
            nullability,
        })
    }

    /// Create an array type with explicit copy.
    pub fn type_array(
        &mut self,
        element: LocalNodeId<Type>,
        length: u64,
        copy: Copy,
    ) -> LocalNodeId<Type> {
        self.tree.insert_type(Type::Array {
            element: element.into(),
            length,
            copy,
        })
    }

    /// Create a slice type with explicit storage semantics.
    pub fn type_slice_with(
        &mut self,
        kind: ReferenceKind,
        element: LocalNodeId<Type>,
        access: Access,
        address_space: AddressSpace,
    ) -> LocalNodeId<Type> {
        self.type_slice_with_lifetime(kind, element, Lifetime::empty(), access, address_space)
    }

    /// Create a slice type with explicit storage semantics and lifetime.
    pub fn type_slice_with_lifetime(
        &mut self,
        kind: ReferenceKind,
        element: LocalNodeId<Type>,
        lifetime: Lifetime,
        access: Access,
        address_space: AddressSpace,
    ) -> LocalNodeId<Type> {
        self.tree.insert_type(Type::Slice {
            kind,
            lifetime,
            element: element.into(),
            address_space,
            access,
            nullability: Nullability::None,
        })
    }

    /// Create a local mutable slice type.
    pub fn type_slice(&mut self, element: LocalNodeId<Type>) -> LocalNodeId<Type> {
        self.type_slice_with(
            ReferenceKind::Managed,
            element,
            Access::Mutable,
            AddressSpace::Local,
        )
    }

    /// Create a tuple type with explicit copy.
    pub fn type_tuple(
        &mut self,
        elements: Vec<LocalNodeId<Type>>,
        copy: Copy,
    ) -> LocalNodeId<Type> {
        let elements = elements.into_iter().map(TypeReference::from).collect();

        self.tree.insert_type(Type::Tuple { elements, copy })
    }

    /// Create a struct type with explicit copy.
    pub fn type_struct(
        &mut self,
        fields: Vec<LocalNodeId<Field>>,
        copy: Copy,
    ) -> LocalNodeId<Type> {
        self.tree.insert_type(Type::Struct { fields, copy })
    }

    /// Create a physical variant type with explicit copy.
    pub fn type_variant(
        &mut self,
        tag: LocalNodeId<Type>,
        storage: LocalNodeId<Type>,
        cases: Vec<(Constant, LocalNodeId<Type>)>,
        copy: Copy,
    ) -> LocalNodeId<Type> {
        let cases = cases
            .into_iter()
            .map(|(tag, ty)| VariantCase { tag, ty: ty.into() })
            .collect();

        self.tree.insert_type(Type::Variant {
            tag: tag.into(),
            storage: storage.into(),
            cases,
            copy,
        })
    }

    /// Create a field definition for a struct type.
    pub fn field(&mut self, name: Option<StringId>, ty: LocalNodeId<Type>) -> LocalNodeId<Field> {
        self.tree.insert(Field {
            name,
            ty: ty.into(),
        })
    }

    /// Create a bare function signature type.
    pub fn type_function_signature(
        &mut self,
        parameters: Vec<LocalNodeId<Type>>,
        result: LocalNodeId<Type>,
    ) -> LocalNodeId<Type> {
        let parameters = parameters.into_iter().map(TypeReference::from).collect();

        self.tree.insert_type(Type::FunctionSignature {
            parameters,
            result: result.into(),
        })
    }

    /// Create a function pointer type.
    pub fn type_function_pointer(&mut self, signature: LocalNodeId<Type>) -> LocalNodeId<Type> {
        self.tree.insert_type(Type::FunctionPointer {
            signature: signature.into(),
        })
    }

    /// Create an opaque callable type.
    pub fn type_callable(&mut self, signature: LocalNodeId<Type>) -> LocalNodeId<Type> {
        self.tree.ensure_callable_environment_type();
        self.tree.insert_type(Type::Callable {
            signature: signature.into(),
        })
    }
}
