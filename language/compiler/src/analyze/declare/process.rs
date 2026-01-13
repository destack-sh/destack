use crate::{AnalyzeError, AnalyzeResult, Compiler, TaskDependencyError, TaskResultCollector};
use destack_dir::{LocalTypeId, Type};
use destack_source::ModuleId;
use destack_workspace::ProfileId;

impl Compiler {
    /// Ensure a module's types have been declared (evaluated).
    pub fn require_analyze_module_declare(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<(), TaskDependencyError> {
        use crate::AnalyzeTask;
        self.do_require_task_internal_only(AnalyzeTask::AnalyzeModuleDeclare { module, profile })
    }

    /// Phase 1: Evaluate declarations.
    pub(crate) fn analyze_module_declare(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> AnalyzeResult<()> {
        // load module state and dir tables
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let dir = module.dir(profile);
        let tree = dir.tree.read();
        let symbols = dir.symbols.read();
        let mut types = dir.types.write();
        let mut collector = TaskResultCollector::new();
        let mut has_dependency = false;

        // evaluate unevaluated types to a fixed point
        let mut did_change = true;
        while did_change {
            did_change = false;
            let type_count = types.type_count();

            // attempt evaluation for each unevaluated type
            for i in 0..type_count {
                let ty_id = LocalTypeId::new(i);
                let was_unevaluated = matches!(types.get_type(ty_id), Type::Unevaluated(_));
                if !was_unevaluated {
                    continue;
                }

                // evaluate the type and track progress
                self.collect(
                    &mut collector,
                    self.evaluate_type(&module, profile, ty_id, &tree, &symbols, &mut types),
                );

                // stop early once a dependency yield is detected
                if collector.has_dependencies() {
                    has_dependency = true;
                    break;
                }
                // record progress on any resolved types
                let is_unevaluated = matches!(types.get_type(ty_id), Type::Unevaluated(_));
                if was_unevaluated && !is_unevaluated {
                    did_change = true;
                }
            }

            // stop after dependency detection
            if has_dependency {
                break;
            }

            // repeat when new types were added
            if types.type_count() != type_count {
                did_change = true;
            }
        }

        // yield early when a dependency blocked evaluation
        if has_dependency {
            if let Some(dependency) = collector.try_into_yield_any() {
                return Err(AnalyzeError::Yield { dependency });
            }

            return Ok(());
        }

        // reset collector for declaration steps
        let mut collector = TaskResultCollector::new();

        // declare type-level declarations and shapes
        self.collect(
            &mut collector,
            self.declare_module_declarations(&module, profile, &tree, &symbols, &mut types),
        );

        // declare exported value types with local-only inference
        let exported_symbols = dir.exported_symbols.read();
        self.collect(
            &mut collector,
            self.declare_exported_value_types(
                &module,
                profile,
                &exported_symbols,
                &tree,
                &symbols,
                &mut types,
            ),
        );

        // FUGU #Incomplete: implement #Extensions
        // register extensions?
        // 1) register inherent extensions from imported symbols
        // 2) register local extensions from local symbols
        // 3) register named extensions from imported symbols

        // yield on any yields
        if let Some(dependency) = collector.try_into_yield_any() {
            return Err(AnalyzeError::Yield { dependency });
        }

        // return the collected result
        Ok(())
    }
}
