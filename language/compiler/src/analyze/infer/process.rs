use std::sync::Arc;

use crate::{
    AnalyzeError, AnalyzeResult, Compiler, FlowContext, InferContext, TaskDependencyError,
    TaskResultCollector,
};
use destack_dir::{FlowGraphBuilder, InferTable};
use destack_source::ModuleId;
use destack_workspace::ProfileId;

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
}
