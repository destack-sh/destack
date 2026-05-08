use destack_dir::{Expression, GlobalSymbolId};
use {destack_ast as ast, destack_dir as dir};

use crate::core::{AstQueryContext, DirQueryContext};

/// Return the recorded symbol target for one expression.
pub(crate) fn expression_symbol_target(
    dir: DirQueryContext<'_>,
    expression_id: dir::LocalNodeId<Expression>,
) -> Option<GlobalSymbolId> {
    let view = dir.view();
    let expression = view.get::<Expression>(expression_id);

    match expression {
        // preserve the underlying symbol through wrappers
        Expression::Parenthesized { expression } => expression_symbol_target(dir, *expression),
        _ => {
            let node_id = expression_id.into_global_any(dir.module_id());
            let symbol_id = dir.types().symbol_resolution(node_id)?;
            if !dir.symbol_is_active(symbol_id) {
                return None;
            }

            Some(symbol_id)
        }
    }
}

/// Return the recorded symbol target for one dependency item.
pub(crate) fn dependency_symbol_target(
    dir: DirQueryContext<'_>,
    item_id: dir::LocalNodeId<dir::DependencyItem>,
) -> Option<GlobalSymbolId> {
    let node_id = item_id.into_global_any(dir.module_id());
    let resolution = dir.types().dependency_resolution(node_id)?;

    match resolution {
        dir::DependencyResolution::Symbol(symbol_id) => {
            if !dir.symbol_is_active(*symbol_id) {
                return None;
            }

            Some(*symbol_id)
        }
        dir::DependencyResolution::Module(_) => None,
    }
}

/// Return the local symbol introduced by one dependency item.
pub(crate) fn dependency_local_symbol(
    dir: DirQueryContext<'_>,
    item: &dir::DependencyItem,
) -> Option<GlobalSymbolId> {
    item.symbol()
        .map(|symbol_id| GlobalSymbolId::new(dir.module_id(), symbol_id))
}

/// Check whether an expression is used in a type position.
pub(crate) fn expression_is_type_position(
    ast: AstQueryContext<'_>,
    dir: DirQueryContext<'_>,
    expression_id: dir::LocalNodeId<Expression>,
) -> bool {
    let mut current_id = dir.view().get_source(expression_id);

    loop {
        let Some(parent_id) = ast.parents().get_by_id(current_id) else {
            return false;
        };

        match ast.tree().get_node_type(parent_id) {
            ast::NodeType::TypeExpression => {
                return true;
            }
            ast::NodeType::GenericArgument => {
                current_id = parent_id;
            }
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
                    ast::Member::AssociatedType { .. } => false,
                    ast::Member::AssociatedConst { .. } => false,
                    ast::Member::Field { .. } => false,
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
