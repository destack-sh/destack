use crate::build::{BuildResult, FunctionBuilder, FunctionHeader, ModuleBuilder};
use crate::{
    Function, Global, GlobalInitializer, LocalNodeId, Mutability, Type, finalize_function_names,
};

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
        FunctionBuilder::new(&mut self.tree, &self.strings, header)
    }

    /// Start building a body for an existing declared function.
    pub fn function_body(
        &mut self,
        function_id: LocalNodeId<Function>,
    ) -> BuildResult<FunctionBuilder<'_>> {
        FunctionBuilder::from_declared(&mut self.tree, &self.strings, function_id)
    }

    /// Declare a local function without a body.
    pub fn declare_function(&mut self, header: FunctionHeader) -> LocalNodeId<Function> {
        let FunctionHeader {
            name,
            lifetimes,
            parameters,
            result,
        } = header;
        let parameters = FunctionHeader::parameters_from_types(parameters);
        let function_id = self.tree.insert(Function::declare(
            name,
            lifetimes,
            parameters,
            result.into(),
        ));
        finalize_function_names(&mut self.tree, &self.strings, function_id);

        function_id
    }

    /// Declare an external function (defined elsewhere).
    pub fn external_function(&mut self, header: FunctionHeader) -> LocalNodeId<Function> {
        let FunctionHeader {
            name,
            lifetimes,
            parameters,
            result,
        } = header;
        let parameters = FunctionHeader::parameters_from_types(parameters);
        let function_id =
            self.tree
                .insert(Function::import(name, lifetimes, parameters, result.into()));
        finalize_function_names(&mut self.tree, &self.strings, function_id);

        function_id
    }
}
