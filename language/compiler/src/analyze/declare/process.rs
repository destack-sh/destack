use std::collections::VecDeque;

use crate::timing::tags;
use crate::{
    AnalyzeError, AnalyzeResult, AnalyzeTask, Compiler, ModuleCheckOptions, Task,
    TaskDependencyError, TaskResultCollector,
};
use destack_dir::{LocalTypeId, Type};
use destack_source::{ModuleId, ModuleVersion, ProfileVersion};
use destack_workspace::{ModuleSource, ProfileId};

impl Compiler {
    /// Ensure a module's types have been declared (evaluated).
    pub fn require_analyze_module_declare(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<(), TaskDependencyError> {
        // avoid self dependency when already declaring this module
        if let Some(Task::Analyze(AnalyzeTask::AnalyzeModuleDeclare {
            module: current_module,
            profile: current_profile,
        })) = self.current_task()
            && current_module.id == module
            && current_profile.id == profile
        {
            return Ok(());
        }

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
        let _timing = self.timing_scope(tags::ANALYZE_MODULE_DECLARE);

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
        self.ensure_ambient_libs_declared(&module, profile)?;

        // snapshot the module dir tables for analysis
        let dir = module.dir(profile);
        let tree = dir.tree.read();
        let mut types = dir.types.write();
        let symbols = dir.symbols.read();
        let mut collector = TaskResultCollector::new();
        let module_checks = self.module_check_options_for_module(module.id);
        {
            let _timing = self.timing_scope(tags::ANALYZE_DECLARE_TYPES);

            // run eager evaluation when required
            if self.should_eager_evaluate_declared_types(&module, module_checks) {
                let has_dependency = self.evaluate_unevaluated_types_to_fixpoint(
                    &module,
                    profile,
                    &tree,
                    &symbols,
                    &mut types,
                    &mut collector,
                );
                if has_dependency {
                    if let Some(dependency) = collector.try_into_yield_any() {
                        return Err(AnalyzeError::Yield { dependency });
                    }

                    return Ok(());
                }
            }
        }

        // reset collector for declaration steps
        let mut collector = TaskResultCollector::new();

        {
            let _timing = self.timing_scope(tags::ANALYZE_DECLARE_DECLARATIONS);

            // declare type level declarations and shapes
            self.collect(
                &mut collector,
                self.declare_module_declarations(&module, profile, &tree, &symbols, &mut types),
            );
        }

        // drop the read guard before taking a mutable lock for decorators
        drop(symbols);

        {
            let _timing = self.timing_scope(tags::ANALYZE_DECLARE_DECORATORS);

            // attach well known decorator metadata to symbols
            let mut symbols = dir.symbols.write();
            let mut captures = dir.captures.write();
            self.collect(
                &mut collector,
                self.register_symbol_decorators(
                    &module,
                    profile,
                    &tree,
                    &mut symbols,
                    &mut captures,
                ),
            );
            drop(symbols);
            drop(captures);
        }

        // cache well-known intrinsics after decorator registration
        if module.is_user() {
            self.collect(
                &mut collector,
                self.ensure_well_known_intrinsics_for_profile(profile),
            );
        }

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

    /// Ensure ambient libs are declared before analyzing a user module.
    fn ensure_ambient_libs_declared(
        &self,
        module: &destack_workspace::Module,
        profile: ProfileId,
    ) -> AnalyzeResult<()> {
        // collect dependency yields
        let mut collector = TaskResultCollector::new();

        if self.options.load_libs && module.is_user() {
            // resolve libs before declaring ambient modules
            if let Err(error) = self.require_resolve_libs(profile)
                && let Some(error) = collector.try_collect::<(), _>(Err(error))
            {
                return Err(AnalyzeError::from(error));
            }

            if let Some(builtins) = self.program.builtins.as_ref() {
                let profile_key = &self.program.profile(profile).key;
                if let Some(ambient_libs) = builtins.ambient_libs(profile_key) {
                    for lib_module_id in ambient_libs {
                        if lib_module_id == module.id {
                            continue;
                        }
                        if let Err(error) =
                            self.require_analyze_module_declare(lib_module_id, profile)
                            && let Some(error) = collector.try_collect::<(), _>(Err(error))
                        {
                            return Err(AnalyzeError::from(error));
                        }
                    }
                }
            }
        }

        if let Some(dependency) = collector.try_into_yield_any() {
            return Err(AnalyzeError::Yield { dependency });
        }

        Ok(())
    }

    /// Decide whether declared types should be eagerly evaluated for a module.
    fn should_eager_evaluate_declared_types(
        &self,
        module: &destack_workspace::Module,
        module_checks: ModuleCheckOptions,
    ) -> bool {
        // eager evaluation is required for non-declaration modules
        if !module.language_type.is_declaration() {
            return true;
        }

        // eager evaluation is only required when lib checks are enabled
        !(module_checks.skip_lib_check || matches!(module.source, ModuleSource::Builtin(_)))
    }

    /// Evaluate unevaluated types to a fixed point.
    ///
    /// Returns true if evaluation yielded dependencies.
    fn evaluate_unevaluated_types_to_fixpoint(
        &self,
        module: &destack_workspace::Module,
        profile: ProfileId,
        tree: &destack_dir::NodeTree,
        symbols: &destack_dir::SymbolTable,
        types: &mut destack_dir::TypeTable,
        collector: &mut TaskResultCollector,
    ) -> bool {
        // seed the worklist
        let mut pending: VecDeque<LocalTypeId> = (0..types.type_count())
            .map(LocalTypeId::new)
            .filter(|ty_id| matches!(types.get_type(*ty_id), Type::Unevaluated(_)))
            .collect();
        let mut did_change = true;

        while did_change && !pending.is_empty() {
            did_change = false;
            let mut next_pending = VecDeque::new();
            let type_count = types.type_count();

            // evaluate pending types
            while let Some(ty_id) = pending.pop_front() {
                if !matches!(types.get_type(ty_id), Type::Unevaluated(_)) {
                    continue;
                }

                self.collect(
                    collector,
                    self.evaluate_type(module, profile, ty_id, tree, symbols, types),
                );

                if collector.has_dependencies() {
                    return true;
                }

                if matches!(types.get_type(ty_id), Type::Unevaluated(_)) {
                    next_pending.push_back(ty_id);
                } else {
                    did_change = true;
                }
            }

            // add newly created unevaluated types
            let new_type_count = types.type_count();
            if new_type_count > type_count {
                for i in type_count..new_type_count {
                    let ty_id = LocalTypeId::new(i);
                    if matches!(types.get_type(ty_id), Type::Unevaluated(_)) {
                        next_pending.push_back(ty_id);
                    }
                }
            }

            pending = next_pending;
        }

        false
    }
}
