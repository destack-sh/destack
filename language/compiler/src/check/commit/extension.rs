use destack_artifact::GlobalEnvironment;
use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{CheckState, ExtensionDefinition};

use super::CheckModuleOutput;

impl CheckState<'_> {
    /// Commit extension definitions into the DIR extension table.
    pub(super) fn commit_extension_table(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
    ) -> dir::ExtensionSegment {
        let mut table = dir::ExtensionSegment::new(module);
        let definitions = self
            .extensions
            .definitions_in(module)
            .map(|(symbol, definition)| (symbol, definition.clone()))
            .collect::<Vec<_>>();

        // commit definitions in build order
        for (symbol, definition) in definitions {
            let source = definition.source;
            let extension =
                self.commit_extension_definition(module, output, environment, symbol, definition);

            table.insert_extension(source, extension);
        }

        table
    }

    /// Commit one extension definition.
    fn commit_extension_definition(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        symbol: dir::GlobalSymbolId,
        definition: ExtensionDefinition,
    ) -> dir::Extension {
        let source = definition.source.local_id;
        let target_type = self
            .commit_type_operand(module, output, environment, definition.target_type, source)
            .unwrap_or_else(|| {
                panic!(
                    "extension target {:?} has no committed type",
                    definition.source
                )
            });
        let where_clauses =
            self.commit_extension_where_clauses(module, output, environment, &definition);

        dir::Extension::new(
            symbol,
            definition.form,
            definition.target_symbol,
            target_type,
            where_clauses,
        )
    }

    /// Commit extension where clauses.
    fn commit_extension_where_clauses(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        definition: &ExtensionDefinition,
    ) -> Vec<dir::ExtensionWhereClause> {
        let mut where_clauses = Vec::with_capacity(definition.where_clauses.len());

        // commit each where clause operand pair
        for where_clause in &definition.where_clauses {
            let source = where_clause.source.local_id;
            let left = self
                .commit_type_operand(module, output, environment, where_clause.left, source)
                .unwrap_or_else(|| {
                    panic!(
                        "extension where clause {:?} has no committed left type",
                        where_clause.source
                    )
                });
            let right = self
                .commit_type_operand(module, output, environment, where_clause.right, source)
                .unwrap_or_else(|| {
                    panic!(
                        "extension where clause {:?} has no committed right type",
                        where_clause.source
                    )
                });

            where_clauses.push(dir::ExtensionWhereClause {
                source: where_clause.source,
                left,
                right,
            });
        }

        where_clauses
    }
}
