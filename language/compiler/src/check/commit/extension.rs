use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::CheckState;

use super::CheckModuleOutput;

impl CheckState<'_> {
    /// Commit extension declarations into the checked extension table.
    pub(super) fn commit_extension_table(
        &self,
        module: ModuleId,
        output: &CheckModuleOutput,
    ) -> dir::ExtensionSegment {
        let mut table = dir::ExtensionSegment::new(module);
        let symbols = self.extension_symbols(module);

        // commit extensions in source order
        for symbol in symbols {
            let extension = self.commit_extension(module, output, symbol);

            table.insert_extension(extension);
        }

        table
    }

    /// Commit one extension declaration.
    fn commit_extension(
        &self,
        module: ModuleId,
        output: &CheckModuleOutput,
        symbol: dir::GlobalSymbolId,
    ) -> dir::Extension {
        let source = self.symbol_source_node(symbol);
        let declaration_id = source.into_typed::<dir::Declaration>();
        let declaration = self.module(module).view().get(declaration_id);
        let dir::Declaration::Extension(declaration) = declaration else {
            panic!("extension symbol {symbol:?} does not point at an extension declaration");
        };
        let target = declaration.target_type.into_global_any(module);
        let target_symbol = output
            .resolutions
            .symbol_resolution(target)
            .unwrap_or_else(|| panic!("extension target {target:?} has no nominal resolution"));
        let target_type = output
            .types
            .get_node_type_id(target)
            .unwrap_or_else(|| panic!("extension target {target:?} has no checked type"));
        let form = self.extension_form(module, declaration, target_symbol);

        dir::Extension::new(symbol, form, target_symbol, target_type)
    }

    /// Return extension symbols declared in one checked module.
    fn extension_symbols(&self, module: ModuleId) -> Vec<dir::GlobalSymbolId> {
        let state = self.module(module);
        let bindings = state.binding_table();
        let mut symbols = Vec::new();

        // collect extension declaration symbols in source order
        for (source, symbol) in bindings.declaration_symbols() {
            if source.module_id != module || source.local_id.ty != dir::NodeType::Declaration {
                continue;
            }
            let symbol = symbol.into_global(module);
            let kind = bindings.get_symbol(symbol.local_id).kind;

            if kind == dir::SymbolKind::Extension {
                symbols.push(symbol);
            }
        }

        symbols
    }

    /// Return how one extension should be made visible.
    fn extension_form(
        &self,
        module: ModuleId,
        declaration: &dir::ExtensionDeclaration,
        target_symbol: dir::GlobalSymbolId,
    ) -> dir::ExtensionForm {
        // same module extensions are inherent
        if target_symbol.module_id == module {
            return dir::ExtensionForm::Inherent;
        }

        // named foreign extensions are imported explicitly
        if declaration.name.is_some() {
            return dir::ExtensionForm::Named;
        }

        dir::ExtensionForm::Local
    }
}
