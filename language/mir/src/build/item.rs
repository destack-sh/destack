use destack_source::ProvenanceId;

use crate::build::{BuildResult, FunctionBuilder, FunctionHeader, ModuleBuilder};
use crate::{Binding, Function, Global, GlobalInitializer, LocalNodeId, Mutability, TypeId};

impl ModuleBuilder {
    /// Create a global variable (mutable).
    pub fn global_variable(
        &mut self,
        name: &str,
        ty: TypeId,
        init: GlobalInitializer,
        source: ProvenanceId,
    ) -> LocalNodeId<Global> {
        let name_id = self.strings.intern(name);

        self.insert(
            Global::new(name_id, ty, Mutability::Mutable, init),
            &[source],
        )
    }

    /// Create a Program constant.
    pub fn constant(
        &mut self,
        name: &str,
        ty: TypeId,
        init: GlobalInitializer,
        source: ProvenanceId,
    ) -> LocalNodeId<Global> {
        let name_id = self.strings.intern(name);

        self.insert(Global::constant(name_id, ty, init), &[source])
    }

    /// Create a global with explicit mutability.
    pub fn global(
        &mut self,
        name: &str,
        ty: TypeId,
        mutability: Mutability,
        init: GlobalInitializer,
        source: ProvenanceId,
    ) -> LocalNodeId<Global> {
        let name_id = self.strings.intern(name);

        self.insert(Global::new(name_id, ty, mutability, init), &[source])
    }

    /// Declare an external global (defined elsewhere).
    pub fn external_global(
        &mut self,
        name: &str,
        ty: TypeId,
        mutability: Mutability,
        source: ProvenanceId,
    ) -> LocalNodeId<Global> {
        let name_id = self.strings.intern(name);

        self.insert(Global::import(name_id, ty, mutability), &[source])
    }

    /// Start building a new function.
    pub fn function(
        &mut self,
        header: FunctionHeader,
        inputs: &[ProvenanceId],
    ) -> FunctionBuilder<'_> {
        let provenance = self.produce(inputs);

        FunctionBuilder::new(
            &mut self.tree,
            &mut self.provenance,
            &mut self.effects,
            self.target_layout.pointer_bits(),
            self.transform,
            header,
            provenance,
        )
    }

    /// Start building a body for an existing declared function.
    pub fn function_body(
        &mut self,
        function_id: LocalNodeId<Function>,
    ) -> BuildResult<FunctionBuilder<'_>> {
        FunctionBuilder::from_declared(
            &mut self.tree,
            &mut self.provenance,
            &mut self.effects,
            self.target_layout.pointer_bits(),
            self.transform,
            function_id,
        )
    }

    /// Declare a local function without a body.
    pub fn declare_function(
        &mut self,
        header: FunctionHeader,
        inputs: &[ProvenanceId],
    ) -> LocalNodeId<Function> {
        let FunctionHeader {
            name,
            arguments,
            symbol,
            lifetimes,
            parameters,
            result,
        } = header;
        let provenance = self.produce(inputs);
        let function = Function::declare(name, lifetimes, parameters, result)
            .with_arguments(arguments)
            .with_symbol(symbol);

        self.tree.insert(function, provenance)
    }

    /// Declare an external function (defined elsewhere).
    pub fn external_function(
        &mut self,
        header: FunctionHeader,
        inputs: &[ProvenanceId],
    ) -> LocalNodeId<Function> {
        let FunctionHeader {
            name,
            arguments,
            symbol,
            lifetimes,
            parameters,
            result,
        } = header;
        let provenance = self.produce(inputs);
        let function = Function::import(name, lifetimes, parameters, result)
            .with_arguments(arguments)
            .with_symbol(symbol);

        self.tree.insert(function, provenance)
    }

    /// Declare an external function dispatched through one runtime binding.
    pub fn binding_function(
        &mut self,
        header: FunctionHeader,
        binding: Binding,
        inputs: &[ProvenanceId],
    ) -> LocalNodeId<Function> {
        let function_id = self.external_function(header, inputs);
        self.tree.get_mut(function_id).binding = Some(Box::new(binding));

        function_id
    }
}
