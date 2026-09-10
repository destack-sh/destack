use destack_core::StringId;

use crate::build::ModuleBuilder;
use crate::{
    Access, Constant, Copy, Field, FloatType, Lifetime, LocalNodeId, Multiplicity, Reference,
    Static, Storage, Type, TypeId, VariantCase,
};

#[allow(clippy::too_many_arguments)]
impl ModuleBuilder {
    /// Create a void type.
    pub fn type_void(&mut self) -> LocalNodeId<Type> {
        self.tree.intern_type(Type::Void)
    }

    /// Create a boolean type.
    pub fn type_boolean(&mut self) -> LocalNodeId<Type> {
        self.tree.intern_type(Type::Boolean)
    }

    /// Create a character type.
    pub fn type_character(&mut self) -> LocalNodeId<Type> {
        self.tree.intern_type(Type::Character)
    }

    /// Create an integer type.
    pub fn type_int(&mut self, width: u16, signed: bool) -> LocalNodeId<Type> {
        self.tree.intern_type(Type::Int {
            width,
            is_signed: signed,
        })
    }

    /// Create a pointer-sized signed integer type.
    pub fn type_isize(&mut self) -> LocalNodeId<Type> {
        self.tree.intern_type(Type::Isize)
    }

    /// Create a pointer-sized unsigned integer type.
    pub fn type_usize(&mut self) -> LocalNodeId<Type> {
        self.tree.intern_type(Type::Usize)
    }

    /// Create a float type.
    pub fn type_float(&mut self, float_type: FloatType) -> LocalNodeId<Type> {
        self.tree.intern_type(Type::Float(float_type))
    }
    /// Create a type id value type.
    pub fn type_type_id(&mut self) -> LocalNodeId<Type> {
        self.tree.intern_type(Type::TypeId)
    }

    /// Create a dynamic erased value type.
    pub fn type_dynamic(
        &mut self,
        kind: Reference,
        lifetime: Lifetime,
        constraint: LocalNodeId<Type>,
        access: Access,
        storage: Storage,
    ) -> LocalNodeId<Type> {
        self.tree.intern_type(Type::Dynamic {
            kind,
            lifetime,
            constraint,
            storage,
            access,
        })
    }

    /// Create a reference type.
    pub fn type_reference(
        &mut self,
        kind: Reference,
        lifetime: Lifetime,
        pointee: LocalNodeId<Type>,
        access: Access,
        storage: Storage,
    ) -> LocalNodeId<Type> {
        self.tree.intern_type(Type::Reference {
            kind,
            lifetime,
            storage,
            access,
            pointee,
        })
    }

    /// Create a process-local machine pointer type.
    pub fn type_pointer(
        &mut self,
        pointee: LocalNodeId<Type>,
        access: Access,
    ) -> LocalNodeId<Type> {
        self.tree.intern_type(Type::Pointer { pointee, access })
    }

    /// Create a vector type.
    pub fn type_vector(&mut self, element: LocalNodeId<Type>, lanes: u32) -> LocalNodeId<Type> {
        let lanes = self.tree.intern_static(Static::Integer(i64::from(lanes)));

        self.tree.intern_type(Type::Vector { element, lanes })
    }

    /// Create a fixed array type.
    pub fn type_fixed_array(
        &mut self,
        element: LocalNodeId<Type>,
        length: u64,
    ) -> LocalNodeId<Type> {
        let length = i64::try_from(length)
            .unwrap_or_else(|_| unreachable!("fixed array length {length} exceeds MIR range"));
        let length = self.tree.intern_static(Static::Integer(length));

        self.tree.intern_type(Type::FixedArray { element, length })
    }

    /// Create a slice type.
    pub fn type_slice(
        &mut self,
        kind: Reference,
        lifetime: Lifetime,
        element: LocalNodeId<Type>,
        access: Access,
        storage: Storage,
    ) -> LocalNodeId<Type> {
        self.tree.intern_type(Type::Slice {
            kind,
            lifetime,
            element,
            storage,
            access,
        })
    }

    /// Create a tuple type.
    pub fn type_tuple(&mut self, elements: Vec<LocalNodeId<Type>>) -> LocalNodeId<Type> {
        let elements = elements.into_iter().collect();

        self.tree.intern_type(Type::Tuple { elements })
    }

    /// Create a struct type with explicit copy.
    pub fn type_struct(
        &mut self,
        fields: Vec<LocalNodeId<Field>>,
        copy: Copy,
    ) -> LocalNodeId<Type> {
        self.tree.intern_type(Type::Struct { fields, copy })
    }

    /// Create a sum type with explicit copy.
    pub fn type_variant(
        &mut self,
        discriminant: LocalNodeId<Type>,
        cases: Vec<(Constant, LocalNodeId<Type>)>,
        copy: Copy,
    ) -> LocalNodeId<Type> {
        let cases = cases
            .into_iter()
            .map(|(discriminant, ty)| VariantCase { discriminant, ty })
            .collect();

        self.tree.intern_type(Type::Variant {
            discriminant,
            cases,
            copy,
        })
    }

    /// Create a field definition for a struct type.
    pub fn field(&mut self, name: Option<StringId>, ty: LocalNodeId<Type>) -> LocalNodeId<Field> {
        self.tree.intern_field(Field { name, ty }, Vec::new())
    }

    /// Create a bare function signature type.
    pub fn type_function_signature(
        &mut self,
        parameters: Vec<LocalNodeId<Type>>,
        result: LocalNodeId<Type>,
    ) -> LocalNodeId<Type> {
        let parameters = parameters
            .into_iter()
            .map(|ty| crate::SignatureParameter::new(TypeId::from(ty)))
            .collect();

        self.tree.intern_type(Type::FunctionSignature {
            lifetimes: Vec::new(),
            parameters,
            result,
        })
    }

    /// Create a function pointer type.
    pub fn type_function_pointer(&mut self, signature: LocalNodeId<Type>) -> LocalNodeId<Type> {
        self.tree.intern_type(Type::FunctionPointer { signature })
    }

    /// Create a function value type.
    pub fn type_function(
        &mut self,
        signature: LocalNodeId<Type>,
        multiplicity: Multiplicity,
        kind: Reference,
        lifetime: Lifetime,
        storage: Storage,
        access: Access,
    ) -> LocalNodeId<Type> {
        self.tree.intern_type(Type::Function {
            signature,
            multiplicity,
            kind,
            lifetime,
            storage,
            access,
        })
    }
}
