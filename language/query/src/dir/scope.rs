use destack_source::{EnclosingSpan, NodeSpanType};
use {destack_ast as ast, destack_dir as dir};

use crate::ast::enclosing_spans_with_previous;
use crate::core::{AstQueryContext, DirQueryContext};

/// A scope and mark resolved for one cursor position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ScopeAtOffset {
    /// The scope id.
    pub scope_id: dir::LocalScopeId,
    /// The scope mark within the scope.
    pub scope_mark: dir::LocalScopeMark,
}

/// Resolve the best visible scope at one offset.
pub(crate) fn scope_at_offset(
    ast: AstQueryContext<'_>,
    dir: DirQueryContext<'_>,
    offset: u32,
) -> Option<ScopeAtOffset> {
    let enclosing = enclosing_spans_with_previous(ast, offset);

    // prefer block scopes because statement and argument positions usually live there
    if let Some(scope) = scope_from_enclosing_dir_nodes(ast, dir, &enclosing, offset, true) {
        return Some(scope);
    }

    // otherwise accept expression and owned declaration scopes
    if let Some(scope) = scope_from_enclosing_dir_nodes(ast, dir, &enclosing, offset, false) {
        return Some(scope);
    }

    // damaged span stacks may still recover through ast parents
    ast_parent_scope_at_offset(ast, dir, &enclosing, offset)
}

/// Resolve the nearest enclosing block or owned declaration scope at one offset.
pub(crate) fn block_scope_at_offset(
    ast: AstQueryContext<'_>,
    dir: DirQueryContext<'_>,
    offset: u32,
) -> Option<ScopeAtOffset> {
    let enclosing = enclosing_spans_with_previous(ast, offset);

    // prefer the nearest enclosing block scope
    if let Some(scope) = scope_from_enclosing_dir_blocks(ast, dir, &enclosing, offset) {
        return Some(scope);
    }

    // otherwise use owned declaration scopes from mapped dir nodes
    if let Some(scope) = scope_from_enclosing_owned_declarations(ast, dir, &enclosing, offset) {
        return Some(scope);
    }

    // damaged span stacks may still recover through ast parents
    ast_parent_block_scope_at_offset(ast, dir, &enclosing, offset)
}

/// Resolve the scope owned by one block span.
pub(crate) fn scope_from_block_span(
    ast: AstQueryContext<'_>,
    dir: DirQueryContext<'_>,
    ast_block_id: u32,
    offset: u32,
) -> Option<ScopeAtOffset> {
    let dir_tree = dir.tree();
    let symbols = dir.symbols();
    let dir_node_id = dir_tree.get_node_id_by_source_id(ast_block_id)?;

    // only dir blocks own statement scopes
    if dir_node_id.ty != dir::NodeType::Block {
        return None;
    }

    let Ok(block_id) = dir_node_id.try_into_typed() else {
        return None;
    };

    let (scope_id, _) = dir_tree.get_scope::<dir::Block>(block_id);
    let scope_mark = scope_mark_at_offset(ast, scope_id, offset, dir_tree, symbols);

    Some(ScopeAtOffset {
        scope_id,
        scope_mark,
    })
}

/// Resolve the visible scope for one expression at an offset.
pub(crate) fn expression_scope_at_offset(
    ast: AstQueryContext<'_>,
    dir: DirQueryContext<'_>,
    expr_id: dir::LocalNodeId<dir::Expression>,
    offset: u32,
) -> ScopeAtOffset {
    let dir_tree = dir.tree();
    let (scope_id, _) = dir_tree.get_scope::<dir::Expression>(expr_id);

    scope_at_offset(ast, dir, offset).unwrap_or(ScopeAtOffset {
        scope_id,
        scope_mark: dir::LocalScopeMark(0),
    })
}

/// Resolve the best mapped dir scope from enclosing spans.
fn scope_from_enclosing_dir_nodes(
    ast: AstQueryContext<'_>,
    dir: DirQueryContext<'_>,
    enclosing: &[EnclosingSpan],
    offset: u32,
    block_only: bool,
) -> Option<ScopeAtOffset> {
    let dir_tree = dir.tree();
    let symbols = dir.symbols();

    for enc in enclosing {
        let Some(dir_node_id) = dir_tree.get_node_id_by_source_id(enc.idx) else {
            continue;
        };

        // blocks define the local statement scope
        if dir_node_id.ty == dir::NodeType::Block {
            let Ok(block_id) = dir_node_id.try_into_typed() else {
                continue;
            };

            let (scope_id, _) = dir_tree.get_scope::<dir::Block>(block_id);
            let scope_mark = scope_mark_at_offset(ast, scope_id, offset, dir_tree, symbols);

            return Some(ScopeAtOffset {
                scope_id,
                scope_mark,
            });
        }

        if block_only {
            continue;
        }

        // expressions reuse the surrounding lexical scope with an offset aware mark
        if dir_node_id.ty == dir::NodeType::Expression {
            let Ok(expr_id) = dir_node_id.try_into_typed() else {
                continue;
            };

            let (scope_id, _) = dir_tree.get_scope::<dir::Expression>(expr_id);
            let scope_mark = scope_mark_at_offset(ast, scope_id, offset, dir_tree, symbols);

            return Some(ScopeAtOffset {
                scope_id,
                scope_mark,
            });
        }

        // owned declarations such as functions and classes expose their inner scope
        if dir_node_id.ty == dir::NodeType::Declaration
            && let Some(scope_id) = owned_scope_for_declaration_id(symbols, dir_node_id.id)
        {
            let scope_mark = scope_mark_at_offset(ast, scope_id, offset, dir_tree, symbols);

            return Some(ScopeAtOffset {
                scope_id,
                scope_mark,
            });
        }
    }

    None
}

