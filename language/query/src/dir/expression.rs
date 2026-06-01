use destack_dir as dir;

use crate::core::DirQueryContext;

impl DirQueryContext<'_> {
    /// Return the recorded symbol target for one expression.
    pub(crate) fn expression_symbol_target(
        self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::GlobalSymbolId> {
        let view = self.view();
        let expression = view.get::<dir::Expression>(expression_id);

        match expression {
            // preserve the underlying symbol through wrappers
            dir::Expression::Parenthesized { expression } => {
                self.expression_symbol_target(*expression)
            }
            _ => {
                let node_id = expression_id.into_global_any(self.module_id());
                let symbol_id = self.resolutions().symbol_resolution(node_id)?;
                if !self.symbol_is_visible(symbol_id) {
                    return None;
                }

                Some(symbol_id)
            }
        }
    }

    /// Return the recorded symbol target for one dependency item.
    pub(crate) fn dependency_symbol_target(
        self,
        item_id: dir::LocalNodeId<dir::DependencyItem>,
    ) -> Option<dir::GlobalSymbolId> {
        let node_id = item_id.into_global_any(self.module_id());
        let symbol_id = self.resolutions().symbol_resolution(node_id)?;

        if !self.symbol_is_visible(symbol_id) {
            return None;
        }

        Some(symbol_id)
    }

    /// Return the local symbol introduced by one dependency item.
    pub(crate) fn dependency_local_symbol(
        self,
        item_id: dir::LocalNodeId<dir::DependencyItem>,
    ) -> Option<dir::GlobalSymbolId> {
        self.global_symbol_for_node(item_id.into())
    }

    /// Check whether an expression is used in a type position.
    pub(crate) fn expression_is_type_position(
        self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        let ctx = self;
        let mut current_id = ctx.view().get_source(expression_id);

        loop {
            let Some(parent_id) = ctx.parents().get_by_id(current_id) else {
                return false;
            };

            match ctx.tree().get_node_type(parent_id) {
                dir::NodeType::TypeExpression => {
                    return true;
                }
                dir::NodeType::GenericArgument => {
                    current_id = parent_id;
                }
                dir::NodeType::Expression => {
                    current_id = parent_id;
                }
                dir::NodeType::Declarator => {
                    let declarator = ctx
                        .tree()
                        .get(dir::LocalNodeId::<dir::Declarator>::new(parent_id));
                    return declarator.ty.is_some_and(|ty| ty.id == current_id);
                }
                dir::NodeType::Parameter => {
                    let parameter = ctx
                        .tree()
                        .get(dir::LocalNodeId::<dir::Parameter>::new(parent_id));
                    return match parameter {
                        dir::Parameter::Named { declared_type, .. }
                        | dir::Parameter::Pattern { declared_type, .. }
                        | dir::Parameter::VariadicNamed { declared_type, .. }
                        | dir::Parameter::VariadicPattern { declared_type, .. } => {
                            declared_type.is_some_and(|ty| ty.id == current_id)
                        }
                        dir::Parameter::Error => false,
                    };
                }
                dir::NodeType::Member => {
                    let member = ctx
                        .tree()
                        .get(dir::LocalNodeId::<dir::Member>::new(parent_id));
                    return match member {
                        dir::Member::AssociatedType { .. } => false,
                        dir::Member::AssociatedConst { .. } => false,
                        dir::Member::Field { .. } => false,
                        _ => false,
                    };
                }
                dir::NodeType::Declaration => {
                    let declaration = ctx
                        .tree()
                        .get(dir::LocalNodeId::<dir::Declaration>::new(parent_id));
                    return match declaration {
                        dir::Declaration::Type(declaration) => declaration.value.id == current_id,
                        _ => false,
                    };
                }
                _ => return false,
            }
        }
    }
}
