use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{CheckState, ExtensionDefinition, ExtensionWhereClause};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Build extension declarations from walked operands.
    pub(in crate::check) fn build_extension_table(&mut self) -> CompilerResult<()> {
        let modules = self.modules.keys().copied().collect::<Vec<_>>();

        // build definitions in component order
        for module in modules {
            let symbols = self.extension_symbols(module);
            for symbol in symbols {
                let definition = self.build_extension_definition(module, symbol)?;
                self.extensions.insert_definition(symbol, definition);
            }
        }

        Ok(())
    }

    /// Build one extension definition.
    fn build_extension_definition(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<ExtensionDefinition> {
        let source = self
            .module(module)
            .symbol_declaration_node(symbol.local_id)
            .into_global(module);
        let declaration_id = source.local_id.into_typed::<dir::Declaration>();
        let declaration = self.module(module).view().get(declaration_id).clone();
        let dir::Declaration::Extension(declaration) = declaration else {
            return Err(CompilerError::Internal {
                message: format!(
                    "extension symbol {symbol:?} does not point at an extension declaration"
                ),
            });
        };
        let target_source = declaration.target_type.into_global_any(module);
        let Some(target_symbol) = self.selected_name(target_source) else {
            return Err(CompilerError::Internal {
                message: format!("extension target {target_source:?} has no selected symbol"),
            });
        };
        let target_type = self.node_type_operand(target_source)?;
        let form = Self::extension_form(module, &declaration, target_symbol);
        let where_clauses =
            self.build_extension_where_clauses(module, &declaration.where_clauses)?;

        Ok(ExtensionDefinition {
            source,
            form,
            target_symbol,
            target_type,
            where_clauses,
        })
    }

    /// Return extension symbols declared in one component module.
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

    /// Build extension where clauses.
    fn build_extension_where_clauses(
        &self,
        module: ModuleId,
        clauses: &[dir::LocalNodeId<dir::WhereClause>],
    ) -> CompilerResult<Vec<ExtensionWhereClause>> {
        let mut where_clauses = Vec::with_capacity(clauses.len());
        let view = self.module(module).view();

        // collect checked where clause operands
        for clause in clauses {
            let source = clause.into_global_any(module);
            let clause = view.get(*clause);
            let left = self.node_type_operand(clause.left.into_global_any(module))?;
            let right = self.node_type_operand(clause.right.into_global_any(module))?;

            where_clauses.push(ExtensionWhereClause {
                source,
                left,
                right,
            });
        }

        Ok(where_clauses)
    }

    /// Return how one extension should be made visible.
    fn extension_form(
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
