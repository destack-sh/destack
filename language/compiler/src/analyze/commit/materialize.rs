use crate::analyze::StaticSubstitutionEnvironment;
use crate::{AnalyzeResult, Compiler};
use destack_dir::{GlobalSymbolId, InferTable, Instance, LocalInstanceId, TypeTable};
use destack_workspace::Module;

impl Compiler {
    /// Commit infer-recorded provisional resolutions and instances into canonical tables.
    pub(super) fn commit_provisional_resolutions_and_instances(
        &self,
        module: &Module,
        infer: &InferTable,
        types: &mut TypeTable,
    ) {
        // commit recorded resolutions in deterministic node-id order
        let mut resolution_entries = infer
            .iter_provisional_resolution_nodes()
            .collect::<Vec<_>>();
        resolution_entries.sort_by_key(|(node_id, _)| *node_id);
        for (node_id, resolution) in resolution_entries {
            if node_id.module_id == module.id {
                let resolution_id = types.insert_resolution(resolution.clone());
                types.set_resolution_for_node(node_id, resolution_id);
            }
        }

        // commit recorded instance attachments in deterministic node-id order
        let mut instance_entries = infer.iter_provisional_instance_nodes().collect::<Vec<_>>();
        instance_entries.sort_by_key(|(node_id, _)| *node_id);
        for (node_id, instance_id) in instance_entries {
            if node_id.module_id == module.id {
                types.set_instance_for_node(node_id, instance_id);
            }
        }
    }

    /// Commit one instance for one symbol and one substitution environment.
    pub(super) fn commit_instance_for_symbol_environment(
        &self,
        symbol_id: GlobalSymbolId,
        environment: StaticSubstitutionEnvironment,
        types: &mut TypeTable,
    ) -> AnalyzeResult<LocalInstanceId> {
        let environment = self.normalize_instance_environment_for_commit(symbol_id, environment)?;
        let existing_instance = self.query_instance_for_symbol_environment(
            symbol_id,
            &environment.arguments,
            &environment.parameter_symbols,
            environment.inherited_arity,
            types,
        )?;
        if let Some(existing_instance) = existing_instance {
            return Ok(existing_instance);
        }

        let arguments = environment.arguments;
        let parameter_symbols = environment.parameter_symbols;
        let inherited_arity = environment.inherited_arity;
        let instance =
            Instance::with_environment(symbol_id, arguments, parameter_symbols, inherited_arity)
                .map_err(|error| {
                    self.internal_analyze_error(format!(
                        "commit_instance_for_symbol_environment: invalid committed instance environment for symbol {symbol_id:?}: {error:?}"
                    ))
                })?;
        Ok(types.insert_instance(instance))
    }
}
