use std::collections::HashSet;

use crate::analyze::common::{CanonicalSymbolMode, ModuleTypeView, SymbolTypeView};
use crate::{AnalyzeError, AnalyzeResult, Compiler};
use destack_dir::{Extension, ExtensionKind, GlobalSymbolId, Lineage, ModuleTarget, SymbolType};
use destack_workspace::{ModuleSource, ProfileId};

impl Compiler {
    /// Query visible extension symbols for a target when dependency state is ready.
    pub(crate) fn query_visible_extension_symbols_for_target(
        &self,
        ctx: SymbolTypeView<'_>,
        target_symbol: GlobalSymbolId,
    ) -> Option<Vec<GlobalSymbolId>> {
        match self.visible_extension_symbols_for_target(ctx, target_symbol) {
            Ok(symbols) => Some(symbols),
            Err(AnalyzeError::Yield { .. } | AnalyzeError::UnsatisfiedRequirement { .. }) => None,
            Err(error) => {
                self.error(error);
                None
            }
        }
    }

    /// Collect extension symbols visible for a target in the current module.
    pub(crate) fn visible_extension_symbols_for_target(
        &self,
        ctx: SymbolTypeView<'_>,
        target_symbol: GlobalSymbolId,
    ) -> AnalyzeResult<Vec<GlobalSymbolId>> {
        // normalize reference-like symbols without taking tree locks
        let target_symbol =
            self.normalize_reference_symbol_id(ctx.module_symbol_view(), target_symbol);

        // canonicalize and declaration normalize the target symbol
        let canonical_target = self.canonical_symbol_id(
            ctx.module_symbol_view(),
            target_symbol,
            CanonicalSymbolMode::FollowAliases,
        );
        let declaration_target = self
            .declaration_symbol_id(ctx.module_symbol_view(), canonical_target)
            .unwrap_or(canonical_target);

        let mut extensions = Vec::new();
        let mut seen = HashSet::new();

        // include extensions declared in the current module
        if let Some(extension_ids) = ctx.types.get_extensions_for_target(declaration_target) {
            for extension_id in extension_ids {
                let extension = ctx.types.get_extension(*extension_id);
                if seen.insert(extension.symbol) {
                    extensions.push(extension.symbol);
                }
            }
        }

        // include local extensions from directly imported modules
        let declared_dir = self
            .require_artifact_dir_resolved(ctx.module.id, ctx.profile)
            .map_err(AnalyzeError::from)?;
        let mut imported_module_ids = HashSet::new();
        for resolution in declared_dir.imported_modules.values() {
            for target in [resolution.value, resolution.ty] {
                let Some(ModuleTarget::Module(module_id)) = target else {
                    continue;
                };
                imported_module_ids.insert(module_id);
            }
        }
        for imported_module_id in imported_module_ids {
            let imported_dir = self
                .require_artifact_dir_declared(imported_module_id, ctx.profile)
                .map_err(AnalyzeError::from)?;
            if let Some(extension_ids) = imported_dir
                .types
                .get_extensions_for_target(declaration_target)
            {
                for extension_id in extension_ids {
                    let extension = imported_dir.types.get_extension(*extension_id);
                    if extension.kind != ExtensionKind::Local {
                        continue;
                    }
                    if seen.insert(extension.symbol) {
                        extensions.push(extension.symbol);
                    }
                }
            }
        }

        // include inherent extensions from the target module
        let mut include_inherent_extensions = |target_symbol: GlobalSymbolId| -> AnalyzeResult<()> {
            self.with_module_types_or_local_for_artifact(
                ctx.module,
                ctx.profile,
                target_symbol.module_id,
                ctx.types,
                destack_workspace::ArtifactKey::dir_declared,
                |_, target_types| {
                    if let Some(extension_ids) =
                        target_types.get_extensions_for_target(target_symbol)
                    {
                        for extension_id in extension_ids {
                            let extension = target_types.get_extension(*extension_id);
                            if extension.kind != ExtensionKind::Inherent {
                                continue;
                            }
                            if seen.insert(extension.symbol) {
                                extensions.push(extension.symbol);
                            }
                        }
                    }
                },
            )
            .map_err(AnalyzeError::from)?;

            Ok(())
        };

        include_inherent_extensions(declaration_target)?;

        // include inherent extensions for global symbol groups
        let (should_scan_global_group, target_key, target_space) = self
            .with_module_symbols_or_local_for_artifact(
                ctx.module,
                ctx.profile,
                declaration_target.module_id,
                ctx.symbols,
                destack_workspace::ArtifactKey::dir_declared,
                |target_module, target_symbols| {
                    let symbol_entry = target_symbols.get_symbol(declaration_target.local_id);
                    let should_scan_global_group = if declaration_target.module_id == ctx.module.id
                    {
                        symbol_entry.origin.is_global_augmentation()
                    } else {
                        matches!(target_module.source, ModuleSource::Builtin(_))
                    };

                    (
                        should_scan_global_group,
                        symbol_entry.key,
                        symbol_entry.space,
                    )
                },
            )
            .map_err(AnalyzeError::from)?;

        if should_scan_global_group && let Some(key) = target_key {
            let mut merge_symbols = Vec::new();

            // target-discovery globals
            if let Some(global_symbols) =
                self.get_global_symbol_group(ctx.module.id, ctx.profile, key, target_space)
            {
                merge_symbols.extend(global_symbols);
            }

            // selected lib globals
            if let Some(lib_symbols) =
                self.get_library_symbol_sources_for_merge(ctx.profile, key, target_space)
            {
                merge_symbols.extend(lib_symbols);
            }

            for global_symbol in merge_symbols {
                let global_symbol = self.canonical_symbol_id(
                    ctx.module_symbol_view(),
                    global_symbol,
                    CanonicalSymbolMode::FollowAliases,
                );
                let global_symbol = self
                    .declaration_symbol_id(ctx.module_symbol_view(), global_symbol)
                    .unwrap_or(global_symbol);
                if global_symbol == declaration_target {
                    continue;
                }
                include_inherent_extensions(global_symbol)?;
            }
        }

        // include explicitly imported named extensions
        for local_symbol_id in ctx.symbols.active_symbol_ids() {
            if local_symbol_id.ty != SymbolType::Extension {
                continue;
            }

            let extension_symbol = local_symbol_id.into_global(ctx.module.id);
            let canonical_extension = self.canonical_symbol_id(
                ctx.module_symbol_view(),
                extension_symbol,
                CanonicalSymbolMode::FollowAliases,
            );
            if canonical_extension.module_id == ctx.module.id {
                continue;
            }
            if !seen.insert(canonical_extension) {
                continue;
            }

            let Some(extension) =
                self.extension_for_symbol_in_module(ctx.module_type_view(), canonical_extension)?
            else {
                continue;
            };
            let extension_target = self.canonical_symbol_id(
                ctx.module_symbol_view(),
                extension.target,
                CanonicalSymbolMode::FollowAliases,
            );
            let extension_target = self
                .declaration_symbol_id(ctx.module_symbol_view(), extension_target)
                .unwrap_or(extension_target);
            if extension_target == declaration_target {
                extensions.push(canonical_extension);
            }
        }

        Ok(extensions)
    }
    /// Load extension metadata for a symbol from its defining module.
    pub(crate) fn extension_for_symbol(
        &self,
        profile: ProfileId,
        extension_symbol: GlobalSymbolId,
    ) -> AnalyzeResult<Option<Extension>> {
        let dir = self
            .require_artifact_dir_declared(extension_symbol.module_id, profile)
            .map_err(AnalyzeError::from)?;
        let extension_id = dir.types.get_extension_id_for_symbol(extension_symbol);

        Ok(extension_id.map(|extension_id| dir.types.get_extension(extension_id).clone()))
    }

