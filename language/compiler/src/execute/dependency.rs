use destack_dir as dir;
use destack_dir::NodeVisitor;

/// A visitor that collects comptime dependencies.
#[derive(Debug)]
struct ComptimeDependencyCollector {
    /// The root id of the comptime expression.
    root_id: dir::LocalNodeIdAny,
    /// The dependencies collected.
    dependencies: Vec<dir::LocalNodeIdAny>,
    /// The options for the visitor.
    options: dir::NodeVisitorOptions,
}

impl ComptimeDependencyCollector {
    /// Create a collector rooted at the given comptime expression.
    fn new(root_id: dir::LocalNodeId<dir::Expression>) -> Self {
        Self {
            root_id: root_id.into_any(),
            dependencies: Vec::new(),
            options: dir::NodeVisitorOptions::default(),
        }
    }

    /// Return the collected dependency node ids.
    fn into_dependencies(self) -> Vec<dir::LocalNodeIdAny> {
        self.dependencies
    }
}

impl dir::NodeVisitor for ComptimeDependencyCollector {
    fn options(&self) -> &dir::NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // collect nested comptime expressions and skip the root
        if matches!(expression, dir::Expression::Comptime { .. })
            && id.into_any() != self.root_id
            && !self.dependencies.contains(&id.into_any())
        {
            self.dependencies.push(id.into_any());
        }

        // keep walking to find deeper dependencies
        dir::walk_expression(self, tree, id, expression);
    }
}

/// Collect nested comptime expressions for dependency tracking.
pub(crate) fn collect_comptime_dependencies(
    tree: &dir::Tree,
    root_id: dir::LocalNodeId<dir::Expression>,
) -> Vec<dir::LocalNodeIdAny> {
    let mut visitor = ComptimeDependencyCollector::new(root_id);
    let root = tree.get(root_id);
    visitor.visit_expression(tree, root_id, root);
    visitor.into_dependencies()
}
