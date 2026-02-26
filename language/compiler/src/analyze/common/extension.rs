use std::collections::HashSet;

use crate::analyze::common::{
    AnalyzeDependencyStage, CanonicalSymbolMode, ModuleTypeView, SymbolTypeView,
};
use crate::{AnalyzeError, AnalyzeResult, Compiler};
use destack_dir::{Extension, ExtensionKind, GlobalSymbolId, Lineage, SymbolType};
use destack_workspace::{ModuleSource, ProfileId};

impl Compiler {
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

        // include inherent extensions from the target module
        let mut include_inherent_extensions = |target_symbol: GlobalSymbolId| -> AnalyzeResult<()> {
            self.with_module_types_or_local_at_stage(
                ctx.module,
                ctx.profile,
                target_symbol.module_id,
                ctx.types,
                AnalyzeDependencyStage::Declare,
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
            .with_module_symbols_or_local_at_stage(
                ctx.module,
                ctx.profile,
                declaration_target.module_id,
                ctx.symbols,
                AnalyzeDependencyStage::Declare,
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

        if should_scan_global_group
            && let Some(key) = target_key
            && let Some(global_symbols) =
                self.get_global_symbol_group(ctx.module.id, ctx.profile, key, target_space)
        {
            for global_symbol in global_symbols {
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

            let Some(extension) = self.extension_for_symbol(ctx.profile, canonical_extension)?
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
        self.with_module_types_by_id_at_stage(
            profile,
            extension_symbol.module_id,
            AnalyzeDependencyStage::Declare,
            |_, types| {
                let extension_id = types.get_extension_id_for_symbol(extension_symbol)?;

                Some(types.get_extension(extension_id).clone())
            },
        )
        .map_err(AnalyzeError::from)
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

        self.with_module_types_by_id_at_stage(
            ctx.profile,
            extension_symbol.module_id,
            AnalyzeDependencyStage::Declare,
            |_, owner_types| Some(owner_types.get_lineage(lineage_id).clone()),
        )
        .map_err(AnalyzeError::from)
    }
}
