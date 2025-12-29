use crate::{Compiler, ExecuteError, ExecuteResult, TaskResultCollector};

use destack_source::ModuleId;
use destack_workspace::{BUILTIN_PACKAGE_ID, ModuleComptime, ProfileId};

use super::ComptimePatch;
use destack_dir as dir;

impl Compiler {
    /// Prepare comptime state for a module within a profile.
    pub(crate) fn execute_module_prepare(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> ExecuteResult<()> {
        // ensure DIR exists
        self.require_elaborate_module(module_id, profile_id)?;

        let profile_version = self.program.profile(profile_id).version;
        let module = self.program.modules.get(module_id);
        let mut module = module.write();
        let module_version = module.version;

        // initialize or refresh module comptime entry
        match module
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
                module.comptimes.push(ModuleComptime {
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
    ) -> ExecuteResult<()> {
        // ensure module comptime state exists
        self.require_execute_module_prepare(module_id, profile_id)?;

        // collect comptime expressions in this module
        let comptime_nodes: Vec<dir::LocalNodeIdAny> = {
            let module = self.program.modules.get(module_id);
            let module = module.read();
            if module.package_id == BUILTIN_PACKAGE_ID {
                return Ok(());
            }

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
                self.execute_expression(module_id, profile_id, expression_id.into_global(module_id)),
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
            let module = self.program.modules.get(module_id);
            let module = module.read();
            let dir = module.dir(profile_id);
            let mut tree = dir.tree.write();
            for patch in patches {
                self.apply_comptime_patch(module_id, &mut tree, patch);
            }
        }

        Ok(())
    }

    /// Execute comptime code for a single expression.
    pub(crate) fn execute_expression(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        expression: dir::GlobalNodeIdAny,
    ) -> ExecuteResult<()> {
        if module_id != expression.module_id {
            return Err(ExecuteError::UnsupportedConstruct { node: expression });
        }
        let expression_id = expression
            .local_id
            .try_into_typed::<dir::Expression>()
            .map_err(|_| ExecuteError::UnsupportedConstruct { node: expression })?;

        // ensure module comptime state exists
        self.require_execute_module_prepare(module_id, profile_id)?;

        // run comptime evaluation for this expression
        let result = {
            let module = self.program.modules.get(module_id);
            let module = module.read();
            let dir = module.dir(profile_id);
            let tree = dir.tree.read();
            let expression = tree.get(expression_id);
            let dir::Expression::Comptime { body } = expression else {
                return Ok(());
            };
            let _profile_ref = self.program.profile(profile_id);
            let _body = *body;

            // NOTE #Architecture: should we return execute error (and fail the task) or just return None?
            self.error(ExecuteError::FailedExecution {
                module: module_id,
                message: "FUGU: implement comptime execution".to_string(),
            });
            None
        };

        // store the result for later patching
        let module = self.program.modules.get(module_id);
        let mut module = module.write();
        let entry = module.comptime_mut(profile_id);
        entry.results.insert(expression.local_id, result);

        Ok(())
    }
}
