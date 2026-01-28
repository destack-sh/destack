use destack_dir::{Expression, GlobalSymbolId, LocalNodeId};

use crate::Session;
use crate::query::common::{QueryContext, is_identifier_byte, resolve_member_access_symbol};

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
            CallTarget::new(name, Some(symbol))
        }
        Expression::Member { left, name, .. } => {
            // resolve the member name string
            let member_name = ctx.ast.strings.get(*name).to_string();

            // resolve the member symbol when possible
            let member_symbol =
                resolve_member_access_symbol(session, ctx, left_expression_id, *left, *name);

            // return the member name and symbol
            CallTarget::new(Some(member_name), member_symbol)
        }
        _ => {
            // fall back to extracting a simple identifier from source
            let name = fallback_call_target_name(session, ctx, left_expression_id);
            CallTarget::new(name, None)
        }
    }
}

/// Resolve a symbol name string when possible.
fn resolve_symbol_name(session: &Session, symbol_id: GlobalSymbolId) -> Option<String> {
    // read the target module and query context
    let module = session.modules.get(symbol_id.module_id);
    let module = module.read();
    let ctx = session.query_context(&module)?;

    // read the symbol name
    let symbols = ctx.symbols();
    let symbol = symbols.get_symbol(symbol_id.local_id);
    symbol
        .name()
        .map(|name_id| ctx.ast.strings.get(name_id).to_string())
}

/// Resolve a fallback call target name from source text.
fn fallback_call_target_name(
    session: &Session,
    ctx: &QueryContext<'_>,
    left_expression_id: LocalNodeId<Expression>,
) -> Option<String> {
    // resolve the left expression span
    let ast_node_id = ctx.tree().get_source(left_expression_id.id);
    let span = ctx.ast.tree.source_map.get(ast_node_id);

    // read the source text for the span
    let source_file = session.files.get(ctx.file_id);
    let source = source_file.text();
    let slice = source.get(span.start as usize..span.end as usize)?;

    // extract a simple identifier when the slice contains exactly one
    single_identifier_in_text(slice)
}

/// Extract the single identifier from a text slice when it is unambiguous.
fn single_identifier_in_text(text: &str) -> Option<String> {
    // initialize scan state
    let bytes = text.as_bytes();
    let mut found: Option<String> = None;
    let mut index = 0;

    // scan for identifier runs
    while index < bytes.len() {
        // skip non identifier bytes
        while index < bytes.len() && !is_identifier_byte(bytes[index]) {
            index += 1;
        }

        // capture the identifier run
        let start = index;
        while index < bytes.len() && is_identifier_byte(bytes[index]) {
            index += 1;
        }

        if start == index {
            continue;
        }

        let slice = text.get(start..index)?;

        // allow only a single identifier in the slice
        if found.is_some() {
            return None;
        }

        found = Some(slice.to_string());
    }

    found
}
