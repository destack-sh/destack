use destack_dir::{Expression, GlobalSymbolId, LocalNodeId, SymbolType};

use crate::common::{
    QueryContext, get_canonical_symbol, resolve_member_access_symbol, resolve_symbol_name,
};
use destack_workspace::Session;

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
    session: &Session,
    ctx: &QueryContext<'_>,
    left_expression_id: LocalNodeId<Expression>,
) -> CallTarget {
    // resolve the left expression node
    let dir_tree = ctx.tree();
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
            let name = resolve_symbol_name(session, symbol).or_else(|| {
                path.last_segment()
                    .map(|name_id| session.strings.get(name_id).to_string())
            });

            // prefer the canonical function symbol when possible
            let canonical_symbol = get_canonical_symbol(session, symbol);
            let resolved_symbol = symbol_is_function(session, canonical_symbol)
                .then_some(canonical_symbol)
                .or_else(|| symbol_is_function(session, symbol).then_some(symbol));

            CallTarget::new(name, resolved_symbol)
        }
        Expression::Member { name, .. } => {
            let Some(name) = *name else {
                return CallTarget::new(None, None);
            };

            // resolve the member name string
            let member_name = session.strings.get(name).to_string();

            // resolve the member symbol when possible
            let member_symbol = resolve_member_access_symbol(ctx, left_expression_id);

            // return the member name and symbol
            CallTarget::new(Some(member_name), member_symbol)
        }
        Expression::UnresolvedPath { path, .. } => {
            // resolve the unresolved path name
            let name = path
                .last_segment()
                .map(|name_id| session.strings.get(name_id).to_string());
            CallTarget::new(name, None)
        }
        _ => CallTarget::new(None, None),
    }
}

/// Check whether a symbol id refers to a function declaration.
fn symbol_is_function(session: &Session, symbol_id: GlobalSymbolId) -> bool {
    let module = session.modules.get(symbol_id.module_id);
    let module = module.as_ref();
    let Some(ctx) = crate::query_context(session, module) else {
        return false;
    };

    let symbols = ctx.symbols();
    let symbol = symbols.get_symbol(symbol_id.local_id);
    symbol.ty == SymbolType::Function
}
