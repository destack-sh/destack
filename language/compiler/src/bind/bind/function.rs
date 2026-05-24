use destack_dir as dir;
use dir::NodeVisitor as _;

use super::super::state::{BindState, BindingContext};

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

    /// Bind one generic parameter.
    pub(in crate::bind) fn bind_generic_parameter(
        &self,
        state: &mut BindState<'_>,
        tree: &dir::Tree,
        node_id: dir::LocalNodeId<dir::GenericParameter>,
        parameter: &dir::GenericParameter,
    ) {
        state.bind_node(node_id.into_any());

        // ignore malformed parameters
        let Some(key) = parameter.symbol_key() else {
            return;
        };
        let Some(form) = parameter.symbol_form() else {
            return;
        };

        // visit generic parameter bounds and defaults before self declaration
        match parameter {
            dir::GenericParameter::Type {
                constraint,
                default,
                ..
            }
            | dir::GenericParameter::VariadicType {
                constraint,
                default,
                ..
            } => {
                // visit type constraint
                if let Some(constraint) = constraint {
                    let constraint_node = tree.get(*constraint);
                    state.visit_type_expression(tree, *constraint, constraint_node);
                }

                // visit type default
                if let Some(default) = default {
                    let default_node = tree.get(*default);
                    state.visit_type_expression(tree, *default, default_node);
                }
            }
            dir::GenericParameter::Value {
                declared_type,
                default,
                ..
            }
            | dir::GenericParameter::VariadicValue {
                declared_type,
                default,
                ..
            } => {
                // visit value parameter type
                if let Some(declared_type) = declared_type {
                    let declared_type_node = tree.get(*declared_type);
                    state.visit_type_expression(tree, *declared_type, declared_type_node);
                }

                // visit value default
                if let Some(default) = default {
                    let default_node = tree.get(*default);
                    state.visit_expression(tree, *default, default_node);
                }
            }
            dir::GenericParameter::Error => {}
        }

        // declare generic parameter symbol
        let symbol_id = state.insert_symbol(dir::SymbolRole::Local, form, Some(key), None);

        state.declare_symbol(symbol_id, node_id);
    }

    /// Bind one callable parameter.
    pub(in crate::bind) fn bind_parameter(
        &self,
        state: &mut BindState<'_>,
        tree: &dir::Tree,
        node_id: dir::LocalNodeId<dir::Parameter>,
        parameter: &dir::Parameter,
    ) {
        state.bind_node(node_id.into_any());

        match parameter {
            dir::Parameter::Named {
                declared_type,
                default,
                ..
            } => {
                // visit named parameter type
                if let Some(declared_type) = declared_type {
                    let declared_type_node = tree.get(*declared_type);
                    state.visit_type_expression(tree, *declared_type, declared_type_node);
                }

                // visit named parameter default
                if let Some(default) = default {
                    let default_node = tree.get(*default);
                    state.visit_expression(tree, *default, default_node);
                }

                // declare named parameter
                self.bind_parameter_symbol(state, node_id, parameter);
            }
            dir::Parameter::VariadicNamed { declared_type, .. } => {
                // visit variadic parameter type
                if let Some(declared_type) = declared_type {
                    let declared_type_node = tree.get(*declared_type);
                    state.visit_type_expression(tree, *declared_type, declared_type_node);
                }

                // declare variadic parameter
                self.bind_parameter_symbol(state, node_id, parameter);
            }
            dir::Parameter::Pattern {
                pattern,
                declared_type,
                default,
                ..
            } => {
                // visit pattern parameter type
                if let Some(declared_type) = declared_type {
                    let declared_type_node = tree.get(*declared_type);
                    state.visit_type_expression(tree, *declared_type, declared_type_node);
                }

                // visit pattern parameter default
                if let Some(default) = default {
                    let default_node = tree.get(*default);
                    state.visit_expression(tree, *default, default_node);
                }

                // bind pattern parameter
                self.bind_parameter_pattern(state, tree, *pattern);
            }
            dir::Parameter::VariadicPattern {
                pattern,
                declared_type,
                ..
            } => {
                // visit variadic pattern type
                if let Some(declared_type) = declared_type {
                    let declared_type_node = tree.get(*declared_type);
                    state.visit_type_expression(tree, *declared_type, declared_type_node);
                }

                // bind variadic pattern
                self.bind_parameter_pattern(state, tree, *pattern);
            }
            dir::Parameter::Error => {}
        }
    }

    /// Bind one callable parameter symbol.
    fn bind_parameter_symbol(
        &self,
        state: &mut BindState<'_>,
        node_id: dir::LocalNodeId<dir::Parameter>,
        parameter: &dir::Parameter,
    ) {
        // ignore malformed parameters
        let Some(key) = parameter.symbol_key() else {
            return;
        };

        // declare parameter symbol
        let symbol_id = state.insert_symbol(
            dir::SymbolRole::Local,
            dir::SymbolForm::Variable,
            Some(key),
            None,
        );

        state.declare_symbol(symbol_id, node_id);
    }

    /// Bind one callable parameter pattern.
    fn bind_parameter_pattern(
        &self,
        state: &mut BindState<'_>,
        tree: &dir::Tree,
        pattern_id: dir::LocalNodeId<dir::Pattern>,
    ) {
        // bind pattern in parameter context
        state.push_binding(BindingContext::default());
        let pattern = tree.get(pattern_id);
        state.visit_pattern(tree, pattern_id, pattern);
        state.pop_binding();
    }
}
