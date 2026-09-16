use destack_core::StringId;

use crate::build::ModuleBuilder;
use crate::{
    Access, Constant, Copy, Field, FieldId, FloatType, Lifetime, Multiplicity, Reference,
    SignatureParameter, Static, Storage, Type, TypeId, VariantCase,
};

#[allow(clippy::too_many_arguments)]
impl ModuleBuilder {
    /// Create a void type.
    pub fn type_void(&mut self) -> TypeId {
        self.tree.intern_type(Type::Void, Copy::Yes)
    }

    /// Create a boolean type.
    pub fn type_boolean(&mut self) -> TypeId {
        self.tree.intern_type(Type::Boolean, Copy::Yes)
    }

    /// Create a character type.
    pub fn type_character(&mut self) -> TypeId {
        self.tree.intern_type(Type::Character, Copy::Yes)
    }

    /// Create an integer type.
    pub fn type_int(&mut self, width: u16, signed: bool) -> TypeId {
        self.tree.intern_type(
            Type::Int {
                width,
                is_signed: signed,
            },
            Copy::Yes,
        )
    }

    /// Create a float type.
    pub fn type_float(&mut self, float_type: FloatType) -> TypeId {
        self.tree.intern_type(Type::Float(float_type), Copy::Yes)
    }
    /// Create a dynamic erased value type.
    pub fn type_dynamic(
        &mut self,
        kind: Reference,
        lifetime: Lifetime,
        constraint: TypeId,
        access: Access,
        storage: Storage,
    ) -> TypeId {
        self.tree.intern_type(
            Type::Dynamic {
                kind,
                lifetime,
                constraint,
                storage,
                access,
            },
            kind.copy(),
        )
    }

    /// Create a reference type.
    pub fn type_reference(
        &mut self,
        kind: Reference,
        lifetime: Lifetime,
        pointee: TypeId,
        access: Access,
        storage: Storage,
    ) -> TypeId {
        self.tree.intern_type(
            Type::Reference {
                kind,
                lifetime,
                storage,
                access,
                pointee,
            },
            kind.copy(),
        )
    }

    /// Create a process-local machine pointer type.
    pub fn type_pointer(&mut self, pointee: TypeId, access: Access) -> TypeId {
        self.tree.intern_type(Type::Pointer { pointee, access }, Copy::Yes)
    }

    /// Create a fixed array type.
    pub fn type_fixed_array(&mut self, element: TypeId, length: u64) -> TypeId {
        let length = i64::try_from(length)
            .unwrap_or_else(|_| unreachable!("fixed array length {length} exceeds MIR range"));
        let length = self.tree.intern_static(Static::Integer(length));

        let copy = self.tree.copy(element);

        self.tree.intern_type(Type::FixedArray { element, length }, copy)
    }

    /// Create a slice type.
    pub fn type_slice(
        &mut self,
        kind: Reference,
        lifetime: Lifetime,
        element: TypeId,
        access: Access,
        storage: Storage,
    ) -> TypeId {
        self.tree.intern_type(
            Type::Slice {
                kind,
                lifetime,
                element,
                storage,
                access,
            },
            kind.copy(),
        )
    }

    /// Create a tuple type with explicit copy.
    pub fn type_tuple(&mut self, elements: Vec<TypeId>, copy: Copy) -> TypeId {
        let elements = elements.into_iter().collect();

        self.tree.intern_type(Type::Tuple { elements }, copy)
    }

    /// Create a struct type with explicit copy.
    pub fn type_struct(&mut self, fields: Vec<FieldId>, copy: Copy) -> TypeId {
        self.tree.intern_type(Type::Struct { fields }, copy)
    }

    /// Create a sum type with explicit copy.
    pub fn type_variant(
        &mut self,
        discriminant: TypeId,
        cases: Vec<(Constant, TypeId)>,
        copy: Copy,
    ) -> TypeId {
        let cases = cases
            .into_iter()
            .map(|(discriminant, ty)| VariantCase { discriminant, ty })
            .collect();

        self.tree.intern_type(
            Type::Variant {
                discriminant,
                cases,
            },
            copy,
        )
    }

    /// Create a field definition for a struct type.
    pub fn field(&mut self, name: Option<StringId>, ty: TypeId) -> FieldId {
        self.tree.intern_field(Field {
            name,
            ty,
            attributes: Vec::new(),
        })
    }

    /// Create a bare function signature type.
    pub fn type_function_signature(&mut self, parameters: Vec<TypeId>, result: TypeId) -> TypeId {
        let parameters = parameters
            .into_iter()
            .map(SignatureParameter::new)
            .collect();

        self.tree.intern_type(
            Type::FunctionSignature {
                lifetimes: Vec::new(),
                parameters,
                result,
            },
            Copy::Yes,
        )
    }

    /// Create a function pointer type.
    pub fn type_function_pointer(&mut self, signature: TypeId) -> TypeId {
        self.tree.intern_type(Type::FunctionPointer { signature }, Copy::Yes)
    }

    /// Create a function value type.
    pub fn type_function(
        &mut self,
        signature: TypeId,
        multiplicity: Multiplicity,
        kind: Reference,
        lifetime: Lifetime,
        storage: Storage,
        access: Access,
    ) -> TypeId {
        self.tree.intern_type(
            Type::Function {
                signature,
                multiplicity,
                kind,
                lifetime,
                storage,
                access,
            },
            kind.copy(),
        )
    }
}
