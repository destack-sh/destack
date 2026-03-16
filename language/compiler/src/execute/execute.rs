use indexmap::IndexMap;

use crate::{BuildRequirementCollector, Compiler, ExecuteError, ExecuteResult};

use destack_source::{CacheKind, ModuleId, ModuleVersion, ProfileVersion};
use destack_workspace::{ComptimeOutput, ModuleDir, ProfileId, TrustPolicy};

use super::{ComptimePatch, collect_comptime_dependencies};
use vm::{Heap, MemoryContext, SharedSpace};
use {destack_dir as dir, destack_vm as vm};

/// In-flight comptime results for one module build.
type ComptimeResults = IndexMap<dir::LocalNodeIdAny, Option<ComptimeOutput>>;

impl Compiler {
    /// Execute comptime code for a module within a profile.
    pub(crate) fn execute_module_patch(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        module_version: ModuleVersion,
        profile_version: ProfileVersion,
    ) -> Result<ModuleDir, ExecuteError> {
        // skip stale tasks
        self.ensure_module_profile_matches::<ExecuteError>(
            module_id,
            module_version,
            profile_id,
            profile_version,
        )?;

        // resolve cache handle
        let cache_handle =
            self.cache_handle_for_module(module_id, Some(profile_id), None, CacheKind::DirPatched);

        // try to load executed DIR from cache
        if let Some(cache) = cache_handle.as_ref()
            && let Ok(Some(entry)) = cache.read_dir_patched()
        {
            self.ensure_module_profile_matches::<ExecuteError>(
                module_id,
                module_version,
                profile_id,
                profile_version,
            )?;
            let payload = entry.payload;
            tracing::trace!(?module_id, ?profile_id, "execute.module.cache");
            return Ok(payload);
        }

        // skip modules without executable comptime state
        if !self.is_code_module(module_id) {
            return Ok(self
                .require_dir_elaborated_data(module_id, profile_id)?
                .as_ref()
                .clone());
        }

        // collect comptime expressions in this module
        let comptime_nodes: Vec<dir::LocalNodeIdAny> = {
            let dir = self.require_dir_elaborated_data(module_id, profile_id)?;
            let tree = &dir.tree;
            let mut nodes = Vec::new();
            for (expression_id, expression) in tree.iter_nodes_of_type::<dir::Expression>() {
                if matches!(expression, dir::Expression::Comptime { .. }) {
                    nodes.push(expression_id.into_any());
                }
            }
            nodes
        };

        // execute each comptime expression
        let mut collector = BuildRequirementCollector::new();
        let mut results = ComptimeResults::new();
        for expression_id in &comptime_nodes {
            self.collect(
                &mut collector,
                self.execute_expression(
                    module_id,
                    profile_id,
                    module_version,
                    profile_version,
                    expression_id.into_global(module_id),
                    &mut results,
                ),
            );
        }
        if let Some(requirement) = collector.try_into_requirement() {
            return Err(ExecuteError::Yield { requirement });
        }

        // gather execute results for this module
        let patches = results
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
            .collect::<Vec<_>>();

        // patch comptime results into one transient DIR snapshot
        self.ensure_module_profile_matches::<ExecuteError>(
            module_id,
            module_version,
            profile_id,
            profile_version,
        )?;
        let mut payload = self
            .require_dir_elaborated_data(module_id, profile_id)?
            .as_ref()
            .clone();
        for patch in patches {
            let tree = payload.tree_mut();
            self.apply_comptime_patch(module_id, profile_id, tree, patch);
        }

        // write executed DIR to cache
        if let Some(cache) = cache_handle.as_ref() {
            self.ensure_module_profile_matches::<ExecuteError>(
                module_id,
                module_version,
                profile_id,
                profile_version,
            )?;
            if let Err(error) = cache.write_dir_patched(payload.clone()) {
                tracing::debug!(
                    ?module_id,
                    ?profile_id,
                    ?error,
                    "execute.module.cache.write"
                );
            }
        }

        Ok(payload)
    }

    /// Execute comptime code for a single expression.
    pub(crate) fn execute_expression(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        module_version: ModuleVersion,
        profile_version: ProfileVersion,
        expression: dir::GlobalNodeIdAny,
        results: &mut ComptimeResults,
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

        // skip modules without executable comptime state
        if !self.is_code_module(module_id) {
            return Ok(());
        }

        // reuse already computed outputs for this build
        if results.contains_key(&expression.local_id) {
            return Ok(());
        }

        // reserve the slot before recursing through nested comptime dependencies
        results.insert(expression.local_id, None);

        // run comptime evaluation for this expression
        let result = {
            // get the comptime expression
            let module = self.program.modules.get(module_id);
            let module = module.as_ref();
            let dir = self.require_dir_elaborated_data(module_id, profile_id)?;
            let tree = &dir.tree;
            let expression = tree.get(expression_id);
            let dir::Expression::Comptime { body } = expression else {
                return Ok(());
            };

            // ensure nested comptime expressions are executed first
            let dependencies = collect_comptime_dependencies(tree, *body);
            for dependency in dependencies {
                // execute each nested comptime dependency first
                let dependency_id = dependency.into_global(module_id);
                self.execute_expression(
                    module_id,
                    profile_id,
                    module_version,
                    profile_version,
                    dependency_id,
                    results,
                )?;
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

            let mut isolate = vm::Isolate::build_with_options(
                mir_tree,
                strings.into_immutable(), // TODO #Performance: avoid cloning the string pool
                options,
            )
            .map_err(|error| ExecuteError::FailedExecution {
                module: module_id,
                message: format!("{error}"),
            })?;
            let mut heap = Heap::new();
            let mut shared = SharedSpace::new();
            let mut memory = MemoryContext::new(&mut heap, &mut shared);

            isolate
                .initialize(&mut memory)
                .map_err(|error| ExecuteError::FailedExecution {
                    module: module_id,
                    message: format!("{error}"),
                })?;
            let output = isolate
                .run_function(&mut memory, function_id, &[])
                .map_err(|error| ExecuteError::FailedExecution {
                    module: module_id,
                    message: format!("{error}"),
                })?;

            // convert the vm value to a static expression
            let vm_value = output.value;
            let dir_value = self
                .value_to_static_expression(&isolate, &heap, &vm_value)
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
        results.insert(expression.local_id, result);

        Ok(())
    }
}
