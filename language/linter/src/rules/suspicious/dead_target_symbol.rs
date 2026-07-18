use crate::rules::declare_lint;
use crate::{LinterError, MirProgramContext};
use destack_mir as mir;

declare_lint! {
    /// Warn on target symbols unreachable from program roots.
    pub DEAD_TARGET_SYMBOL {
        id: "dead-target-symbol",
        code: "LU061",
        description: "Warn on target symbols unreachable from program roots",
        category: Suspicious,
        level: Warning,
        fixable: Never,
        check: MirProgram(check),
    }
}

/// Check dead-target-symbol.
fn check(mut context: MirProgramContext<'_>) -> Result<(), LinterError> {
    // report every defined symbol outside whole-program reachability
    for module in context.modules.iter() {
        if !context.program.owns(module.id) {
            continue;
        }

        for dead in dead_symbols(module.mir.as_ref(), context.analysis) {
            let anchor = module.anchor(dead.node);
            let message = match dead.kind {
                DeadSymbolKind::Function => "function is unreachable from every target root",
                DeadSymbolKind::Global => "global is unreachable from every target root",
            };
            let diagnostic = context
                .diagnostic(message, anchor)
                .label("this symbol is not part of the target program");
            context.report(diagnostic);
        }
    }

    Ok(())
}

/// One defined MIR symbol outside whole-program reachability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DeadSymbol {
    /// The MIR declaration node.
    node: mir::LocalNodeIdAny,
    /// The kind of declaration.
    kind: DeadSymbolKind,
}

/// The kind of one unreachable MIR symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DeadSymbolKind {
    /// A function definition.
    Function,
    /// A global definition.
    Global,
}

/// Return defined MIR symbols outside whole-program reachability.
fn dead_symbols(
    module: &destack_artifact::MirLowered,
    analysis: &destack_artifact::ProgramAnalysis,
) -> Vec<DeadSymbol> {
    let mut dead = Vec::new();

    // collect unreachable function definitions
    for (function_id, function) in module.tree.iter_nodes::<mir::Function>() {
        if !function.is_import() && !analysis.is_live(function.symbol) {
            dead.push(DeadSymbol {
                node: function_id.into_any(),
                kind: DeadSymbolKind::Function,
            });
        }
    }

    // collect unreachable global definitions
    for (global_id, global) in module.tree.iter_nodes::<mir::Global>() {
        if !global.is_import() && !analysis.is_live(global.symbol) {
            dead.push(DeadSymbol {
                node: global_id.into_any(),
                kind: DeadSymbolKind::Global,
            });
        }
    }

    dead
}

#[cfg(test)]
mod tests {
    use destack_artifact::{MirLowered, ProgramAnalysis};
    use destack_core::StringId;

    use super::*;

    /// Return only definitions unreachable from the selected roots.
    #[test]
    fn test_collect_dead_symbols() {
        let mut module = MirLowered::new();
        let void = module.tree.insert_type(mir::Type::Void);
        let live = insert_function(&mut module, "live", void);
        let dead = insert_function(&mut module, "dead", void);
        let analyses =
            mir::TreeAnalysisCache::new(&module.dispatch, &module.memory, &module.effects);
        let links = analyses.get::<mir::LinkGraph>(&module.tree);
        let graph = mir::LinkSupergraph::build([links.as_ref()]);
        let analysis = ProgramAnalysis::analyze(&graph, &[module.tree.get(live).symbol]);

        let actual = dead_symbols(&module, &analysis);
        let expected = vec![DeadSymbol {
            node: dead.into_any(),
            kind: DeadSymbolKind::Function,
        }];

        assert_eq!(actual, expected);
    }

    /// Insert one empty local function definition.
    fn insert_function(
        module: &mut MirLowered,
        name: &str,
        return_type: mir::TypeId,
    ) -> mir::FunctionId {
        let terminator = module.tree.insert(mir::Terminator::Return { value: None });
        let block = module.tree.insert(mir::Block::new(terminator));
        let function = mir::Function::define(
            StringId::for_text(name),
            Vec::new(),
            Vec::new(),
            return_type,
            block,
        );

        module.tree.insert(function)
    }
}
