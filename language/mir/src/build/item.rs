use crate::build::{FunctionBuilder, ModuleBuilder};
use crate::validate::Validator;
use crate::{
    Function, Global, GlobalInitializer, LocalNodeId, Mutability, Parameter, Type, Value,
    finalize_function_names,
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
            .insert(Global::new(name_id, ty.into(), Mutability::Mutable, init))
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
            .insert(Global::new(name_id, ty.into(), Mutability::Immutable, init))
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
        self.tree
            .insert(Global::new(name_id, ty.into(), mutability, init))
    }

    /// Declare an external global (defined elsewhere).
    pub fn extern_global(
        &mut self,
        name: &str,
        ty: LocalNodeId<Type>,
        mutability: Mutability,
    ) -> LocalNodeId<Global> {
        let name_id = self.strings.intern(name);
        self.tree
            .insert(Global::import(name_id, ty.into(), mutability))
    }

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
            &mut self.strings,
            name_id,
            parameter_types,
            return_type,
            self.verify,
        )
    }

    /// Start building a body for an existing declared function.
    pub fn function_body(&mut self, function_id: LocalNodeId<Function>) -> FunctionBuilder<'_> {
        FunctionBuilder::from_declared(&mut self.tree, &mut self.strings, function_id, self.verify)
    }

    /// Declare a local function without a body.
    pub fn declare_function(
        &mut self,
        name: &str,
        parameter_types: &[LocalNodeId<Type>],
        return_type: LocalNodeId<Type>,
    ) -> LocalNodeId<Function> {
        let name_id = self.strings.intern(name);
        let parameters: Vec<Parameter> = parameter_types
            .iter()
            .enumerate()
            .map(|(index, &ty)| Parameter {
                value: Value::new(index as u32).into(),
                ty: ty.into(),
            })
            .collect();
        let function_id =
            self.tree
                .insert(Function::declare(name_id, parameters, return_type.into()));
        finalize_function_names(&mut self.tree, &mut self.strings, function_id);

        let validator = Validator::new(&self.tree);
        if let Err(error) = validator.validate_function(function_id) {
            panic!("mir validation failed: {error}");
        }

        function_id
    }

    /// Declare an external function (defined elsewhere).
    pub fn extern_function(
        &mut self,
        name: &str,
        parameter_types: &[LocalNodeId<Type>],
        return_type: LocalNodeId<Type>,
    ) -> LocalNodeId<Function> {
        let name_id = self.strings.intern(name);
        let parameters: Vec<Parameter> = parameter_types
            .iter()
            .enumerate()
            .map(|(index, &ty)| Parameter {
                value: Value::new(index as u32).into(),
                ty: ty.into(),
            })
            .collect();
        let function_id =
            self.tree
                .insert(Function::import(name_id, parameters, return_type.into()));
        finalize_function_names(&mut self.tree, &mut self.strings, function_id);

        let validator = Validator::new(&self.tree);
        if let Err(error) = validator.validate_function(function_id) {
            panic!("mir validation failed: {error}");
        }

        function_id
    }
}
