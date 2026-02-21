use std::collections::HashSet;

use destack_builtin::BuiltinLibKind;
use destack_dir::{DependencyKind, DependencySource, Expression, NodeTree, ScalarLiteral};
use destack_source::ModuleId;
use destack_workspace::{Module, ModuleGraphKey, ModuleSource, ProfileId};

use crate::{AnalyzeError, AnalyzeResult, Compiler};

impl Compiler {
    /// Require interface analysis for the type-import dependency closure used during infer.
    pub(super) fn require_type_import_interface_dependency_closure_for_infer(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
    ) -> AnalyzeResult<()> {
        let required =
            self.collect_type_import_interface_dependency_modules_for_infer(module, profile, tree)?;
        for required_module_id in required {
            if let Err(error) = self.require_analyze_module_interface(required_module_id, profile) {
                return Err(AnalyzeError::from(error));
            }
        }

        Ok(())
    }

    /// Collect interface dependency modules for type-import expressions used during infer.
    fn collect_type_import_interface_dependency_modules_for_infer(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
    ) -> AnalyzeResult<HashSet<ModuleId>> {
        let dir = module.dir(profile);
        let mut required = HashSet::new();

        // collect direct and projection-owner interface dependencies
        for (expression_id, expression) in tree.iter_nodes_of_type::<Expression>() {
            let Expression::TypeImport {
                target, qualifier, ..
            } = expression
            else {
                continue;
            };
            let Expression::ScalarLiteral {
                value: ScalarLiteral::String(target_string),
            } = tree.get(*target)
            else {
                continue;
            };

            // collect direct target-module interface dependency
            let source_node_id = expression_id.into_global_any(module.id);
            let target_module = match self.resolve_import(
                module,
                dir,
                profile,
                source_node_id,
                DependencySource::ImportStatement,
                *target_string,
                DependencyKind::Type,
            ) {
                Ok(target_module) => target_module,
                Err(error) => {
                    self.error(error);
                    continue;
                }
            };
            if let Some(target_module_id) = target_module.module_id()
                && target_module_id != module.id
            {
                required.insert(target_module_id);
            }

            // collect transitive owner dependency for qualified projections
            let resolved_symbol = self.resolve_import_type_symbol(
                module,
                profile,
                expression_id.into_any(),
                *target_string,
                qualifier.as_ref(),
            )?;
            let Some(resolved_symbol) = resolved_symbol else {
                continue;
            };
            if resolved_symbol.module_id != module.id {
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

        // require declare analysis for the transitive dependency closure
        // this prevents late declare-stage yields during remote alias or template evaluation
        let mut pending = graph.dependencies_for(module_id);
        let mut visited = HashSet::new();
        let mut first_error = None;
        while let Some(dependency) = pending.pop() {
            if !visited.insert(dependency) || dependency == module_id {
                continue;
            }

            if self.skip_builtin_lib_dependency_for_infer(dependency) {
                continue;
            }

            if let Err(error) = self.require_analyze_module_declare(dependency, profile)
                && first_error.is_none()
            {
                first_error = Some(error);
            }

            for nested in graph.dependencies_for(dependency) {
                pending.push(nested);
            }
        }

        if let Some(error) = first_error {
            return Err(AnalyzeError::from(error));
        }

        Ok(())
    }

    /// Ensure declare analysis is complete for builtin modules used during infer.
    pub(super) fn require_declare_inference_for_builtin_modules(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> AnalyzeResult<()> {
        // skip when builtins are not loaded
        let Some(builtins) = self.builtins() else {
            return Ok(());
        };

        // resolve the profile key for ambient lib lookup
        let profile_entry = self
            .program
            .profiles
            .get(profile)
            .ok_or(AnalyzeError::Internal {
                message: format!(
                    "missing profile data for infer declare builtin deps: {profile:?}"
                ),
            })?;

        // require declare analysis for core builtin modules
        let core_modules = builtins
            .core_module_by_path
            .values()
            .copied()
            .filter(|candidate_module_id| *candidate_module_id != module_id);
        self.require_declared_modules_for_infer(profile, core_modules)?;

        // require declare analysis for ambient lib modules when libs are enabled
        if !self.options.load_libs {
            return Ok(());
        }
        let Some(lib_modules) = builtins.ambient_libs(&profile_entry.key) else {
            return Ok(());
        };
        let ambient_modules = lib_modules
            .into_iter()
            .filter(|candidate_module_id| *candidate_module_id != module_id);
        self.require_declared_modules_for_infer(profile, ambient_modules)?;

        Ok(())
    }

    /// Ensure interface analysis is complete for infer dependency modules.
    pub(crate) fn require_interface_dependencies_for_infer(
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

        // skip when builtins are not loaded
        let Some(builtins) = self.builtins() else {
            return Ok(());
        };

        // resolve the profile key for ambient lib lookup
        let profile_entry = self
            .program
            .profiles
            .get(profile)
            .ok_or(AnalyzeError::Internal {
                message: format!(
                    "missing profile data for infer interface ambient libs: {profile:?}"
                ),
            })?;

        // require interface analysis for ambient lib modules
        let Some(lib_modules) = builtins.ambient_libs(&profile_entry.key) else {
            return Ok(());
        };
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
        let mut first_error = None;
        for module_id in modules {
            if let Err(error) = self.require_analyze_module_declare(module_id, profile)
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
        let mut first_error = None;
        for module_id in modules {
            if let Err(error) = self.require_analyze_module_interface(module_id, profile)
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
}
