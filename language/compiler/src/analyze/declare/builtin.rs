use destack_dir::{GlobalNodeIdAny, GlobalSymbolId, LocalNodeIdAny, NodeType};
use destack_source::ModuleId;
use destack_workspace::{ProfileId, WellKnownIntrinsics};

use crate::{AnalyzeError, AnalyzeResult, Compiler, TaskResultCollector};

impl Compiler {
    /// Ensure well-known intrinsic bindings are cached for a profile.
    pub(crate) fn ensure_well_known_intrinsics_for_profile(
        &self,
        profile: ProfileId,
    ) -> AnalyzeResult<()> {
        // return early when builtins are unavailable
        let Some(builtins) = self.program.builtins.as_ref() else {
            return Ok(());
        };

        // reuse cached intrinsics when available
        let profile_key = self.program.profile(profile).key.clone();
        if builtins.well_known_intrinsics(&profile_key).is_some() {
            return Ok(());
        }

        // collect builtin modules that can host intrinsic bindings
        let mut collector = TaskResultCollector::new();
        let module_ids = builtins.intrinsic_module_ids();

        // ensure builtin module decorators are registered before scanning bindings
        for module_id in module_ids.iter().copied() {
            if let Err(error) = self.require_analyze_module_declare(module_id, profile)
                && let Some(error) = collector.try_collect::<(), _>(Err(error))
            {
                return Err(AnalyzeError::from(error));
            }
        }

        // yield if any dependencies are outstanding
        if let Some(dependency) = collector.try_into_yield_any() {
            return Err(AnalyzeError::Yield { dependency });
        }

        // build and cache the intrinsic binding table
        let intrinsics = self.build_well_known_intrinsics(module_ids, profile)?;
        builtins.set_well_known_intrinsics(&profile_key, intrinsics);

        Ok(())
    }

    /// Build well-known intrinsic bindings from builtin modules.
    fn build_well_known_intrinsics(
        &self,
        module_ids: Vec<ModuleId>,
        profile: ProfileId,
    ) -> AnalyzeResult<WellKnownIntrinsics> {
        // seed the intrinsic table
        let mut intrinsics = WellKnownIntrinsics::new();

        // scan builtin modules for intrinsic binding decorators
        for module_id in module_ids {
            // load the builtin module
            let module = self.program.modules.get(module_id);
            let module = module.read();

            // skip non-builtin modules defensively
            if !module.is_builtin() {
                continue;
            }

            // scan active symbols for intrinsic bindings
            let symbols = module.dir(profile).symbols.read();
            for local_symbol_id in symbols.active_symbol_ids() {
                // load the symbol and decorator
                let symbol = symbols.get_symbol(local_symbol_id);
                let Some(binding) = symbol.decorators.intrinsic_binding.as_ref() else {
                    continue;
                };

                // resolve the binding name id
                let symbol_id = local_symbol_id.into_global(module_id);
                let Some(name_id) = binding.name.or(symbol.name()) else {
                    let message = self
                        .program
                        .strings
                        .intern("intrinsic binding missing symbol name");
                    return Err(AnalyzeError::InvalidWellKnownDecorator {
                        node: intrinsic_binding_anchor(symbol, symbol_id, profile),
                        message,
                    });
                };

                // reject duplicate binding names
                if let Some(existing) = intrinsics.symbols_by_name.get(&name_id)
                    && *existing != symbol_id
                {
                    let name = self.program.strings.get(name_id);
                    let message = self.program.strings.intern(&format!(
                        "intrinsic name '{}' is already bound",
                        name.as_ref()
                    ));
                    return Err(AnalyzeError::InvalidWellKnownDecorator {
                        node: intrinsic_binding_anchor(symbol, symbol_id, profile),
                        message,
                    });
                }

                // record the binding mapping
                intrinsics.names_by_symbol.insert(symbol_id, name_id);
                intrinsics.symbols_by_name.insert(name_id, symbol_id);
            }
        }

        Ok(intrinsics)
    }
}

/// Create a diagnostic anchor for an intrinsic binding symbol.
fn intrinsic_binding_anchor(
    symbol: &destack_dir::Symbol,
    symbol_id: GlobalSymbolId,
    profile: ProfileId,
) -> destack_dir::AnchoredGlobalNodeId {
    // prefer the primary declaration when available
    if let Some(node_id) = symbol.primary_declaration {
        return node_id.into_anchored(Some(profile));
    }

    // synthesize a fallback anchor when no declaration exists
    let fallback = GlobalNodeIdAny::new(
        symbol_id.module_id,
        LocalNodeIdAny::new(symbol_id.local_id.id, NodeType::Declaration),
    );
    fallback.into_anchored(Some(profile))
}
