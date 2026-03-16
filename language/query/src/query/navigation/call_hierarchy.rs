use std::collections::HashMap;

use destack_dir::{
    self as dir, Expression, GlobalNodeIdAny, GlobalSymbolId, LocalNodeId, NodeTree, NodeVisitor,
    NodeVisitorOptions, Resolution, SymbolType, walk_expression,
};
use destack_source::{FileId, Span, Uri};
use serde::{Deserialize, Serialize};

use crate::common::{
    find_symbol_at_offset, get_canonical_symbol, get_dir_node_span, get_symbol_declaration_span,
    get_symbol_definition_span, resolve_symbol_name, sort_and_dedup_spans,
};
use destack_workspace::Session;

/// An item in the call hierarchy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CallHierarchyItem {
    /// The name of the item (function/method name).
    pub name: String,
    /// The kind of item.
    pub kind: CallHierarchyKind,
    /// Detail (e.g., signature).
    pub detail: Option<String>,
    /// The file containing this item.
    pub file: FileId,
    /// The full range of the item.
    pub range: Span,
    /// The range of the item's name.
    pub selection_range: Span,
    /// The symbol ID (internal use for follow-up queries).
    pub symbol_id: GlobalSymbolId,
}

/// Kind of call hierarchy item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CallHierarchyKind {
    Function,
    Method,
    Constructor,
}

/// An incoming call (who calls this function).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CallHierarchyIncomingCall {
    /// The item that contains the call sites.
    pub from: CallHierarchyItem,
    /// The ranges of the actual call expressions within `from`.
    pub from_ranges: Vec<Span>,
}

/// An outgoing call (what does this function call).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CallHierarchyOutgoingCall {
    /// The item being called.
    pub to: CallHierarchyItem,
    /// The ranges of the call expressions to `to`.
    pub from_ranges: Vec<Span>,
}

/// Request prepare call hierarchy at a cursor position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrepareCallHierarchyRequest {
    /// The document URI.
    pub uri: Uri,
    /// The byte offset in the document.
    pub offset: u32,
}

/// Response payload for prepare call hierarchy queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrepareCallHierarchyResponse {
    /// Call hierarchy item, if available.
    pub item: Option<CallHierarchyItem>,
}

/// Request incoming call hierarchy edges.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CallHierarchyIncomingRequest {
    /// The call hierarchy item to expand.
    pub item: CallHierarchyItem,
}

/// Response payload for call hierarchy incoming queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CallHierarchyIncomingResponse {
    /// Incoming calls.
    pub calls: Vec<CallHierarchyIncomingCall>,
}

/// Request outgoing call hierarchy edges.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CallHierarchyOutgoingRequest {
    /// The call hierarchy item to expand.
    pub item: CallHierarchyItem,
}

/// Response payload for call hierarchy outgoing queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CallHierarchyOutgoingResponse {
    /// Outgoing calls.
    pub calls: Vec<CallHierarchyOutgoingCall>,
}

/// Prepare a call hierarchy item at the given position.
///
/// Returns the item if the position is on a callable (function, method).
pub fn prepare_call_hierarchy(
    session: &Session,
    file: FileId,
    offset: u32,
) -> Option<CallHierarchyItem> {
    // find the symbol at offset
    let symbol_at = find_symbol_at_offset(session, file, offset)?;
    let canonical_id = get_canonical_symbol(session, symbol_at.symbol_id);
    let module = session.modules.get(canonical_id.module_id);
    let module = module.as_ref();
    let ctx = crate::query_context(session, &module)?;
    let name = {
        let symbols = ctx.symbols();
        let symbol = symbols.get_symbol(canonical_id.local_id);

        // check if it's a function
        if symbol.ty != SymbolType::Function {
            return None;
        }

        // get the name
        resolve_symbol_name(session, canonical_id)?
    };

    // resolve the selection range at the symbol name
    let selection_range = get_symbol_definition_span(session, canonical_id)?;

    // resolve the full declaration range, fall back to the selection range
    let range = get_symbol_declaration_span(session, canonical_id).unwrap_or(selection_range);

    Some(CallHierarchyItem {
        name,
        kind: CallHierarchyKind::Function,
        detail: None,
        file: selection_range.file,
        range,
        selection_range,
        symbol_id: canonical_id,
    })
}

