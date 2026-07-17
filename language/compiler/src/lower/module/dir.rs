use destack_dir as dir;

use crate::lower::ModuleLowerer;
use crate::{CompilerError, CompilerResult};

impl ModuleLowerer<'_> {
    /// Return the checked type behind one expression node.
    pub(in crate::lower) fn node_type(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::Type> {
        let node = expression.into_global_any(self.module);
        let ty = self
            .types
            .get_node_type_id(node)
            .ok_or_else(|| CompilerError::Internal {
                message: format!(
                    "checked DIR is missing a type for node {}",
                    node.local_id.id
                ),
            })?;

        self.ty(ty)
    }

    /// Return the checked type of one symbol.
    pub(in crate::lower) fn symbol_type(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.types
            .get_symbol_type_id(symbol)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("checked DIR is missing a type for symbol {symbol:?}"),
            })
    }

    /// Return one checked type by id.
    pub(in crate::lower) fn ty(&self, ty: dir::GlobalTypeId) -> CompilerResult<dir::Type> {
        if ty.module_id != self.module {
            return Err(CompilerError::Internal {
                message: "checked DIR referenced a foreign module type".to_string(),
            });
        }

        self.types
            .get_type_maybe(ty.local_id)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("checked DIR is missing type {:?}", ty.local_id),
            })
    }

    /// Return the checked return type behind one callable type.
    pub(in crate::lower) fn signature_return(
        &self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let signature = match self.ty(ty)? {
            dir::Type::Function(function) => self.ty(function.signature)?,
            signature @ dir::Type::FunctionSignature(_) => signature,
            other => {
                return Err(CompilerError::Internal {
                    message: format!("checked DIR declared a non-callable function: {other:?}"),
                });
            }
        };
        let dir::Type::FunctionSignature(signature) = signature else {
            return Err(CompilerError::Internal {
                message: "checked DIR is missing a signature behind one function type".to_string(),
            });
        };

        Ok(self.types.signature(signature).return_type)
    }

    /// Return the symbol declared at one node.
    pub(in crate::lower) fn symbol_declared_at(
        &self,
        node: dir::GlobalNodeIdAny,
    ) -> Option<dir::GlobalSymbolId> {
        // find the symbol whose declaration is this node
        for id in self.bindings.symbol_ids() {
            let symbol = self.bindings.get_symbol(id);
            if symbol.declaration == Some(node) {
                return Some(id.into_global(self.module));
            }
        }

        None
    }

    /// Return the resolved symbol behind one name reference.
    pub(in crate::lower) fn resolved_symbol(
        &self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<dir::GlobalSymbolId> {
        self.resolutions
            .name_resolution(node)
            .and_then(|resolution| resolution.symbols().first().copied())
            .ok_or_else(|| CompilerError::Internal {
                message: format!(
                    "checked DIR is missing a name resolution for node {}",
                    node.local_id.id
                ),
            })
    }

    /// Return the checked call resolution of one applying expression.
    pub(in crate::lower) fn call_resolution(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::CallResolution> {
        let node = expression.into_global_any(self.module);

        self.resolutions
            .call_resolution(node)
            .cloned()
            .ok_or_else(|| CompilerError::Internal {
                message: format!(
                    "checked DIR is missing a call resolution for node {}",
                    node.local_id.id
                ),
            })
    }

    /// Return the checked place resolution of one place expression.
    pub(in crate::lower) fn place_resolution(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::PlaceResolution> {
        let node = expression.into_global_any(self.module);

        self.resolutions
            .place_resolution(node)
            .cloned()
            .ok_or_else(|| CompilerError::Internal {
                message: format!(
                    "checked DIR is missing a place resolution for node {}",
                    node.local_id.id
                ),
            })
    }

    /// Return the checked resolution of one assignment pattern.
    pub(in crate::lower) fn assign_resolution(
        &self,
        pattern: dir::LocalNodeId<dir::AssignPattern>,
    ) -> CompilerResult<dir::AssignPatternResolution> {
        let node = pattern.into_global_any(self.module);

        self.resolutions
            .assign_pattern_resolution(node)
            .cloned()
            .ok_or_else(|| CompilerError::Internal {
                message: format!(
                    "checked DIR is missing a resolution for assignment pattern {}",
                    node.local_id.id
                ),
            })
    }

    /// Return the checked coercion on one expression node.
    pub(in crate::lower) fn coercion(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::Coercion> {
        self.coercions
            .coercion(expression.into_global_any(self.module))
    }

    /// Return one node's type after its checked coercion applies.
    pub(in crate::lower) fn coerced_type(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::Type> {
        match self.coercion(expression) {
            Some(coercion) => self.ty(coercion.target),
            None => self.node_type(expression),
        }
    }
}
