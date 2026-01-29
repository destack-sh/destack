use destack_base::StringId;
use destack_dir::{
    Declaration, DependencyItem, DependencyKind, DependencyMode, DynamicKey, EnumField, Expression,
    GlobalSymbolId, LocalNodeIdAny, Member, NodeType, Parameter, Pattern, SymbolSpace,
};
use destack_source::{FileId, Span};
use std::collections::HashSet;
use {destack_ast as ast, destack_dir as dir};

use super::resolve::{global_symbol, resolve_module_id_for_import_target};
use super::{QueryContext, get_dir_node_main_span, get_dir_node_span, get_module_by_file_id};
use crate::Session;
use crate::program::{ModuleAst, ModuleDir};

pub(crate) use super::resolve::{
    dependency_item_matches_name, matches_symbol_space_filter, owned_scope_for_symbol,
    resolve_member_access_symbol, resolve_nominal_symbol_from_type_expression,
    resolve_type_symbol_from_dependency_symbol, resolve_type_symbol_from_module,
};

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
    // resolve the module and query context
    let module = get_module_by_file_id(session, file_id)?;
    let module = module.read();
    let ctx = session.query_context(&module)?;

    // find AST nodes at the offset
    let enclosing = ctx.ast.tree.source_map.get_enclosing_spans(offset, offset);
    if enclosing.is_empty() {
        return None;
    }

    // sort by length (smallest first) to get most specific node
    let mut enclosing = enclosing;

    // prefer the most specific span when multiple nodes tie
    enclosing.sort_by_key(|span| (span.length, -(span.idx as i64)));

    // check if we're in a doc/comment first (these should not return symbols)
    for enclosing_span in &enclosing {
        let node_type = ctx.ast.tree.get_node_type(enclosing_span.idx);
        if matches!(node_type, ast::NodeType::Doc | ast::NodeType::Comment) {
            return None;
        }
    }

    let dir_tree = ctx.tree();

    // scan member access expressions first to lock onto the member name span
    for (expr_id, expr) in dir_tree.iter_nodes_of_type::<Expression>() {
        let Expression::Member { left, name, .. } = expr else {
            continue;
        };

        // resolve member symbols when the cursor is on the member name
        if let Some(result) =
            member_symbol_at_offset(session, &ctx, expr_id, expr_id.into(), *left, *name, offset)
        {
            return Some(result);
        }
    }

    // try each AST node from smallest to largest
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

                // resolve member symbols when the cursor is on the member name
                if let Expression::Member { left, name, .. } = expr {
                    let result = member_symbol_at_offset(
                        session,
                        &ctx,
                        expr_id,
                        dir_node_id,
                        *left,
                        *name,
                        offset,
                    );
                    if let Some(result) = result {
                        return Some(result);
                    }

                    // check if the cursor is on the left expression and resolve that symbol
                    if let Some(left_span) = get_dir_node_span(ctx.ast, ctx.dir, (*left).into())
                        && offset >= left_span.start
                        && offset <= left_span.end
                    {
                        let left_expr = dir_tree.get::<Expression>(*left);
                        if let Some(target_symbol) = left_expr.target_symbol() {
                            return Some(SymbolAtOffset {
                                symbol_id: target_symbol,
                                node_id: (*left).into(),
                                span: left_span,
                            });
                        }
                    }
                }

                // resolve direct target symbols from the expression
                if let Some(target_symbol) = expr.target_symbol() {
                    return Some(SymbolAtOffset {
                        symbol_id: target_symbol,
                        node_id: dir_node_id,
                        span: Span::new(
                            ctx.file_id,
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
                    let symbol_id = global_symbol(ctx.module_id, local_symbol);
                    return Some(SymbolAtOffset {
                        symbol_id,
                        node_id: dir_node_id,
                        span: Span::new(
                            ctx.file_id,
                            enclosing_span.span.start,
                            enclosing_span.span.end,
                        ),
                    });
                }
            }
            // check if it's a declaration (function/struct/class definition)
            NodeType::Declaration => {
                let Ok(declaration_id): Result<dir::LocalNodeId<Declaration>, _> =
                    dir_node_id.try_into()
                else {
                    continue;
                };
                let declaration = dir_tree.get::<Declaration>(declaration_id);
                let local_symbol = declaration.symbol();
                let symbol_id = global_symbol(ctx.module_id, local_symbol);
                return Some(SymbolAtOffset {
                    symbol_id,
                    node_id: dir_node_id,
                    span: Span::new(
                        ctx.file_id,
                        enclosing_span.span.start,
                        enclosing_span.span.end,
                    ),
                });
            }
            // check if it's a member (class/struct field or method)
            NodeType::Member => {
                let Ok(member_id): Result<dir::LocalNodeId<Member>, _> = dir_node_id.try_into()
                else {
                    continue;
                };
                let member = dir_tree.get::<Member>(member_id);
                let local_symbol = member.symbol();
                let symbol_id = global_symbol(ctx.module_id, local_symbol);
                return Some(SymbolAtOffset {
                    symbol_id,
                    node_id: dir_node_id,
                    span: Span::new(
                        ctx.file_id,
                        enclosing_span.span.start,
                        enclosing_span.span.end,
                    ),
                });
            }
            // check if it's an enum field
            NodeType::EnumField => {
                let Ok(field_id) = dir_node_id.try_into() else {
                    continue;
                };
                let field = dir_tree.get::<EnumField>(field_id);
                let symbol_id = global_symbol(ctx.module_id, field.symbol);
                return Some(SymbolAtOffset {
                    symbol_id,
                    node_id: dir_node_id,
                    span: Span::new(
                        ctx.file_id,
                        enclosing_span.span.start,
                        enclosing_span.span.end,
                    ),
                });
            }
            // check if it's a parameter
            NodeType::Parameter => {
                let Ok(param_id) = dir_node_id.try_into() else {
                    continue;
                };
                let param = dir_tree.get::<Parameter>(param_id);
                let local_symbol = param.symbol();
                let symbol_id = global_symbol(ctx.module_id, local_symbol);
                return Some(SymbolAtOffset {
                    symbol_id,
                    node_id: dir_node_id,
                    span: Span::new(
                        ctx.file_id,
                        enclosing_span.span.start,
                        enclosing_span.span.end,
                    ),
                });
            }
            // check if it's a dependency item (import or re export specifier)
            NodeType::DependencyItem => {
                let Ok(item_id) = dir_node_id.try_into() else {
                    continue;
                };
                let item = dir_tree.get::<DependencyItem>(item_id);
                let symbol_id = if let Some(local_symbol) = item.symbol() {
                    global_symbol(ctx.module_id, local_symbol)
                } else if let Some(target_symbol) = item.target_symbol() {
                    target_symbol
                } else {
                    // resolve type-only dependency targets when no symbol is bound
                    let (kind, name, alias, target, target_module) = match item {
                        DependencyItem::Remote {
                            kind,
                            name,
                            alias,
                            target,
                            target_module,
                            ..
                        } => (*kind, *name, *alias, Some(*target), Some(*target_module)),
                        DependencyItem::UnresolvedRemote {
                            kind,
                            name,
                            alias,
                            target,
                            target_module,
                            ..
                        } => (*kind, *name, *alias, Some(*target), *target_module),
                        _ => continue,
                    };

                    if kind != DependencyKind::Type {
                        continue;
                    }

                    let Some(name_id) = alias.or(name) else {
                        continue;
                    };

                    let mut target_module_id = target_module.and_then(|target| target.module_id());
                    if target_module_id.is_none()
                        && let Some(target) = target
                    {
                        let target_text = session.strings.get(target).to_string();
                        target_module_id = resolve_module_id_for_import_target(
                            session,
                            &ctx,
                            target_text.as_str(),
                        );
                    }

                    let Some(target_module_id) = target_module_id else {
                        continue;
                    };

                    let mut visited = HashSet::new();
                    let Some(resolved_symbol) = resolve_type_symbol_from_module(
                        session,
                        target_module_id,
                        name_id,
                        &mut visited,
                    ) else {
                        continue;
                    };

                    resolved_symbol
                };

                return Some(SymbolAtOffset {
                    symbol_id,
                    node_id: dir_node_id,
                    span: Span::new(
                        ctx.file_id,
                        enclosing_span.span.start,
                        enclosing_span.span.end,
                    ),
                });
            }
            _ => {}
        }
    }

    None
}