/// Get incoming calls to a call hierarchy item.
///
/// "Who calls this function?"
pub fn incoming_calls(
    session: &Session,
    item: &CallHierarchyItem,
) -> Vec<CallHierarchyIncomingCall> {
    let canonical_id = get_canonical_symbol(session, item.symbol_id);
    let mut incoming: Vec<CallHierarchyIncomingCall> = Vec::new();

    // find all call expressions that target this function
    for module in session.modules.iter() {
        let module = module.as_ref();
        let Some(ctx) = crate::query_context(session, &module) else {
            continue;
        };
        let module_id = ctx.module_id;
        let call_sites_by_function: HashMap<GlobalSymbolId, Vec<Span>> = {
            let dir_tree = ctx.tree();
            let types = ctx.types();

            // collect call sites and their containing functions
            let mut call_sites_by_function: HashMap<GlobalSymbolId, Vec<Span>> = HashMap::new();

            // check call expressions for direct and resolved targets
            let resolution_matches = |expression_id: LocalNodeId<Expression>| {
                let node_id = GlobalNodeIdAny {
                    module_id,
                    local_id: expression_id.into(),
                };
                let Some(resolution_id) = types.get_resolution_for_node(node_id) else {
                    return false;
                };
                let resolution = types.get_resolution(resolution_id);
                let candidates = match resolution {
                    Resolution::Static { candidate, .. } => std::slice::from_ref(candidate),
                    Resolution::Dynamic { candidates, .. } => candidates.as_slice(),
                    _ => return false,
                };
                candidates.iter().any(|candidate| {
                    get_canonical_symbol(session, candidate.target_symbol) == canonical_id
                })
            };

            for (expr_id, expr) in dir_tree.iter_nodes_of_type::<Expression>() {
                let Expression::Call { left, .. } = expr else {
                    continue;
                };

                let mut matches = false;
                let left_expr = dir_tree.get::<Expression>(*left);
                if let Some(target) = left_expr.target_symbol() {
                    let target_canonical = get_canonical_symbol(session, target);
                    matches = target_canonical == canonical_id;
                }

                if !matches && (resolution_matches(expr_id) || resolution_matches(*left)) {
                    matches = true;
                }

                if !matches {
                    continue;
                }

                let Some(call_span) = get_dir_node_span(&ctx.ast, &ctx.dir, expr_id.into()) else {
                    continue;
                };

                if let Some(containing_fn) =
                    find_containing_function(dir_tree, module_id, expr_id.into())
                {
                    call_sites_by_function
                        .entry(containing_fn)
                        .or_default()
                        .push(call_span);
                }
            }

            call_sites_by_function
        };

        // convert to incoming calls
        for (fn_symbol_id, call_spans) in call_sites_by_function {
            if let Some(fn_item) = call_hierarchy_item_from_symbol(session, fn_symbol_id) {
                incoming.push(CallHierarchyIncomingCall {
                    from: fn_item,
                    from_ranges: call_spans,
                });
            }
        }
    }

    // sort call ranges and incoming callers for stable protocol output
    for call in &mut incoming {
        sort_and_dedup_spans(&mut call.from_ranges);
    }

    incoming.sort_by(|left, right| call_item_key(&left.from).cmp(&call_item_key(&right.from)));

    incoming
}

/// Get outgoing calls from a call hierarchy item.
///
/// "What does this function call?"
pub fn outgoing_calls(
    session: &Session,
    item: &CallHierarchyItem,
) -> Vec<CallHierarchyOutgoingCall> {
    let canonical_id = get_canonical_symbol(session, item.symbol_id);

    // get the module containing this function
    let module = session.modules.get(canonical_id.module_id);
    let module = module.as_ref();
    let Some(ctx) = crate::query_context(session, &module) else {
        return Vec::new();
    };
    let calls_with_spans: HashMap<GlobalSymbolId, Vec<Span>> = {
        let dir_tree = ctx.tree();
        let symbols = ctx.symbols();

        // find the declaration for this symbol
        let symbol = symbols.get_symbol(canonical_id.local_id);
        let Some(primary_decl) = symbol.primary_declaration else {
            return Vec::new();
        };

        // get the function's body expression
        let decl_id: dir::LocalNodeId<dir::Declaration> = match primary_decl.try_into() {
            Ok(id) => id,
            Err(_) => return Vec::new(),
        };
        let decl = dir_tree.get::<dir::Declaration>(decl_id);
        let dir::Declaration::Function { body, .. } = decl else {
            return Vec::new();
        };
        let Some(body_id) = body else {
            return Vec::new();
        };

        // collect all call expressions in the body
        let mut collector = CallCollector::new(session);
        let body_expr = dir_tree.get::<Expression>(*body_id);
        collector.visit_expression(dir_tree, *body_id, body_expr);

        // get spans for collected calls
        let mut calls_with_spans: HashMap<GlobalSymbolId, Vec<Span>> = HashMap::new();
        for (target_id, expr_id) in collector.calls {
            if let Some(span) = get_dir_node_span(&ctx.ast, &ctx.dir, expr_id.into()) {
                calls_with_spans.entry(target_id).or_default().push(span);
            }
        }

        calls_with_spans
    };

    // convert to outgoing calls
    let mut outgoing = Vec::new();
    for (target_symbol_id, call_spans) in calls_with_spans {
        // only include function calls
        let target_module = session.modules.get(target_symbol_id.module_id);
        let Some(target_ctx) = crate::query_context(session, &target_module) else {
            continue;
        };
        let is_function = {
            let target_symbols = target_ctx.symbols();
            let target_symbol = target_symbols.get_symbol(target_symbol_id.local_id);
            target_symbol.ty == SymbolType::Function
        };
        if !is_function {
            continue;
        }

        if let Some(target_item) = call_hierarchy_item_from_symbol(session, target_symbol_id) {
            outgoing.push(CallHierarchyOutgoingCall {
                to: target_item,
                from_ranges: call_spans,
            });
        }
    }

    // sort call ranges and outgoing callees for stable protocol output
    for call in &mut outgoing {
        sort_and_dedup_spans(&mut call.from_ranges);
    }

    outgoing.sort_by(|left, right| call_item_key(&left.to).cmp(&call_item_key(&right.to)));

    outgoing
}

