use destack_base::{ImmutableStringPool, StringId, StringPool};

#[cfg(any(test, debug_assertions))]
use crate::validate::{Validator, ValidatorOptions};
use crate::{
    AddressSpace, Copyability, Field, Function, Global, GlobalInitializer, LocalNodeId, Mutability,
    NodeTree, ReferenceKind, TensorDimension, TensorLayout, Type, TypedValue, Value,
};

use super::FunctionBuilder;

/// Builder for constructing a MIR module (collection of functions and types).
#[derive(Debug)]
pub struct ModuleBuilder {
    /// The node tree being built.
    tree: NodeTree,
    /// String pool for names.
    strings: StringPool,
    /// Whether to verify functions as they are built.
    verify: bool,
}

#[allow(clippy::too_many_arguments)]
impl ModuleBuilder {
    /// Create a new unverified module builder.
    pub fn unchecked() -> Self {
        Self::new_with_verify(false)
    }

    /// Create a new module builder with function verification enabled.
    pub fn checked() -> Self {
        Self::new_with_verify(true)
    }

    /// Create a new module builder with the given verify flag.
    pub fn new_with_verify(verify: bool) -> Self {
        Self {
            tree: NodeTree::new(),
            strings: StringPool::new(),
            verify,
        }
    }

    /// Get a reference to the node tree.
    pub fn tree(&self) -> &NodeTree {
        &self.tree
    }

    /// Get a mutable reference to the node tree.
    pub fn tree_mut(&mut self) -> &mut NodeTree {
        &mut self.tree
    }

    /// Get a reference to the string pool.
    pub fn strings(&self) -> &StringPool {
        &self.strings
    }

    /// Intern a string and return its id.
    pub fn intern(&mut self, s: &str) -> StringId {
        self.strings.intern(s)
    }

    // type constructors

    /// Create a void type.
    pub fn type_void(&mut self) -> LocalNodeId<Type> {
        self.tree.insert_type(Type::Void)
    }

    /// Create a boolean type.
    pub fn type_bool(&mut self) -> LocalNodeId<Type> {
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

    /// Create a type tag handle type.
    pub fn type_type_tag(&mut self) -> LocalNodeId<Type> {
        self.tree.insert_type(Type::Type)
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
            pointee,
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
            AddressSpace::Generic,
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
            AddressSpace::Generic,
            false,
        )
    }

    /// Create a raw pointer type (manual memory management).
    pub fn type_raw_pointer(&mut self, pointee: LocalNodeId<Type>) -> LocalNodeId<Type> {
        self.type_reference(
            ReferenceKind::Raw,
            pointee,
            Mutability::Immutable,
            AddressSpace::Generic,
            false,
        )
    }

    /// Create a managed reference type (runtime-tracked).
    pub fn type_managed_reference(&mut self, pointee: LocalNodeId<Type>) -> LocalNodeId<Type> {
        self.type_reference(
            ReferenceKind::Managed,
            pointee,
            Mutability::Immutable,
            AddressSpace::Generic,
            false,
        )
    }

    /// Create a nullable managed reference type.
    pub fn type_managed_reference_nullable(
        &mut self,
        pointee: LocalNodeId<Type>,
    ) -> LocalNodeId<Type> {
        self.type_reference(
            ReferenceKind::Managed,
            pointee,
            Mutability::Immutable,
            AddressSpace::Generic,
            true,
        )
    }

    /// Create a vector type.
    pub fn type_vector(
        &mut self,
        element: LocalNodeId<Type>,
        lanes: u32,
        copyability: Copyability,
    ) -> LocalNodeId<Type> {
        self.tree.insert_type(Type::Vector {
            element,
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
            element,
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
            element,
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
            element,
            length,
            copyability,
        })
    }

