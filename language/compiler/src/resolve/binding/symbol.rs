use destack_dir::{
    Expression, GenericArgument, GlobalNodeIdAny, LocalNodeId, LocalScopeId, LocalScopeMark,
    LocalSymbolId, NodeType, Path, ProvenanceReason, Scope, ScopeKind, StaticKey, StringId,
    SymbolSpace, SymbolTable, SymbolType, Tree,
};
use destack_workspace::workspace::{Module, ProfileId};

use crate::{Compiler, ResolveError, ResolveResult};

impl Compiler {
    pub(crate) fn build_member_chain(
        &self,
        expression_id: LocalNodeId<Expression>,
        root_expr: Expression,
        remaining_path: &Path,
        generic_arguments: Option<Vec<LocalNodeId<GenericArgument>>>,
        tree: &mut Tree,
    ) -> Expression {
        let original_scope = tree.get_scope(expression_id);

        // create a new node for the root expression
        let root_node_id = tree.reserve_from(
            NodeType::Expression,
            expression_id.into_any(),
            original_scope,
            Some(expression_id.into_any()),
            Some(ProvenanceReason::Resolved),
        );
        tree.insert(root_node_id, root_expr);
        let mut current_id: LocalNodeId<Expression> = LocalNodeId::new(root_node_id.id);

        // create Member chain for remaining segments
        let segments = &remaining_path.segments;
        for (i, &segment) in segments.iter().enumerate() {
            let is_last = i == segments.len() - 1;
            let member_expression = Expression::Member {
                left: current_id,
                name: Some(segment),
            };

            // return the final member expression
            if is_last {
                if let Some(generic_arguments) = generic_arguments.clone() {
                    let member_id = tree.reserve_from(
                        NodeType::Expression,
                        expression_id.into_any(),
                        original_scope,
                        Some(expression_id.into_any()),
                        Some(ProvenanceReason::Resolved),
                    );
                    let member_id = tree.insert_as_owner(member_id, member_expression);

                    return Expression::Instantiation {
                        left: member_id,
                        generic_arguments,
                    };
                }

                return member_expression;
            }
            // create intermediate member node
            else {
                let new_node_id = tree.reserve_from(
                    NodeType::Expression,
                    expression_id.into_any(),
                    original_scope,
                    Some(expression_id.into_any()),
                    Some(ProvenanceReason::Resolved),
                );
                tree.insert(new_node_id, member_expression);
                current_id = LocalNodeId::new(new_node_id.id);
            }
        }

        unreachable!("remaining_path is not empty")
    }

    /// Find the module binding scope for a global augmentation expression.
    pub(crate) fn resolve_symbol_to_expression(
        &self,
        module: &Module,
        symbol_id: LocalSymbolId,
        path: &Path,
        generic_arguments: Option<Vec<LocalNodeId<GenericArgument>>>,
        symbols: &SymbolTable,
    ) -> Expression {
        let symbol = symbols.get_symbol(symbol_id);
        let scope = symbols.get_scope_by_id(symbol.scope.0);
        let global_id = symbol_id.into_global(module.id);
        if scope.kind == ScopeKind::Block {
            Expression::LocalReference {
                path: path.clone(),
                generic_arguments: generic_arguments.unwrap_or_default(),
                target_symbol: global_id,
            }
        } else {
            Expression::ModuleReference {
                path: path.clone(),
                generic_arguments: generic_arguments.unwrap_or_default(),
                target_symbol: global_id,
            }
        }
    }

    /// Resolve a label symbol by name, walking up scopes.
    /// Labels are in the Label symbol space and can only be found within the same module.
    pub(crate) fn resolve_label_symbol(
        &self,
        _module: &Module,
        profile_id: ProfileId,
        node: GlobalNodeIdAny,
        scope: (LocalScopeId, &Scope, LocalScopeMark),
        label: StringId,
        symbols: &SymbolTable,
    ) -> ResolveResult<LocalSymbolId> {
        let key = StaticKey::Name(label);
        let mut scope = scope;
        loop {
            // search for label symbol in current scope
            for (candidate_key, symbol_id) in symbols.active_named_symbols(scope.1) {
                if candidate_key == key {
                    let symbol = symbols.get_symbol(symbol_id);
                    if symbol.space == SymbolSpace::Label {
                        return Ok(symbol_id);
                    }
                }
            }

            // labels cannot cross function boundaries
            if let Some(owner_id) = scope.1.owner_id {
                let owner_symbol = symbols.get_symbol(owner_id);
                if owner_symbol.ty == SymbolType::Function {
                    break;
                }
            }

            // go to parent scope
            if let Some((parent_scope_id, parent_mark)) = scope.1.parent {
                scope = (
                    parent_scope_id,
                    symbols.get_scope_by_id(parent_scope_id),
                    parent_mark,
                );
            }
            // no more scopes
            else {
                break;
            }
        }

        Err(ResolveError::MissingTarget {
            node: node.into_anchored(Some(profile_id)),
            target: Some(label),
        })
    }
}
