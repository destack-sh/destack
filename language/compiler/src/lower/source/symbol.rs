use std::hash::Hasher;

use destack_core::StableHasher;
use destack_dir as dir;
use destack_source::ModuleId;

use crate::lower::{LowerError, ModuleLowerer};
use crate::{CompilerError, CompilerResult};

impl ModuleLowerer<'_> {
    /// Return the source declaration of one symbol.
    pub(in crate::lower) fn declaration(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<dir::GlobalNodeIdAny> {
        let binding = self
            .state(symbol.module_id)?
            .bindings
            .get_symbol(symbol.local_id);
        let declaration = binding.declaration.ok_or_else(|| CompilerError::Internal {
            message: format!("symbol {symbol:?} has no declaration node"),
        })?;

        Ok(declaration)
    }

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

    /// Return the stable identity bits of one declared symbol.
    pub(in crate::lower) fn symbol_identity(symbol: dir::GlobalSymbolId) -> u64 {
        let mut hasher = StableHasher::new();
        hasher.update_len_prefixed(b"destack.lower.symbol.v1");
        hasher.write_u64(symbol.module_id.package_id.0);
        hasher.write_u64(symbol.module_id.module_key.0);
        hasher.write_u64(u64::from(symbol.local_id.id));

        hasher.finish_u64()
    }

    /// Return the module-qualified lexical path of one symbol.
    pub(in crate::lower) fn symbol_path(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<String> {
        let module = symbol.module_id;
        let state = self.state(module)?;
        let local_path = state.bindings.symbol_path(symbol.local_id);
        let mut names = Vec::with_capacity(local_path.symbols().len());
        for symbol in local_path.symbols() {
            // synthesize stable names for anonymous segments such as closures
            match state.bindings.get_symbol(*symbol).name() {
                Some(name) => names.push(self.strings.get(name).to_string()),
                None => {
                    let segment = match self.member_role(symbol.into_global(module))? {
                        Some(dir::FunctionRole::Constructor) => "constructor".to_string(),
                        _ => Self::closure_segment(&state.bindings, *symbol),
                    };
                    names.push(segment);
                }
            }
        }

        self.qualified_name(module, &names.join("."))
    }

    /// Qualify one name under its module path, keeping standard library names bare.
    pub(in crate::lower) fn qualified_name(
        &self,
        module: ModuleId,
        name: &str,
    ) -> CompilerResult<String> {
        let path = &self.state(module)?.path;

        // standard library names stay bare: the library owns their uniqueness
        if path == "destack" || path.starts_with("destack.") {
            return Ok(name.to_string());
        }

        Ok(format!("{path}.{name}"))
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

    /// Return whether one intersection operand names an interface constraint.
    pub(in crate::lower) fn is_interface_operand(
        &self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let dir::Type::Application(instance) = self.ty(id)? else {
            return Ok(false);
        };

        let symbol = instance.symbol;
        let kind = self
            .state(symbol.module_id)?
            .bindings
            .get_symbol(symbol.local_id)
            .kind;

        Ok(kind.is_interface())
    }

    /// Return the definition of one symbol in its owning module.
    pub(in crate::lower) fn definition(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<&dir::Definition>> {
        Ok(self.state(symbol.module_id)?.definitions.definition(symbol))
    }

    /// Return whether one definition declares parameters beyond the memory kinds.
    pub(in crate::lower) fn definition_is_parameterized(
        &self,
        module: ModuleId,
        definition: &dir::Definition,
    ) -> CompilerResult<bool> {
        let Some(template) = definition.template() else {
            return Ok(false);
        };

        // skip memory parameters, which ground at each use
        let generics = &self.state(module)?.generics;
        let template = generics.get_template(template);
        for parameter in &template.parameters {
            let parameter = generics.get_parameter(*parameter);
            if parameter.memory_parameter().is_none() {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Return the declared member role of one symbol, when its owner declares it.
    fn member_role(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::FunctionRole>> {
        let state = self.state(symbol.module_id)?;
        let Some(owner) = state.bindings.symbol_owner(symbol.local_id) else {
            return Ok(None);
        };

        let Some(definition) = state
            .definitions
            .definition(owner.into_global(symbol.module_id))
        else {
            return Ok(None);
        };

        // read the role of the matching method member
        let Some(dir::DefinitionMember::Method(method)) = definition.member(symbol) else {
            return Ok(None);
        };

        Ok(method.role)
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