/// Resolve a member access symbol when the cursor is on the member name.
fn member_symbol_at_offset(
    session: &Session,
    ctx: &QueryContext<'_>,
    expr_id: dir::LocalNodeId<Expression>,
    node_id: LocalNodeIdAny,
    left: dir::LocalNodeId<Expression>,
    name: StringId,
    offset: u32,
) -> Option<SymbolAtOffset> {
    // resolve the member name text
    let member_name = session.strings.get(name).to_string();

    // resolve the precise name span
    let name_span = get_member_access_name_span(ctx, expr_id, &member_name)?;

    // skip when the cursor is not on the member name
    if offset < name_span.start || offset > name_span.end {
        return None;
    }

    // resolve the member symbol id
    let symbol_id = resolve_member_access_symbol(session, ctx, expr_id, left, name)?;

    // return the resolved member symbol at the cursor
    Some(SymbolAtOffset {
        symbol_id,
        node_id,
        span: name_span,
    })
}

/// Resolve the span for a member access name inside its expression span.
pub(crate) fn get_member_access_name_span(
    ctx: &QueryContext<'_>,
    expression_id: dir::LocalNodeId<Expression>,
    member_name: &str,
) -> Option<Span> {
    // return the full expression span when the member name is empty
    if member_name.is_empty() {
        return get_dir_node_span(ctx.ast, ctx.dir, expression_id.into());
    }

    // resolve the member name span from the AST expression
    let ast_node_id = ctx.tree().get_source(expression_id.id);
    if let Some(span) = ctx.ast.tree.get_main_span_by_id(ast_node_id) {
        return Some(span);
    }

    Some(ctx.ast.tree.source_map.get_main_or_enclosing(ast_node_id))
}

