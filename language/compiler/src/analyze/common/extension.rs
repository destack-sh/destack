use std::collections::HashSet;

use crate::analyze::common::CanonicalSymbolMode;
use crate::{AnalyzeError, AnalyzeResult, Compiler};
use destack_dir::{
    Extension, ExtensionKind, GlobalSymbolId, Lineage, SymbolTable, SymbolType, TypeTable,
};
use destack_workspace::{Module, ModuleSource, ProfileId};

impl Compiler {
    /// Collect extension symbols visible for a target in the current module.
    pub(crate) fn visible_extension_symbols_for_target(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        types: &TypeTable,
        target_symbol: GlobalSymbolId,
    ) -> AnalyzeResult<Vec<GlobalSymbolId>> {
        // canonicalize the target symbol
        let canonical_target = self.canonical_symbol_id(
            module,
            symbols,
            profile,
            target_symbol,
            CanonicalSymbolMode::FollowAliases,
        );

        let mut extensions = Vec::new();
        let mut seen = HashSet::new();

        // include extensions declared in the current module
        if let Some(extension_ids) = types.get_extensions_for_target(canonical_target) {
            for extension_id in extension_ids {
                let extension = types.get_extension(*extension_id);
                if seen.insert(extension.symbol) {
                    extensions.push(extension.symbol);
                }
            }
        }

        // include inherent extensions from the target module
        let mut include_inherent_extensions = |target_symbol: GlobalSymbolId| -> AnalyzeResult<()> {
            if target_symbol.module_id == module.id {
                return Ok(());
            }

            self.require_analyze_module_declare(target_symbol.module_id, profile)
                .map_err(AnalyzeError::from)?;

            let target_module = self.program.modules.get(target_symbol.module_id);
            let target_module = target_module.read();
            let target_types = target_module.dir(profile).types.read();

            if let Some(extension_ids) = target_types.get_extensions_for_target(target_symbol) {
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

            Ok(())
        };

        include_inherent_extensions(canonical_target)?;

        // include inherent extensions for global symbol groups
        let should_scan_global_group;
        let (target_key, target_space) = if canonical_target.module_id == module.id {
            let symbol_entry = symbols.get_symbol(canonical_target.local_id);
            should_scan_global_group = symbol_entry.origin.is_global_augmentation();
            (symbol_entry.key, symbol_entry.space)
        } else {
            let target_module = self.program.modules.get(canonical_target.module_id);
            let target_module = target_module.read();
            should_scan_global_group = matches!(target_module.source, ModuleSource::Builtin(_));
            let target_symbols = target_module.dir(profile).symbols.read();
            let symbol_entry = target_symbols.get_symbol(canonical_target.local_id);
            (symbol_entry.key, symbol_entry.space)
        };

        if should_scan_global_group
            && let Some(key) = target_key
            && let Some(global_symbols) =
                self.get_global_symbol_group(module.id, profile, key, target_space)
        {
            for global_symbol in global_symbols {
                if global_symbol == canonical_target {
                    continue;
                }
                include_inherent_extensions(global_symbol)?;
            }
        }

        // include explicitly imported named extensions
        for local_symbol_id in symbols.active_symbol_ids() {
            if local_symbol_id.ty != SymbolType::Extension {
                continue;
            }

            let extension_symbol = local_symbol_id.into_global(module.id);
            let canonical_extension = self.canonical_symbol_id(
                module,
                symbols,
                profile,
                extension_symbol,
                CanonicalSymbolMode::FollowAliases,
            );
            if canonical_extension.module_id == module.id {
                continue;
            }
            if !seen.insert(canonical_extension) {
                continue;
            }

            let Some(extension) = self.extension_for_symbol(profile, canonical_extension)? else {
                continue;
            };
            if extension.target == canonical_target {
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
        self.require_analyze_module_declare(extension_symbol.module_id, profile)
            .map_err(AnalyzeError::from)?;

        let module = self.program.modules.get(extension_symbol.module_id);
        let module = module.read();
        let types = module.dir(profile).types.read();
        let Some(extension_id) = types.get_extension_id_for_symbol(extension_symbol) else {
            return Ok(None);
        };

        Ok(Some(types.get_extension(extension_id).clone()))
    }

    /// Load extension metadata using local tables when possible.
    pub(crate) fn extension_for_symbol_in_module(
        &self,
        module: &Module,
        profile: ProfileId,
        extension_symbol: GlobalSymbolId,
        types: &TypeTable,
    ) -> AnalyzeResult<Option<Extension>> {
        if extension_symbol.module_id == module.id {
            let Some(extension_id) = types.get_extension_id_for_symbol(extension_symbol) else {
                return Ok(None);
            };
            return Ok(Some(types.get_extension(extension_id).clone()));
        }

        self.extension_for_symbol(profile, extension_symbol)
    }

    /// Load extension lineage data using local tables when possible.
    pub(crate) fn extension_lineage_for_symbol_in_module(
        &self,
        module: &Module,
        profile: ProfileId,
        extension_symbol: GlobalSymbolId,
        types: &TypeTable,
    ) -> AnalyzeResult<Option<Lineage>> {
        let Some(extension) =
            self.extension_for_symbol_in_module(module, profile, extension_symbol, types)?
        else {
            return Ok(None);
        };
        let Some(lineage_id) = extension.lineage else {
            return Ok(None);
        };

        if extension_symbol.module_id == module.id {
            return Ok(Some(types.get_lineage(lineage_id).clone()));
        }

        let module = self.program.modules.get(extension_symbol.module_id);
        let module = module.read();
        let types = module.dir(profile).types.read();
        Ok(Some(types.get_lineage(lineage_id).clone()))
    }
}
