use std::collections::HashSet;

use destack_builtin::BuiltinLibraryKind;
use destack_dir::{DependencyKind, Expression, ImportSource, ScalarLiteral, TypeExpression};
use destack_source::ModuleId;
use destack_workspace::{Module, ModuleSource, ProfileId, Revision};

use crate::{AnalyzeError, AnalyzeResult, Compiler, RequirementSet, ResolveError};

impl Compiler {
    /// Require declared and interface analysis for type-import dependencies used during infer.
    pub(super) fn require_type_import_dependencies_for_infer(
        &self,
        revision: Revision,
        module: &Module,
        profile: ProfileId,
        tree: &destack_dir::NodeTree,
    ) -> AnalyzeResult<()> {
        // collect the direct and projected type-import owner modules
        let required_modules =
            self.collect_type_import_dependencies_for_infer(revision, module, profile, tree)?;
        let module_ids = self.sorted_unique_module_ids(required_modules);
        let mut first_error = None;

        // require declared and interface state for each type-import dependency
        for module_id in module_ids {
            if let Err(error) = self.require_dir_declared(revision, module_id, profile)
                && first_error.is_none()
            {
                first_error = Some(error);
            }

            if let Err(error) = self.require_dir_interface(revision, module_id, profile)
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

    /// Collect dependency modules for type-import expressions used during infer.
    fn collect_type_import_dependencies_for_infer(
        &self,
        revision: Revision,
        module: &Module,
        profile: ProfileId,
        tree: &destack_dir::NodeTree,
    ) -> AnalyzeResult<HashSet<ModuleId>> {
        let dir = self
            .require_artifact_dir_resolved(revision, module.id, profile)
            .map_err(AnalyzeError::from)?;
        let mut required = HashSet::new();

        // collect direct target modules and qualified projection owners
        for (type_expression_id, expression) in tree.iter_nodes_of_type::<TypeExpression>() {
            let TypeExpression::Import {
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

            // resolve the direct type-import target
            let source_node_id = type_expression_id.into_global_any(module.id);
            let target_module = match self.resolve_import_from_resolved_artifact(
                revision,
                module,
                dir.as_ref(),
                profile,
                source_node_id,
                ImportSource::ImportStatement,
                *target_string,
                DependencyKind::Type,
            ) {
                Ok(target_module) => target_module,

                // recover direct module requirements from deferred import resolution
                Err(error) => match error {
                    ResolveError::Yield { requirement } => {
                        if self.collect_required_modules_from_requirement(
                            &mut required,
                            module.id,
                            &requirement,
                        ) {
                            continue;
                        }

                        return Err(AnalyzeError::Yield { requirement });
                    }
                    ResolveError::UnsatisfiedRequirement { requirement } => {
                        if self.collect_required_modules_from_requirement(
                            &mut required,
                            module.id,
                            &requirement,
                        ) {
                            continue;
                        }

                        return Err(AnalyzeError::UnsatisfiedRequirement { requirement });
                    }
                    error => {
                        self.error(error);
                        continue;
                    }
                },
            };

            // record the direct target module when it differs from the origin
            if let Some(target_module_id) = target_module.module_id()
                && target_module_id != module.id
            {
                required.insert(target_module_id);
            }

            // query the resolved owner for qualified projections
            //
            // direct target modules are the hard precondition here
            // owner refinement can wait until those modules are ready
            let resolved_symbol = self.query_import_type_symbol(
                revision,
                module,
                profile,
                type_expression_id.into_any(),
                *target_string,
                qualifier.as_ref(),
            );
            let Some(resolved_symbol) = resolved_symbol else {
                continue;
            };
            if resolved_symbol.module_id != module.id {
                required.insert(resolved_symbol.module_id);
            }
        }

        Ok(required)
    }

    /// Collect direct module ids from one deferred requirement set.
    fn collect_required_modules_from_requirement(
        &self,
        required: &mut HashSet<ModuleId>,
        origin_module_id: ModuleId,
        requirement: &RequirementSet,
    ) -> bool {
        let mut did_collect = false;

        // collect module ids from exact artifact requirements
        requirement.for_each_artifact(|requirement| {
            let key = &requirement.version.key;
            let Some(module_id) = key.module_id() else {
                return;
            };
            if module_id == origin_module_id {
                return;
            }

            did_collect = true;
            required.insert(module_id);
        });

        did_collect
    }

    /// Ensure declare analysis is complete for infer dependency modules.
    pub(crate) fn require_declare_dependencies_for_infer(
        &self,
        revision: Revision,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> AnalyzeResult<()> {
        // require the module graph snapshot before infer dependency preconditions
        let graph = self.module_graph(profile).ok_or(AnalyzeError::Internal {
            message: format!("missing module graph artifact: profile={profile:?}"),
        })?;

        // require declared artifacts for the transitive dependency closure
        // this prevents late declared-boundary yields during remote alias or template evaluation
        let mut pending = graph.dependencies_for(module_id);
        let mut visited = HashSet::new();
        while let Some(dependency) = pending.pop() {
            if !visited.insert(dependency) || dependency == module_id {
                continue;
            }

            if self.skip_builtin_lib_dependency_for_infer(revision, dependency) {
                continue;
            }

            for nested in graph.dependencies_for(dependency) {
                pending.push(nested);
            }
        }

        let module_ids = self.sorted_unique_module_ids(visited);
        let mut first_error = None;
        for module_id in module_ids {
            if let Err(error) = self.require_dir_declared(revision, module_id, profile)
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

    /// Ensure interface analysis is complete for infer dependency modules.
    pub(crate) fn require_interface_dependencies(
        &self,
        revision: Revision,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> AnalyzeResult<()> {
        // require the module graph snapshot before infer dependency preconditions
        let graph = self.module_graph(profile).ok_or(AnalyzeError::Internal {
            message: format!("missing module graph artifact: profile={profile:?}"),
        })?;

        // require interface analysis for direct dependencies
        let dependencies = graph
            .dependencies_for(module_id)
            .into_iter()
            .filter(|dependency| {
                !self.skip_builtin_lib_dependency_for_infer(revision, *dependency)
            });
        let module_ids = self.sorted_unique_module_ids(dependencies);
        let mut first_error = None;
        for module_id in module_ids {
            if let Err(error) = self.require_dir_interface(revision, module_id, profile)
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

    /// Return true when one dependency should be skipped for infer preconditions.
    fn skip_builtin_lib_dependency_for_infer(
        &self,
        revision: Revision,
        module_id: ModuleId,
    ) -> bool {
        if self.options.load_libraries {
            return false;
        }

        let Some(module) = self.repository.module(revision, module_id).ok().flatten() else {
            return false;
        };
        let module = module.as_ref();
        matches!(
            module.source,
            ModuleSource::Builtin(BuiltinLibraryKind::Library)
        )
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
