use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, walk_expression};

use crate::rules::common::expression_enters_nested_declaration_scope;

/// The return usage found within one callable body.
#[derive(Debug, Default)]
pub struct CallableReturnUsage {
    /// Whether the callable contains any explicit `return value`.
    pub has_return_value: bool,
    /// Whether the callable uses one expression body that returns a value.
    pub has_expression_body_return_value: bool,
    /// Explicit `return value` nodes found in the callable body.
    pub return_value_nodes: Vec<dir::LocalNodeId<dir::Expression>>,
}

impl CallableReturnUsage {
    /// Return true when the callable returns any value.
    pub fn returns_value(&self) -> bool {
        self.has_return_value || self.has_expression_body_return_value
    }
}

/// Analyze return value usage for one callable body.
pub fn callable_return_usage(
    tree: &dir::Tree,
    signature: &dir::FunctionSignature,
    body_expression_id: Option<dir::LocalNodeId<dir::Expression>>,
) -> CallableReturnUsage {
    let Some(body_expression_id) = body_expression_id else {
        return CallableReturnUsage::default();
    };

    // treat concise lambda bodies as implicit value returns
    let body_expression = tree.get(body_expression_id);
    if signature.kind == dir::FunctionKind::Lambda
        && !matches!(body_expression, dir::Expression::Block(..))
    {
        return CallableReturnUsage {
            has_return_value: false,
            has_expression_body_return_value: true,
            return_value_nodes: Vec::new(),
        };
    }

    // walk explicit returns inside the callable body
    let mut visitor = ReturnValueVisitor::new(body_expression_id);
    visitor.visit_expression(tree, body_expression_id, body_expression);
    CallableReturnUsage {
        has_return_value: visitor.has_return_value,
        has_expression_body_return_value: false,
        return_value_nodes: visitor.return_value_nodes,
    }
}

/// Collect explicit `return value` nodes while skipping nested callable scopes.
struct ReturnValueVisitor {
    /// The callable body root expression.
    root_expression_id: dir::LocalNodeId<dir::Expression>,
    /// Whether the current callable contains any explicit `return value`.
    has_return_value: bool,
    /// Explicit `return value` nodes found in the current callable.
    return_value_nodes: Vec<dir::LocalNodeId<dir::Expression>>,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl ReturnValueVisitor {
    /// Build a visitor for one callable body.
    fn new(root_expression_id: dir::LocalNodeId<dir::Expression>) -> Self {
        Self {
            root_expression_id,
            has_return_value: false,
            return_value_nodes: Vec::new(),
            options: NodeVisitorOptions::default(),
        }
    }
}

impl NodeVisitor for ReturnValueVisitor {
    /// Return visitor options.
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    /// Visit one expression in the callable body.
    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // record explicit returns with values
        if let dir::Expression::Return { value: Some(_) } = expression {
            self.has_return_value = true;
            self.return_value_nodes.push(id);
            return;
        }

        // keep nested callable scopes out of the current analysis
        if id != self.root_expression_id
            && expression_enters_nested_declaration_scope(tree, expression)
        {
            return;
        }

        // walk the current expression subtree
        walk_expression(self, tree, id, expression);
    }
}
