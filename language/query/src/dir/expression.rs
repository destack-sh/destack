use destack_dir::{Expression, GlobalSymbolId};
use {destack_ast as ast, destack_dir as dir};

use crate::core::{AstQuery, DirQuery};

/// Resolve the best symbol for an expression from DIR data.
pub(crate) fn resolve_expression_symbol(
    dir: DirQuery<'_>,
    expression_id: dir::LocalNodeId<Expression>,
) -> Option<GlobalSymbolId> {
    let dir_tree = dir.tree();
    let expression = dir_tree.get::<Expression>(expression_id);

    match expression {
        // preserve the underlying symbol through wrappers
        Expression::Parenthesized { expression } => resolve_expression_symbol(dir, *expression),
        _ => expression.target_symbol(),
    }
}

/// Check whether an expression is used in a type position.
pub(crate) fn expression_is_type_position(
    ast: AstQuery<'_>,
    dir: DirQuery<'_>,
    expression_id: dir::LocalNodeId<Expression>,
) -> bool {
    let mut current_id = dir.tree().get_source(expression_id.id);

    loop {
        let Some(parent_id) = ast.parents().get_by_id(current_id) else {
            return false;
        };

        match ast.tree().get_node_type(parent_id) {
            ast::NodeType::Expression => {
                current_id = parent_id;
            }
            ast::NodeType::Declarator => {
                let declarator = ast
                    .tree()
                    .get(ast::LocalNodeId::<ast::Declarator>::new(parent_id));
                return declarator.ty.is_some_and(|ty| ty.id == current_id);
            }
            ast::NodeType::Parameter => {
                let parameter = ast
                    .tree()
                    .get(ast::LocalNodeId::<ast::Parameter>::new(parent_id));
                return match parameter {
                    ast::Parameter::Named { declared_type, .. }
                    | ast::Parameter::Pattern { declared_type, .. }
                    | ast::Parameter::VariadicNamed { declared_type, .. }
                    | ast::Parameter::VariadicPattern { declared_type, .. } => {
                        declared_type.is_some_and(|ty| ty.id == current_id)
                    }
                    ast::Parameter::Error => false,
                };
            }
            ast::NodeType::Member => {
                let member = ast
                    .tree()
                    .get(ast::LocalNodeId::<ast::Member>::new(parent_id));
                return match member {
                    ast::Member::Type {
                        declared_type,
                        value,
                        ..
                    } => {
                        declared_type.is_some_and(|ty| ty.id == current_id)
                            || value.is_some_and(|value| value.id == current_id)
                    }
                    ast::Member::ComptimeConst { declared_type, .. } => {
                        declared_type.is_some_and(|ty| ty.id == current_id)
                    }
                    ast::Member::Field { default, .. } => {
                        default.is_some_and(|value| value.id == current_id)
                    }
                    ast::Member::Embed { .. } => false,
                    _ => false,
                };
            }
            ast::NodeType::Declaration => {
                let declaration = ast
                    .tree()
                    .get(ast::LocalNodeId::<ast::Declaration>::new(parent_id));
                return match declaration {
                    ast::Declaration::Type(declaration) => declaration.value.id == current_id,
                    _ => false,
                };
            }
            _ => return false,
        }
    }
}
