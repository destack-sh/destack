use destack_dir as dir;
use dir::NodeVisitor as _;

use super::super::state::BindState;

use crate::Compiler;

impl Compiler {
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
}
