use crate::build::{BuildResult, FunctionBuilder, FunctionHeader, ModuleBuilder};
use crate::{Binding, Function, Global, GlobalInitializer, LocalNodeId, Mutability, TypeId};

impl ModuleBuilder {
    /// Create one mutable global variable.
    pub fn global_variable(
        &mut self,
        name: &str,
        ty: TypeId,
        initializer: GlobalInitializer,
    ) -> LocalNodeId<Global> {
        let name = self.strings.intern(name);
        self.tree.insert(Global::new(
            self.module,
            name,
            ty,
            Mutability::Mutable,
            initializer,
        ))
    }

    /// Create one program constant.
    pub fn constant(
        &mut self,
        name: &str,
        ty: TypeId,
        initializer: GlobalInitializer,
    ) -> LocalNodeId<Global> {
        let name = self.strings.intern(name);
        self.tree
            .insert(Global::constant(self.module, name, ty, initializer))
    }

    /// Create one global at an explicit mutability.
    pub fn global(
        &mut self,
        name: &str,
        ty: TypeId,
        mutability: Mutability,
        initializer: GlobalInitializer,
    ) -> LocalNodeId<Global> {
        let name = self.strings.intern(name);
        self.tree
            .insert(Global::new(self.module, name, ty, mutability, initializer))
    }

    /// Declare one external global.
    pub fn external_global(
        &mut self,
        name: &str,
        ty: TypeId,
        mutability: Mutability,
    ) -> LocalNodeId<Global> {
        let name = self.strings.intern(name);
        self.tree
            .insert(Global::import(self.module, name, ty, mutability))
    }

    /// Start building one new function.
    pub fn function(&mut self, header: FunctionHeader) -> FunctionBuilder<'_> {
        FunctionBuilder::new(
            &mut self.tree,
            &mut self.effects,
            self.target_layout.pointer_bits(),
            header,
        )
    }

    /// Start building the body of one declared function.
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

    /// Declare one local function without a body.
    pub fn declare_function(&mut self, header: FunctionHeader) -> LocalNodeId<Function> {
        self.tree.insert(header.declared())
    }

    /// Declare one external function.
    pub fn external_function(&mut self, header: FunctionHeader) -> LocalNodeId<Function> {
        self.tree.insert(header.imported())
    }

    /// Declare an external function dispatched through one runtime binding.
    pub fn binding_function(
        &mut self,
        header: FunctionHeader,
        binding: Binding,
    ) -> LocalNodeId<Function> {
        self.tree.insert(header.imported().with_binding(binding))
    }
}
