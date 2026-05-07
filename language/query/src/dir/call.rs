use destack_dir::{
    self as dir, Expression, GlobalNodeIdAny, GlobalSymbolId, LocalNodeId, Resolution, SymbolType,
};
use destack_source::{ModuleId, ProfileId};
use destack_workspace::{Repository, Revision};

use crate::ast::get_node_tree_span;
use crate::core::{
    CallEntry, DirQueryContext, QueryContext, query_context, query_context_for_profile,
};

use super::{
    get_canonical_symbol, resolve_expression_symbol, resolve_member_access_symbol,
    resolve_symbol_name,
};
/// Information about a resolved call target.
#[derive(Debug, Clone)]
pub(crate) struct CallTarget {
    /// The resolved name of the target.
    pub name: Option<String>,
    /// The resolved symbol id of the target.
    pub symbol: Option<GlobalSymbolId>,
}

impl CallTarget {
    /// Create a call target with the given name and symbol.
    pub(crate) fn new(name: Option<String>, symbol: Option<GlobalSymbolId>) -> Self {
        Self { name, symbol }
    }
}

/// Resolve the call target name and symbol for a call expression.
pub(crate) fn resolve_call_target(
    repository: &Repository,
    dir: DirQueryContext<'_>,
    left_expression_id: LocalNodeId<Expression>,
) -> CallTarget {
    // resolve the left expression node
    let dir_tree = dir.tree();
    let left_expression = dir_tree.get::<Expression>(left_expression_id);

    // resolve the target name and symbol based on expression kind
    match left_expression {
        Expression::GlobalReference {
            target_symbol,
            path,
            ..
        }
        | Expression::LocalReference {
            target_symbol,
            path,
            ..
        }
        | Expression::ModuleReference {
            target_symbol,
            path,
            ..
        } => {
            // resolve the referenced symbol and name
            let symbol = *target_symbol;
            let name = resolve_symbol_name(repository, dir.revision(), symbol).or_else(|| {
                path.last_segment()
                    .map(|name_id| dir.strings().get(name_id).to_string())
            });

            // prefer the canonical function symbol when possible
            let canonical_symbol = get_canonical_symbol(repository, dir.revision(), symbol);
            let resolved_symbol = symbol_is_function(repository, dir.revision(), canonical_symbol)
                .then_some(canonical_symbol)
                .or_else(|| {
                    symbol_is_function(repository, dir.revision(), symbol).then_some(symbol)
                });

            CallTarget::new(name, resolved_symbol)
        }
        Expression::Member { name, .. } => {
            let Some(name) = *name else {
                return CallTarget::new(None, None);
            };

            // resolve the member name string
            let member_name = dir.strings().get(name).to_string();

            // resolve the member symbol when possible
            let member_symbol = resolve_member_access_symbol(dir, left_expression_id);

            // return the member name and symbol
            CallTarget::new(Some(member_name), member_symbol)
        }
        Expression::UnresolvedPath { path, .. } => {
            // resolve the unresolved path name
            let name = path
                .last_segment()
                .map(|name_id| dir.strings().get(name_id).to_string());
            CallTarget::new(name, None)
        }
        _ => CallTarget::new(None, None),
    }
}

/// Check whether a symbol id refers to a function declaration.
fn symbol_is_function(
    repository: &Repository,
    revision: Revision,
    symbol_id: GlobalSymbolId,
) -> bool {
    let Some(ctx) = query_context(repository, revision, symbol_id.module_id) else {
        return false;
    };

    let symbols = ctx.dir().symbols();
    let symbol = symbols.get_symbol(symbol_id.local_id);
    symbol.ty == SymbolType::Function
}

/// Build call index entries for one module.
pub(crate) fn build_call_candidates_for_module(
    repository: &Repository,
    revision: Revision,
    module_id: ModuleId,
    profile_id: ProfileId,
) -> Vec<CallEntry> {
    let Some(ctx) = query_context_for_profile(repository, revision, module_id, profile_id) else {
        return Vec::new();
    };

    let mut entries = Vec::new();
    let dir_tree = ctx.dir().tree();

    for (expression_id, expression) in dir_tree.iter_nodes_of_type::<Expression>() {
        let left_expression = match expression {
            Expression::Call { left, .. } | Expression::New { left, .. } => *left,
            _ => continue,
        };

        let call_span = get_node_tree_span(ctx.ast(), ctx.dir().tree(), expression_id.into());
        let caller_symbol = find_containing_function_symbol(&ctx, expression_id.into());
        let callee_symbols = call_target_symbols(repository, &ctx, expression_id, left_expression);
        for callee_symbol in callee_symbols {
            entries.push(CallEntry {
                module_id,
                caller_symbol,
                callee_symbol,
                span: call_span,
            });
        }
    }

    entries
}

/// Resolve the canonical function symbols targeted by one call.
fn call_target_symbols(
    repository: &Repository,
    ctx: &QueryContext,
    expression_id: LocalNodeId<Expression>,
    left_expression_id: LocalNodeId<Expression>,
) -> Vec<GlobalSymbolId> {
    let mut targets = Vec::new();

    if let Some(target_symbol) = resolve_expression_symbol(ctx.dir(), left_expression_id) {
        targets.push(target_symbol);
        targets.push(get_canonical_symbol(
            repository,
            ctx.revision(),
            target_symbol,
        ));
    }

    let node_id = GlobalNodeIdAny {
        module_id: ctx.module_id(),
        local_id: expression_id.into(),
    };
    let Some(resolution_id) = ctx.dir().types().node_resolution_id(node_id) else {
        return targets;
    };

    let resolution = ctx.dir().types().get_resolution(resolution_id);
    let candidates = match resolution {
        Resolution::Static { candidate, .. } => std::slice::from_ref(candidate),
        Resolution::Dynamic { candidates, .. } => candidates.as_slice(),
        _ => return targets,
    };

    for candidate in candidates {
        targets.push(candidate.target_symbol);
        targets.push(get_canonical_symbol(
            repository,
            ctx.revision(),
            candidate.target_symbol,
        ));
    }

    targets.sort();
    targets.dedup();
    targets
}

/// Find the containing function symbol for one node.
fn find_containing_function_symbol(
    ctx: &QueryContext,
    node_id: dir::LocalNodeIdAny,
) -> Option<GlobalSymbolId> {
    let mut current = Some(node_id);

    while let Some(node_id) = current {
        if node_id.ty == dir::NodeType::Declaration {
            let declaration_id: dir::LocalNodeId<dir::Declaration> = node_id.try_into().ok()?;
            let declaration = ctx.dir().tree().get::<dir::Declaration>(declaration_id);
            if matches!(declaration, dir::Declaration::Function { .. }) {
                return Some(GlobalSymbolId::new(ctx.module_id(), declaration.symbol()));
            }
        }

        current = ctx.dir().tree().get_parent(node_id.id);
    }

    None
}