/// Build a stable ordering key for a call hierarchy item.
fn call_item_key(item: &CallHierarchyItem) -> (u32, u32, u32, u32, u32, u8, &str) {
    (
        item.file.0,
        item.range.start,
        item.range.end,
        item.selection_range.start,
        item.selection_range.end,
        call_kind_rank(item.kind),
        item.name.as_str(),
    )
}

/// Rank call hierarchy kinds for stable ordering.
fn call_kind_rank(kind: CallHierarchyKind) -> u8 {
    match kind {
        CallHierarchyKind::Function => 0,
        CallHierarchyKind::Method => 1,
        CallHierarchyKind::Constructor => 2,
    }
}

/// Visitor that collects Call expressions and their targets.
struct CallCollector<'a> {
    session: &'a Session,
    calls: Vec<(GlobalSymbolId, LocalNodeId<Expression>)>,
    options: NodeVisitorOptions,
}

impl<'a> CallCollector<'a> {
    /// Create a call collector for one query pass.
    fn new(session: &'a Session) -> Self {
        Self {
            session,
            calls: Vec::new(),
            options: NodeVisitorOptions::default(),
        }
    }
}

impl NodeVisitor for CallCollector<'_> {
    /// Return node visitor options for call collection.
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    /// Visit expressions and record call targets.
    fn visit_expression(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<Expression>,
        expression: &Expression,
    ) {
        // check if this is a call expression
        if let Expression::Call { left, .. } = expression {
            // the left side of the call might be a reference
            let left_expr = tree.get::<Expression>(*left);
            if let Some(target) = left_expr.target_symbol() {
                let canonical = get_canonical_symbol(self.session, target);
                self.calls.push((canonical, id));
            }
        }

        // continue walking
        walk_expression(self, tree, id, expression);
    }
}

/// Find the containing function for a given expression.
fn find_containing_function(
    dir_tree: &dir::NodeTree,
    module_id: destack_source::ModuleId,
    node_id: dir::LocalNodeIdAny,
) -> Option<GlobalSymbolId> {
    // walk up the parent chain to find a function declaration
    let mut current = Some(node_id);

    while let Some(node) = current {
        if node.ty == dir::NodeType::Declaration {
            let decl_id: dir::LocalNodeId<dir::Declaration> = node.try_into().ok()?;
            let decl = dir_tree.get::<dir::Declaration>(decl_id);
            if matches!(decl, dir::Declaration::Function { .. }) {
                let symbol_id = decl.symbol();
                return Some(GlobalSymbolId {
                    module_id,
                    local_id: symbol_id,
                });
            }
        }
        current = dir_tree.get_parent(node.id);
    }

    None
}

/// Convert a symbol ID to a CallHierarchyItem.
fn call_hierarchy_item_from_symbol(
    session: &Session,
    symbol_id: GlobalSymbolId,
) -> Option<CallHierarchyItem> {
    let canonical_id = get_canonical_symbol(session, symbol_id);
    let module = session.modules.get(canonical_id.module_id);
    let module = module.as_ref();
    let ctx = crate::query_context(session, &module)?;
    let name = {
        let symbols = ctx.symbols();
        let symbol = symbols.get_symbol(canonical_id.local_id);
        if symbol.ty != SymbolType::Function {
            return None;
        }
        resolve_symbol_name(session, canonical_id)?
    };

    // resolve the selection range at the symbol name
    let selection_range = get_symbol_definition_span(session, canonical_id)?;

    // resolve the full declaration range, fall back to the selection range
    let range = get_symbol_declaration_span(session, canonical_id).unwrap_or(selection_range);

    Some(CallHierarchyItem {
        name,
        kind: CallHierarchyKind::Function,
        detail: None,
        file: selection_range.file,
        range,
        selection_range,
        symbol_id: canonical_id,
    })
}
