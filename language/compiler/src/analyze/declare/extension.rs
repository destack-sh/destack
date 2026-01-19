use std::collections::HashSet;

use crate::{AnalyzeError, AnalyzeResult, Compiler};
use destack_dir::{
    Expression, Extension, ExtensionKind, GlobalSymbolId, Lineage, LocalLineageId, LocalSymbolId,
    NodeTree, SymbolTable, SymbolType, TypeTable,
};
use destack_workspace::{Module, ProfileId};

impl Compiler {
    /// Register extensions visible in a module from imported symbols.
    pub(super) fn register_visible_extensions(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<()> {
        // collect visible nominal types and extension symbols
        let mut target_symbols = Vec::new();
        let mut extension_symbols = Vec::new();
        let mut seen_targets = HashSet::new();
        let mut seen_extensions = HashSet::new();

        // helper to register remote symbols as targets or extensions
        let mut register_symbol = |symbol: GlobalSymbolId| {
            let canonical_symbol = self.canonical_symbol_id(module, symbols, profile, symbol);
            if canonical_symbol.module_id == module.id {
                return;
            }

            match canonical_symbol.ty() {
                SymbolType::Struct
                | SymbolType::Class
                | SymbolType::Interface
                | SymbolType::Enum
                | SymbolType::Newtype => {
                    if seen_targets.insert(canonical_symbol) {
                        target_symbols.push(canonical_symbol);
                    }
                }
                SymbolType::Extension => {
                    if seen_extensions.insert(canonical_symbol) {
                        extension_symbols.push(canonical_symbol);
                    }
                }
                _ => {}
            }
        };

        // register remote symbols for targets and extensions
        for local_symbol in symbols.active_symbol_ids() {
            let symbol_entry = symbols.get_symbol(local_symbol);
            let symbol_id = LocalSymbolId::new_typed(local_symbol.id, symbol_entry.ty);
            let global_symbol = symbol_id.into_global(module.id);
            register_symbol(global_symbol);
        }

        // register expressions for remote symbols referenced directly
        for (_expression_id, expression) in tree.iter_nodes_of_type::<Expression>() {
            let Some(target_symbol) = expression.target_symbol() else {
                continue;
            };
            register_symbol(target_symbol);
        }

        // register inherent extensions from imported nominal types
        for target_symbol in target_symbols {
            self.import_inherent_extensions_for_target(module, profile, target_symbol, types)?;
        }

        // register named extensions from imported extension symbols
        for extension_symbol in extension_symbols {
            self.import_named_extension(profile, extension_symbol, types)?;
        }

        Ok(())
    }

    /// Import inherent extensions for a target symbol into the local type table.
    fn import_inherent_extensions_for_target(
        &self,
        module: &Module,
        profile: ProfileId,
        target_symbol: GlobalSymbolId,
        types: &mut TypeTable,
    ) -> AnalyzeResult<()> {
        // skip local targets because their extensions are already registered
        if target_symbol.module_id == module.id {
            return Ok(());
        }

        // ensure the target module is declared before reading its types
        self.require_analyze_module_declare(target_symbol.module_id, profile)
            .map_err(AnalyzeError::from)?;

        // load the remote type table for extensions
        let remote_module = self.program.modules.get(target_symbol.module_id);
        let remote_module = remote_module.read();
        let remote_types = remote_module.dir(profile).types.read();

        // import all inherent extensions for the target
        let Some(extension_ids) = remote_types.get_extensions_for_target(target_symbol) else {
            return Ok(());
        };
        for extension_id in extension_ids {
            let extension = remote_types.get_extension(*extension_id);
            if extension.kind != ExtensionKind::Inherent {
                continue;
            }

            self.import_extension_from_remote(extension, &remote_types, types);
        }

        Ok(())
    }

    /// Import a named extension symbol into the local type table.
    fn import_named_extension(
        &self,
        profile: ProfileId,
        extension_symbol: GlobalSymbolId,
        types: &mut TypeTable,
    ) -> AnalyzeResult<()> {
        // skip extensions that are already registered locally
        if types
            .get_extension_id_for_symbol(extension_symbol)
            .is_some()
        {
            return Ok(());
        }

        // ensure the extension module is declared before reading its types
        self.require_analyze_module_declare(extension_symbol.module_id, profile)
            .map_err(AnalyzeError::from)?;

        // load the remote type table for the extension
        let remote_module = self.program.modules.get(extension_symbol.module_id);
        let remote_module = remote_module.read();
        let remote_types = remote_module.dir(profile).types.read();

        // fetch the remote extension declaration
        let Some(remote_extension_id) = remote_types.get_extension_id_for_symbol(extension_symbol)
        else {
            return Ok(());
        };
        let extension = remote_types.get_extension(remote_extension_id);
        if extension.kind != ExtensionKind::Nominal {
            return Ok(());
        }

        self.import_extension_from_remote(extension, &remote_types, types);

        Ok(())
    }

    /// Import a remote extension declaration into the local type table.
    fn import_extension_from_remote(
        &self,
        extension: &Extension,
        remote_types: &TypeTable,
        types: &mut TypeTable,
    ) {
        // avoid duplicating already registered extensions
        if types
            .get_extension_id_for_symbol(extension.symbol)
            .is_some()
        {
            return;
        }

        // copy the extension lineage into the local table
        let lineage = extension
            .lineage
            .map(|lineage_id| self.import_lineage_from_remote(lineage_id, remote_types, types));

        // insert the extension record keyed by the canonical target
        let imported = Extension::new(extension.symbol, extension.kind, extension.target, lineage);
        types.insert_extension(imported);
    }

    /// Import a remote lineage into the local type table.
    fn import_lineage_from_remote(
        &self,
        lineage_id: LocalLineageId,
        remote_types: &TypeTable,
        types: &mut TypeTable,
    ) -> LocalLineageId {
        // clone the remote lineage for local storage
        let remote_lineage = remote_types.get_lineage(lineage_id).clone();
        let local_lineage = Lineage {
            extends: remote_lineage.extends,
            implements: remote_lineage.implements,
            embedded: remote_lineage.embedded,
        };

        types.insert_lineage(local_lineage)
    }
}
