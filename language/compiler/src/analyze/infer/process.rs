use std::sync::Arc;

use crate::{
    AnalyzeError, AnalyzeResult, Compiler, FlowContext, InferContext, TaskDependencyError,
    TaskResultCollector,
};
use destack_dir::{
    FlowGraphBuilder, InferTable, LocalNodeIdAny, NodeType, PrimitiveType, Type, TypeLiteral,
};
use destack_source::ModuleId;
use destack_workspace::{ModuleContent, ModuleType, ProfileId};

use super::super::common::json_value_to_type;

impl Compiler {
    /// Ensure a module's types have been inferred.
    pub fn require_analyze_module_infer(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<(), TaskDependencyError> {
        use crate::AnalyzeTask;
        self.do_require_task_internal_only(AnalyzeTask::AnalyzeModuleInfer { module, profile })
    }

    /// Phase 2: Infer expression types.
    pub(crate) fn analyze_module_infer(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> AnalyzeResult<()> {
        self.require_analyze_module_declare(module_id, profile)?;

        // analyze data modules specially
        if !self.is_code_module(module_id) {
            return self.analyze_data_module_infer(module_id, profile);
        }

        // skip inference when module language is disabled
        if !self.module_language_allowed(module_id) {
            return Ok(());
        }

        let module = self.program.modules.get(module_id);
        let module = module.read();
        let dir = module.dir(profile);
        let tree = dir.tree.read();
        let symbols = dir.symbols.read();
        let mut types = dir.types.write();
        let mut collector = TaskResultCollector::new();

        // require builtins for analysis (#Architecture: should we?)
        self.require_resolve_builtins(profile)?;

        // analyze all expressions
        let mut infer = InferTable::default();
        let options = self.analyze_context_options_for_module(module.id);
        let mut ctx = InferContext::new(profile, options);

        // build a module level flow graph and flow table
        let graph = FlowGraphBuilder::new(module.id, &tree).build_roots(&dir.roots);
        let flow = self.compute_flow_table_for_graph(
            &module, &graph, &tree, &symbols, &mut types, &mut infer, &ctx,
        )?;
        ctx.flow = Some(FlowContext {
            module_id: module.id,
            graph: Arc::new(graph),
            table: Arc::new(flow),
        });

        // infer each root expression
        for root_id in dir.roots.iter() {
            self.collect(
                &mut collector,
                self.infer_expression(
                    &module, *root_id, &tree, &symbols, &mut types, &mut infer, &mut ctx,
                ),
            );
        }

        // register instances
        self.collect(
            &mut collector,
            self.register_instances(&module, profile, &tree, &symbols, &mut types),
        );

        // yield on any yields
        if let Some(dependency) = collector.try_into_yield_any() {
            return Err(AnalyzeError::Yield { dependency });
        }

        // solve constraints (and commit inferred types)
        self.solve_infer_table(&module, profile, &symbols, &infer, &mut types, &ctx.options);

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

        // Get the profile DIR and default symbol
        let dir = module.dir(profile);
        let default_symbol = dir.default_symbol;

        // Use a synthetic source ID since there's no actual AST node
        let source_id = LocalNodeIdAny::new(0, NodeType::Expression);

        // Extract module type and content
        let module_type = module.module_type;

        match module_type {
            ModuleType::Data => {
                // Get the parsed JSON value
                let value = match &module.content {
                    ModuleContent::Data { value, .. } => value.clone(),
                    _ => return Ok(()),
                };

                // Drop the read lock before taking the write lock
                drop(module);

                // Re-acquire the module and infer type
                let module_ref = self.program.modules.get(module_id);
                let module = module_ref.read();
                let dir = module.dir(profile);
                let mut types = dir.types.write();

                let inferred_type =
                    json_value_to_type(&value, source_id, &mut types, &self.program.strings);

                // Associate the inferred type with the default symbol
                types.set_value_type(default_symbol.into_global(module_id), inferred_type);
            }
            ModuleType::Text => {
                // Text modules are always string
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
                // Binary modules are uint8[]
                let mut types = dir.types.write();
                let element = types.insert_type_from_any(
                    Type::TypeLiteral {
                        value: TypeLiteral::Primitive(PrimitiveType::Int(
                            destack_dir::IntType::Uint8,
                        )),
                    },
                    source_id,
                );
                let array_type = types.insert_type_from_any(
                    Type::Array {
                        element: Some(element),
                    },
                    source_id,
                );
                types.set_value_type(default_symbol.into_global(module_id), array_type);
            }
            ModuleType::Code => {
                // Should not reach here - code modules are handled by analyze_module_infer
            }
        }

        Ok(())
    }
}
