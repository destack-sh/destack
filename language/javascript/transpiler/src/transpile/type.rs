use dyst_dir::{self as dir, Module};
use dyst_javascript_ast::{Generics, Heritage, Mutability, NodeId, Type};

use crate::{TranspileError, TranspileResult, Transpiler, TranspilerUnit};

impl<'a> Transpiler<'a> {
    /// Transpile a mutability from DIR into JS AST.
    pub fn transpile_mutability(&self, mutability: dir::Mutability) -> Mutability {
        match mutability {
            dir::Mutability::Immutable => Mutability::Immutable,
            dir::Mutability::Mutable => Mutability::Mutable,
        }
    }

    /// Transpile Generics from DIR into JS AST.
    pub fn transpile_generics(
        &self,
        module: &'a Module,
        generics: &dir::Generics,
        unit: &mut TranspilerUnit,
    ) -> TranspileResult<Generics> {
        let static_parameters = generics
            .static_parameters
            .as_ref()
            .map(|static_parameters| {
                static_parameters
                    .iter()
                    .map(|parameter| self.transpile_parameter(module, *parameter, unit))
                    .collect::<Result<Vec<_>, TranspileError>>()
            })
            .transpose()?;
        let generics = Generics { static_parameters };
        Ok(generics)
    }

    /// Transpile Heritage from DIR into JS AST.
    pub fn transpile_heritage(
        &self,
        module: &'a Module,
        heritage: &dir::Heritage,
        unit: &mut TranspilerUnit,
    ) -> TranspileResult<Heritage> {
        let extends_types = heritage
            .extends_types
            .as_ref()
            .map(|extends_types| {
                extends_types
                    .iter()
                    .map(|extends_type| self.transpile_type(module, *extends_type, unit))
                    .collect::<Result<Vec<_>, TranspileError>>()
            })
            .transpose()?;
        let implements_types = heritage
            .implements_types
            .as_ref()
            .map(|implements_types| {
                implements_types
                    .iter()
                    .map(|implements_type| self.transpile_type(module, *implements_type, unit))
                    .collect::<Result<Vec<_>, TranspileError>>()
            })
            .transpose()?;
        let heritage = Heritage {
            extends_types,
            implements_types,
        };
        Ok(heritage)
    }

    /// Transpile a type from DIR into JS AST.
    pub fn transpile_type(
        &self,
        _module: &'a Module,
        type_id: dir::NodeId<dir::Type>,
        _unit: &mut TranspilerUnit,
    ) -> TranspileResult<NodeId<Type>> {
        Err(TranspileError::UnsupportedNode {
            node: type_id.into_any(),
            message: None,
        })
    }
}
