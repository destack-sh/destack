use crate::{Compiler, ExecuteError, ExecuteResult, TaskResultCollector};

use destack_source::{CacheKind, ModuleId, ModuleVersion, ProfileVersion};
use destack_workspace::{ComptimeOutput, ModuleComptime, ModuleDir, ProfileId, TrustPolicy};

use super::{ComptimePatch, collect_comptime_dependencies};
use {destack_dir as dir, destack_vm as vm};

impl Compiler {
    /// Prepare comptime state for a module within a profile.
    pub(crate) fn execute_module_prepare(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        module_version: ModuleVersion,
        profile_version: ProfileVersion,
    ) -> ExecuteResult<()> {
        // skip stale tasks
        self.ensure_module_profile_matches::<ExecuteError>(
            module_id,
            module_version,
            profile_id,
            profile_version,
        )?;

        // ensure DIR exists
        self.require_elaborate_module(module_id, profile_id)?;
        if !self.is_code_module(module_id) {
            return Ok(());
        }

        let module = self.program.modules.get(module_id);
        let mut module = module.write();
        self.ensure_module_profile_matches_guard::<ExecuteError>(
            &module,
            module_version,
            profile_id,
            profile_version,
        )?;

        // initialize or refresh module comptime entry
        let code = module.code_mut();
        match code
            .comptimes
            .iter_mut()
            .find(|entry| entry.profile_id == profile_id)
        {
            Some(entry) => {
                if entry.module_version != module_version
                    || entry.profile_version != profile_version
                {
                    entry.module_version = module_version;
                    entry.profile_version = profile_version;
                    entry.results.clear();
                }
            }
            None => {
                code.comptimes.push(ModuleComptime {
                    module_id,
                    profile_id,
                    module_version,
                    profile_version,
                    results: indexmap::IndexMap::new(),
                });
            }
        }

        Ok(())
    }

