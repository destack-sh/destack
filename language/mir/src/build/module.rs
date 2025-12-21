use destack_base::{ImmutableStringPool, StringId, StringPool};

use crate::{
    Field, Function, Global, GlobalInitializer, LocalNodeId, Mutability, NodeTree, Type,
    TypedValue, Value,
};

use super::FunctionBuilder;

/// Builder for constructing a MIR module (collection of functions and types).
#[derive(Debug)]
pub struct ModuleBuilder {
    /// The node tree being built.
    tree: NodeTree,
    /// String pool for names.
    strings: StringPool,
}

impl ModuleBuilder {
    /// Create a new module builder.
    pub fn new() -> Self {
        Self {
            tree: NodeTree::new(),
            strings: StringPool::new(),
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
        self.tree.insert(Type::Void)
    }

    /// Create a boolean type.
    pub fn type_bool(&mut self) -> LocalNodeId<Type> {
        self.tree.insert(Type::Boolean)
    }

    /// Create an integer type.
    pub fn type_int(&mut self, width: u16, signed: bool) -> LocalNodeId<Type> {
        self.tree.insert(Type::Int { width, signed })
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
        self.tree.insert(Type::Float { width })
    }

    /// Create a 32-bit float type.
    pub fn type_f32(&mut self) -> LocalNodeId<Type> {
        self.type_float(32)
    }

    /// Create a 64-bit float type.
    pub fn type_f64(&mut self) -> LocalNodeId<Type> {
        self.type_float(64)
    }

    /// Create a raw pointer type (manual memory management).
    pub fn type_raw_pointer(&mut self, pointee: LocalNodeId<Type>) -> LocalNodeId<Type> {
        self.tree.insert(Type::RawPointer { pointee })
    }

    /// Create a managed reference type (runtime-tracked).
    pub fn type_managed_reference(&mut self, pointee: LocalNodeId<Type>) -> LocalNodeId<Type> {
        self.tree.insert(Type::ManagedReference {
            pointee,
            is_nullable: false,
        })
    }

    /// Create a nullable managed reference type.
    pub fn type_managed_reference_nullable(
        &mut self,
        pointee: LocalNodeId<Type>,
    ) -> LocalNodeId<Type> {
        self.tree.insert(Type::ManagedReference {
            pointee,
            is_nullable: true,
        })
    }

    /// Create an array type.
    pub fn type_array(&mut self, element: LocalNodeId<Type>, length: u64) -> LocalNodeId<Type> {
        self.tree.insert(Type::Array { element, length })
    }

    /// Create a tuple type.
    pub fn type_tuple(&mut self, elements: Vec<LocalNodeId<Type>>) -> LocalNodeId<Type> {
        self.tree.insert(Type::Tuple { elements })
    }

    /// Create a struct type from field definitions.
    pub fn type_struct(&mut self, fields: Vec<LocalNodeId<Field>>) -> LocalNodeId<Type> {
        self.tree.insert(Type::Struct { fields })
    }

    /// Create a function pointer type.
    pub fn type_function_pointer(
        &mut self,
        parameters: Vec<LocalNodeId<Type>>,
        result: LocalNodeId<Type>,
    ) -> LocalNodeId<Type> {
        self.tree
            .insert(Type::FunctionPointer { parameters, result })
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
    ///
    /// Returns a `FunctionBuilder` that can be used to construct the function body.
    pub fn function(
        &mut self,
        name: &str,
        parameter_types: &[LocalNodeId<Type>],
        return_type: LocalNodeId<Type>,
    ) -> FunctionBuilder<'_> {
        let name_id = self.strings.intern(name);
        FunctionBuilder::new(&mut self.tree, name_id, parameter_types, return_type)
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
        (self.tree, self.strings.into_immutable())
    }

    /// Finish building the module with a mutable string pool.
    pub fn finish_mutable(self) -> (NodeTree, StringPool) {
        (self.tree, self.strings)
    }
}

impl Default for ModuleBuilder {
    fn default() -> Self {
        Self::new()
    }
}
