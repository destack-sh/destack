use dir::NodeVisitor as _;
use tspp_dir as dir;

use super::super::state::BindState;

use crate::Compiler;

impl Compiler {
    /// Bind one switch case.
    pub(in crate::bind) fn bind_switch_case(
        &self,
        state: &mut BindState<'_>,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::SwitchCase>,
        case: &dir::SwitchCase,
    ) {
        state.bind_node(id.into_any());

        // bind the selector in the surrounding switch scope
        if let dir::SwitchSelector::Case(value) = case.selector {
            let expression = tree.get(value);
            state.visit_expression(tree, value, expression);
        }

        // bind the body in its block scope
        let body = tree.get(case.body);
        state.visit_block(tree, case.body, body);
    }
}
