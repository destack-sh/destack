use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{CheckState, ExtensionDefinition, ExtensionWhereClause};

impl CheckState<'_> {
    /// Return inherent extension symbols declared in one module for one receiver symbol.
    pub(in crate::check) fn extension_symbols_for_target(
        &mut self,
        declaration_module: ModuleId,
        target: dir::GlobalSymbolId,
    ) -> CompilerResult<Vec<dir::GlobalSymbolId>> {
        if self.is_component_module(declaration_module) {
            return Ok(self
                .extensions
                .symbols_for_target(declaration_module, target));
        }

        let dependency = self.dependency(declaration_module);
        let mut symbols = Vec::new();

        // collect dependency extensions indexed by receiver symbol
        for extension_id in dependency.extensions.target_extensions(target) {
            let extension = dependency.extensions.get_extension(extension_id);
            if extension.is_inherent() {
                symbols.push(extension.symbol);
            }
        }

        Ok(symbols)
    }

    /// Return one visible extension definition.
    pub(in crate::check) fn extension_definition(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<ExtensionDefinition>> {
        if self.is_component_module(symbol.module_id) {
            return Ok(self.extensions.definition(symbol).cloned());
        }

        let dependency = self.dependency(symbol.module_id);
        let Some(extension_id) = dependency.extensions.symbol_extension_id(symbol) else {
            return Ok(None);
        };
        let extension = dependency.extensions.get_extension(extension_id).clone();
        let source = dependency
            .extensions
            .extension_source(extension_id)
            .unwrap_or_else(|| panic!("dependency extension {extension_id:?} has no source"));

        Ok(Some(
            self.import_extension_definition(module, source, extension),
        ))
    }

    /// Import one extension definition.
    fn import_extension_definition(
        &mut self,
        module: ModuleId,
        source: dir::GlobalNodeIdAny,
        extension: dir::Extension,
    ) -> ExtensionDefinition {
        let target_type = self.import_type_operand(module, extension.target_type);
        let where_clauses = extension
            .where_clauses
            .into_iter()
            .map(|where_clause| ExtensionWhereClause {
                source: where_clause.source,
                left: self.import_type_operand(module, where_clause.left),
                right: self.import_type_operand(module, where_clause.right),
            })
            .collect();

        ExtensionDefinition {
            source,
            form: extension.form,
            target_symbol: extension.target_symbol,
            target_type,
            where_clauses,
        }
    }
}
