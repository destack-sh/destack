use destack_dir as dir;
use destack_dir::{
    Expression, GlobalNodeIdAny, GlobalSymbolId, LocalNodeId, Resolution, SymbolForm,
};
use destack_workspace::{Repository, Revision};

use crate::core::{CallEntry, DirQueryContext, QueryContext, query_context};
use crate::source::get_node_tree_span;

use super::{
    expression_symbol_target, get_canonical_symbol, member_access_symbol_target,
    resolve_symbol_name,
};
/// Information about a call target.
#[derive(Debug, Clone)]
pub(crate) struct CallTarget {
    /// The name of the target.
    pub name: Option<String>,
    /// The symbol id of the target.
    pub symbol: Option<GlobalSymbolId>,
}

impl CallTarget {
    /// Create a call target with the given name and symbol.
    pub(crate) fn new(name: Option<String>, symbol: Option<GlobalSymbolId>) -> Self {
        Self { name, symbol }
    }
}

/// Return the call target name and symbol for a call expression.
pub(crate) fn call_target(
    repository: &Repository,
    dir: DirQueryContext<'_>,
    left_expression_id: LocalNodeId<Expression>,
) -> CallTarget {
    // read the left expression node
    let dir_tree = dir.view();
    let left_expression = dir_tree.get::<Expression>(left_expression_id);

    // inspect the target expression shape
    match left_expression {
        Expression::QualifiedReference { path, .. } => {
            // read the referenced symbol and name
            let symbol = expression_symbol_target(dir, left_expression_id);
            let name = symbol
                .and_then(|symbol| resolve_symbol_name(repository, dir.revision(), symbol))
                .or_else(|| {
                    path.last_segment()
                        .map(|name_id| dir.strings().get(name_id).to_string())
                });

            // prefer the canonical function symbol when possible
            let function_symbol = symbol.and_then(|symbol| {
                let canonical_symbol = get_canonical_symbol(repository, dir.revision(), symbol);

                symbol_is_function(repository, dir.revision(), canonical_symbol)
                    .then_some(canonical_symbol)
                    .or_else(|| {
                        symbol_is_function(repository, dir.revision(), symbol).then_some(symbol)
                    })
            });

            CallTarget::new(name, function_symbol)
        }
        Expression::Member { name, .. } => {
            let Some(name) = *name else {
                return CallTarget::new(None, None);
            };

            // read the member name string
            let member_name = dir.strings().get(name).to_string();

            // read the member symbol when possible
            let member_symbol = member_access_symbol_target(dir, left_expression_id);

            CallTarget::new(Some(member_name), member_symbol)
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
    symbol.form == SymbolForm::Function
}

/// Build call index entries for one module.
pub(crate) fn build_call_candidates_for_module(
    repository: &Repository,
    ctx: &QueryContext<'_>,
) -> Vec<CallEntry> {
    let mut entries = Vec::new();
    let dir_tree = ctx.dir().view();
    let module_id = ctx.module_id();

    for (expression_id, expression) in dir_tree.iter_nodes_of_type::<Expression>() {
        let left_expression = match expression {
            Expression::Call { left, .. } | Expression::New { left, .. } => *left,
            _ => continue,
        };

        let call_span = get_node_tree_span(ctx.source(), ctx.dir().view(), expression_id.into());
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

/// Return the canonical function symbols targeted by one call.
fn call_target_symbols(
    repository: &Repository,
    ctx: &QueryContext<'_>,
    expression_id: LocalNodeId<Expression>,
    left_expression_id: LocalNodeId<Expression>,
) -> Vec<GlobalSymbolId> {
    let mut targets = Vec::new();

    if let Some(target_symbol) = expression_symbol_target(ctx.dir(), left_expression_id) {
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
    let Some(resolution) = ctx.dir().types().resolution(node_id) else {
        return targets;
    };

    let candidates = match resolution {
        Resolution::Dispatch(dir::DispatchResolution::Static { target, .. }) => {
            std::slice::from_ref(target)
        }
        Resolution::Dispatch(dir::DispatchResolution::Dynamic { targets, .. }) => {
            targets.as_slice()
        }
        _ => return targets,
    };

    for candidate in candidates {
        targets.push(candidate.symbol);
        targets.push(get_canonical_symbol(
            repository,
            ctx.revision(),
            candidate.symbol,
        ));
    }

    targets.sort();
    targets.dedup();
    targets
}

/// Find the containing function symbol for one node.
fn find_containing_function_symbol(
    ctx: &QueryContext<'_>,
    node_id: dir::LocalNodeIdAny,
) -> Option<GlobalSymbolId> {
    let mut current = Some(node_id);

    while let Some(node_id) = current {
        if node_id.ty == dir::NodeType::Declaration {
            let declaration_id: dir::LocalNodeId<dir::Declaration> = node_id.try_into().ok()?;
            let declaration = ctx.dir().view().get::<dir::Declaration>(declaration_id);
            if matches!(declaration, dir::Declaration::Function(_)) {
                return ctx
                    .dir()
                    .symbol_for_node(declaration_id.into())
                    .map(|symbol_id| GlobalSymbolId::new(ctx.module_id(), symbol_id));
            }
        }

        current = ctx.dir().view().get_parent_any(node_id);
    }

    None
}