/// Resolve the nearest mapped dir block scope from enclosing spans.
fn scope_from_enclosing_dir_blocks(
    ast: AstQueryContext<'_>,
    dir: DirQueryContext<'_>,
    enclosing: &[EnclosingSpan],
    offset: u32,
) -> Option<ScopeAtOffset> {
    let dir_tree = dir.tree();
    let symbols = dir.symbols();

    for enc in enclosing {
        let Some(dir_node_id) = dir_tree.get_node_id_by_source_id(enc.idx) else {
            continue;
        };

        if dir_node_id.ty != dir::NodeType::Block {
            continue;
        }

        let Ok(block_id) = dir_node_id.try_into_typed() else {
            continue;
        };

        let (scope_id, _) = dir_tree.get_scope::<dir::Block>(block_id);
        let scope_mark = scope_mark_at_offset(ast, scope_id, offset, dir_tree, symbols);

        return Some(ScopeAtOffset {
            scope_id,
            scope_mark,
        });
    }

    None
}

/// Resolve the nearest mapped owned declaration scope from enclosing spans.
fn scope_from_enclosing_owned_declarations(
    ast: AstQueryContext<'_>,
    dir: DirQueryContext<'_>,
    enclosing: &[EnclosingSpan],
    offset: u32,
) -> Option<ScopeAtOffset> {
    let dir_tree = dir.tree();
    let symbols = dir.symbols();

    for enc in enclosing {
        let Some(dir_node_id) = dir_tree.get_node_id_by_source_id(enc.idx) else {
            continue;
        };

        if dir_node_id.ty != dir::NodeType::Declaration {
            continue;
        }

        let Some(scope_id) = owned_scope_for_declaration_id(symbols, dir_node_id.id) else {
            continue;
        };
        let scope_mark = scope_mark_at_offset(ast, scope_id, offset, dir_tree, symbols);

        return Some(ScopeAtOffset {
            scope_id,
            scope_mark,
        });
    }

    None
}

/// Resolve one scope through ast parent recovery when direct span mapping failed.
fn ast_parent_scope_at_offset(
    ast: AstQueryContext<'_>,
    dir: DirQueryContext<'_>,
    enclosing: &[EnclosingSpan],
    offset: u32,
) -> Option<ScopeAtOffset> {
    let start_id = enclosing.first().map(|enc| enc.idx)?;

    let dir_tree = dir.tree();
    let symbols = dir.symbols();

    for parent_id in ast.parents().walk_parents_by_id(start_id) {
        let Some(dir_node_id) = dir_tree.get_node_id_by_source_id(parent_id) else {
            if ast.tree().get_node_type(parent_id) == ast::NodeType::Declaration
                && let Some(scope_id) =
                    owned_scope_for_ast_declaration_id(symbols, dir_tree, parent_id)
            {
                let scope_mark = scope_mark_at_offset(ast, scope_id, offset, dir_tree, symbols);

                return Some(ScopeAtOffset {
                    scope_id,
                    scope_mark,
                });
            }

            continue;
        };

        // blocks still win inside damaged syntax
        if dir_node_id.ty == dir::NodeType::Block {
            let Ok(block_id) = dir_node_id.try_into_typed() else {
                continue;
            };

            let (scope_id, _) = dir_tree.get_scope::<dir::Block>(block_id);
            let scope_mark = scope_mark_at_offset(ast, scope_id, offset, dir_tree, symbols);

            return Some(ScopeAtOffset {
                scope_id,
                scope_mark,
            });
        }

        // expressions carry the surrounding lexical scope
        if dir_node_id.ty == dir::NodeType::Expression {
            let Ok(expr_id) = dir_node_id.try_into_typed() else {
                continue;
            };

            let (scope_id, _) = dir_tree.get_scope::<dir::Expression>(expr_id);
            let scope_mark = scope_mark_at_offset(ast, scope_id, offset, dir_tree, symbols);

            return Some(ScopeAtOffset {
                scope_id,
                scope_mark,
            });
        }

        // owned declarations expose an inner scope
        if dir_node_id.ty == dir::NodeType::Declaration
            && let Some(scope_id) = owned_scope_for_declaration_id(symbols, dir_node_id.id)
        {
            let scope_mark = scope_mark_at_offset(ast, scope_id, offset, dir_tree, symbols);

            return Some(ScopeAtOffset {
                scope_id,
                scope_mark,
            });
        }
    }

    None
}

