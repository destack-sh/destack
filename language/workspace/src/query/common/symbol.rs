use destack_ast as ast;
use destack_base::StringId;
use destack_dir::{
    Declaration, Declarator, DependencyItem, DynamicKey, EnumField, Expression, ExtensionKind,
    GlobalNodeIdAny, GlobalSymbolId, LocalNodeIdAny, Member, NodeType, Parameter, Pattern,
    Resolution, SymbolSpace, Type,
};
use destack_source::{FileContent, FileId, ModuleId, Span};

use super::QueryContext;
use super::span::{get_dir_node_main_span, get_dir_node_span, get_module_by_file_id};
use crate::Session;
use crate::program::{ModuleAst, ModuleDir};

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

/// Build a global symbol id from a module and local symbol id.
fn global_symbol(module_id: ModuleId, local_id: destack_dir::LocalSymbolId) -> GlobalSymbolId {
    GlobalSymbolId {
        module_id,
        local_id,
    }
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
                if let Expression::Member { left, name, .. } = expr
                    && let Some(result) = member_symbol_at_offset(
                        session,
                        &ctx,
                        expr_id,
                        dir_node_id,
                        *left,
                        *name,
                        offset,
                    )
                {
                    return Some(result);
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
                let Ok(declaration_id): Result<destack_dir::LocalNodeId<Declaration>, _> =
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
                let Ok(member_id): Result<destack_dir::LocalNodeId<Member>, _> =
                    dir_node_id.try_into()
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
            // check if it's a dependency item (import or re-export specifier)
            NodeType::DependencyItem => {
                let Ok(item_id) = dir_node_id.try_into() else {
                    continue;
                };
                let item = dir_tree.get::<DependencyItem>(item_id);
                let Some(local_symbol) = item.symbol() else {
                    continue;
                };
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
            _ => {}
        }
    }

    // fallback: scan expressions by span when direct mapping fails
    let mut best_match: Option<(destack_dir::LocalNodeId<Expression>, Span, bool)> = None;
    for (expr_id, _expr) in dir_tree.iter_nodes_of_type::<Expression>() {
        let Some(main_span) = get_dir_node_main_span(ctx.ast, ctx.dir, expr_id.into()) else {
            continue;
        };

        if offset >= main_span.start && offset <= main_span.end {
            let length = main_span.end.saturating_sub(main_span.start);
            if best_match
                .map(|(_, span, _)| length < span.end.saturating_sub(span.start))
                .unwrap_or(true)
            {
                best_match = Some((expr_id, main_span, true));
            }
            continue;
        }

        let Some(full_span) = get_dir_node_span(ctx.ast, ctx.dir, expr_id.into()) else {
            continue;
        };
        if offset >= full_span.start && offset <= full_span.end {
            let length = full_span.end.saturating_sub(full_span.start);
            if best_match
                .map(|(_, span, _)| length < span.end.saturating_sub(span.start))
                .unwrap_or(true)
            {
                best_match = Some((expr_id, full_span, false));
            }
        }
    }

    if let Some((expr_id, span, _)) = best_match {
        let expr = dir_tree.get::<Expression>(expr_id);

        // resolve member symbols when the cursor is on the member name
        if let Expression::Member { left, name, .. } = expr
            && let Some(result) = member_symbol_at_offset(
                session,
                &ctx,
                expr_id,
                expr_id.into(),
                *left,
                *name,
                offset,
            )
        {
            return Some(result);
        }

        if let Some(target_symbol) = expr.target_symbol() {
            return Some(SymbolAtOffset {
                symbol_id: target_symbol,
                node_id: expr_id.into(),
                span,
            });
        }
    }

    None
}

