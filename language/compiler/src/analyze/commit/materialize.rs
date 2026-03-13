use crate::analyze::StaticSubstitutionEnvironment;
use crate::analyze::common::{CommitContext, TypeRewriteCache};
use crate::{AnalyzeResult, Compiler};
use destack_dir::{
    GlobalSymbolId, InferTable, Instance, LocalInstanceId, LocalSymbolId, LocalTypeId, NodeType,
    Type, TypeTable,
};

impl Compiler {
    /// Reject one published committed type surface that still exposes unevaluated state.
    fn require_published_committed_type_surface(
        &self,
        symbol_id: GlobalSymbolId,
        surface: &str,
        type_id: LocalTypeId,
        types: &TypeTable,
    ) -> AnalyzeResult<()> {
        let mut visited = Default::default();
        if self.type_has_unevaluated_state(type_id, types, &mut visited) {
            let ty = types.get_type(type_id);
            return Err(self.internal_analyze_error(format!(
                "materialize_committed_symbol_type_surfaces: published {surface} surface for symbol {symbol_id:?} still contains unevaluated type state: {ty:?}",
            )));
        }

        Ok(())
    }

    /// Resolve one committed type slot to its published surface.
    fn resolve_committed_type_surface(
        &self,
        ctx: &mut CommitContext<'_>,
        type_id: LocalTypeId,
    ) -> AnalyzeResult<()> {
        let expression_id = match ctx.types.get_type(type_id).clone() {
            Type::Unevaluated(expression_id) => expression_id,
            _ => return Ok(()),
        };

        // publish committed value surfaces with fully resolved static arguments,
        // even for declaration modules
        let resolved_type = self.resolve_declared_type_expression_value(
            &mut ctx.type_context_reborrow(),
            expression_id,
            true,
            true,
            true,
            true,
            true,
        )?;
        if matches!(resolved_type, Type::Unevaluated(_)) {
            return Ok(());
        }

        ctx.types.update_type(type_id, resolved_type);
        Ok(())
    }

    /// Materialize one committed type slot for downstream consumers.
    fn materialize_committed_type_surface(
        &self,
        ctx: &mut CommitContext<'_>,
        type_id: LocalTypeId,
        cache: &mut TypeRewriteCache,
    ) -> AnalyzeResult<LocalTypeId> {
        let mut committed_type_id = type_id;

        loop {
            loop {
                let unevaluated_type_ids =
                    self.collect_unevaluated_type_ids(committed_type_id, ctx.types);
                if unevaluated_type_ids.is_empty() {
                    break;
                }

                let mut made_progress = false;
                for unevaluated_type_id in unevaluated_type_ids {
                    self.resolve_committed_type_surface(ctx, unevaluated_type_id)?;
                    if !matches!(
                        ctx.types.get_type(unevaluated_type_id),
                        Type::Unevaluated(_)
                    ) {
                        made_progress = true;
                    }
                }

                if !made_progress {
                    break;
                }
            }

            let materialized_type_id = self.materialize_static_arguments_in_type(
                &mut ctx.type_context_reborrow(),
                committed_type_id,
                cache,
            );
            if materialized_type_id == committed_type_id {
                return Ok(committed_type_id);
            }

            committed_type_id = materialized_type_id;
        }
    }

    /// Materialize committed value and instance types for downstream consumers.
    pub(super) fn materialize_committed_symbol_type_surfaces(
        &self,
        ctx: &mut CommitContext<'_>,
    ) -> AnalyzeResult<()> {
        let mut value_updates = Vec::new();
        let mut instance_updates = Vec::new();
        let mut materialize_cache = TypeRewriteCache::new();

        // commit one stable published surface per local symbol
        for (symbol_index, symbol) in ctx.module.symbols.symbols().enumerate() {
            let symbol_id = GlobalSymbolId::new(
                ctx.module.module.id,
                LocalSymbolId::new_typed(symbol_index as u32, symbol.ty),
            );
            let is_published_symbol = symbol.export.is_some()
                && symbol.is_active()
                && symbol
                    .primary_declaration
                    .is_some_and(|declaration| declaration.local_id.ty == NodeType::Declaration);
            let instance_type_id = ctx.types.get_instance_type_id(symbol_id);
            if let Some(value_type_id) = ctx.types.get_value_type_id(symbol_id) {
                let materialized_type_id = self.materialize_committed_type_surface(
                    ctx,
                    value_type_id,
                    &mut materialize_cache,
                )?;
                if is_published_symbol {
                    self.require_published_committed_type_surface(
                        symbol_id,
                        "value",
                        materialized_type_id,
                        ctx.types,
                    )?;
                }
                if materialized_type_id != value_type_id {
                    value_updates.push((symbol_id, materialized_type_id));
                }
            }

            if let Some(instance_type_id) = instance_type_id {
                let materialized_type_id = self.materialize_committed_type_surface(
                    ctx,
                    instance_type_id,
                    &mut materialize_cache,
                )?;
                if is_published_symbol {
                    self.require_published_committed_type_surface(
                        symbol_id,
                        "instance",
                        materialized_type_id,
                        ctx.types,
                    )?;
                }
                if materialized_type_id != instance_type_id {
                    instance_updates.push((symbol_id, materialized_type_id));
                }
            }
        }

        // publish the committed materialized value surface
        for (symbol_id, materialized_type_id) in value_updates {
            ctx.types.set_value_type(symbol_id, materialized_type_id);
        }

        // publish the committed materialized instance surface
        for (symbol_id, materialized_type_id) in instance_updates {
            ctx.types.set_instance_type(symbol_id, materialized_type_id);
        }

        Ok(())
    }

    /// Commit infer-recorded provisional resolutions and instances into canonical ctx.
    pub(super) fn commit_provisional_resolutions_and_instances(
        &self,
        ctx: &mut CommitContext<'_>,
        infer: &InferTable,
    ) {
        // commit recorded resolutions in deterministic node-id order
        let mut resolution_entries = infer
            .iter_provisional_resolution_nodes()
            .collect::<Vec<_>>();
        resolution_entries.sort_by_key(|(node_id, _)| *node_id);
        for (node_id, resolution) in resolution_entries {
            if node_id.module_id == ctx.module.module.id {
                let resolution_id = ctx.types.insert_resolution(resolution.clone());
                ctx.types.set_resolution_for_node(node_id, resolution_id);
            }
        }

        // commit recorded instance attachments in deterministic node-id order
        let mut instance_entries = infer.iter_provisional_instance_nodes().collect::<Vec<_>>();
        instance_entries.sort_by_key(|(node_id, _)| *node_id);
        for (node_id, instance_id) in instance_entries {
            if node_id.module_id == ctx.module.module.id {
                ctx.types.set_instance_for_node(node_id, instance_id);
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