    /// Execute comptime code for a module within a profile.
    pub(crate) fn execute_module_patch(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        module_version: ModuleVersion,
        profile_version: ProfileVersion,
    ) -> ExecuteResult<()> {
        // skip stale tasks
        self.ensure_module_profile_matches::<ExecuteError>(
            module_id,
            module_version,
            profile_id,
            profile_version,
        )?;

        // resolve cache handle
        let cache_handle =
            self.cache_handle_for_module(module_id, Some(profile_id), None, CacheKind::DirExecuted);

        // try to load executed DIR from cache
        if let Some(cache) = cache_handle.as_ref()
            && let Ok(Some(entry)) = cache.read_dir_executed()
        {
            self.ensure_module_profile_matches::<ExecuteError>(
                module_id,
                module_version,
                profile_id,
                profile_version,
            )?;
            let dir = ModuleDir::from_data(entry.payload);
            let module = self.program.modules.get(module_id);
            let mut module = module.write();
            self.ensure_module_profile_matches_guard::<ExecuteError>(
                &module,
                module_version,
                profile_id,
                profile_version,
            )?;
            module.set_dir(profile_id, dir);
            tracing::trace!(?module_id, ?profile_id, "execute.module.cache");
            return Ok(());
        }

        // ensure module comptime state exists
        self.require_execute_module_prepare(module_id, profile_id)?;
        if !self.is_code_module(module_id) {
            return Ok(());
        }

        // collect comptime expressions in this module
        let comptime_nodes: Vec<dir::LocalNodeIdAny> = {
            let module = self.program.modules.get(module_id);
            let module = module.read();
            let dir = module.dir(profile_id);
            let tree = dir.tree.read();
            tree.iter_nodes_of_type::<dir::Expression>()
                .filter_map(|(id, expression)| {
                    if matches!(expression, dir::Expression::Comptime { .. }) {
                        Some(id.into())
                    } else {
                        None
                    }
                })
                .collect()
        };

        // execute each comptime expression
        let mut collector = TaskResultCollector::new();
        for expression_id in &comptime_nodes {
            self.collect(
                &mut collector,
                self.execute_expression(
                    module_id,
                    profile_id,
                    module_version,
                    profile_version,
                    expression_id.into_global(module_id),
                ),
            );
        }
        if let Some(dependency) = collector.try_into_yield_any() {
            return Err(ExecuteError::Yield { dependency });
        }

        // gather execute results for this module
        let patches = {
            let module = self.program.modules.get(module_id);
            let module = module.read();
            let Some(comptime) = module.comptime_maybe(profile_id) else {
                return Ok(());
            };
            comptime
                .results
                .iter()
                .filter_map(|(expression_id, result)| {
                    if comptime_nodes.contains(expression_id) {
                        Some(ComptimePatch {
                            expression_id: *expression_id,
                            result: result.clone(),
                        })
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
        };

        // patch comptime results into DIR
        if !patches.is_empty() {
            self.ensure_module_profile_matches::<ExecuteError>(
                module_id,
                module_version,
                profile_id,
                profile_version,
            )?;
            let module = self.program.modules.get(module_id);
            let module = module.read();
            let dir = module.dir(profile_id);
            let mut tree = dir.tree.write();
            for patch in patches {
                self.apply_comptime_patch(module_id, profile_id, &mut tree, patch);
            }
        }

        // write executed DIR to cache
        if let Some(cache) = cache_handle.as_ref() {
            self.ensure_module_profile_matches::<ExecuteError>(
                module_id,
                module_version,
                profile_id,
                profile_version,
            )?;
            let module = self.program.modules.get(module_id);
            let module = module.read();
            let payload = module.dir(profile_id).to_data();
            if let Err(error) = cache.write_dir_executed(payload) {
                tracing::debug!(
                    ?module_id,
                    ?profile_id,
                    ?error,
                    "execute.module.cache.write"
                );
            }
        }

        Ok(())
    }

    /// Execute comptime code for a single expression.
    pub(crate) fn execute_expression(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        module_version: ModuleVersion,
        profile_version: ProfileVersion,
        expression: dir::GlobalNodeIdAny,
    ) -> ExecuteResult<()> {
        // skip stale tasks
        self.ensure_module_profile_matches::<ExecuteError>(
            module_id,
            module_version,
            profile_id,
            profile_version,
        )?;

        // ensure the expression belongs to the target module
        if module_id != expression.module_id {
            return Err(ExecuteError::UnsupportedConstruct {
                node: expression.into_anchored(Some(profile_id)),
            });
        }
        let expression_id = expression
            .local_id
            .try_into_typed::<dir::Expression>()
            .map_err(|_| ExecuteError::UnsupportedConstruct {
                node: expression.into_anchored(Some(profile_id)),
            })?;

        // ensure module comptime state exists
        self.require_execute_module_prepare(module_id, profile_id)?;
        if !self.is_code_module(module_id) {
            return Ok(());
        }

        // run comptime evaluation for this expression
        let result = {
            // get the comptime expression
            let module = self.program.modules.get(module_id);
            let module = module.read();
            let dir = module.dir(profile_id);
            let tree = dir.tree.read();
            let expression = tree.get(expression_id);
            let dir::Expression::Comptime { body } = expression else {
                return Ok(());
            };

            // ensure nested comptime expressions are executed first
            let dependencies = collect_comptime_dependencies(&tree, *body);
            let mut dependency_collector = TaskResultCollector::new();
            for dependency in dependencies {
                // require each comptime dependency before execution
                let dependency_id = dependency.into_global(module_id);
                let error = dependency_collector.try_collect(self.require_execute_expression(
                    module_id,
                    profile_id,
                    dependency_id,
                ));
                if let Some(error) = error {
                    return Err(ExecuteError::UnsatisfiedDependency {
                        dependency: error.into_dependency(),
                    });
                }
            }
            if let Some(dependency) = dependency_collector.try_into_yield_any() {
                return Err(ExecuteError::Yield { dependency });
            }

            // lower the comptime expression to MIR
            let (mir_tree, strings, function_id) =
                self.lower_comptime_expression(&module, profile_id, *body)?;

            // execute the MIR with the interpreter
            let mut options = vm::IsolateOptions::comptime();
            let trust_policy = match self.comptime_target.trust_policy {
                TrustPolicy::Untrusted => vm::TrustPolicy::Untrusted,
                TrustPolicy::Trusted => vm::TrustPolicy::Trusted,
                TrustPolicy::Internal => vm::TrustPolicy::Internal,
            };
            options.apply_trust_policy(trust_policy);

            let mut isolate = vm::Isolate::with_options(
                mir_tree,
                strings.into_immutable(), // TODO #Performance: avoid cloning the string pool
                options,
            )
            .map_err(|error| ExecuteError::FailedExecution {
                module: module_id,
                message: format!("{error}"),
            })?;
            let output = isolate.run_function(function_id, &[]).map_err(|error| {
                ExecuteError::FailedExecution {
                    module: module_id,
                    message: format!("{error}"),
                }
            })?;

            // convert the vm value to a static expression
            let vm_value = output.value;
            let dir_value = self
                .value_to_static_expression(&isolate, &vm_value)
                .ok_or_else(|| ExecuteError::FailedExecution {
                    module: module_id,
                    message: "unsupported comptime result".to_string(),
                })?;

            Some(ComptimeOutput {
                dir: Some(dir_value),
                mir: None,
            })
        };

        // store the output for later patching
        let module = self.program.modules.get(module_id);
        let mut module = module.write();
        self.ensure_module_profile_matches_guard::<ExecuteError>(
            &module,
            module_version,
            profile_id,
            profile_version,
        )?;
        let entry = module.comptime_mut(profile_id);
        entry.results.insert(expression.local_id, result);

        Ok(())
    }
}
