use std::collections::HashSet;

use destack_builtin::BuiltinLibKind;
use destack_dir::{DependencyKind, DependencySource, Expression, ScalarLiteral};
use destack_source::ModuleId;
use destack_workspace::{ModuleGraphKey, ModuleSource, ProfileId};

use crate::analyze::common::ModuleTreeView;
use crate::{AnalyzeError, AnalyzeResult, Compiler, ResolveError};

impl Compiler {
    /// Require interface analysis for the type-import dependency closure used during infer.
    pub(super) fn require_type_import_interface_dependencies(
        &self,
        view: ModuleTreeView<'_>,
    ) -> AnalyzeResult<()> {
        // collect and require interface dependencies in deterministic module order
        let required_modules = self.collect_type_import_interface_dependencies(view)?;
        self.require_interface_modules_for_infer(view.profile, required_modules)
    }

    /// Collect interface dependency modules for type-import expressions used during infer.
    fn collect_type_import_interface_dependencies(
        &self,
        view: ModuleTreeView<'_>,
    ) -> AnalyzeResult<HashSet<ModuleId>> {
        let dir = view.module.dir(view.profile);
        let mut required = HashSet::new();

        // collect direct and projection-owner interface dependencies
        for (expression_id, expression) in view.tree.iter_nodes_of_type::<Expression>() {
            let Expression::TypeImport {
                target, qualifier, ..
            } = expression
            else {
                continue;
            };
            let Expression::ScalarLiteral {
                value: ScalarLiteral::String(target_string),
            } = view.tree.get(*target)
            else {
                continue;
            };

            // collect direct target-module interface dependency
            let source_node_id = expression_id.into_global_any(view.module.id);
            let target_module = match self.resolve_import(
                view.module,
                dir,
                view.profile,
                source_node_id,
                DependencySource::ImportStatement,
                *target_string,
                DependencyKind::Type,
            ) {
                Ok(target_module) => target_module,
                Err(error) => match error {
                    ResolveError::Yield { requirement } => {
                        return Err(AnalyzeError::Yield { requirement });
                    }
                    ResolveError::UnsatisfiedRequirement { requirement } => {
                        return Err(AnalyzeError::UnsatisfiedRequirement { requirement });
                    }
                    error => {
                        self.error(error);
                        continue;
                    }
                },
            };
            if let Some(target_module_id) = target_module.module_id()
                && target_module_id != view.module.id
            {
                required.insert(target_module_id);
            }

            // collect transitive owner dependency for qualified projections
            let resolved_symbol = self.resolve_import_type_symbol(
                view,
                expression_id.into_any(),
                *target_string,
                qualifier.as_ref(),
            )?;
            let Some(resolved_symbol) = resolved_symbol else {
                continue;
            };
            if resolved_symbol.module_id != view.module.id {
                required.insert(resolved_symbol.module_id);
            }
        }

        Ok(required)
    }

    /// Ensure declare analysis is complete for infer dependency modules.
    pub(crate) fn require_declare_dependencies_for_infer(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> AnalyzeResult<()> {
        // require the module graph snapshot before infer dependency preconditions
        let key = ModuleGraphKey::new(profile);
        let graph = self
            .program
            .index
            .module_graphs
            .get(&key)
            .ok_or(AnalyzeError::Internal {
                message: format!("missing module graph snapshot for infer declare deps: profile={profile:?}, module={module_id:?}"),
            })?;

        // require declared artifacts for the transitive dependency closure
        // this prevents late declared-boundary yields during remote alias or template evaluation
        let mut pending = graph.dependencies_for(module_id);
        let mut visited = HashSet::new();
        while let Some(dependency) = pending.pop() {
            if !visited.insert(dependency) || dependency == module_id {
                continue;
            }

            if self.skip_builtin_lib_dependency_for_infer(dependency) {
                continue;
            }

            for nested in graph.dependencies_for(dependency) {
                pending.push(nested);
            }
        }

        self.require_declared_modules_for_infer(profile, visited)
    }

    /// Ensure interface analysis is complete for infer dependency modules.
    pub(crate) fn require_interface_dependencies(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> AnalyzeResult<()> {
        // require the module graph snapshot before infer dependency preconditions
        let key = ModuleGraphKey::new(profile);
        let graph = self
            .program
            .index
            .module_graphs
            .get(&key)
            .ok_or(AnalyzeError::Internal {
                message: format!("missing module graph snapshot for infer interface deps: profile={profile:?}, module={module_id:?}"),
            })?;

        // require interface analysis for direct dependencies
        let dependencies = graph
            .dependencies_for(module_id)
            .into_iter()
            .filter(|dependency| !self.skip_builtin_lib_dependency_for_infer(*dependency));
        self.require_interface_modules_for_infer(profile, dependencies)?;

        Ok(())
    }

    /// Ensure interface analysis is complete for ambient lib modules.
    pub(super) fn require_interface_inference_for_ambient_libs(
        &self,
        profile: ProfileId,
    ) -> AnalyzeResult<()> {
        if !self.options.load_libs {
            return Ok(());
        }

        // require interface analysis for ambient lib modules
        let lib_modules = self.lib_environment_modules(profile);
        if lib_modules.is_empty() {
            return Ok(());
        }
        self.require_interface_modules_for_infer(profile, lib_modules)?;

        Ok(())
    }

    /// Return true when one dependency should be skipped for infer preconditions.
    fn skip_builtin_lib_dependency_for_infer(&self, module_id: ModuleId) -> bool {
        if self.options.load_libs {
            return false;
        }

        let module = self.program.modules.get(module_id);
        let module = module.read();
        matches!(module.source, ModuleSource::Builtin(BuiltinLibKind::Lib))
    }

    /// Require declare analysis for one module set and return the first dependency error.
    fn require_declared_modules_for_infer(
        &self,
        profile: ProfileId,
        modules: impl IntoIterator<Item = ModuleId>,
    ) -> AnalyzeResult<()> {
        // require declare tasks in deterministic order and return the first dependency
        let module_ids = self.sorted_unique_module_ids(modules);
        let mut first_error = None;
        for module_id in module_ids {
            if let Err(error) = self.require_dir_declared(module_id, profile)
                && first_error.is_none()
            {
                first_error = Some(error);
            }
        }

        if let Some(error) = first_error {
            return Err(AnalyzeError::from(error));
        }

        Ok(())
    }

    /// Require interface analysis for one module set and return the first dependency error.
    fn require_interface_modules_for_infer(
        &self,
        profile: ProfileId,
        modules: impl IntoIterator<Item = ModuleId>,
    ) -> AnalyzeResult<()> {
        // require interface tasks in deterministic order and return the first dependency
        let module_ids = self.sorted_unique_module_ids(modules);
        let mut first_error = None;
        for module_id in module_ids {
            if let Err(error) = self.require_dir_interface(module_id, profile)
                && first_error.is_none()
            {
                first_error = Some(error);
            }
        }

        if let Some(error) = first_error {
            return Err(AnalyzeError::from(error));
        }

        Ok(())
    }

    /// Return one deterministic, deduplicated module id vector.
    fn sorted_unique_module_ids(
        &self,
        modules: impl IntoIterator<Item = ModuleId>,
    ) -> Vec<ModuleId> {
        let mut module_ids = modules.into_iter().collect::<Vec<_>>();
        module_ids.sort_unstable();
        module_ids.dedup();
        module_ids
    }
}
