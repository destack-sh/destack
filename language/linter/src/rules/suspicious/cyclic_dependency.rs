use destack_core::FxIndexSet;
use destack_source::ComponentId;

use crate::rules::declare_lint;
use crate::{DirProgramContext, LinterError};

declare_lint! {
    /// Disallow cyclic dependencies between source modules.
    pub CYCLIC_DEPENDENCY {
        id: "cyclic-dependency",
        code: "LU064",
        description: "Disallow cyclic dependencies between source modules",
        category: Suspicious,
        level: Warning,
        fixable: Never,
        check: DirProgram(check),
    }
}

/// Check cyclic-dependency.
fn check(mut context: DirProgramContext<'_>) -> Result<(), LinterError> {
    let graph = context.program.graph.as_ref();
    let modules = context
        .modules
        .iter()
        .map(|module| module.id)
        .filter(|module| context.program.owns(*module))
        .collect::<Vec<_>>();
    let components = cyclic_components(graph, &modules)?;

    // report each cyclic component owned by this target once
    for component in components {
        let (module, edge) = cycle_edge(component, &context)?;
        let anchor = module.anchor(edge.source.local_id);
        let diagnostic = context
            .diagnostic("source modules form a dependency cycle", anchor)
            .label("this dependency participates in the cycle");
        context.report(diagnostic);
    }

    Ok(())
}

/// Return cyclic components containing one target-owned module.
fn cyclic_components(
    graph: &destack_artifact::ComponentGraph,
    modules: &[destack_source::ModuleId],
) -> Result<Vec<ComponentId>, LinterError> {
    let mut seen = FxIndexSet::<ComponentId>::default();
    let mut cyclic = Vec::new();

    // classify each reachable component once
    for module in modules.iter().copied() {
        let component = graph
            .component(module)
            .ok_or_else(|| LinterError::Internal {
                message: format!("target module {module:?} is absent from its component graph"),
            })?;
        if !seen.insert(component) {
            continue;
        }

        let members = graph.members(component);
        let is_self_cycle = members.len() == 1 && graph.edges(module).contains(&module);
        if members.len() > 1 || is_self_cycle {
            cyclic.push(component);
        }
    }

    Ok(cyclic)
}

/// Return one source edge belonging to a cyclic component.
fn cycle_edge<'a>(
    component: ComponentId,
    context: &'a DirProgramContext<'_>,
) -> Result<(&'a crate::LintModule, &'a destack_dir::ModuleEdge), LinterError> {
    for module in context.modules.iter() {
        if !context.program.owns(module.id) {
            continue;
        }
        if context.program.graph.component(module.id) != Some(component) {
            continue;
        }
        for edge in module.modules.iter() {
            let Some(target) = edge.target else {
                continue;
            };
            if context.program.graph.component(target) == Some(component) {
                return Ok((module, edge));
            }
        }
    }

    Err(LinterError::Internal {
        message: format!("cyclic component {component:?} has no internal module edge"),
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use destack_artifact::ComponentGraph;
    use destack_source::{ModuleId, PackageId, ProfileId};
    use indexmap::IndexMap;

    use super::*;

    /// Return each reachable cyclic component exactly once.
    #[test]
    fn test_collect_cyclic_components() {
        let package = PackageId::new(1);
        let first = ModuleId::new(package, 1);
        let second = ModuleId::new(package, 2);
        let acyclic = ModuleId::new(package, 3);
        let mut edges = IndexMap::new();
        edges.insert(first, Arc::from([second]));
        edges.insert(second, Arc::from([first]));
        edges.insert(acyclic, Arc::from([]));
        let graph = ComponentGraph::from_edges(ProfileId::new(1), edges);

        let actual = cyclic_components(&graph, &[first, second, acyclic]).unwrap();
        let expected = vec![graph.component(first).unwrap()];

        assert_eq!(actual, expected);
        assert_eq!(graph.members(expected[0]), &[first, second]);
    }
}