/// Resolve one block or owned declaration scope through ast parent recovery.
fn ast_parent_block_scope_at_offset(
    ast: AstQueryContext<'_>,
    dir: DirQueryContext<'_>,
    enclosing: &[EnclosingSpan],
    offset: u32,
) -> Option<ScopeAtOffset> {
    let start_id = enclosing.first().map(|enc| enc.idx)?;

    let dir_tree = dir.tree();
    let symbols = dir.symbols();

    for parent_id in ast.parents().walk_parents_by_id(start_id) {
        let Some(dir_node_id) = dir_tree.get_node_id_by_source_id(parent_id) else {
            if ast.tree().get_node_type(parent_id) == ast::NodeType::Declaration
                && let Some(scope_id) =
                    owned_scope_for_ast_declaration_id(symbols, dir_tree, parent_id)
            {
                let scope_mark = scope_mark_at_offset(ast, scope_id, offset, dir_tree, symbols);

                return Some(ScopeAtOffset {
                    scope_id,
                    scope_mark,
                });
            }

            continue;
        };

        // prefer the nearest enclosing block
        if dir_node_id.ty == dir::NodeType::Block {
            let Ok(block_id) = dir_node_id.try_into_typed() else {
                continue;
            };

            let (scope_id, _) = dir_tree.get_scope::<dir::Block>(block_id);
            let scope_mark = scope_mark_at_offset(ast, scope_id, offset, dir_tree, symbols);

            return Some(ScopeAtOffset {
                scope_id,
                scope_mark,
            });
        }

        // otherwise accept owned declaration scopes
        if dir_node_id.ty == dir::NodeType::Declaration
            && let Some(scope_id) = owned_scope_for_declaration_id(symbols, dir_node_id.id)
        {
            let scope_mark = scope_mark_at_offset(ast, scope_id, offset, dir_tree, symbols);

            return Some(ScopeAtOffset {
                scope_id,
                scope_mark,
            });
        }
    }

    None
}

/// Resolve the current scope mark at one offset.
fn scope_mark_at_offset(
    ast: AstQueryContext<'_>,
    scope_id: dir::LocalScopeId,
    offset: u32,
    dir_tree: &dir::Tree,
    symbols: &dir::SymbolTable,
) -> dir::LocalScopeMark {
    let scope = symbols.get_scope_by_id(scope_id);
    if scope.named_symbols.is_empty() {
        return dir::LocalScopeMark(0);
    }

    // count symbols whose declaration begins before the cursor
    let mut mark_index = 0u32;

    for (index, (_, symbol_id)) in scope.named_symbols.iter().enumerate() {
        let symbol = symbols.get_symbol(*symbol_id);
        let Some(declaration) = symbol.primary_declaration else {
            mark_index = (index + 1) as u32;
            continue;
        };

        let source_id = dir_tree.get_source(declaration.local_id.id);
        let span = ast
            .tree()
            .source_map
            .get_side_or_main_or_enclosing(source_id, NodeSpanType::Main);

        if span.start <= offset {
            mark_index = (index + 1) as u32;
        }
    }

    dir::LocalScopeMark(mark_index)
}

/// Resolve the owned scope for one declaration id.
fn owned_scope_for_declaration_id(
    symbols: &dir::SymbolTable,
    declaration_id: u32,
) -> Option<dir::LocalScopeId> {
    for (index, scope) in symbols.scopes().enumerate() {
        let Some(owner_id) = scope.owner_id else {
            continue;
        };

        let owner = symbols.get_symbol(owner_id);
        let Some(declaration) = owner.primary_declaration else {
            continue;
        };

        if declaration.local_id.id == declaration_id {
            return Some(dir::LocalScopeId::new(index as u32));
        }
    }

    None
}

/// Resolve the owned scope for one ast declaration id.
fn owned_scope_for_ast_declaration_id(
    symbols: &dir::SymbolTable,
    dir_tree: &dir::Tree,
    ast_id: u32,
) -> Option<dir::LocalScopeId> {
    for (index, scope) in symbols.scopes().enumerate() {
        let Some(owner_id) = scope.owner_id else {
            continue;
        };

        let owner = symbols.get_symbol(owner_id);
        let Some(declaration) = owner.primary_declaration else {
            continue;
        };

        if dir_tree.get_source(declaration.local_id.id) == ast_id {
            return Some(dir::LocalScopeId::new(index as u32));
        }
    }

    None
}
