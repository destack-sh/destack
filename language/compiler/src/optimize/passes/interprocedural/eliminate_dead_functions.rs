use destack_artifact::ProgramAnalysis;
use destack_mir as mir;
use destack_mir::Mutation;

use crate::optimize::{MirOptimized, ModulePass, PipelineContext, declare_pass};

declare_pass! {
    /// Remove functions the analysis scope cannot reach.
    ///
    /// Scope-agnostic: with module analysis it drops module-dead functions, with program
    /// analysis it drops program-dead functions. A function survives only when some root
    /// reaches it over call or address edges; a never-address-taken function is dead even
    /// amid unknown indirect calls, since no pointer to it can exist.
    ///
    /// ```mir
    /// export function root(): void {
    /// b0:
    ///     call live()
    ///     return
    /// }
    /// function live(): void {
    /// b0:
    ///     return
    /// }
    /// function dead(): void {
    /// b0:
    ///     return
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// export function root(): void {
    /// b0:
    ///     call live()
    ///     return
    /// }
    /// function live(): void {
    /// b0:
    ///     return
    /// }
    /// external function dead(): void
    /// ```
    #[pass(id = "eliminate-dead-functions")]
    pub EliminateDeadFunctions,
    "Eliminate dead functions"
}

impl ModulePass for EliminateDeadFunctions {
    /// Strip functions the analysis scope cannot reach.
    fn run(
        &self,
        optimized: &mut MirOptimized,
        ctx: &PipelineContext<'_>,
        _analyses: &mir::TreeAnalysisCache,
    ) -> Mutation {
        let tree = &mut optimized.tree;
        let memory = &mut optimized.memory;
        let changed = run_eliminate_dead_functions(tree, memory, ctx.program_analysis());

        // report stripped definitions as control-flow changes
        if changed {
            Mutation::CONTROL
        } else {
            Mutation::NONE
        }
    }

    /// Return the pass display name.
    fn name(&self) -> &'static str {
        "EliminateDeadFunctions"
    }

    /// Return the pass identifier.
    fn id(&self) -> &'static str {
        "eliminate-dead-functions"
    }
}

/// Strip every defined function the analysis scope cannot reach.
pub(crate) fn run_eliminate_dead_functions(
    tree: &mut mir::Tree,
    memory: &mut mir::MemoryTable,
    program: &ProgramAnalysis,
) -> bool {
    // an empty scope defines no symbols, so nothing can be proven dead
    if program.is_empty() {
        return false;
    }

    // collect defined functions no root reaches
    let dead: Vec<_> = tree
        .iter_nodes::<mir::Function>()
        .filter_map(|(id, function)| {
            (function.entry().is_some() && !program.is_live(function.symbol)).then_some(id)
        })
        .collect();

    // strip each unreachable function down to an external declaration
    for function_id in &dead {
        strip_function_body(*function_id, tree, memory);
    }

    !dead.is_empty()
}

/// Strip the body of a function, leaving an import declaration.
pub(crate) fn strip_function_body(
    function_id: mir::LocalNodeId<mir::Function>,
    tree: &mut mir::Tree,
    memory: &mut mir::MemoryTable,
) {
    // collect blocks and instructions before stripping the body
    let block_ids = tree.get(function_id).blocks().to_vec();

    // convert the definition into an import declaration
    let function = tree.get_mut(function_id);
    function.linkage = mir::Linkage::Import;
    function.clear_body();

    // remove instruction tables tied to stripped blocks
    for block_id in &block_ids {
        let instruction_ids = tree.get(*block_id).instructions.clone();
        for instruction_id in instruction_ids {
            memory.remove_memory_accesses(instruction_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Build a program analysis treating the module as a standalone program.
    fn module_analysis(test: &TestProgram) -> ProgramAnalysis {
        let analyses = test.tree_analysis_cache();
        let links = analyses.get::<mir::LinkGraph>(&test.optimized.tree);
        let roots: Vec<_> = links
            .nodes()
            .filter(|(_, node)| node.linkage().is_exported())
            .map(|(symbol, _)| symbol)
            .collect();
        let supergraph = mir::LinkSupergraph::build([&*links]);

        ProgramAnalysis::analyze(&supergraph, &roots)
    }

    #[test]
    fn test_eliminates_functions_no_root_reaches() {
        let mut test = TestProgram::new(
            r#"
export function root(): void {
entry:
    call live(): () => void
    return
}

function live(): void {
entry:
    return
}

function dead(): void {
entry:
    return
}
"#,
        );
        let root_id = test.function_id_by_name("root");
        let live_id = test.function_id_by_name("live");
        let dead_id = test.function_id_by_name("dead");

        // the export reaches `live`; `dead` is reached by nothing
        let program = module_analysis(&test);
        let changed = run_eliminate_dead_functions(
            &mut test.optimized.tree,
            &mut test.optimized.memory,
            &program,
        );

        // reachable functions keep their bodies; the unreachable one is externalized
        assert!(changed);
        assert!(test.optimized.tree.get(root_id).entry().is_some());
        assert!(test.optimized.tree.get(live_id).entry().is_some());
        assert!(test.optimized.tree.get(dead_id).entry().is_none());
        assert_eq!(
            test.optimized.tree.get(dead_id).linkage,
            mir::Linkage::Import
        );
    }
}
