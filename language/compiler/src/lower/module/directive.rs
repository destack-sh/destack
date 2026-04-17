use std::collections::HashMap;
use {destack_dir as dir, destack_mir as mir};

use crate::lower::ModuleLowerer;

impl ModuleLowerer<'_> {
    /// Extract return lifetime from @lifetime decorator metadata on a function.
    pub(crate) fn extract_lifetime_annotation(
        &self,
        symbol_id: dir::LocalSymbolId,
        signature: &dir::FunctionSignature,
    ) -> mir::Lifetime {
        let symbol = self.symbols.get_symbol(symbol_id);
        let Some(lifetime) = symbol.decorators.lifetime.as_ref() else {
            return mir::Lifetime::Inferred;
        };

        // short-circuit static lifetimes
        if matches!(lifetime, dir::LifetimeAnnotation::Static) {
            return mir::Lifetime::Static;
        }

        // map parameter names to indices
        let mut param_name_to_index: HashMap<dir::StringId, u32> = HashMap::new();
        for (index, param_id) in signature.parameters.iter().enumerate() {
            let param: &dir::Parameter = self.dir_tree.get(*param_id);
            let param_name = match param {
                dir::Parameter::Named { name, .. } => Some(*name),
                dir::Parameter::VariadicNamed { name, .. } => Some(*name),
                dir::Parameter::Pattern { .. }
                | dir::Parameter::VariadicPattern { .. }
                | dir::Parameter::Error { .. } => None,
            };
            if let Some(name_id) = param_name {
                param_name_to_index.insert(name_id, index as u32);
            }
        }

        // resolve annotated parameter names to indices
        let dir::LifetimeAnnotation::Parameters(names) = lifetime else {
            return mir::Lifetime::Inferred;
        };
        let mut param_indices = Vec::new();
        for name_id in names {
            if let Some(&index) = param_name_to_index.get(name_id)
                && !param_indices.contains(&index)
            {
                param_indices.push(index);
            }
        }

        // fall back to inferred if nothing matches
        if param_indices.is_empty() {
            return mir::Lifetime::Inferred;
        }

        mir::Lifetime::Parameters(param_indices)
    }
}