/// Get the canonical symbol for a given symbol id.
///
/// Follows the canonical_symbol chain to get the original definition.
/// For imports, this returns the imported symbol.
/// For regular symbols, this returns the same symbol_id.
pub fn get_canonical_symbol(session: &Session, symbol_id: GlobalSymbolId) -> GlobalSymbolId {
    // resolve the module query context
    let module = session.modules.get(symbol_id.module_id);
    let module = module.read();
    let Some(ctx) = session.query_context(&module) else {
        return symbol_id;
    };

    let symbols = ctx.symbols();
    let symbol = symbols.get_symbol(symbol_id.local_id);

    if let Some(canonical) = symbol.canonical_symbol
        && canonical != symbol_id
    {
        drop(symbols);
        drop(module);
        return get_canonical_symbol(session, canonical);
    }

    // return the original symbol id
    symbol_id
}

/// Resolve a symbol name string when possible.
pub(crate) fn resolve_symbol_name(
    session: &Session,
    symbol_id: dir::GlobalSymbolId,
) -> Option<String> {
    // resolve the module and query context for the symbol
    let module = session.modules.get(symbol_id.module_id);
    let module = module.read();
    let ctx = session.query_context(&module)?;

    // read the symbol name string
    let symbols = ctx.symbols();
    let symbol = symbols.get_symbol(symbol_id.local_id);
    symbol
        .name()
        .map(|name_id| session.strings.get(name_id).to_string())
}

/// Check whether a symbol is a local import alias for a canonical target.
pub(crate) fn is_dependency_alias_for_target(
    session: &Session,
    ctx: &QueryContext<'_>,
    symbol_id: dir::GlobalSymbolId,
    canonical_target: dir::GlobalSymbolId,
) -> bool {
    // resolve the symbol's primary declaration
    let symbols = ctx.symbols();
    let symbol = symbols.get_symbol(symbol_id.local_id);
    let Some(declaration) = symbol.primary_declaration else {
        return false;
    };
    drop(symbols);

    // bail out when the declaration is not a dependency item
    if declaration.local_id.ty != NodeType::DependencyItem {
        return false;
    }

    // resolve the dependency item node
    let Ok(item_id): Result<dir::LocalNodeId<DependencyItem>, _> = declaration.local_id.try_into()
    else {
        return false;
    };

    // check for an alias that targets the canonical symbol
    let dir_tree = ctx.tree();
    let item = dir_tree.get::<DependencyItem>(item_id);
    let (alias, target_symbol) = match item {
        DependencyItem::Local {
            alias,
            target_symbol,
            ..
        }
        | DependencyItem::Remote {
            alias,
            target_symbol,
            ..
        } => (alias, target_symbol),
        _ => return false,
    };

    // require an explicit alias
    if alias.is_none() {
        return false;
    }

    // compare canonical targets
    let target_canonical = get_canonical_symbol(session, *target_symbol);
    target_canonical == canonical_target
}

/// Resolve a symbol name id, loading the owning module when needed.
pub(crate) fn resolve_symbol_name_id(
    session: &Session,
    ctx: &QueryContext<'_>,
    symbol_id: dir::GlobalSymbolId,
) -> Option<StringId> {
    // prefer the current query context when the symbol is local
    if symbol_id.module_id == ctx.module_id {
        let symbols = ctx.symbols();
        let symbol = symbols.get_symbol(symbol_id.local_id);
        return symbol.name();
    }

    // fall back to the symbol's module context
    let module = session.modules.get(symbol_id.module_id);
    let module = module.read();
    let target_ctx = session.query_context(&module)?;
    let symbols = target_ctx.symbols();
    let symbol = symbols.get_symbol(symbol_id.local_id);
    symbol.name()
}

/// Resolve a static member name from a dynamic key.
pub(crate) fn member_key_name(session: &Session, key: &DynamicKey) -> Option<String> {
    // resolve name and numeric keys into strings
    match key {
        DynamicKey::Name(name_id) | DynamicKey::Number(name_id) => {
            Some(session.strings.get(*name_id).to_string())
        }
        _ => None,
    }
}

