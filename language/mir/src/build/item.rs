use crate::build::{BuildResult, FunctionBuilder, FunctionHeader, ModuleBuilder};
use crate::{Binding, Function, Global, GlobalInitializer, LocalNodeId, Mutability, Type};

impl ModuleBuilder {
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

    /// Create a Program constant.
    pub fn constant(
        &mut self,
        name: &str,
        ty: LocalNodeId<Type>,
        init: GlobalInitializer,
    ) -> LocalNodeId<Global> {
        let name_id = self.strings.intern(name);
        self.tree.insert(Global::constant(name_id, ty, init))
    }

    /// Create an immortal pre-built object with reference identity.
    pub fn immortal(
        &mut self,
        name: &str,
        ty: LocalNodeId<Type>,
        init: GlobalInitializer,
    ) -> LocalNodeId<Global> {
        let name_id = self.strings.intern(name);
        self.tree.insert(Global::immortal(name_id, ty, init))
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
    pub fn external_global(
        &mut self,
        name: &str,
        ty: LocalNodeId<Type>,
        mutability: Mutability,
    ) -> LocalNodeId<Global> {
        let name_id = self.strings.intern(name);
        self.tree.insert(Global::import(name_id, ty, mutability))
    }

    /// Start building a new function.
    pub fn function(&mut self, header: FunctionHeader) -> FunctionBuilder<'_> {
        FunctionBuilder::new(
            &mut self.tree,
            &mut self.effects,
            self.target_layout.pointer_bits(),
            header,
        )
    }

    /// Start building a body for an existing declared function.
    pub fn function_body(
        &mut self,
        function_id: LocalNodeId<Function>,
    ) -> BuildResult<FunctionBuilder<'_>> {
        FunctionBuilder::from_declared(
            &mut self.tree,
            &mut self.effects,
            self.target_layout.pointer_bits(),
            function_id,
        )
    }

    /// Declare a local function without a body.
    pub fn declare_function(&mut self, header: FunctionHeader) -> LocalNodeId<Function> {
        let FunctionHeader {
            name,
            arguments,
            symbol,
            lifetimes,
            parameters,
            result,
        } = header;
        let parameters = FunctionHeader::parameters_from_types(parameters);
        let function = Function::declare(name, lifetimes, parameters, result)
            .with_arguments(arguments)
            .with_symbol(symbol);

        self.tree.insert(function)
    }

    /// Declare an external function (defined elsewhere).
    pub fn external_function(&mut self, header: FunctionHeader) -> LocalNodeId<Function> {
        let FunctionHeader {
            name,
            arguments,
            symbol,
            lifetimes,
            parameters,
            result,
        } = header;
        let parameters = FunctionHeader::parameters_from_types(parameters);
        let function = Function::import(name, lifetimes, parameters, result)
            .with_arguments(arguments)
            .with_symbol(symbol);

        self.tree.insert(function)
    }

    /// Declare an external function dispatched through one runtime binding.
    pub fn binding_function(
        &mut self,
        header: FunctionHeader,
        binding: Binding,
    ) -> LocalNodeId<Function> {
        let function_id = self.external_function(header);
        self.tree.get_mut(function_id).binding = Some(Box::new(binding));

        function_id
    }
}
