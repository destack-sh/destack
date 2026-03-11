use destack_dir::{AnchoredGlobalNodeId, Symbol, SymbolType};
use destack_source::ModuleId;
use destack_workspace::{ArtifactKey, IntrinsicEnvironment, ProfileId, WellKnownIntrinsics};

use crate::analyze::common::{CanonicalSymbolMode, ModuleSymbolView};
use crate::{
    AnalyzeError, AnalyzeResult, BuildKey, BuildRequirementCollector, BuildRequirementError,
    Compiler,
};

impl Compiler {
    /// Process the intrinsic environment for a profile.
    pub(crate) fn process_intrinsic_environment(&self, profile: ProfileId) -> AnalyzeResult<()> {
        self.resolve_intrinsic_environment(profile)
    }

    /// Require the intrinsic environment for a profile.
    pub(crate) fn require_intrinsic_environment(
        &self,
        profile: ProfileId,
    ) -> Result<(), BuildRequirementError> {
        self.require_build_key(BuildKey::Artifact(ArtifactKey::IntrinsicEnvironment {
            profile,
        }))
    }

    /// Resolve the intrinsic environment for a profile.
    pub(crate) fn resolve_intrinsic_environment(&self, profile: ProfileId) -> AnalyzeResult<()> {
        // skip when builtins are unavailable
        if self.program.builtins.is_none() {
            return Ok(());
        }

        // reuse the committed environment when available
        if self
            .program
            .artifacts
            .intrinsic_environment(profile)
            .is_some()
        {
            return Ok(());
        }

        // collect builtin modules that can host intrinsic bindings
        let mut collector = BuildRequirementCollector::new();
        let builtins = self
            .program
            .builtins
            .as_ref()
            .unwrap_or_else(|| unreachable!());
        let module_ids = builtins.intrinsic_module_ids();

        // ensure builtin module decorators are registered before scanning bindings
        for module_id in module_ids.iter().copied() {
            if let Err(error) = self.require_dir_declared(module_id, profile)
                && let Some(error) = collector.try_collect::<(), _>(Err(error))
            {
                return Err(AnalyzeError::from(error));
            }
        }

        // yield if any dependencies are outstanding
        if let Some(requirement) = collector.try_into_requirement() {
            return Err(AnalyzeError::Yield { requirement });
        }

        // build and publish the intrinsic binding table
        let intrinsics = self.build_well_known_intrinsics(module_ids, profile)?;
        self.program
            .artifacts
            .set_intrinsic_environment(profile, IntrinsicEnvironment { intrinsics });

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
                // intrinsic bindings are only meaningful on callable function symbols
                if local_symbol_id.ty != SymbolType::Function {
                    continue;
                }

                // load the symbol and decorator
                let symbol = symbols.get_symbol(local_symbol_id);
                let Some(binding) = symbol.decorators.intrinsic_binding.as_ref() else {
                    continue;
                };

                // resolve the binding name id
                let symbol_id = local_symbol_id.into_global(module_id);
                let canonical_symbol_id = self.canonical_symbol_id(
                    ModuleSymbolView::new(&module, profile, &symbols),
                    symbol_id,
                    CanonicalSymbolMode::FollowAliases,
                );
                let Some(anchor) = intrinsic_binding_anchor(symbol, profile) else {
                    return Err(AnalyzeError::Internal {
                        message: format!(
                            "intrinsic binding symbol is missing declaration anchor: {symbol_id:?}"
                        ),
                    });
                };

                let Some(name_id) = binding.name.or(symbol.name()) else {
                    let message = self
                        .program
                        .strings
                        .intern("intrinsic binding missing symbol name");
                    return Err(AnalyzeError::InvalidWellKnownDecorator {
                        node: anchor,
                        message,
                    });
                };

                // reject duplicate binding names
                if let Some(existing) = intrinsics.symbols_by_name.get(&name_id)
                    && *existing != canonical_symbol_id
                {
                    // copy name text before interning to avoid read-write lock inversion
                    let name = {
                        let name = self.program.strings.get(name_id);
                        name.to_string()
                    };
                    let message = self
                        .program
                        .strings
                        .intern(&format!("intrinsic name '{name}' is already bound"));
                    return Err(AnalyzeError::InvalidWellKnownDecorator {
                        node: anchor,
                        message,
                    });
                }

                // record the binding mapping
                intrinsics
                    .names_by_symbol
                    .insert(canonical_symbol_id, name_id);
                intrinsics.names_by_symbol.insert(symbol_id, name_id);
                intrinsics
                    .symbols_by_name
                    .insert(name_id, canonical_symbol_id);
            }
        }

        Ok(intrinsics)
    }
}

/// Resolve the declaration-backed diagnostic anchor for an intrinsic binding symbol.
fn intrinsic_binding_anchor(symbol: &Symbol, profile: ProfileId) -> Option<AnchoredGlobalNodeId> {
    // prefer the primary declaration when available
    if let Some(node_id) = symbol.primary_declaration {
        return Some(node_id.into_anchored(Some(profile)));
    }

    // otherwise use the first secondary declaration when available
    symbol
        .secondary_declarations
        .as_deref()
        .and_then(|nodes| nodes.first().copied())
        .map(|node_id| node_id.into_anchored(Some(profile)))
}