/// Check whether a member field is the synthetic `function` keyword placeholder.
pub(crate) fn is_synthetic_function_keyword_field(
    member: &Member,
    name: &str,
    range: Span,
) -> bool {
    // only skip field members that match the exact `function` token length
    if !matches!(member, Member::Field { .. }) || name != "function" {
        return false;
    }

    let full_len = range.end.saturating_sub(range.start);
    full_len == 8
}

fn get_symbol_span_with(
    session: &Session,
    symbol_id: GlobalSymbolId,
    span_for_declaration: impl Fn(&ModuleAst, &ModuleDir, LocalNodeIdAny) -> Option<Span> + Copy,
) -> Option<Span> {
    // resolve the module and query context for this symbol
    let module = session.modules.get(symbol_id.module_id);
    let module = module.read();
    let ctx = session.query_context(&module)?;

    // read the symbol and follow the canonical chain
    let symbols = ctx.symbols();
    let symbol = symbols.get_symbol(symbol_id.local_id);
    let canonical_id = symbol.canonical_symbol.unwrap_or(symbol_id);

    // recurse when the canonical symbol lives in another module
    if canonical_id.module_id != symbol_id.module_id {
        drop(symbols);
        drop(module);
        return get_symbol_span_with(session, canonical_id, span_for_declaration);
    }

    // resolve the primary declaration for the canonical symbol
    let canonical_symbol = symbols.get_symbol(canonical_id.local_id);
    let declaration = canonical_symbol.primary_declaration;
    let target_symbol = canonical_symbol.target_symbol;

    drop(symbols);

    // extract the desired span from the declaration node
    if let Some(declaration) = declaration {
        return span_for_declaration(ctx.ast, ctx.dir, declaration.local_id);
    }

    // follow target symbols when the canonical symbol is an alias
    if let Some(target) = target_symbol {
        drop(module);
        return get_symbol_span_with(session, target, span_for_declaration);
    }

    // fall back to default or export assignment spans when present
    fallback_export_span(&ctx, canonical_id, span_for_declaration)
}

/// Get the definition span of a symbol.
///
/// Returns the span of the symbol's primary declaration identifier.
pub fn get_symbol_definition_span(session: &Session, symbol_id: GlobalSymbolId) -> Option<Span> {
    get_symbol_span_with(session, symbol_id, get_dir_node_main_span)
}

/// Resolve a definition span when the symbol is a type symbol.
pub(crate) fn type_definition_span_for_symbol(
    session: &Session,
    symbol_id: GlobalSymbolId,
) -> Option<Span> {
    // resolve the module and query context for the symbol
    let module = session.modules.get(symbol_id.module_id);
    let module = module.read();
    let ctx = session.query_context(&module)?;

    // check whether the symbol participates in the type namespace
    let symbols = ctx.symbols();
    let symbol = symbols.get_symbol(symbol_id.local_id);
    if !matches_symbol_space_filter(symbol.ty, symbol.space, Some(SymbolSpace::Type)) {
        return None;
    }

    drop(symbols);
    drop(module);

    // resolve the definition span for the type symbol
    get_symbol_definition_span(session, symbol_id)
}

/// Get the full declaration span of a symbol.
pub fn get_symbol_declaration_span(session: &Session, symbol_id: GlobalSymbolId) -> Option<Span> {
    get_symbol_span_with(session, symbol_id, get_dir_node_span)
}

fn fallback_export_span(
    ctx: &QueryContext<'_>,
    symbol_id: GlobalSymbolId,
    span_for_declaration: impl Fn(&ModuleAst, &ModuleDir, LocalNodeIdAny) -> Option<Span> + Copy,
) -> Option<Span> {
    // handle export assignment symbols
    let is_export_assignment = symbol_id.local_id == ctx.dir.export_assignment_symbol;
    if is_export_assignment {
        let item_id = *ctx.dir.export_assignment.read();
        if let Some(item_id) = item_id {
            return span_for_declaration(ctx.ast, ctx.dir, item_id.into());
        }
    }

    // skip when the symbol is not the default export
    if symbol_id.local_id != ctx.dir.default_symbol {
        return None;
    }

    // scan dependency items for default exports
    let dir_tree = ctx.tree();
    for item_id in dir_tree.iter_node_ids_of_type::<DependencyItem>() {
        let DependencyItem::Value { mode, .. } = dir_tree.get(item_id) else {
            continue;
        };

        if *mode != DependencyMode::Default {
            continue;
        }

        return span_for_declaration(ctx.ast, ctx.dir, item_id.into());
    }

    None
}
