use destack_dir as dir;
use dir::NodeVisitor as _;

use super::super::state::BindState;

use crate::Compiler;

impl Compiler {
    /// Bind generic parameters in order.
    pub(in crate::bind) fn bind_generic_parameters(
        &self,
        state: &mut BindState<'_>,
        tree: &dir::Tree,
        generic_parameters: &[dir::LocalNodeId<dir::GenericParameter>],
    ) {
        // visit generic parameters
        for parameter_id in generic_parameters {
            let parameter = tree.get(*parameter_id);
            state.visit_generic_parameter(tree, *parameter_id, parameter);
        }
    }

    /// Bind where clauses in order.
    pub(in crate::bind) fn bind_where_clauses(
        &self,
        state: &mut BindState<'_>,
        tree: &dir::Tree,
        where_clauses: &[dir::LocalNodeId<dir::WhereClause>],
    ) {
        // visit where clauses
        for clause_id in where_clauses {
            let clause = tree.get(*clause_id);
            state.visit_where_clause(tree, *clause_id, clause);
        }
    }

    /// Bind one function-like signature in the current function or type scope.
    pub(in crate::bind) fn bind_function_signature(
        &self,
        state: &mut BindState<'_>,
        tree: &dir::Tree,
        signature: &dir::FunctionSignature,
    ) {
        // bind static parameters first
        self.bind_generic_parameters(state, tree, &signature.generic_parameters);

        // bind constraints against static parameters
        self.bind_where_clauses(state, tree, &signature.where_clauses);

        // bind runtime parameters in order
        if let Some(this_parameter_id) = signature.this_parameter {
            let this_parameter = tree.get(this_parameter_id);
            state.visit_parameter(tree, this_parameter_id, this_parameter);
        }

        // bind regular parameters
        for parameter_id in &signature.parameters {
            let parameter = tree.get(*parameter_id);
            state.visit_parameter(tree, *parameter_id, parameter);
        }

        // bind return type after parameters
        if let Some(return_type_id) = signature.return_type {
            let return_type = tree.get(return_type_id);
            state.visit_type_expression(tree, return_type_id, return_type);
        }
    }
}
