use dyst_dir::{self as dir, Module};
use dyst_javascript_ast::{Mutability, NodeId, Parameter, Type};

use crate::{TranspileError, TranspileResult, Transpiler, TranspilerUnit};

impl<'a> Transpiler<'a> {
    /// Transpile a mutability from DIR into JS AST.
    pub fn transpile_mutability(&self, mutability: dir::Mutability) -> Mutability {
        match mutability {
            dir::Mutability::Immutable => Mutability::Immutable,
            dir::Mutability::Mutable => Mutability::Mutable,
        }
    }

    /// Transpile a generics from DIR into JS AST static parameters.
    pub fn transpile_generics_to_static_parameters(
        &self,
        module: &'a Module,
        generics: dir::Generics,
        unit: &mut TranspilerUnit,
    ) -> TranspileResult<Option<Vec<NodeId<Parameter>>>> {
        let static_parameters = generics
            .static_parameters
            .as_ref()
            .map(|static_parameters| {
                static_parameters
                    .iter()
                    .map(|parameter| self.transpile_parameter(module, *parameter, unit))
                    .collect()
            })
            .transpose()?;
        Ok(static_parameters)
    }

    /// Transpile a type from DIR into JS AST.
    pub fn transpile_type(
        &self,
        module: &'a Module,
        type_id: dir::NodeId<dir::Type>,
        unit: &mut TranspilerUnit,
    ) -> TranspileResult<NodeId<Type>> {
        Err(TranspileError::UnsupportedType { node: type_id })
    }
}
