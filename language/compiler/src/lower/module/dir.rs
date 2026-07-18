use destack_dir as dir;

use destack_source::ModuleId;

use crate::lower::{LowerModuleState, ModuleLowerer};
use crate::{CompilerError, CompilerResult};

impl ModuleLowerer<'_> {
    /// Return the sealed check output of one loaded module.
    pub(in crate::lower) fn state(
        &self,
        module: ModuleId,
    ) -> CompilerResult<&LowerModuleState> {
        self.modules
            .get(&module)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("checked DIR referenced the unloaded module {module:?}"),
            })
    }

    /// Return the sealed check output of the module being lowered.
    pub(in crate::lower) fn local(&self) -> &LowerModuleState {
        match self.modules.get(&self.module) {
            Some(state) => state,
            None => unreachable!("the lowered module is always loaded"),
        }
    }

    /// Return the sealed check output of the module the current body reads.
    pub(in crate::lower) fn source(&self) -> &LowerModuleState {
        match self.modules.get(&self.source) {
            Some(state) => state,
            None => unreachable!("the source module is always loaded"),
        }
    }


    /// Return the checked type behind one expression node.
    pub(in crate::lower) fn node_type(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::Type> {
        self.ty(self.node_type_id(expression)?)
    }

    /// Return the checked type id behind one expression node.
    pub(in crate::lower) fn node_type_id(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let node = expression.into_global_any(self.source);

        self.source()
            .types
            .get_node_type_id(node)
            .ok_or_else(|| CompilerError::Internal {
                message: format!(
                    "checked DIR is missing a type for node {}",
                    node.local_id.id
                ),
            })
    }

    /// Return the checked type of one symbol.
    pub(in crate::lower) fn symbol_type(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.types(symbol.module_id)?
            .get_symbol_type_id(symbol)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("checked DIR is missing a type for symbol {symbol:?}"),
            })
    }

    /// Return the sealed type table of one module.
    pub(in crate::lower) fn types(
        &self,
        module: ModuleId,
    ) -> CompilerResult<&dir::TypeTable<'static>> {
        Ok(&self.state(module)?.types)
    }

    /// Return the sealed definition of one symbol in its owning module.
    pub(in crate::lower) fn definition(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<&dir::Definition>> {
        Ok(self.state(symbol.module_id)?.definitions.definition(symbol))
    }

    /// Return the declared name of one symbol in its owning module.
    pub(in crate::lower) fn symbol_name(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<destack_core::StringId>> {
        let bindings = &self.state(symbol.module_id)?.bindings;

        Ok(bindings.get_symbol(symbol.local_id).name())
    }

    /// Return one checked type by id.
    pub(in crate::lower) fn ty(&self, ty: dir::GlobalTypeId) -> CompilerResult<dir::Type> {
        self.types(ty.module_id)?
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
        let (signature, owner) = self.signature_of(ty)?;

        Ok(self.types(owner)?.signature(signature).return_type)
    }

    /// Return the signature type and its pool-owning module behind one callable.
    pub(in crate::lower) fn signature_of(
        &self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<(dir::FunctionSignatureId, ModuleId)> {
        let (signature, owner) = match self.ty(ty)? {
            dir::Type::Function(function) => {
                (self.ty(function.signature)?, function.signature.module_id)
            }
            signature @ dir::Type::FunctionSignature(_) => (signature, ty.module_id),
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

        Ok((signature, owner))
    }

    /// Return the symbol declared at one node in its owning module.
    pub(in crate::lower) fn symbol_declared_at(
        &self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Option<dir::GlobalSymbolId>> {
        // find the symbol whose declaration is this node
        let bindings = &self.state(node.module_id)?.bindings;
        for id in bindings.symbol_ids() {
            let symbol = bindings.get_symbol(id);
            if symbol.declaration == Some(node) {
                return Ok(Some(id.into_global(node.module_id)));
            }
        }

        Ok(None)
    }

    /// Return the resolved symbol behind one name reference.
    pub(in crate::lower) fn resolved_symbol(
        &self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<dir::GlobalSymbolId> {
        self.source().resolutions
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
        let node = expression.into_global_any(self.source);

        self.source().resolutions
            .call_resolution(node)
            .cloned()
            .ok_or_else(|| CompilerError::Internal {
                message: format!(
                    "checked DIR is missing a call resolution for node {}",
                    node.local_id.id
                ),
            })
    }

    /// Return the checked construct resolution on one call expression.
    pub(in crate::lower) fn construct_resolution(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::ConstructResolution> {
        self.source().resolutions
            .construct_resolution(expression.into_global_any(self.source))
            .cloned()
    }

    /// Return the checked place resolution of one place expression.
    pub(in crate::lower) fn place_resolution(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::PlaceResolution> {
        let node = expression.into_global_any(self.source);

        self.source().resolutions
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
        let node = pattern.into_global_any(self.source);

        self.source().resolutions
            .assign_pattern_resolution(node)
            .cloned()
            .ok_or_else(|| CompilerError::Internal {
                message: format!(
                    "checked DIR is missing a resolution for assignment pattern {}",
                    node.local_id.id
                ),
            })
    }

    /// Return the checked member resolution of one member expression.
    pub(in crate::lower) fn member_resolution(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::MemberResolution> {
        let node = expression.into_global_any(self.source);

        self.source().resolutions
            .member_resolution(node)
            .cloned()
            .ok_or_else(|| CompilerError::Internal {
                message: format!(
                    "checked DIR is missing a member resolution for node {}",
                    node.local_id.id
                ),
            })
    }

    /// Return one checked static term by id.
    pub(in crate::lower) fn static_term(
        &self,
        id: dir::GlobalStaticId,
    ) -> CompilerResult<dir::StaticTerm> {
        self.state(id.module_id)?
            .statics
            .get_static_maybe(id.local_id)
            .cloned()
            .ok_or_else(|| CompilerError::Internal {
                message: format!("checked DIR is missing static {:?}", id.local_id),
            })
    }

    /// Return the checked resolution of one pattern node.
    pub(in crate::lower) fn pattern_resolution(
        &self,
        pattern: dir::LocalNodeId<dir::Pattern>,
    ) -> CompilerResult<dir::PatternResolution> {
        let node = pattern.into_global_any(self.source);

        self.source().resolutions
            .pattern_resolution(node)
            .cloned()
            .ok_or_else(|| CompilerError::Internal {
                message: format!(
                    "checked DIR is missing a pattern resolution for node {}",
                    node.local_id.id
                ),
            })
    }

    /// Return the checked coercion on one expression node.
    pub(in crate::lower) fn coercion(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::Coercion> {
        self.source().coercions
            .coercion(expression.into_global_any(self.source))
    }

    /// Return one node's type after its checked coercion applies.
    pub(in crate::lower) fn coerced_type(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::Type> {
        self.ty(self.coerced_type_id(expression)?)
    }

    /// Return one node's type id after its checked coercion applies.
    pub(in crate::lower) fn coerced_type_id(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        match self.coercion(expression) {
            Some(coercion) => Ok(coercion.target),
            None => self.node_type_id(expression),
        }
    }
}
