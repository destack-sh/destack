use std::hash::Hasher;

use tspp_core::StableHasher;
use tspp_dir as dir;
use tspp_source::Span;

use crate::lower::{LowerError, ModuleLowerer};
use crate::{CompilerError, CompilerResult};

impl ModuleLowerer<'_> {
    /// Return the type of one symbol.
    pub(in crate::lower) fn symbol_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.types(symbol.module_id)?
            .get_symbol_type_id(symbol)
            .ok_or_else(|| {
                let record = self
                    .state(symbol.module_id)
                    .map(|state| state.bindings.get_symbol(symbol.local_id));
                let name = record
                    .ok()
                    .and_then(|record| record.name())
                    .map(|name| self.strings.get(name).to_string());
                let kind = self
                    .state(symbol.module_id)
                    .map(|state| state.bindings.get_symbol(symbol.local_id).kind);
                CompilerError::Internal {
                    message: format!("a missing type for the {kind:?} symbol {name:?} {symbol:?}"),
                }
            })
    }

    /// Return the declared name of one symbol in its owning module.
    pub(in crate::lower) fn symbol_name(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<tspp_core::StringId>> {
        let bindings = &self.state(symbol.module_id)?.bindings;

        Ok(bindings.get_symbol(symbol.local_id).name())
    }

    /// Return the stable identity bits of one declared symbol.
    pub(in crate::lower) fn symbol_identity(symbol: dir::GlobalSymbolId) -> u64 {
        let mut hasher = StableHasher::new();
        hasher.update_len_prefixed(b"tspp.lower.symbol.v1");
        hasher.write_u64(symbol.module_id.package_id.0);
        hasher.write_u64(symbol.module_id.module_key.0);
        hasher.write_u64(u64::from(symbol.local_id.id));

        hasher.finish_u64()
    }

    /// Return the declaring node id and source extent of one symbol, when it has an extent.
    pub(in crate::lower) fn declaration_anchor(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<(u32, Span)>> {
        let Some(node) = self.declaration_node(symbol)? else {
            return Ok(None);
        };
        let span = self
            .state(node.module_id)?
            .tree()
            .get_source_extent_by_id(node.local_id.id);

        Ok(span.map(|span| (node.local_id.id, span)))
    }

    /// Return the node declaring one symbol.
    pub(in crate::lower) fn declaration_node(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::GlobalNodeIdAny>> {
        let state = self.state(symbol.module_id)?;

        Ok(state.bindings.get_symbol(symbol.local_id).declaration)
    }

    /// Return the module-qualified lexical path of one symbol.
    pub(in crate::lower) fn symbol_path(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<String> {
        // walk the lexical path of the symbol's owners
        let module = symbol.module_id;
        let segments: Vec<_> = {
            let state = self.state(module)?;
            let local_path = state.bindings.symbol_path(symbol.local_id);
            local_path
                .symbols()
                .iter()
                .map(|symbol| {
                    let name = state.bindings.get_symbol(*symbol).name();
                    let closure = Self::closure_segment(&state.bindings, *symbol);
                    (*symbol, name, closure)
                })
                .collect()
        };
        let mut names = Vec::with_capacity(segments.len());
        for (symbol, name, closure) in segments {
            // synthesize stable names for anonymous segments such as closures
            match name {
                Some(name) => names.push(self.strings.get(name).to_string()),
                None => {
                    let segment = match self.member_role(symbol.into_global(module))? {
                        Some(dir::FunctionRole::Constructor) => "constructor".to_string(),
                        _ => closure,
                    };
                    names.push(segment);
                }
            }
        }

        self.qualified_name(module, &names.join("."))
    }

    /// Qualify one name under its module path, keeping standard library names bare.
    pub(in crate::lower) fn qualified_name(
        &mut self,
        module: tspp_source::ModuleId,
        name: &str,
    ) -> CompilerResult<String> {
        let path = &self.state(module)?.path;

        // keep standard library names bare
        if path == "tspp" || path.starts_with("tspp.") {
            return Ok(name.to_string());
        }

        Ok(format!("{path}.{name}"))
    }

    /// Return the symbol declared at one node in its owning module.
    pub(in crate::lower) fn symbol_declared_at(
        &mut self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Option<dir::GlobalSymbolId>> {
        let declared = self.state(node.module_id)?.declared_symbol(node);

        Ok(declared.map(|symbol| symbol.into_global(node.module_id)))
    }

    /// Return the resolved symbol behind one name reference.
    pub(in crate::lower) fn resolved_symbol(
        &mut self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<dir::GlobalSymbolId> {
        let state = self.state(node.module_id)?;

        // read the selected function value ahead of the lexical group
        if let Some(symbol) = state.decisions.function_symbol(node) {
            return Ok(symbol);
        }

        // read the symbol a declaration expression declares
        if let Ok(expression) = node.local_id.try_into_typed::<dir::Expression>()
            && let dir::Expression::Declaration(declaration) = *state.tree().get(expression)
            && let Some(symbol) = state.declared_symbol(declaration.into_global_any(node.module_id))
        {
            return Ok(symbol.into_global(node.module_id));
        }

        // read the lexical resolution at the node
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
                message: format!("a missing name resolution for node {}", node.local_id.id),
            })
    }

    /// Return whether one intersection operand names an interface constraint.
    pub(in crate::lower) fn is_interface_operand(
        &mut self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let dir::Type::Application(instance) = self.ty(id)? else {
            return Ok(false);
        };

        // read the kind of the applied symbol
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
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<&dir::Definition>> {
        Ok(self.state(symbol.module_id)?.definitions.definition(symbol))
    }

    /// Return whether one definition's template writes a type parameter of its own.
    pub(in crate::lower) fn definition_has_written_parameters(
        &mut self,
        module: tspp_source::ModuleId,
        definition: &dir::Definition,
    ) -> CompilerResult<bool> {
        let Some(template) = definition.template() else {
            return Ok(false);
        };
        let generics = &self.state(module)?.generics;
        let template = generics.get_template(template);
        let written = template.parameters.iter().any(|parameter| {
            let parameter = generics.get_parameter(*parameter);
            parameter.origin != dir::GenericParameterOrigin::Receiver
                && parameter.is_representation_parameter()
        });

        Ok(written)
    }

    /// Return whether one definition's representation ranges over any parameter.
    pub(in crate::lower) fn definition_is_parameterized(
        &mut self,
        module: tspp_source::ModuleId,
        definition: &dir::Definition,
    ) -> CompilerResult<bool> {
        let Some(template) = definition.template() else {
            return Ok(false);
        };

        // read the parameters declared for the representation
        let generics = &self.state(module)?.generics;
        let template = generics.get_template(template);
        let parameterized = template.parameters.iter().any(|parameter| {
            generics
                .get_parameter(*parameter)
                .is_representation_parameter()
        });

        Ok(parameterized)
    }

    /// Return the declared member role of one symbol, when its owner declares it.
    fn member_role(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::FunctionRole>> {
        let state = self.state(symbol.module_id)?;
        let Some(owner) = state.bindings.symbol_owner(symbol.local_id) else {
            return Ok(None);
        };

        // read the owner's definition
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