    /// Create a tuple type with explicit copyability.
    pub fn type_tuple(
        &mut self,
        elements: Vec<LocalNodeId<Type>>,
        copyability: Copyability,
    ) -> LocalNodeId<Type> {
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
    pub fn field(
        &mut self,
        name: Option<StringId>,
        ty: LocalNodeId<Type>,
        offset: u32,
    ) -> LocalNodeId<Field> {
        self.tree.insert(Field { name, ty, offset })
    }

    /// Create a function pointer type.
    pub fn type_function_pointer(
        &mut self,
        parameters: Vec<LocalNodeId<Type>>,
        result: LocalNodeId<Type>,
    ) -> LocalNodeId<Type> {
        self.tree
            .insert_type(Type::FunctionPointer { parameters, result })
    }

    // global building

    /// Create a global variable (mutable).
    pub fn global_variable(
        &mut self,
        name: &str,
        ty: LocalNodeId<Type>,
        init: GlobalInitializer,
    ) -> LocalNodeId<Global> {
        let name_id = self.strings.intern(name);
        self.tree
            .insert(Global::new(name_id, ty, Mutability::Mutable, init))
    }

    /// Create a global constant (immutable).
    pub fn global_constant(
        &mut self,
        name: &str,
        ty: LocalNodeId<Type>,
        init: GlobalInitializer,
    ) -> LocalNodeId<Global> {
        let name_id = self.strings.intern(name);
        self.tree
            .insert(Global::new(name_id, ty, Mutability::Immutable, init))
    }

    /// Create a global with explicit mutability.
    pub fn global(
        &mut self,
        name: &str,
        ty: LocalNodeId<Type>,
        mutability: Mutability,
        init: GlobalInitializer,
    ) -> LocalNodeId<Global> {
        let name_id = self.strings.intern(name);
        self.tree.insert(Global::new(name_id, ty, mutability, init))
    }

    /// Declare an external global (defined elsewhere).
    pub fn extern_global(
        &mut self,
        name: &str,
        ty: LocalNodeId<Type>,
        mutability: Mutability,
    ) -> LocalNodeId<Global> {
        let name_id = self.strings.intern(name);
        self.tree.insert(Global::import(name_id, ty, mutability))
    }

    // function building

    /// Start building a new function.
    pub fn function(
        &mut self,
        name: &str,
        parameter_types: &[LocalNodeId<Type>],
        return_type: LocalNodeId<Type>,
    ) -> FunctionBuilder<'_> {
        let name_id = self.strings.intern(name);
        FunctionBuilder::new(
            &mut self.tree,
            name_id,
            parameter_types,
            return_type,
            self.verify,
        )
    }

    /// Start building a body for an existing declared function.
    pub fn function_body(&mut self, function_id: LocalNodeId<Function>) -> FunctionBuilder<'_> {
        FunctionBuilder::from_declared(&mut self.tree, function_id, self.verify)
    }

    /// Declare a local function without a body.
    pub fn declare_function(
        &mut self,
        name: &str,
        parameter_types: &[LocalNodeId<Type>],
        return_type: LocalNodeId<Type>,
    ) -> LocalNodeId<Function> {
        let name_id = self.strings.intern(name);
        let parameters: Vec<TypedValue> = parameter_types
            .iter()
            .enumerate()
            .map(|(i, &ty)| TypedValue {
                value: Value::new(i as u32),
                ty,
            })
            .collect();
        self.tree
            .insert(Function::declare(name_id, parameters, return_type))
    }

    /// Declare an external function (defined elsewhere).
    pub fn extern_function(
        &mut self,
        name: &str,
        parameter_types: &[LocalNodeId<Type>],
        return_type: LocalNodeId<Type>,
    ) -> LocalNodeId<Function> {
        let name_id = self.strings.intern(name);
        // create typed parameters (external functions still need typed params for signature)
        let parameters: Vec<TypedValue> = parameter_types
            .iter()
            .enumerate()
            .map(|(i, &ty)| TypedValue {
                value: Value::new(i as u32),
                ty,
            })
            .collect();
        self.tree
            .insert(Function::import(name_id, parameters, return_type))
    }

    /// Finish building the module.
    pub fn finish_immutable(self) -> (NodeTree, ImmutableStringPool) {
        // keep the verify flag alive in release builds
        #[cfg(not(any(test, debug_assertions)))]
        let _ = self.verify;

        // validate in debug and test builds
        #[cfg(any(test, debug_assertions))]
        {
            if self.verify {
                // set up the validator
                let validator = Validator::new_with_options(&self.tree, ValidatorOptions::strict());

                // fail fast on invalid mir
                if let Err(error) = validator.validate_tree() {
                    panic!("mir validation failed: {error}");
                }
            }
        }

        (self.tree, self.strings.into_immutable())
    }

    /// Finish building the module with a mutable string pool.
    pub fn finish_mutable(self) -> (NodeTree, StringPool) {
        // keep the verify flag alive in release builds
        #[cfg(not(any(test, debug_assertions)))]
        let _ = self.verify;

        // validate in debug and test builds
        #[cfg(any(test, debug_assertions))]
        {
            if self.verify {
                // set up the validator
                let validator = Validator::new_with_options(&self.tree, ValidatorOptions::strict());

                // fail fast on invalid mir
                if let Err(error) = validator.validate_tree() {
                    panic!("mir validation failed: {error}");
                }
            }
        }

        (self.tree, self.strings)
    }
}

impl Default for ModuleBuilder {
    fn default() -> Self {
        Self::unchecked()
    }
}
