use destack_artifact::ProgramAnalysis;
use destack_mir as mir;
use destack_mir::Mutation;
use destack_source::{ProvenanceBuilder, ProvenanceJournal};

use crate::optimize::{MirOptimized, ModulePass, PipelineContext, declare_pass};

declare_pass! {
    /// Remove functions the analysis scope cannot reach.
    ///
    /// ```mir
    /// export function root(): void {
    /// b0:
    ///     call live(): () => void
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
    ///     call live(): () => void
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
        provenance: &mut ProvenanceBuilder,
        ctx: &PipelineContext<'_>,
        _analyses: &mut mir::AnalysisCache,
    ) -> Mutation {
        let mut journal = provenance.record(Self::metadata().id);
        let tree = &mut optimized.tree;
        let accesses = &mut optimized.accesses;
        let drops = &mut optimized.drops;
        let changed = run_eliminate_dead_functions(
            tree,
            accesses,
            drops,
            ctx.program_analysis(),
            &mut journal,
        );

        // report stripped definitions as control-flow changes
        if changed {
            Mutation::CONTROL
        } else {
            Mutation::NONE
        }
    }
}

/// Strip every defined function the analysis scope cannot reach.
pub(crate) fn run_eliminate_dead_functions(
    tree: &mut mir::Tree,
    accesses: &mut mir::AccessTable,
    drops: &mut mir::DropTable,
    program: &ProgramAnalysis,
    provenance: &mut ProvenanceJournal<'_>,
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
        strip_function_body(*function_id, tree, accesses, provenance);
        drops.remove_function(*function_id);
    }

    !dead.is_empty()
}

/// Strip the body of a function, leaving an import declaration.
pub(crate) fn strip_function_body(
    function_id: mir::LocalNodeId<mir::Function>,
    tree: &mut mir::Tree,
    accesses: &mut mir::AccessTable,
    provenance: &mut ProvenanceJournal<'_>,
) {
    // collect blocks and instructions before stripping the body
    let mut function = tree.get(function_id).clone();
    let block_ids = function.blocks().to_vec();

    // remove function body provenance
    let mut removed = function
        .locals()
        .iter()
        .map(|local| tree.provenance(*local))
        .collect::<Vec<_>>();
    for block_id in &block_ids {
        let block = tree.get(*block_id);
        removed.push(tree.provenance(*block_id));
        removed.push(tree.provenance(block.terminator));
        removed.extend(
            block
                .parameters
                .iter()
                .map(|parameter| parameter.provenance),
        );
        removed.extend(
            block
                .instructions
                .iter()
                .map(|instruction| tree.provenance(*instruction)),
        );
    }
    provenance.remove(&removed);

    // convert the definition into an import declaration
    function.linkage = mir::Linkage::Import;
    function.clear_body();
    tree.rewrite(function_id, function, provenance);

    // remove instruction tables tied to stripped blocks
    for block_id in &block_ids {
        let instruction_ids = tree.get(*block_id).instructions.clone();
        for instruction_id in instruction_ids {
            accesses.remove(instruction_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Build a program analysis treating the module as a standalone program.
    fn module_analysis(test: &TestProgram) -> ProgramAnalysis {
        let mut analyses = test.analysis_cache();
        let links = analyses.link(
            &test.optimized.tree,
            &test.optimized.effects,
            &test.optimized.dispatch,
            &test.optimized.drops,
        );
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
        let mut provenance = test.optimized.provenance.extend();
        let changed = {
            let mut journal = provenance.record("eliminate-dead-functions");
            run_eliminate_dead_functions(
                &mut test.optimized.tree,
                &mut test.optimized.accesses,
                &mut test.optimized.drops,
                &program,
                &mut journal,
            )
        };
        test.optimized.provenance = provenance.finish();

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

    #[test]
    fn test_retains_referenced_destructors_and_removes_dead_entries() {
        let mut test = TestProgram::new(
            r#"
type Dropped {
    value: ref<int32, unique, mutable>;
}

type Allocated {
    value: ref<int32, unique, mutable>;
}

type Dead {
    value: ref<int32, unique, mutable>;
}

export function root(v0: Dropped): void {
entry(v0: Dropped):
    drop v0
    v1: ref<Allocated, managed, mutable> = new.zeroed Allocated
    return
}

function dropDropped(v0: ref<Dropped, borrowed, exclusive, frame>): void {
entry(v0: ref<Dropped, borrowed, exclusive, frame>):
    return
}

function dropAllocated(v0: ref<Allocated, borrowed, exclusive>): void {
entry(v0: ref<Allocated, borrowed, exclusive>):
    return
}

function dropDead(v0: ref<Dead, borrowed, exclusive, frame>): void {
entry(v0: ref<Dead, borrowed, exclusive, frame>):
    return
}
"#,
        );
        let dropped = test.type_id_by_name("Dropped");
        let allocated = test.type_id_by_name("Allocated");
        let dead = test.type_id_by_name("Dead");
        let drop_dropped = test.function_id_by_name("dropDropped");
        let drop_allocated = test.function_id_by_name("dropAllocated");
        let drop_dead = test.function_id_by_name("dropDead");
        test.optimized
            .drops
            .set_destructor(dropped, mir::Storage::Frame, drop_dropped);
        test.optimized
            .drops
            .set_destructor(allocated, mir::Storage::LocalHeap, drop_allocated);
        test.optimized
            .drops
            .set_destructor(dead, mir::Storage::Frame, drop_dead);

        // retain destructors selected by execution and remove the unused entry
        let program = module_analysis(&test);
        let mut provenance = test.optimized.provenance.extend();
        let changed = {
            let mut journal = provenance.record("eliminate-dead-functions");
            run_eliminate_dead_functions(
                &mut test.optimized.tree,
                &mut test.optimized.accesses,
                &mut test.optimized.drops,
                &program,
                &mut journal,
            )
        };
        test.optimized.provenance = provenance.finish();

        assert!(changed);
        assert!(test.optimized.tree.get(drop_dropped).entry().is_some());
        assert!(test.optimized.tree.get(drop_allocated).entry().is_some());
        assert!(test.optimized.tree.get(drop_dead).entry().is_none());
        assert_eq!(
            test.optimized
                .drops
                .destructor(dropped, mir::Storage::Frame),
            Some(drop_dropped)
        );
        assert_eq!(
            test.optimized
                .drops
                .destructor(allocated, mir::Storage::LocalHeap),
            Some(drop_allocated)
        );
        assert_eq!(
            test.optimized.drops.destructor(dead, mir::Storage::Frame),
            None
        );
    }
}
