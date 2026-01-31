use std::sync::Arc;

use crate::timing::tags;
use crate::{
    AnalyzeError, AnalyzeResult, Compiler, FlowContext, InferContext, TaskDependencyError,
    TaskResultCollector,
};
use destack_dir::{
    Declaration, Expression, FlowGraphBuilder, InferTable, IntType, LocalNodeId, NodeTree,
    PrimitiveType, Type, TypeLiteral,
};
use destack_source::{ModuleId, ModuleVersion, ProfileVersion};
use destack_workspace::{
    Module, ModuleContent, ModuleGraphKey, ModuleSource, ModuleType, ProfileId,
};

use super::super::common::json_value_to_type;

impl Compiler {
    /// Ensure a module's types have been inferred.
    pub fn require_analyze_module_infer(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<(), TaskDependencyError> {
        use crate::AnalyzeTask;
        let module = self.module_stamp(module);
        let profile = self.profile_stamp(profile);
        self.do_require_task_internal_only(AnalyzeTask::AnalyzeModuleInfer { module, profile })
    }

    /// Phase 2: Infer expression types.
    pub(crate) fn analyze_module_infer(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        module_version: ModuleVersion,
        profile_version: ProfileVersion,
    ) -> AnalyzeResult<()> {
        // skip stale tasks
        self.ensure_module_profile_matches::<AnalyzeError>(
            module_id,
            module_version,
            profile,
            profile_version,
        )?;
        let _timing = self.timing_scope(tags::ANALYZE_MODULE_INFER);

        // analyze data modules specially
        if !self.is_code_module(module_id) {
            return self.analyze_data_module_infer(module_id, profile);
        }

        // skip inference when module language is disabled
        if !self.module_language_allowed(module_id) {
            return Ok(());
        }

        self.require_analyze_module_declare(module_id, profile)?;

        let module = self.program.modules.get(module_id);
        let module = module.read();
        let dir = module.dir(profile);
        let tree = dir.tree.read();
        let symbols = dir.symbols.read();
        let mut types = dir.types.write();

        if module.language_type.is_declaration() {
            let module_checks = self.module_check_options_for_module(module.id);
            let should_skip_enum_validation = module_checks.skip_lib_check
                || (matches!(module.source, ModuleSource::Builtin(_))
                    && !self.options.validate_builtin_libs);
            if should_skip_enum_validation {
                return Ok(());
            }

            // infer enum backing types when declaration validation is enabled
            for root_id in dir.roots.iter() {
                let Expression::Declaration { declaration } = tree.get(*root_id) else {
                    continue;
                };
                let Declaration::Enum {
                    descriptor, fields, ..
                } = tree.get(*declaration)
                else {
                    continue;
                };
                let enum_symbol = descriptor.symbol.into_global(module.id);
                let backing_type = self.infer_enum_field_values(
                    &module,
                    profile,
                    enum_symbol,
                    fields,
                    &tree,
                    &symbols,
                    &mut types,
                )?;
                types.set_enum_backing_type(enum_symbol, backing_type);
            }

            return Ok(());
        }

        // select runtime roots for the inference pass
        let runtime_roots = self.collect_runtime_roots(&module, &tree, &dir.roots);
        let infer_roots = runtime_roots.clone();

        if infer_roots.is_empty() {
            return Ok(());
        }

        drop(types);
        drop(symbols);
        drop(tree);

        self.require_analyze_module_export(module_id, profile)?;
        self.require_export_inference_dependencies(module_id, profile)?;
        self.require_export_inference_for_ambient_libs(profile)?;

        let dir = module.dir(profile);
        let tree = dir.tree.read();
        let symbols = dir.symbols.read();
        let mut types = dir.types.write();
        let mut collector = TaskResultCollector::new();
        let options = self.analyze_context_options_for_module(module.id);

        // require builtins for inference
        self.require_resolve_builtins(profile)?;

        // analyze all expressions
        let mut infer = InferTable::default();
        let mut ctx = InferContext::new(profile, options);

        // build a module level flow graph and flow table when needed
        let flow_roots = {
            let _timing = self.timing_scope(tags::ANALYZE_FLOW_REQUIREMENTS);
            runtime_roots
                .iter()
                .copied()
                .filter(|root_id| self.expression_requires_flow(&tree, *root_id))
                .collect::<Vec<_>>()
        };
        if !flow_roots.is_empty() {
            let graph = {
                let _timing = self.timing_scope(tags::ANALYZE_FLOW_GRAPH_BUILD);
                FlowGraphBuilder::new(module.id, &tree).build_roots(&flow_roots)
            };
            let flow = {
                let _timing = self.timing_scope(tags::ANALYZE_FLOW_TABLE_COMPUTE);
                self.compute_flow_table_for_graph(
                    &module, &graph, &tree, &symbols, &mut types, &mut infer, &ctx,
                )?
            };
            ctx.flow = Some(FlowContext {
                module_id: module.id,
                graph: Arc::new(graph),
                table: Arc::new(flow),
            });
        }

        // infer each root expression
        {
            let _timing = self.timing_scope(tags::ANALYZE_EXPRESSION_INFER);
            for root_id in infer_roots.iter() {
                self.collect(
                    &mut collector,
                    self.infer_expression(
                        &module, *root_id, &tree, &symbols, &mut types, &mut infer, &mut ctx,
                    ),
                );
            }
        }

        // register instances
        {
            let _timing = self.timing_scope(tags::ANALYZE_INFER_REGISTER_INSTANCES);
            self.collect(
                &mut collector,
                self.register_instances(&module, profile, &tree, &symbols, &mut types),
            );
        }

        // yield on any yields
        if let Some(dependency) = collector.try_into_yield_any() {
            return Err(AnalyzeError::Yield { dependency });
        }

        // solve constraints (and commit inferred types)
        if !infer.vars.is_empty() || !infer.constraints.is_empty() {
            let _timing = self.timing_scope(tags::ANALYZE_INFER_SOLVE_CONSTRAINTS);
            self.solve_infer_table(&module, profile, &symbols, &infer, &mut types, &ctx.options);
        }

        Ok(())
    }

    /// Collect root expressions that can produce runtime behavior.
    fn collect_runtime_roots(
        &self,
        module: &Module,
        tree: &NodeTree,
        roots: &[LocalNodeId<Expression>],
    ) -> Vec<LocalNodeId<Expression>> {
        let mut runtime_roots = Vec::new();
        for root_id in roots {
            if self.expression_requires_infer(module, *root_id, tree) {
                runtime_roots.push(*root_id);
            }
        }

        runtime_roots
    }

    /// Ensure export inference tasks are complete for direct module dependencies.
    pub(crate) fn require_export_inference_dependencies(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> AnalyzeResult<()> {
        // skip when the module graph is unavailable
        let key = ModuleGraphKey::new(profile);
        let Some(graph) = self.program.index.module_graphs.get(&key) else {
            return Ok(());
        };

        // require export inference for direct dependencies
        for dependency in graph.dependencies_for(module_id) {
            if let Err(error) = self.require_analyze_module_export(dependency, profile) {
                return Err(AnalyzeError::from(error));
            }
        }

        Ok(())
    }

    /// Ensure export inference tasks are complete for ambient lib modules.
    fn require_export_inference_for_ambient_libs(&self, profile: ProfileId) -> AnalyzeResult<()> {
        // skip when builtins are not loaded
        let Some(builtins) = self.builtins() else {
            return Ok(());
        };

        // resolve the profile key for ambient lib lookup
        let profile_entry = self
            .program
            .profiles
            .get(profile)
            .unwrap_or_else(|| panic!("missing profile data for {profile:?}"));

        // require export inference for ambient lib modules
        let Some(lib_modules) = builtins.ambient_libs(&profile_entry.key) else {
            return Ok(());
        };
        for module_id in lib_modules {
            if let Err(error) = self.require_analyze_module_export(module_id, profile) {
                return Err(AnalyzeError::from(error));
            }
        }

        Ok(())
    }

    /// Infer types for data modules (JSON, TOML, YAML), text modules, and binary modules.
    ///
    /// This function converts the parsed data into a structural DIR type and associates
    /// it with the module's default export symbol.
    fn analyze_data_module_infer(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> AnalyzeResult<()> {
        let module_ref = self.program.modules.get(module_id);
        let module = module_ref.read();
        let dir = module.dir(profile);
        let default_symbol = dir.default_symbol;
        let source_id = dir.anchor_node;
        let module_type = module.module_type;

        match module_type {
            ModuleType::Data => {
                let value = match &module.content {
                    ModuleContent::Data { value, .. } => value.clone(),
                    _ => return Ok(()),
                };
                drop(module);

                // re-acquire the module and infer type
                let module_ref = self.program.modules.get(module_id);
                let module = module_ref.read();
                let dir = module.dir(profile);
                let mut types = dir.types.write();

                let inferred_type =
                    json_value_to_type(&value, source_id, &mut types, &self.program.strings);

                // associate the inferred type with the default symbol
                types.set_value_type(default_symbol.into_global(module_id), inferred_type);
            }
            ModuleType::Text => {
                // text modules are always string
                let mut types = dir.types.write();
                let string_type = types.insert_type_from_any(
                    Type::TypeLiteral {
                        value: TypeLiteral::Primitive(PrimitiveType::String),
                    },
                    source_id,
                );
                types.set_value_type(default_symbol.into_global(module_id), string_type);
            }
            ModuleType::Binary => {
                // binary modules are uint8[]
                let mut types = dir.types.write();
                let element = types.insert_type_from_any(
                    Type::TypeLiteral {
                        value: TypeLiteral::Primitive(PrimitiveType::Int(IntType::Uint8)),
                    },
                    source_id,
                );
                let array_type = types.insert_type_from_any(
                    Type::Array {
                        element: Some(element),
                        is_readonly: false,
                    },
                    source_id,
                );
                types.set_value_type(default_symbol.into_global(module_id), array_type);
            }
            ModuleType::Code => {
                unreachable!("code modules are handled by analyze_module_infer");
            }
        }

        Ok(())
    }
}