    /// Load extension metadata using local ctx when possible.
    pub(crate) fn extension_for_symbol_in_module(
        &self,
        ctx: ModuleTypeView<'_>,
        extension_symbol: GlobalSymbolId,
    ) -> AnalyzeResult<Option<Extension>> {
        if extension_symbol.module_id == ctx.module.id {
            let Some(extension_id) = ctx.types.get_extension_id_for_symbol(extension_symbol) else {
                return Ok(None);
            };
            return Ok(Some(ctx.types.get_extension(extension_id).clone()));
        }

        self.extension_for_symbol(ctx.profile, extension_symbol)
    }

    /// Query extension metadata using local ctx when dependency state is ready.
    pub(crate) fn query_extension_for_symbol_in_module(
        &self,
        ctx: ModuleTypeView<'_>,
        extension_symbol: GlobalSymbolId,
    ) -> Option<Extension> {
        match self.extension_for_symbol_in_module(ctx, extension_symbol) {
            Ok(extension) => extension,
            Err(AnalyzeError::Yield { .. } | AnalyzeError::UnsatisfiedRequirement { .. }) => None,
            Err(error) => {
                self.error(error);
                None
            }
        }
    }

    /// Load extension lineage data using local ctx when possible.
    pub(crate) fn extension_lineage_for_symbol_in_module(
        &self,
        ctx: ModuleTypeView<'_>,
        extension_symbol: GlobalSymbolId,
    ) -> AnalyzeResult<Option<Lineage>> {
        let Some(extension) = self.extension_for_symbol_in_module(ctx, extension_symbol)? else {
            return Ok(None);
        };
        let Some(lineage_id) = extension.lineage else {
            return Ok(None);
        };

        if extension_symbol.module_id == ctx.module.id {
            return Ok(Some(ctx.types.get_lineage(lineage_id).clone()));
        }

        let owner_dir = self
            .require_artifact_dir_declared(extension_symbol.module_id, ctx.profile)
            .map_err(AnalyzeError::from)?;
        Ok(Some(owner_dir.types.get_lineage(lineage_id).clone()))
    }

    /// Query extension lineage data when dependency state is ready.
    pub(crate) fn query_extension_lineage_for_symbol_in_module(
        &self,
        ctx: ModuleTypeView<'_>,
        extension_symbol: GlobalSymbolId,
    ) -> Option<Lineage> {
        match self.extension_lineage_for_symbol_in_module(ctx, extension_symbol) {
            Ok(lineage) => lineage,
            Err(AnalyzeError::Yield { .. } | AnalyzeError::UnsatisfiedRequirement { .. }) => None,
            Err(error) => {
                self.error(error);
                None
            }
        }
    }
}
