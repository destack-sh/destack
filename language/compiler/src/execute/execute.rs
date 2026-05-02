use indexmap::IndexMap;
use std::sync::Arc;

use crate::{Compiler, CompilerContext, ExecuteError, ExecuteResult, RequirementCollector};

use destack_artifact::DirPatched;
use destack_source::ModuleId;
use destack_workspace::{ProfileId, TrustPolicy};

use super::{ComptimeOutput, ComptimePatch, collect_comptime_dependencies};
use vm::{Allocator, Heap, HeapLimits, HeapOptions, SharedHeap, SharedHeapLimits, StaticSpace};
use {destack_dir as dir, destack_vm as vm};

/// In-flight comptime results for one module build.
type ComptimeResults = IndexMap<dir::LocalNodeIdAny, Option<ComptimeOutput>>;

impl Compiler {
    /// Execute comptime code for a module within a profile.
    pub(crate) fn execute_module_patch(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        context: &CompilerContext<'_>,
    ) -> Result<DirPatched, ExecuteError> {
        // skip modules without executable comptime state
        if !context.is_code_module(module_id) {
            let elaborated =
                self.require_dir_elaborated_data(context.revision(), module_id, profile_id)?;
            let payload = DirPatched::from_elaborated_with(
                elaborated.as_ref(),
                elaborated.tree.as_ref().clone(),
            );
            return Ok(payload);
        }

        // collect comptime expressions in this module
        let comptime_nodes: Vec<dir::LocalNodeIdAny> = {
            let dir =
                self.require_dir_elaborated_data(context.revision(), module_id, profile_id)?;
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
        let mut collector = RequirementCollector::new();
        let mut results = ComptimeResults::new();
        for expression_id in &comptime_nodes {
            self.collect(
                &mut collector,
                self.execute_expression(
                    module_id,
                    profile_id,
                    expression_id.into_global(module_id),
                    &mut results,
                    context,
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
        let elaborated =
            self.require_dir_elaborated_data(context.revision(), module_id, profile_id)?;
        let mut tree = elaborated.tree.as_ref().clone();
        for patch in patches {
            self.apply_comptime_patch(
                module_id,
                profile_id,
                &mut tree,
                elaborated.types.as_ref(),
                patch,
            );
        }

        // publish the patched artifact with updated comptime tree state
        let payload = DirPatched::from_elaborated_with(elaborated.as_ref(), tree);

        Ok(payload)
    }

    /// Execute comptime code for a single expression.
    pub(crate) fn execute_expression(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        expression: dir::GlobalNodeIdAny,
        results: &mut ComptimeResults,
        context: &CompilerContext<'_>,
    ) -> ExecuteResult<()> {
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
        if !context.is_code_module(module_id) {
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
            let module = context.module(module_id);
            let module = module.as_ref();
            let dir =
                self.require_dir_elaborated_data(context.revision(), module_id, profile_id)?;
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
                self.execute_expression(module_id, profile_id, dependency_id, results, context)?;
            }

            // lower the comptime expression to MIR
            let (mir_tree, strings, function_id) =
                self.lower_comptime_expression(module, profile_id, *body, context)?;

            // execute the MIR with the interpreter
            let mut options = vm::IsolateOptions::comptime();
            if matches!(self.comptime_target.trust_policy, TrustPolicy::Untrusted) {
                options.checks = vm::CheckOptions::debug();
            }

            // TODO #Cleanup: figure out a nicer way to create the Isolate for comptime
            let mut isolate = vm::Isolate::build_with_options(
                vm::IsolateId::new(1),
                mir_tree,
                strings.into_immutable(), // TODO #Performance: avoid cloning the string pool
                options,
            )
            .map_err(|error| ExecuteError::FailedExecution {
                module: module_id,
                message: format!("{error}"),
            })?;
            let local_options = HeapOptions::local();
            let shared_options = HeapOptions::shared();
            let allocator = Arc::new(
                Allocator::try_new(
                    local_options.page_bytes,
                    local_options.allocator_chunk_bytes,
                )
                .map_err(|error| ExecuteError::FailedExecution {
                    module: module_id,
                    message: format!("{error}"),
                })?,
            );
            let mut statics = StaticSpace::empty();
            let mut heap = Heap::with_allocator_limits_and_options(
                allocator.clone(),
                HeapLimits::default(),
                local_options,
            )
            .map_err(|error| ExecuteError::FailedExecution {
                module: module_id,
                message: format!("{error}"),
            })?;
            let shared = SharedHeap::with_allocator_limits_and_options(
                allocator,
                SharedHeapLimits::default(),
                shared_options,
            )
            .map_err(|error| ExecuteError::FailedExecution {
                module: module_id,
                message: format!("{error}"),
            })?;
            let shared_gc = shared.register_collector_worker();
            let mut shared_allocator = shared.allocator();

            isolate
                .initialize(&heap, &shared, &mut statics)
                .map_err(|error| ExecuteError::FailedExecution {
                    module: module_id,
                    message: format!("{error}"),
                })?;
            let vm_value = isolate
                .run_function(
                    &mut statics,
                    &mut heap,
                    &shared,
                    &mut shared_allocator,
                    &shared_gc,
                    function_id,
                    &[],
                )
                .map_err(|error| ExecuteError::FailedExecution {
                    module: module_id,
                    message: format!("{error}"),
                })?;

            // convert the vm value to a static expression
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
