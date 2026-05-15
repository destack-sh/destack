use destack_dir as dir;
use dir::NodeVisitor as _;

use super::super::state::{BindState, BindingContext};

use crate::Compiler;

impl Compiler {
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
                self.bind_parameter_pattern(state, tree, *pattern, *declared_type);
            }
            dir::Parameter::VariadicPattern {
                pattern,
                declared_type,
            } => {
                // visit variadic pattern type
                if let Some(declared_type) = declared_type {
                    let declared_type_node = tree.get(*declared_type);
                    state.visit_type_expression(tree, *declared_type, declared_type_node);
                }

                // bind variadic pattern
                self.bind_parameter_pattern(state, tree, *pattern, *declared_type);
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

        state.set_declared_type(node_id.into_any(), parameter.declared_type());
        state.declare_symbol(symbol_id, node_id);
    }

    /// Bind one callable parameter pattern.
    fn bind_parameter_pattern(
        &self,
        state: &mut BindState<'_>,
        tree: &dir::Tree,
        pattern_id: dir::LocalNodeId<dir::Pattern>,
        declared_type: Option<dir::LocalNodeId<dir::TypeExpression>>,
    ) {
        let binding = BindingContext {
            export: None,
            mutability: None,
            declared_type,
        };

        // bind pattern in parameter context
        state.push_binding(binding);
        let pattern = tree.get(pattern_id);
        state.visit_pattern(tree, pattern_id, pattern);

        state.pop_binding();
    }
}
