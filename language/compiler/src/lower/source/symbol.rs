use destack_dir as dir;

use crate::lower::{LowerError, ModuleLowerer};
use crate::{CompilerError, CompilerResult};

impl ModuleLowerer<'_> {
    /// Return the type of one symbol.
    pub(in crate::lower) fn symbol_type(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.types(symbol.module_id)?
            .get_symbol_type_id(symbol)
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

    /// Return the module-qualified lexical path of one symbol.
    pub(in crate::lower) fn symbol_path(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<String> {
        let state = self.state(symbol.module_id)?;
        let local_path = state.bindings.symbol_path(symbol.local_id);
        let mut names = Vec::with_capacity(local_path.symbols().len());
        for symbol in local_path.symbols() {
            // synthesize stable names for anonymous segments such as closures
            match state.bindings.get_symbol(*symbol).name() {
                Some(name) => names.push(self.strings.get(name).to_string()),
                None => names.push(Self::closure_segment(&state.bindings, *symbol)),
            }
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
        let state = self.state(node.module_id)?;

        // read the selected function value ahead of the lexical group
        if let Some(symbol) = state.decisions.function_symbol(node) {
            return Ok(symbol);
        }

        let resolution = state.resolutions.name_resolution(node);

        // report a type literal name read as a runtime value
        if resolution.is_some_and(|resolution| resolution.denoted_type().is_some()) {
            return Err(LowerError::Unsupported {
                anchor: self.module.into(),
                construct: "a type literal used as a runtime value".to_string(),
            }
            .into());
        }

        resolution
            .and_then(|resolution| resolution.single_symbol())
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

    /// Return whether one definition declares parameters beyond lifetimes.
    pub(in crate::lower) fn definition_is_parameterized(
        &self,
        module: destack_source::ModuleId,
        definition: &dir::Definition,
    ) -> CompilerResult<bool> {
        let Some(template) = definition.template() else {
            return Ok(false);
        };

        // skip lifetime parameters, grounded implicitly at every use
        let generics = &self.state(module)?.generics;
        let template = generics.get_template(template);
        for parameter in &template.parameters {
            let parameter = generics.get_parameter(*parameter);
            if parameter.memory_parameter() != Some(dir::MemoryParameter::Lifetime) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Synthesize the ordinal path segment of one anonymous symbol under its owner.
    fn closure_segment(bindings: &dir::BindingTable<'_>, symbol: dir::LocalSymbolId) -> String {
        // count the earlier anonymous same-role siblings in declaration order
        let owner = bindings.symbol_owner(symbol);
        let role = bindings.get_symbol(symbol).role;
        let mut ordinal = 0;
        for candidate in bindings.symbol_ids() {
            let record = bindings.get_symbol(candidate);
            if candidate.id < symbol.id
                && record.name().is_none()
                && record.role == role
                && bindings.symbol_owner(candidate) == owner
            {
                ordinal += 1;
            }
        }

        format!("closure#{ordinal}")
    }
}
