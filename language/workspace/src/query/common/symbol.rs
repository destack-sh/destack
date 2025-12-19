use destack_dir::{
    Declaration, EnumField, Expression, GlobalSymbolId, LocalNodeIdAny, Member, NodeType,
    Parameter, Pattern,
};
use destack_source::{FileId, Span};

use super::span::{get_dir_node_main_span, get_module_by_file_id};
use crate::Session;

/// Result of finding a symbol at an offset.
#[derive(Debug, Clone)]
pub struct SymbolAtOffset {
    /// The symbol that was referenced.
    pub symbol_id: GlobalSymbolId,
    /// The DIR node that contains the reference.
    pub node_id: LocalNodeIdAny,
    /// The span of the reference.
    pub span: Span,
}

/// Find the symbol referenced at a given offset.
///
/// Returns the GlobalSymbolId of the symbol being referenced at the position.
/// Works for both references (like `foo` in `let x = foo`) and definitions
/// (like `foo` in `const foo = 1` or `function foo() {}`).
pub fn find_symbol_at_offset(
    session: &Session,
    file_id: FileId,
    offset: u32,
) -> Option<SymbolAtOffset> {
    // get module AST/DIR
    let module = get_module_by_file_id(session, file_id)?;
    let module = module.read();
    let (Some(ast), Some(dir)) = (&module.ast, &module.dir) else {
        return None;
    };
    let module_id = module.id;

    // find AST nodes at the offset
    let enclosing = ast.tree.source_map.get_enclosing_spans(offset, offset);
    if enclosing.is_empty() {
        return None;
    }

    // sort by length (smallest first) to get most specific node
    let mut enclosing = enclosing;
    enclosing.sort_by_key(|span| (span.length, -(span.idx as i64))); // "smallest outermost"

    // check if we're in a doc/comment first (these should not return symbols)
    for enclosing_span in &enclosing {
        let node_type = ast.tree.get_node_type(enclosing_span.idx);
        if matches!(
            node_type,
            destack_ast::NodeType::Doc | destack_ast::NodeType::Comment
        ) {
            return None;
        }
    }

    // try each AST node from smallest to largest
    let dir_tree = dir.tree.read();
    for enclosing_span in &enclosing {
        // try to get the DIR node for this AST node
        let Some(dir_node_id) = dir_tree.get_node_id_by_source_id(enclosing_span.idx) else {
            continue;
        };

        match dir_node_id.ty {
            // check if it's an expression with a target_symbol (reference)
            NodeType::Expression => {
                let Ok(expr_id) = dir_node_id.try_into() else {
                    continue;
                };
                let expr = dir_tree.get::<Expression>(expr_id);
                if let Some(target_symbol) = expr.target_symbol() {
                    return Some(SymbolAtOffset {
                        symbol_id: target_symbol,
                        node_id: dir_node_id,
                        span: Span::new(
                            file_id,
                            enclosing_span.span.start,
                            enclosing_span.span.end,
                        ),
                    });
                }
            }
            // check if it's a pattern (variable binding definition)
            NodeType::Pattern => {
                let Ok(pattern_id) = dir_node_id.try_into() else {
                    continue;
                };
                let pattern = dir_tree.get::<Pattern>(pattern_id);
                if let Some(local_symbol) = pattern.symbol() {
                    let symbol_id = GlobalSymbolId {
                        module_id,
                        local_id: local_symbol,
                    };
                    return Some(SymbolAtOffset {
                        symbol_id,
                        node_id: dir_node_id,
                        span: Span::new(
                            file_id,
                            enclosing_span.span.start,
                            enclosing_span.span.end,
                        ),
                    });
                }
            }
            // check if it's a declaration (function/struct/class definition)
            NodeType::Declaration => {
                let Ok(declaration_id) = dir_node_id.try_into() else {
                    continue;
                };
                let declaration = dir_tree.get::<Declaration>(declaration_id);
                let local_symbol = declaration.symbol();
                let symbol_id = GlobalSymbolId {
                    module_id,
                    local_id: local_symbol,
                };
                return Some(SymbolAtOffset {
                    symbol_id,
                    node_id: dir_node_id,
                    span: Span::new(file_id, enclosing_span.span.start, enclosing_span.span.end),
                });
            }
            // check if it's a member (class/struct field or method)
            NodeType::Member => {
                let Ok(member_id) = dir_node_id.try_into() else {
                    continue;
                };
                let member = dir_tree.get::<Member>(member_id);
                let local_symbol = member.symbol();
                let symbol_id = GlobalSymbolId {
                    module_id,
                    local_id: local_symbol,
                };
                return Some(SymbolAtOffset {
                    symbol_id,
                    node_id: dir_node_id,
                    span: Span::new(file_id, enclosing_span.span.start, enclosing_span.span.end),
                });
            }
            // check if it's an enum field
            NodeType::EnumField => {
                let Ok(field_id) = dir_node_id.try_into() else {
                    continue;
                };
                let field = dir_tree.get::<EnumField>(field_id);
                let symbol_id = GlobalSymbolId {
                    module_id,
                    local_id: field.symbol,
                };
                return Some(SymbolAtOffset {
                    symbol_id,
                    node_id: dir_node_id,
                    span: Span::new(file_id, enclosing_span.span.start, enclosing_span.span.end),
                });
            }
            // check if it's a parameter
            NodeType::Parameter => {
                let Ok(param_id) = dir_node_id.try_into() else {
                    continue;
                };
                let param = dir_tree.get::<Parameter>(param_id);
                let local_symbol = param.symbol();
                let symbol_id = GlobalSymbolId {
                    module_id,
                    local_id: local_symbol,
                };
                return Some(SymbolAtOffset {
                    symbol_id,
                    node_id: dir_node_id,
                    span: Span::new(file_id, enclosing_span.span.start, enclosing_span.span.end),
                });
            }
            _ => {}
        }
    }

    None
}

