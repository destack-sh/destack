use crate::{
    AnalyzeError, AnalyzeResult, AnalyzeTask, Compiler, TaskDependencyError, TaskResultCollector,
};
use destack_dir::{LocalTypeId, Type};
use destack_source::{ModuleId, ModuleVersion, ProfileVersion};
use destack_workspace::ProfileId;

impl Compiler {
    /// Ensure a module's types have been declared (evaluated).
    pub fn require_analyze_module_declare(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<(), TaskDependencyError> {
        let module = self.module_stamp(module);
        let profile = self.profile_stamp(profile);
        self.do_require_task_internal_only(AnalyzeTask::AnalyzeModuleDeclare { module, profile })
    }

    /// Phase 1: Evaluate declarations.
    pub(crate) fn analyze_module_declare(
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

        self.require_resolve_module_canonical(module_id, profile)?;
        if !self.is_code_module(module_id) {
            return Ok(());
        }

        // load module state and dir tables
        let module = self.program.modules.get(module_id);
        let module = module.read();

        // skip analysis when module language is disabled
        if !self.module_language_allowed(module_id) {
            return Ok(());
        }

        // ensure builtins are resolved before declaring symbols
        self.require_resolve_builtins(profile)?;

        // ensure ambient libs are declared before user modules
        let mut lib_collector = TaskResultCollector::new();
        if self.options.load_libs && module.is_user() {
            if let Err(error) = self.require_resolve_libs(profile)
                && let Some(error) = lib_collector.try_collect::<(), _>(Err(error))
            {
                return Err(AnalyzeError::from(error));
            }
            if let Some(builtins) = self.program.builtins.as_ref() {
                let profile_key = &self.program.profile(profile).key;
                if let Some(ambient_libs) = builtins.ambient_libs(profile_key) {
                    for lib_module_id in ambient_libs {
                        if lib_module_id == module_id {
                            continue;
                        }
                        if let Err(error) =
                            self.require_analyze_module_declare(lib_module_id, profile)
                            && let Some(error) = lib_collector.try_collect::<(), _>(Err(error))
                        {
                            return Err(AnalyzeError::from(error));
                        }
                    }
                }
            }
        }
        if let Some(dependency) = lib_collector.try_into_yield_any() {
            return Err(AnalyzeError::Yield { dependency });
        }

        // snapshot the module dir tables for analysis
        let dir = module.dir(profile);
        let tree = dir.tree.read();
        let mut types = dir.types.write();
        let symbols = dir.symbols.read();
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

            // repeat when new unevaluated types were added
            let new_type_count = types.type_count();
            if new_type_count != type_count {
                for i in type_count..new_type_count {
                    let ty_id = LocalTypeId::new(i);
                    if matches!(types.get_type(ty_id), Type::Unevaluated(_)) {
                        did_change = true;
                        break;
                    }
                }
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

        // drop the read guard before taking a mutable lock for decorators
        drop(symbols);

        // attach well known decorator metadata to symbols
        let mut symbols = dir.symbols.write();
        self.collect(
            &mut collector,
            self.register_symbol_decorators(&module, profile, &tree, &mut symbols),
        );
        drop(symbols);

        // register visible extensions from imported symbols
        let symbols = dir.symbols.read();
        self.collect(
            &mut collector,
            self.register_visible_extensions(&module, profile, &tree, &symbols, &mut types),
        );

        // yield on any yields
        if let Some(dependency) = collector.try_into_yield_any() {
            return Err(AnalyzeError::Yield { dependency });
        }

        // return the collected result
        Ok(())
    }
}
