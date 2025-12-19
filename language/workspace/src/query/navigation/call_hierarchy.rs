use std::collections::HashMap;

use destack_dir::{
    self as dir, Expression, GlobalSymbolId, LocalNodeId, NodeTree, NodeVisitor,
    NodeVisitorOptions, SymbolType, walk_expression,
};
use destack_source::{FileId, Span};

use crate::Session;
use crate::query::common::{
    find_symbol_at_offset, get_canonical_symbol, get_dir_node_span, get_symbol_definition_span,
};

/// An item in the call hierarchy.
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CallHierarchyKind {
    Function,
    Method,
    Constructor,
}

/// An incoming call (who calls this function).
#[derive(Debug, Clone)]
pub struct CallHierarchyIncomingCall {
    /// The item that contains the call sites.
    pub from: CallHierarchyItem,
    /// The ranges of the actual call expressions within `from`.
    pub from_ranges: Vec<Span>,
}

/// An outgoing call (what does this function call).
#[derive(Debug, Clone)]
pub struct CallHierarchyOutgoingCall {
    /// The item being called.
    pub to: CallHierarchyItem,
    /// The ranges of the call expressions to `to`.
    pub from_ranges: Vec<Span>,
}

// NOTE #Incomplete: call hierarchy query should also check Resolutions

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
    let module = module.read();
    let (Some(ast), Some(dir)) = (&module.ast, &module.dir) else {
        return None;
    };
    let symbols = dir.symbols.read();
    let symbol = symbols.get_symbol(canonical_id.local_id);

    // check if it's a function
    if symbol.ty != SymbolType::Function {
        return None;
    }

    // get the name
    let name = symbol.name().map(|id| ast.strings.get(id).to_string())?;

    drop(symbols);
    drop(module);

    // get the definition span
    let selection_range = get_symbol_definition_span(session, canonical_id)?;

    // for the full range, we'd ideally get the function's body range too
    // for now, use selection_range as both
    let range = selection_range;

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
        let module = module.read();
        let (Some(ast), Some(dir)) = (&module.ast, &module.dir) else {
            continue;
        };
        let module_id = module.id;
        let dir_tree = dir.tree.read();

        // collect call sites and their containing functions
        let mut call_sites_by_function: HashMap<GlobalSymbolId, Vec<Span>> = HashMap::new();
        for (expr_id, expr) in dir_tree.iter_nodes_of_type::<Expression>() {
            // check if this is a reference to our target function
            let Some(target) = expr.target_symbol() else {
                continue;
            };
            let target_canonical = get_canonical_symbol(session, target);
            if target_canonical != canonical_id {
                continue;
            }

            // get the span of this reference
            let Some(call_span) = get_dir_node_span(ast, dir, expr_id.into()) else {
                continue;
            };

            // find the containing function for this call site
            if let Some(containing_fn) =
                find_containing_function(&dir_tree, module_id, expr_id.into())
            {
                call_sites_by_function
                    .entry(containing_fn)
                    .or_default()
                    .push(call_span);
            }
        }

        drop(dir_tree);
        drop(module);

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
    let module = module.read();
    let (Some(ast), Some(dir)) = (&module.ast, &module.dir) else {
        return Vec::new();
    };
    let dir_tree = dir.tree.read();
    let symbols = dir.symbols.read();

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
    collector.visit_expression(&dir_tree, *body_id, body_expr);

    // get spans for collected calls
    let mut calls_with_spans: HashMap<GlobalSymbolId, Vec<Span>> = HashMap::new();
    for (target_id, expr_id) in collector.calls {
        if let Some(span) = get_dir_node_span(ast, dir, expr_id.into()) {
            calls_with_spans.entry(target_id).or_default().push(span);
        }
    }

    drop(symbols);
    drop(dir_tree);
    drop(module);

    // convert to outgoing calls
    let mut outgoing = Vec::new();
    for (target_symbol_id, call_spans) in calls_with_spans {
        // only include function calls
        let target_module = session.modules.get(target_symbol_id.module_id);
        let target = target_module.read();
        let Some(target_dir) = &target.dir else {
            continue;
        };
        let target_symbols = target_dir.symbols.read();
        let target_symbol = target_symbols.get_symbol(target_symbol_id.local_id);

        if target_symbol.ty != SymbolType::Function {
            continue;
        }

        drop(target_symbols);
        drop(target);

        if let Some(target_item) = call_hierarchy_item_from_symbol(session, target_symbol_id) {
            outgoing.push(CallHierarchyOutgoingCall {
                to: target_item,
                from_ranges: call_spans,
            });
        }
    }

    outgoing
}

/// Visitor that collects Call expressions and their targets.
struct CallCollector<'a> {
    session: &'a Session,
    calls: Vec<(GlobalSymbolId, LocalNodeId<Expression>)>,
    options: NodeVisitorOptions,
}

impl<'a> CallCollector<'a> {
    fn new(session: &'a Session) -> Self {
        Self {
            session,
            calls: Vec::new(),
            options: NodeVisitorOptions::default(),
        }
    }
}

impl NodeVisitor for CallCollector<'_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

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
pub fn call_hierarchy_item_from_symbol(
    session: &Session,
    symbol_id: GlobalSymbolId,
) -> Option<CallHierarchyItem> {
    let module = session.modules.get(symbol_id.module_id);
    let module = module.read();
    let (Some(ast), Some(dir)) = (&module.ast, &module.dir) else {
        return None;
    };
    let symbols = dir.symbols.read();
    let symbol = symbols.get_symbol(symbol_id.local_id);

    if symbol.ty != SymbolType::Function {
        return None;
    }

    let name = symbol.name().map(|id| ast.strings.get(id).to_string())?;

    drop(symbols);
    drop(module);

    let selection_range = get_symbol_definition_span(session, symbol_id)?;

    Some(CallHierarchyItem {
        name,
        kind: CallHierarchyKind::Function,
        detail: None,
        file: selection_range.file,
        range: selection_range,
        selection_range,
        symbol_id,
    })
}
