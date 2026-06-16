use destack_artifact::ProgramAnalysis;
use destack_mir as mir;
use destack_mir::Mutation;

use crate::optimize::{ModulePass, PipelineContext, declare_pass};

declare_pass! {
    /// Remove functions the analysis scope cannot reach.
    ///
    /// Scope-agnostic: with a module-scoped program analysis it drops module-dead
    /// functions, with a whole-program analysis it drops program-dead functions. A
    /// function survives only when some root reaches it over call or address edges; a
    /// never-address-taken function is dead even amid unknown indirect calls, since no
    /// pointer to it can exist.
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
    #[pass(id = "dead-function-eliminate")]
    pub DeadFunctionEliminate,
    "Eliminate dead functions"
}

impl ModulePass for DeadFunctionEliminate {
    /// Strip functions the analysis scope cannot reach.
    fn run(
        &self,
        tree: &mut mir::Tree,
        ctx: &PipelineContext<'_>,
        _analyses: &mir::ModuleAnalyses,
    ) -> Mutation {
        let changed = run_dead_function_eliminate(tree, ctx.program_analysis());

        // report stripped definitions as control-flow changes
        if changed {
            Mutation::CONTROL_FLOW
        } else {
            Mutation::NONE
        }
    }

    /// Return the pass display name.
    fn name(&self) -> &'static str {
        "DeadFunctionEliminate"
    }

    /// Return the pass identifier.
    fn id(&self) -> &'static str {
        "dead-function-eliminate"
    }
}

/// Strip every defined function the analysis scope cannot reach.
pub(crate) fn run_dead_function_eliminate(tree: &mut mir::Tree, program: &ProgramAnalysis) -> bool {
    // an empty scope defines no symbols, so nothing can be proven dead
    if program.is_empty() {
        return false;
    }

    // collect defined functions no root reaches
    let dead: Vec<_> = tree
        .iter_nodes::<mir::Function>()
        .filter_map(|(id, function)| {
            (function.entry.is_some() && !program.is_live(function.symbol)).then_some(id)
        })
        .collect();

    // strip each unreachable function down to an external declaration
    for function_id in &dead {
        strip_function_body(*function_id, tree);
    }

    !dead.is_empty()
}

/// Strip the body of a function, leaving an import declaration.
pub(crate) fn strip_function_body(
    function_id: mir::LocalNodeId<mir::Function>,
    tree: &mut mir::Tree,
) {
    // collect blocks and instructions before stripping the body
    let block_ids = tree.get(function_id).blocks.clone();

    // convert the definition into an import declaration
    let function = tree.get_mut(function_id);
    function.linkage = mir::Linkage::Import;
    function.locals.clear();
    function.blocks.clear();
    function.entry = None;

    // remove instruction metadata tied to stripped blocks
    for block_id in &block_ids {
        let instruction_ids = tree.get(*block_id).instructions.clone();
        for instruction_id in instruction_ids {
            tree.metadata
                .memory
                .memory_accesses_by_instruction_id
                .remove(&instruction_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Build a program analysis treating the module as a standalone program.
    fn module_analysis(tree: &mir::Tree) -> ProgramAnalysis {
        let links = mir::ModuleAnalyses::new().get::<mir::LinkGraph>(tree);
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
entry0:
    call live(): () -> void
    return
}

function live(): void {
entry0:
    return
}

function dead(): void {
entry0:
    return
}
"#,
        );
        let root_id = test.function_id_by_name("root");
        let live_id = test.function_id_by_name("live");
        let dead_id = test.function_id_by_name("dead");

        // the export reaches `live`; `dead` is reached by nothing
        let program = module_analysis(&test.tree);
        let changed = run_dead_function_eliminate(&mut test.tree, &program);

        // reachable functions keep their bodies; the unreachable one is externalized
        assert!(changed);
        assert!(test.tree.get(root_id).entry.is_some());
        assert!(test.tree.get(live_id).entry.is_some());
        assert!(test.tree.get(dead_id).entry.is_none());
        assert_eq!(test.tree.get(dead_id).linkage, mir::Linkage::Import);
    }
}