/// Get the canonical symbol for a given symbol id.
///
/// Follows the canonical_symbol chain to get the original definition.
/// For imports, this returns the imported symbol. For regular symbols,
/// this returns the same symbol_id.
pub fn get_canonical_symbol(session: &Session, symbol_id: GlobalSymbolId) -> GlobalSymbolId {
    let module = session.modules.get(symbol_id.module_id);
    let module = module.read();
    let Some(dir) = &module.dir else {
        return symbol_id;
    };
    let symbols = dir.symbols.read();
    let symbol = symbols.get_symbol(symbol_id.local_id);

    if let Some(canonical) = symbol.canonical_symbol
        && canonical != symbol_id
    {
        drop(symbols);
        drop(module);
        return get_canonical_symbol(session, canonical);
    }

    symbol_id
}

/// Get the definition span of a symbol.
///
/// Returns the span of the symbol's primary declaration, following the
/// symbol chain (target_symbol/canonical_symbol) to the original definition.
pub fn get_symbol_definition_span(session: &Session, symbol_id: GlobalSymbolId) -> Option<Span> {
    let module = session.modules.get(symbol_id.module_id);
    let module = module.read();

    let (Some(ast), Some(dir)) = (&module.ast, &module.dir) else {
        return None;
    };
    let symbols = dir.symbols.read();
    let symbol = symbols.get_symbol(symbol_id.local_id);

    // follow canonical_symbol chain if present (for imports)
    let canonical_id = symbol.canonical_symbol.unwrap_or(symbol_id);

    // if canonical is different module, recurse
    if canonical_id.module_id != symbol_id.module_id {
        drop(symbols);
        drop(module);
        return get_symbol_definition_span(session, canonical_id);
    }

    // get the symbol's primary declaration
    let canonical_symbol = symbols.get_symbol(canonical_id.local_id);
    let declaration = canonical_symbol.primary_declaration?;

    drop(symbols);

    // get the main span from the declaration node (identifier span)
    get_dir_node_main_span(ast, dir, declaration.local_id)
}
