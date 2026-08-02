use destack_dir as dir;

use crate::lower::ModuleLowerer;
use crate::{CompilerError, CompilerResult};

impl ModuleLowerer<'_> {
    /// Return the type of one symbol.
    pub(in crate::lower) fn symbol_type(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.types(symbol.module_id)?
            .get_reduced_symbol_type_id(symbol)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("missing a type for symbol {symbol:?}"),
            })
    }

    /// Return the declared name of one symbol in its owning module.
    pub(in crate::lower) fn symbol_name(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<destack_core::StringId>> {
        let bindings = &self.state(symbol.module_id)?.bindings;

        Ok(bindings.get_symbol(symbol.local_id).name())
    }

    /// Return the module-qualified lexical path of one named symbol.
    pub(in crate::lower) fn symbol_path(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<String> {
        let state = self.state(symbol.module_id)?;
        let local_path = state.bindings.symbol_path(symbol.local_id);
        let mut names = Vec::with_capacity(local_path.symbols().len());
        for symbol in local_path.symbols() {
            let Some(name) = state.bindings.get_symbol(*symbol).name() else {
                return Err(CompilerError::Internal {
                    message: "an unnamed owner in a runtime symbol path".to_string(),
                });
            };
            names.push(self.strings.get(name));
        }

        Ok(format!("{}.{}", state.path, names.join(".")))
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
        self.state(node.module_id)?
            .resolutions
            .name_resolution(node)
            .and_then(|resolution| resolution.symbols().first().copied())
            .ok_or_else(|| CompilerError::Internal {
                message: format!("missing a name resolution for node {}", node.local_id.id),
            })
    }

    /// Return the definition of one symbol in its owning module.
    pub(in crate::lower) fn definition(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<&dir::Definition>> {
        Ok(self.state(symbol.module_id)?.definitions.definition(symbol))
    }

    /// Return whether one definition declares any generic parameters.
    pub(in crate::lower) fn definition_is_parameterized(
        &self,
        module: destack_source::ModuleId,
        definition: &dir::Definition,
    ) -> CompilerResult<bool> {
        let Some(template) = definition.template() else {
            return Ok(false);
        };

        let generics = &self.state(module)?.generics;
        let template = generics.get_template(template);

        Ok(!template.parameters.is_empty())
    }
}