/// Resolve a member access symbol when the cursor is on the member name.
fn member_symbol_at_offset(
    session: &Session,
    ctx: &QueryContext<'_>,
    expr_id: destack_dir::LocalNodeId<Expression>,
    node_id: LocalNodeIdAny,
    left: destack_dir::LocalNodeId<Expression>,
    name: StringId,
    offset: u32,
) -> Option<SymbolAtOffset> {
    // resolve the member name text
    let member_name = session.strings.get(name).to_string();

    // resolve the precise name span
    let name_span = get_member_access_name_span(session, ctx, expr_id, &member_name)?;

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

/// Get the canonical symbol for a given symbol id.
///
/// Follows the canonical_symbol chain to get the original definition.
/// For imports, this returns the imported symbol. For regular symbols,
/// this returns the same symbol_id.
pub fn get_canonical_symbol(session: &Session, symbol_id: GlobalSymbolId) -> GlobalSymbolId {
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

    symbol_id
}

/// Check whether a symbol is a local import alias for a canonical target.
pub(crate) fn is_dependency_alias_for_target(
    session: &Session,
    ctx: &QueryContext<'_>,
    symbol_id: GlobalSymbolId,
    canonical_target: GlobalSymbolId,
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
    let Ok(item_id): Result<destack_dir::LocalNodeId<DependencyItem>, _> =
        declaration.local_id.try_into()
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

/// Resolve a symbol span using a span extraction function.
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
    let declaration = canonical_symbol.primary_declaration?;

    drop(symbols);

    // extract the desired span from the declaration node
    span_for_declaration(ctx.ast, ctx.dir, declaration.local_id)
}

/// Get the definition span of a symbol.
///
/// Returns the span of the symbol's primary declaration identifier.
pub fn get_symbol_definition_span(session: &Session, symbol_id: GlobalSymbolId) -> Option<Span> {
    get_symbol_span_with(session, symbol_id, get_dir_node_main_span)
}

/// Get the full declaration span of a symbol.
pub fn get_symbol_declaration_span(session: &Session, symbol_id: GlobalSymbolId) -> Option<Span> {
    get_symbol_span_with(session, symbol_id, get_dir_node_span)
}

/// Resolve a member symbol from recorded type resolution data.
fn recorded_member_resolution(
    ctx: &QueryContext<'_>,
    expression_id: destack_dir::LocalNodeId<Expression>,
) -> Option<GlobalSymbolId> {
    // read the recorded resolution for the expression node
    let types = ctx.types();
    let node_id = GlobalNodeIdAny {
        module_id: ctx.module_id,
        local_id: expression_id.into(),
    };
    let resolution_id = types.get_resolution_for_node(node_id)?;

    // extract a target symbol from the recorded resolution
    let resolution = types.get_resolution(resolution_id);
    match resolution {
        Resolution::Static { candidate, .. } => Some(candidate.target_symbol),
        Resolution::Dynamic { candidates, .. } => {
            candidates.first().map(|candidate| candidate.target_symbol)
        }
        Resolution::Unresolved { candidates, .. } => {
            if candidates.len() == 1 {
                candidates.first().map(|candidate| candidate.target_symbol)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Resolve the symbol for a member access expression.
pub fn resolve_member_access_symbol(
    session: &Session,
    ctx: &QueryContext<'_>,
    expression_id: destack_dir::LocalNodeId<Expression>,
    left: destack_dir::LocalNodeId<Expression>,
    name: destack_base::StringId,
) -> Option<GlobalSymbolId> {
    // use recorded member resolution when it is available
    if let Some(target) = recorded_member_resolution(ctx, expression_id) {
        return Some(target);
    }

    // resolve the member name string
    let member_name = session.strings.get(name).to_string();

    // resolve the base nominal symbol for the left expression
    let base_symbol = nominal_symbol_for_expression(session, ctx, left)?;

    // resolve members declared directly on the base type
    if let Some(symbol_id) =
        resolve_member_symbol_from_declaration(session, base_symbol, &member_name)
    {
        return Some(symbol_id);
    }

    // resolve members provided by extensions
    resolve_extension_member_symbol(session, base_symbol, ctx.module_id, &member_name)
}

/// Resolve the nominal symbol for an expression, using type info or fallbacks.
pub(crate) fn nominal_symbol_for_expression(
    session: &Session,
    ctx: &QueryContext<'_>,
    expression_id: destack_dir::LocalNodeId<Expression>,
) -> Option<GlobalSymbolId> {
    // resolve the type id from expression or node type tables
    let mut type_id = ctx
        .get_expression_type(expression_id.into())
        .or_else(|| ctx.get_node_type(expression_id.into()));

    // fall back to the expression target symbol when type info is missing
    if type_id.is_none() {
        let dir_tree = ctx.tree();
        let expression = dir_tree.get::<Expression>(expression_id);
        let target_symbol = expression.target_symbol()?;

        let types = ctx.types();
        let symbols = ctx.symbols();
        type_id = types.get_type_id_for_symbol(&symbols, target_symbol);

        // fall back to resolving from the target symbol initializer
        if type_id.is_none() {
            return resolve_nominal_symbol_from_initializer(session, ctx, target_symbol);
        }
    }

    // resolve the nominal symbol from the resolved type id
    let type_id = type_id?;
    let types = ctx.types();
    resolve_nominal_symbol_from_type(&types, type_id)
}

/// Resolve the span for a member access name inside its expression span.
pub(crate) fn get_member_access_name_span(
    session: &Session,
    ctx: &QueryContext<'_>,
    expression_id: destack_dir::LocalNodeId<Expression>,
    member_name: &str,
) -> Option<Span> {
    let full_span = get_dir_node_span(ctx.ast, ctx.dir, expression_id.into())?;
    let needle = member_name.as_bytes();

    // return the full span when the member name is empty
    if needle.is_empty() {
        return Some(full_span);
    }

    // read source content for the expression span
    let file = session.files.get(full_span.file);
    let content = match &file.content {
        FileContent::Text { content } => content.as_str(),
        FileContent::Json { content, .. } => content.as_str(),
        _ => return Some(full_span),
    };

    // slice the expression span from the source text
    let start = full_span.start as usize;
    let end = full_span.end as usize;
    let Some(slice) = content.get(start..end) else {
        return Some(full_span);
    };

    let hay = slice.as_bytes();

    // bail out when the member name is longer than the expression span
    if hay.len() < needle.len() {
        return Some(full_span);
    }

    // find the last dot within the expression span
    let mut dot_offset = None;
    for (idx, byte) in hay.iter().enumerate() {
        if *byte == b'.' {
            dot_offset = Some(idx);
        }
    }

    // prefer the identifier after the dot when it matches exactly
    if let Some(dot_offset) = dot_offset {
        let mut start = dot_offset + 1;
        while start < hay.len() && hay[start].is_ascii_whitespace() {
            start += 1;
        }
        if start + needle.len() <= hay.len() && hay[start..start + needle.len()] == *needle {
            return Some(Span::new(
                full_span.file,
                full_span.start + start as u32,
                full_span.start + start as u32 + needle.len() as u32,
            ));
        }
    }

    // fall back to searching for the last occurrence of the member name
    let mut offset = None;
    for idx in (0..=hay.len() - needle.len()).rev() {
        if hay[idx..idx + needle.len()] == *needle {
            offset = Some(idx);
            break;
        }
    }

    // return the last matched span when one exists
    if let Some(offset) = offset {
        return Some(Span::new(
            full_span.file,
            full_span.start + offset as u32,
            full_span.start + offset as u32 + needle.len() as u32,
        ));
    }

    Some(full_span)
}

/// Resolve a nominal type symbol from a type id.
fn resolve_nominal_symbol_from_type(
    types: &destack_dir::TypeTable,
    type_id: destack_dir::LocalTypeId,
) -> Option<GlobalSymbolId> {
    match types.get_type(type_id) {
        Type::Reference { symbol, .. } => Some(*symbol),
        Type::Object { .. } => types.symbol_for_instance_type(type_id),
        Type::Value { value } => resolve_nominal_symbol_from_type(types, *value),
        Type::ValueOf { right, .. } => resolve_nominal_symbol_from_type(types, *right),
        Type::ReferenceOf { right, .. } => resolve_nominal_symbol_from_type(types, *right),
        Type::PointerOf { right, .. } => resolve_nominal_symbol_from_type(types, *right),
        Type::Unary { right, .. } => resolve_nominal_symbol_from_type(types, *right),
        Type::Binary { left, right, .. } => resolve_nominal_symbol_from_type(types, *left)
            .or_else(|| resolve_nominal_symbol_from_type(types, *right)),
        Type::Conditional {
            left,
            right,
            then_type,
            else_type,
            ..
        } => resolve_nominal_symbol_from_type(types, *left)
            .or_else(|| resolve_nominal_symbol_from_type(types, *right))
            .or_else(|| resolve_nominal_symbol_from_type(types, *then_type))
            .or_else(|| resolve_nominal_symbol_from_type(types, *else_type)),
        Type::Union { elements } | Type::Intersection { elements } => {
            for element in elements {
                if let Some(symbol_id) = resolve_nominal_symbol_from_type(types, *element) {
                    return Some(symbol_id);
                }
            }
            None
        }
        _ => None,
    }
}

/// Resolve a static member name from a dynamic key.
fn member_key_name(session: &Session, key: &DynamicKey) -> Option<String> {
    match key {
        DynamicKey::Name(name_id) | DynamicKey::Number(name_id) => {
            Some(session.strings.get(*name_id).to_string())
        }
        _ => None,
    }
}

/// Resolve a member symbol from declaration members.
fn resolve_member_from_members(
    session: &Session,
    tree: &destack_dir::NodeTree,
    member_ids: &[destack_dir::LocalNodeId<Member>],
    member_name: &str,
) -> Option<destack_dir::LocalSymbolId> {
    // scan members for a matching key name
    for member_id in member_ids {
        let member = tree.get(*member_id);

        // skip members without a static key
        let Some(key) = member.key() else {
            continue;
        };

        // skip members whose key cannot be resolved to a name
        let Some(key_name) = member_key_name(session, key) else {
            continue;
        };

        // return the member symbol when the name matches
        if key_name == member_name {
            return Some(member.symbol());
        }
    }

    None
}

/// Resolve a member symbol from enum fields.
fn resolve_member_from_enum_fields(
    session: &Session,
    tree: &destack_dir::NodeTree,
    fields: &[destack_dir::LocalNodeId<EnumField>],
    member_name: &str,
) -> Option<destack_dir::LocalSymbolId> {
    // scan enum fields for a matching name
    for field_id in fields {
        let field = tree.get(*field_id);
        let name = session.strings.get(field.name).to_string();

        // return the field symbol when the name matches
        if name == member_name {
            return Some(field.symbol);
        }
    }

    None
}

/// Resolve a member symbol from a type declaration.
fn resolve_member_symbol_from_declaration(
    session: &Session,
    base_symbol: GlobalSymbolId,
    member_name: &str,
) -> Option<GlobalSymbolId> {
    // resolve the module query context
    let module = session.modules.get(base_symbol.module_id);
    let module = module.read();
    let ctx = session.query_context(&module)?;

    // resolve the primary declaration for the base symbol
    let symbols = ctx.symbols();
    let symbol = symbols.get_symbol(base_symbol.local_id);
    let declaration = symbol.primary_declaration?;
    drop(symbols);

    // convert the declaration into a declaration node id
    let Ok(local_decl_id): Result<destack_dir::LocalNodeId<Declaration>, _> =
        declaration.try_into()
    else {
        return None;
    };

    // read the declaration node from the tree
    let tree = ctx.tree();
    let declaration = tree.get(local_decl_id);

    // resolve members declared on the type
    if let Some(member_ids) = declaration.member_ids()
        && let Some(symbol_id) =
            resolve_member_from_members(session, &tree, member_ids, member_name)
    {
        return Some(global_symbol(ctx.module_id, symbol_id));
    }

    // resolve enum fields for enum declarations
    if let Declaration::Enum { fields, .. } = declaration
        && let Some(symbol_id) =
            resolve_member_from_enum_fields(session, &tree, fields, member_name)
    {
        return Some(global_symbol(ctx.module_id, symbol_id));
    }

    // return none when no member matches
    None
}

/// Resolve a member symbol from extensions for a target type.
pub(crate) fn resolve_extension_member_symbol(
    session: &Session,
    target_symbol: GlobalSymbolId,
    current_module_id: ModuleId,
    member_name: &str,
) -> Option<GlobalSymbolId> {
    // normalize the target symbol across imports and re exports
    let canonical_target = get_canonical_symbol(session, target_symbol);

    // scan all modules for extensions that target the canonical symbol
    for module in session.modules.iter() {
        let module = module.read();
        let Some(ctx) = session.query_context(&module) else {
            continue;
        };
        let types = ctx.types();

        // read extensions that target the canonical symbol
        let Some(extension_ids) = types.get_extensions_for_target(canonical_target) else {
            continue;
        };

        for extension_id in extension_ids {
            let extension = types.get_extension(*extension_id);

            // filter out non visible extensions
            let visible = match extension.kind {
                ExtensionKind::Inherent => true,
                ExtensionKind::Local => extension.symbol.module_id == current_module_id,
                ExtensionKind::Nominal => true,
            };
            if !visible {
                continue;
            }

            // resolve the member within the extension declaration
            if let Some(symbol_id) =
                resolve_member_symbol_from_declaration(session, extension.symbol, member_name)
            {
                return Some(symbol_id);
            }
        }
    }

    None
}

/// Resolve a nominal type symbol from a binding initializer or annotation.
fn resolve_nominal_symbol_from_initializer(
    session: &Session,
    ctx: &QueryContext<'_>,
    symbol_id: GlobalSymbolId,
) -> Option<GlobalSymbolId> {
    // resolve the primary declaration for the symbol
    let symbols = ctx.symbols();
    let symbol = symbols.get_symbol(symbol_id.local_id);
    let declaration = symbol.primary_declaration?;
    drop(symbols);

    // find the declarator that owns the declaration
    let dir_tree = ctx.tree();
    let declarator_id = match declaration.local_id.ty {
        NodeType::Declarator => declaration.local_id.try_into().ok(),
        NodeType::Pattern => {
            let parent_id = dir_tree.get_parent(declaration.local_id.id)?;
            if parent_id.ty == NodeType::Declarator {
                parent_id.try_into_typed().ok()
            } else {
                None
            }
        }
        _ => None,
    }?;

    // read the declarator node
    let declarator = dir_tree.get::<Declarator>(declarator_id);

    // resolve the nominal type from the type annotation when present
    if let Some(ty_expr_id) = declarator.ty
        && let Some(symbol) = resolve_nominal_symbol_from_type_expression(session, ctx, ty_expr_id)
    {
        return Some(symbol);
    }

    // fall back to the initializer expression
    let value_expr_id = declarator.value?;
    resolve_nominal_symbol_from_value_expression(session, ctx, value_expr_id)
}

/// Resolve a nominal type symbol from a value expression.
fn resolve_nominal_symbol_from_value_expression(
    session: &Session,
    ctx: &QueryContext<'_>,
    expression_id: destack_dir::LocalNodeId<Expression>,
) -> Option<GlobalSymbolId> {
    let dir_tree = ctx.tree();
    let expression = dir_tree.get::<Expression>(expression_id);

    match expression {
        Expression::TaggedScalarExpression { ty, .. }
        | Expression::TaggedTupleExpression { ty, .. }
        | Expression::TaggedObjectExpression { ty, .. } => {
            resolve_nominal_symbol_from_type_expression(session, ctx, *ty)
        }
        Expression::New { left, .. } => {
            resolve_nominal_symbol_from_type_expression(session, ctx, *left)
        }
        Expression::Parenthesized { expression } => {
            resolve_nominal_symbol_from_value_expression(session, ctx, *expression)
        }
        _ => None,
    }
}

/// Resolve a nominal type symbol from a type expression.
fn resolve_nominal_symbol_from_type_expression(
    session: &Session,
    ctx: &QueryContext<'_>,
    expression_id: destack_dir::LocalNodeId<Expression>,
) -> Option<GlobalSymbolId> {
    let dir_tree = ctx.tree();
    let expression = dir_tree.get::<Expression>(expression_id);
    let target_symbol = expression.target_symbol()?;

    let module = session.modules.get(target_symbol.module_id);
    let module = module.read();
    let ctx = session.query_context(&module)?;
    let symbols = ctx.symbols();
    let symbol = symbols.get_symbol(target_symbol.local_id);

    if symbol.space == SymbolSpace::Type || symbol.space == SymbolSpace::TypeValue {
        Some(target_symbol)
    } else {
        None
    }
}
