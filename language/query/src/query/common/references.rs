use std::collections::HashSet;

use destack_ast as ast;
use destack_base::StringId;
use destack_dir::{
    self as dir, DependencyItem, DependencyKind, DependencyMode, Expression, GlobalSymbolId,
    NodeType, StaticKey, SymbolSpace,
};
use destack_source::{FileId, ModuleId, NodeSpanType, Span};

use super::resolve::resolve_type_symbol_from_imports;
use super::span::{get_dir_node_main_span, get_dir_node_span};
use super::symbol::{
    get_canonical_symbol, get_member_access_name_span, is_dependency_alias_for_target,
    resolve_member_access_symbol,
};
use super::{QueryContext, visible_symbols, visible_symbols_full};
use destack_workspace::Session;

/// Options for collecting symbol references.
#[derive(Debug, Clone, Copy)]
pub struct ReferenceCollectionOptions<'a> {
    /// Whether to include direct expression references.
    pub include_expressions: bool,
    /// Whether to include member access references.
    pub include_members: bool,
    /// Whether to include dependency item references.
    pub include_dependencies: bool,
    /// Whether to include namespace member fallbacks.
    pub include_namespace_members: bool,
    /// Whether to skip dependency aliases that target the symbol.
    pub skip_dependency_aliases: bool,
    /// Whether to prefer name spans inside dependency items.
    pub use_dependency_name_spans: bool,
    /// An optional target name for name span resolution.
    pub target_name: Option<&'a str>,
    /// An optional file filter for collected spans.
    pub limit_to_file: Option<FileId>,
}

impl<'a> ReferenceCollectionOptions<'a> {
    /// Create options that collect all reference types.
    pub fn all() -> Self {
        Self {
            include_expressions: true,
            include_members: true,
            include_dependencies: true,
            include_namespace_members: false,
            skip_dependency_aliases: false,
            use_dependency_name_spans: false,
            target_name: None,
            limit_to_file: None,
        }
    }
}

/// Collect symbol references within a query context.
pub fn collect_symbol_references_in_context(
    session: &Session,
    ctx: &QueryContext<'_>,
    canonical_id: GlobalSymbolId,
    options: ReferenceCollectionOptions<'_>,
) -> Vec<Span> {
    // initialize the collected span list
    let mut spans = Vec::new();

    // compute namespace import aliases when namespace fallbacks are enabled
    let namespace_aliases = if options.include_namespace_members {
        namespace_import_aliases_for_module(ctx, canonical_id.module_id)
    } else {
        Vec::new()
    };

    // collect direct expression references
    if options.include_expressions {
        let expression_spans =
            collect_expression_reference_spans(session, ctx, canonical_id, options);
        spans.extend(expression_spans);
    }

    // collect member access references
    if options.include_members {
        let member_spans =
            collect_member_reference_spans(session, ctx, canonical_id, options, &namespace_aliases);
        spans.extend(member_spans);
    }

    // collect dependency item references
    if options.include_dependencies {
        let dependency_spans =
            collect_dependency_reference_spans(session, ctx, canonical_id, options);
        spans.extend(dependency_spans);
    }

    // collect interface type parameter spans when expression references are missing
    if let Some(target_name) = options.target_name {
        let interface_spans = collect_interface_type_parameter_spans(
            session,
            ctx,
            canonical_id,
            target_name,
            options,
        );
        spans.extend(interface_spans);
    }

    // return the collected spans
    spans
}

/// Collect direct expression reference spans.
fn collect_expression_reference_spans(
    session: &Session,
    ctx: &QueryContext<'_>,
    canonical_id: GlobalSymbolId,
    options: ReferenceCollectionOptions<'_>,
) -> Vec<Span> {
    // collect matching expression ids first to avoid borrow issues
    let matching_expression_ids: Vec<_> = {
        let dir_tree = ctx.tree();
        dir_tree
            .iter_nodes_of_type::<Expression>()
            .filter_map(|(expression_id, expression)| {
                let resolved_target_symbol = resolve_expression_target_symbol(
                    session,
                    ctx,
                    &dir_tree,
                    expression_id,
                    expression,
                    canonical_id,
                    options,
                );

                // skip member expressions when member references are enabled
                if options.include_members && matches!(expression, Expression::Member { .. }) {
                    return None;
                }
                if options.include_members
                    && expression_is_member_receiver_expression(&dir_tree, expression_id)
                {
                    return None;
                }

                // skip dependency aliases when requested
                let is_alias = if options.skip_dependency_aliases {
                    resolved_target_symbol
                        .map(|target_symbol| {
                            is_dependency_alias_for_target(
                                session,
                                ctx,
                                target_symbol,
                                canonical_id,
                            )
                        })
                        .unwrap_or(false)
                } else {
                    false
                };
                if is_alias {
                    return None;
                }

                // check for a canonical target match
                if let Some(target_symbol) = resolved_target_symbol {
                    if symbol_resolves_to_canonical(session, target_symbol, canonical_id) {
                        return Some(expression_id);
                    }

                    return None;
                }

                // fall back to matching unresolved type parameters by scope visibility
                if !matches!(expression, Expression::UnresolvedPath { .. }) {
                    return None;
                }
                if !symbol_is_type_parameter(session, canonical_id) {
                    return None;
                }
                if !symbol_visible_in_scope(ctx, &dir_tree, expression_id, canonical_id) {
                    return None;
                }

                Some(expression_id)
            })
            .collect()
    };

    // resolve spans for the matching expressions
    let dir_tree = ctx.tree();
    let mut spans = Vec::new();
    for expression_id in matching_expression_ids {
        let span = resolve_expression_reference_span(ctx, &dir_tree, expression_id);
        let Some(span) = span else {
            continue;
        };

        // filter out spans outside the requested file
        if options
            .limit_to_file
            .is_some_and(|limit_file| span.file != limit_file)
        {
            continue;
        }

        spans.push(span);
    }

    // capture member receiver spans when the receiver matches the target symbol
    if options.include_members {
        let dir_tree = ctx.tree();
        for (expression_id, expression) in dir_tree.iter_nodes_of_type::<Expression>() {
            let Expression::Member { left, .. } = expression else {
                continue;
            };
            if !member_receiver_matches_target(
                session,
                ctx,
                &dir_tree,
                *left,
                canonical_id,
                options.target_name,
            ) {
                continue;
            }

            let span = resolve_member_receiver_reference_span(ctx, expression_id, *left);
            let Some(span) = span else {
                continue;
            };

            if let Some(limit_file) = options.limit_to_file
                && span.file != limit_file
            {
                continue;
            }

            spans.push(span);
        }

        // recover namespace receiver spans from AST when DIR links are incomplete
        if options.include_namespace_members {
            let namespace_spans =
                collect_namespace_receiver_spans_from_ast(session, ctx, canonical_id, options);
            spans.extend(namespace_spans);
        }
    }

    // return the matching spans
    spans
}

/// Check whether an expression is used as the receiver of a member access.
fn expression_is_member_receiver_expression(
    dir_tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<Expression>,
) -> bool {
    let Some(parent) = dir_tree.get_parent(expression_id.id) else {
        return false;
    };
    if parent.ty != NodeType::Expression {
        return false;
    }

    let Ok(parent_expression_id) = parent.try_into() else {
        return false;
    };
    let parent_expression = dir_tree.get::<Expression>(parent_expression_id);
    matches!(
        parent_expression,
        Expression::Member { left, .. } if *left == expression_id
    )
}

/// Check whether a member receiver expression matches the canonical symbol target.
fn member_receiver_matches_target(
    session: &Session,
    ctx: &QueryContext<'_>,
    dir_tree: &dir::NodeTree,
    receiver_expression_id: dir::LocalNodeId<Expression>,
    canonical_id: GlobalSymbolId,
    target_name: Option<&str>,
) -> bool {
    let receiver_expression = dir_tree.get::<Expression>(receiver_expression_id);

    if let Some(target_symbol) = receiver_expression.target_symbol() {
        return symbol_resolves_to_canonical(session, target_symbol, canonical_id);
    }

    if canonical_id.module_id != ctx.module_id {
        return false;
    }

    let Some(receiver_name_id) =
        path_like_expression_last_segment_name_id(dir_tree, receiver_expression_id)
    else {
        return false;
    };

    if let Some(target_name) = target_name
        && session.strings.get(receiver_name_id) != target_name
    {
        return false;
    }

    if symbol_visible_in_scope_any_space(ctx, dir_tree, receiver_expression_id, canonical_id) {
        return true;
    }

    if !symbol_is_namespace_in_context(ctx, canonical_id) {
        return false;
    }

    let Some(target_name) = target_name else {
        return false;
    };
    if scope_has_conflicting_visible_name(
        session,
        ctx,
        dir_tree,
        receiver_expression_id,
        canonical_id,
        target_name,
    ) {
        return false;
    }

    true
}

/// Resolve the last segment name of a path-like expression.
fn path_like_expression_last_segment_name_id(
    dir_tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<Expression>,
) -> Option<StringId> {
    let mut current_expression_id = expression_id;

    loop {
        let expression = dir_tree.get::<Expression>(current_expression_id);
        match expression {
            Expression::UnresolvedPath { path, .. }
            | Expression::LocalReference { path, .. }
            | Expression::ModuleReference { path, .. }
            | Expression::GlobalReference { path, .. } => {
                return path.last_segment();
            }
            Expression::Parenthesized { expression } => {
                current_expression_id = *expression;
            }
            _ => return None,
        }
    }
}

/// Resolve the target symbol for an expression reference, including import fallbacks.
fn resolve_expression_target_symbol(
    session: &Session,
    ctx: &QueryContext<'_>,
    dir_tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<Expression>,
    expression: &Expression,
    canonical_id: GlobalSymbolId,
    options: ReferenceCollectionOptions<'_>,
) -> Option<GlobalSymbolId> {
    // prefer the resolved target symbol on the expression
    if let Some(target_symbol) = expression.target_symbol() {
        return Some(target_symbol);
    }

    // fall back to resolving unresolved paths through imports when possible
    let Expression::UnresolvedPath { path, .. } = expression else {
        return None;
    };
    let name_id = path.last_segment()?;
    if let Some(target_name) = options.target_name
        && session.strings.get(name_id) != target_name
    {
        return None;
    }

    if let Some(symbol_id) = resolve_type_symbol_from_imports(session, ctx, name_id) {
        return Some(symbol_id);
    }

    if canonical_id.module_id != ctx.module_id {
        return None;
    }

    let (scope_id, mark) = dir_tree.get_scope(expression_id);
    let symbols = ctx.symbols();
    let symbol = symbols.get_symbol(canonical_id.local_id);
    let is_visible = visible_symbols(&symbols, scope_id, mark, Some(symbol.space))
        .any(|visible| visible.id == canonical_id.local_id);
    is_visible.then_some(canonical_id)
}

/// Check whether a symbol resolves to the requested canonical id.
fn symbol_resolves_to_canonical(
    session: &Session,
    symbol_id: GlobalSymbolId,
    canonical_id: GlobalSymbolId,
) -> bool {
    // allow exact symbol identity matches before canonical expansion
    if symbol_id == canonical_id {
        return true;
    }

    // prefer the canonical symbol chain when it matches
    if get_canonical_symbol(session, symbol_id) == canonical_id {
        return true;
    }

    // fall back to following target symbol chains for alias symbols
    let mut current_symbol = symbol_id;
    let mut visited = HashSet::new();

    // walk target symbols until we resolve or cycle
    while visited.insert(current_symbol) {
        // load the module context for the current symbol
        let module = session.modules.get(current_symbol.module_id);
        let module = module.read();
        let Some(ctx) = crate::query_context(session, &module) else {
            return false;
        };

        // read the next target symbol
        let symbols = ctx.symbols();
        let symbol = symbols.get_symbol(current_symbol.local_id);
        let Some(target_symbol) = symbol.target_symbol else {
            return false;
        };

        // stop when the target resolves to the requested canonical id
        if get_canonical_symbol(session, target_symbol) == canonical_id {
            return true;
        }

        // continue walking the target chain
        current_symbol = target_symbol;
    }

    false
}

/// Resolve the best span for a reference expression.
fn resolve_expression_reference_span(
    ctx: &QueryContext<'_>,
    dir_tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<Expression>,
) -> Option<Span> {
    let span = get_dir_node_main_span(ctx.ast, ctx.dir, expression_id.into())
        .or_else(|| get_dir_node_span(ctx.ast, ctx.dir, expression_id.into()))?;

    let Some(parent) = dir_tree.get_parent(expression_id.id) else {
        return Some(span);
    };
    if parent.ty != NodeType::Expression {
        return Some(span);
    }

    let Ok(parent_expression_id) = parent.try_into() else {
        return Some(span);
    };
    let parent_expression = dir_tree.get::<Expression>(parent_expression_id);
    let Expression::Member { left, .. } = parent_expression else {
        return Some(span);
    };
    if *left != expression_id {
        return Some(span);
    }

    let Some(member_name_span) = get_member_access_name_span(ctx, parent_expression_id) else {
        return Some(span);
    };
    let receiver_end = member_name_span.start.saturating_sub(1);

    // clamp spans that include `.member` to only the receiver expression
    if span.file == member_name_span.file && span.start < receiver_end && span.end >= receiver_end {
        return Some(Span::new(span.file, span.start, receiver_end));
    }

    // recover receiver spans when direct mapping points at member names
    if let Some(member_span) = get_dir_node_span(ctx.ast, ctx.dir, parent_expression_id.into())
        && member_span.file == member_name_span.file
        && member_span.start < receiver_end
    {
        return Some(Span::new(member_span.file, member_span.start, receiver_end));
    }

    Some(span)
}

/// Resolve the best span for a member receiver expression.
fn resolve_member_receiver_reference_span(
    ctx: &QueryContext<'_>,
    member_expression_id: dir::LocalNodeId<Expression>,
    receiver_expression_id: dir::LocalNodeId<Expression>,
) -> Option<Span> {
    let dir_tree = ctx.tree();
    let mut receiver_expression_id = receiver_expression_id;
    loop {
        let receiver_expression = dir_tree.get::<Expression>(receiver_expression_id);
        let Expression::Parenthesized { expression } = receiver_expression else {
            break;
        };
        receiver_expression_id = *expression;
    }
    drop(dir_tree);

    let receiver_span = ast_member_receiver_span(ctx, member_expression_id)
        .or_else(|| get_dir_node_main_span(ctx.ast, ctx.dir, receiver_expression_id.into()))
        .or_else(|| get_dir_node_span(ctx.ast, ctx.dir, receiver_expression_id.into()))?;

    let Some(member_name_span) = get_member_access_name_span(ctx, member_expression_id) else {
        return Some(receiver_span);
    };
    let receiver_end = member_name_span.start.saturating_sub(1);

    // clamp spans that include `.member` to only the receiver expression
    if receiver_span.file == member_name_span.file
        && receiver_span.start < receiver_end
        && receiver_span.end >= receiver_end
    {
        return Some(Span::new(
            receiver_span.file,
            receiver_span.start,
            receiver_end,
        ));
    }

    // recover from member spans that only map to the member name
    if let Some(member_span) = get_dir_node_span(ctx.ast, ctx.dir, member_expression_id.into())
        && member_span.file == member_name_span.file
        && member_span.start < receiver_end
    {
        return Some(Span::new(member_span.file, member_span.start, receiver_end));
    }

    Some(receiver_span)
}

/// Resolve a member receiver span from AST when DIR source spans are unavailable.
fn ast_member_receiver_span(
    ctx: &QueryContext<'_>,
    member_expression_id: dir::LocalNodeId<Expression>,
) -> Option<Span> {
    let dir_tree = ctx.tree();
    let source_id = dir_tree.get_source(member_expression_id.id);
    drop(dir_tree);

    let expression_id = ast::LocalNodeId::<ast::Expression>::new(source_id);
    let expression = ctx.ast.tree.get(expression_id);
    let ast::Expression::Member { left, .. } = expression else {
        return None;
    };
    ast_expression_main_span_without_parentheses(ctx, *left)
}

/// Check whether a symbol represents a static type parameter.
fn symbol_is_type_parameter(session: &Session, symbol_id: GlobalSymbolId) -> bool {
    // resolve the module and query context for the symbol
    let module = session.modules.get(symbol_id.module_id);
    let module = module.read();
    let Some(ctx) = crate::query_context(session, &module) else {
        return false;
    };

    // ensure the symbol is declared by a parameter node in type space
    let symbols = ctx.symbols();
    let symbol = symbols.get_symbol(symbol_id.local_id);
    let Some(declaration) = symbol.primary_declaration else {
        return false;
    };
    if declaration.local_id.ty != NodeType::Parameter {
        return false;
    }

    matches!(symbol.space, SymbolSpace::Type | SymbolSpace::TypeValue)
}

/// Check whether a symbol is visible at an expression's scope.
fn symbol_visible_in_scope(
    ctx: &QueryContext<'_>,
    dir_tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<Expression>,
    symbol_id: GlobalSymbolId,
) -> bool {
    if symbol_id.module_id != ctx.module_id {
        return false;
    }

    let (scope_id, mark) = dir_tree.get_scope(expression_id);
    let symbols = ctx.symbols();
    let symbol = symbols.get_symbol(symbol_id.local_id);
    if visible_symbols(&symbols, scope_id, mark, Some(symbol.space))
        .any(|visible| visible.id == symbol_id.local_id)
    {
        return true;
    }

    visible_symbols_full(&symbols, scope_id, Some(symbol.space))
        .any(|visible| visible.id == symbol_id.local_id)
}

/// Check whether a symbol is visible at an expression's scope in any symbol space.
fn symbol_visible_in_scope_any_space(
    ctx: &QueryContext<'_>,
    dir_tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<Expression>,
    symbol_id: GlobalSymbolId,
) -> bool {
    if symbol_id.module_id != ctx.module_id {
        return false;
    }

    let (scope_id, mark) = dir_tree.get_scope(expression_id);
    let symbols = ctx.symbols();
    if visible_symbols(&symbols, scope_id, mark, None)
        .any(|visible| visible.id == symbol_id.local_id)
    {
        return true;
    }

    visible_symbols_full(&symbols, scope_id, None).any(|visible| visible.id == symbol_id.local_id)
}

/// Check whether a canonical symbol is a namespace in the current query context.
fn symbol_is_namespace_in_context(ctx: &QueryContext<'_>, symbol_id: GlobalSymbolId) -> bool {
    if symbol_id.module_id != ctx.module_id {
        return false;
    }

    let symbols = ctx.symbols();
    let symbol = symbols.get_symbol(symbol_id.local_id);
    let Some(declaration) = symbol.primary_declaration else {
        return false;
    };
    drop(symbols);

    if declaration.local_id.ty != NodeType::Declaration {
        return false;
    }

    let Ok(declaration_id) = declaration.local_id.try_into() else {
        return false;
    };
    let dir_tree = ctx.tree();
    let declaration = dir_tree.get::<dir::Declaration>(declaration_id);
    matches!(declaration, dir::Declaration::Namespace { .. })
}

/// Check whether a receiver scope has a conflicting visible name for a fallback match.
fn scope_has_conflicting_visible_name(
    session: &Session,
    ctx: &QueryContext<'_>,
    dir_tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<Expression>,
    canonical_id: GlobalSymbolId,
    target_name: &str,
) -> bool {
    let (scope_id, mark) = dir_tree.get_scope(expression_id);
    let symbols = ctx.symbols();
    let has_conflict = |key: StaticKey, symbol_id: dir::LocalSymbolId| {
        let StaticKey::Name(name_id) = key else {
            return false;
        };
        if session.strings.get(name_id) != target_name {
            return false;
        }

        symbol_id != canonical_id.local_id
    };

    if visible_symbols(&symbols, scope_id, mark, None)
        .any(|visible| has_conflict(visible.key, visible.id))
    {
        return true;
    }

    visible_symbols_full(&symbols, scope_id, None)
        .any(|visible| has_conflict(visible.key, visible.id))
}

/// Collect reference spans for type parameters owned by interfaces.
fn collect_interface_type_parameter_spans(
    session: &Session,
    ctx: &QueryContext<'_>,
    canonical_id: GlobalSymbolId,
    _target_name: &str,
    options: ReferenceCollectionOptions<'_>,
) -> Vec<Span> {
    if canonical_id.module_id != ctx.module_id {
        return Vec::new();
    }
    if !symbol_is_type_parameter(session, canonical_id) {
        return Vec::new();
    }

    let symbols = ctx.symbols();
    let symbol = symbols.get_symbol(canonical_id.local_id);
    let Some(declaration) = symbol.primary_declaration else {
        return Vec::new();
    };
    if declaration.local_id.ty != NodeType::Parameter {
        return Vec::new();
    }
    drop(symbols);

    let dir_tree = ctx.tree();
    let mut current = declaration.local_id.id;
    while let Some(parent) = dir_tree.get_parent(current) {
        if parent.ty == NodeType::Declaration {
            let Ok(declaration_id) = parent.try_into() else {
                break;
            };
            let declaration = dir_tree.get::<dir::Declaration>(declaration_id);
            if !matches!(declaration, dir::Declaration::Interface { .. }) {
                break;
            }

            let Some(interface_span) = get_dir_node_span(ctx.ast, ctx.dir, parent) else {
                break;
            };

            let mut spans = Vec::new();
            for (expression_id, expression) in dir_tree.iter_nodes_of_type::<Expression>() {
                let Some(expression_span) =
                    get_dir_node_span(ctx.ast, ctx.dir, expression_id.into())
                else {
                    continue;
                };
                if expression_span.file != interface_span.file {
                    continue;
                }
                if expression_span.start < interface_span.start
                    || expression_span.end > interface_span.end
                {
                    continue;
                }

                let Some(target_symbol) = resolve_expression_target_symbol(
                    session,
                    ctx,
                    &dir_tree,
                    expression_id,
                    expression,
                    canonical_id,
                    options,
                ) else {
                    continue;
                };
                if !symbol_resolves_to_canonical(session, target_symbol, canonical_id) {
                    continue;
                }

                let Some(span) = get_dir_node_main_span(ctx.ast, ctx.dir, expression_id.into())
                    .or(Some(expression_span))
                else {
                    continue;
                };
                if let Some(limit_file) = options.limit_to_file
                    && span.file != limit_file
                {
                    continue;
                }

                spans.push(span);
            }
            return spans;
        }

        current = parent.id;
    }

    Vec::new()
}

/// Collect namespace member receiver spans from AST when DIR receiver links are missing.
fn collect_namespace_receiver_spans_from_ast(
    session: &Session,
    ctx: &QueryContext<'_>,
    canonical_id: GlobalSymbolId,
    options: ReferenceCollectionOptions<'_>,
) -> Vec<Span> {
    if canonical_id.module_id != ctx.module_id {
        return Vec::new();
    }
    if !symbol_is_namespace_in_context(ctx, canonical_id) {
        return Vec::new();
    }
    let Some(target_name) = options.target_name else {
        return Vec::new();
    };

    let mut spans = Vec::new();
    let dir_tree = ctx.tree();
    for expression_id in ctx.ast.tree.iter_nodes::<ast::Expression>() {
        let expression = ctx.ast.tree.get(expression_id);
        let (source_node_id, span) = match expression {
            ast::Expression::Member { left, .. } => {
                if !ast_path_expression_matches_name(ctx, *left, target_name) {
                    continue;
                }

                let Some(receiver_span) = ast_expression_main_span_without_parentheses(ctx, *left)
                else {
                    continue;
                };
                (left.id, receiver_span)
            }
            ast::Expression::Path { path, .. } => {
                let Some(first_segment) = path.segments.first() else {
                    continue;
                };
                if path.segments.len() < 2 || ctx.ast.strings.get(*first_segment) != target_name {
                    continue;
                }

                let expression_span = ctx.ast.tree.source_map.get(expression_id.id);
                let Some(receiver_span) = first_identifier_span_in_expression(ctx, expression_span)
                else {
                    continue;
                };

                (expression_id.id, receiver_span)
            }
            _ => continue,
        };

        if let Some(dir_node_id) = dir_tree.get_node_id_by_source_id(source_node_id)
            && dir_node_id.ty == NodeType::Expression
            && let Ok(dir_expression_id) = dir_node_id.try_into()
            && scope_has_conflicting_visible_name(
                session,
                ctx,
                &dir_tree,
                dir_expression_id,
                canonical_id,
                target_name,
            )
        {
            continue;
        }

        if let Some(limit_file) = options.limit_to_file
            && span.file != limit_file
        {
            continue;
        }

        spans.push(span);
    }

    spans
}

/// Resolve an AST expression main span, unwrapping parenthesized expressions.
fn ast_expression_main_span_without_parentheses(
    ctx: &QueryContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> Option<Span> {
    let mut expression_id = expression_id;
    loop {
        let expression = ctx.ast.tree.get(expression_id);
        let ast::Expression::Parenthesized { expression } = expression else {
            break;
        };
        expression_id = *expression;
    }

    let expression = ctx.ast.tree.get(expression_id);
    if let ast::Expression::Path { .. } = expression {
        let expression_span = ctx.ast.tree.source_map.get(expression_id.id);
        if let Some(identifier_span) = last_identifier_span_in_expression(ctx, expression_span) {
            return Some(identifier_span);
        }
    }

    let span = ctx
        .ast
        .tree
        .source_map
        .get_main(expression_id.id)
        .unwrap_or_else(|| ctx.ast.tree.source_map.get(expression_id.id));
    Some(Span::new(ctx.file_id, span.start, span.end))
}

/// Check whether an AST path-like expression ends with the requested name.
fn ast_path_expression_matches_name(
    ctx: &QueryContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
    target_name: &str,
) -> bool {
    let expression = ctx.ast.tree.get(expression_id);
    match expression {
        ast::Expression::Path { path, .. } => path
            .segments
            .last()
            .is_some_and(|name_id| ctx.ast.strings.get(*name_id) == target_name),
        ast::Expression::Parenthesized { expression } => {
            ast_path_expression_matches_name(ctx, *expression, target_name)
        }
        _ => false,
    }
}

/// Resolve the first identifier token span inside an expression span.
fn first_identifier_span_in_expression(
    ctx: &QueryContext<'_>,
    expression_span: Span,
) -> Option<Span> {
    for token in &ctx.ast.tokens {
        if token.span.file != ctx.file_id {
            continue;
        }
        if token.token.ty != ast::TokenType::Identifier {
            continue;
        }
        if token.span.start < expression_span.start || token.span.end > expression_span.end {
            continue;
        }

        return Some(token.span);
    }

    None
}

/// Resolve the last identifier token span inside an expression span.
fn last_identifier_span_in_expression(
    ctx: &QueryContext<'_>,
    expression_span: Span,
) -> Option<Span> {
    let mut last_identifier_span = None;
    for token in &ctx.ast.tokens {
        if token.span.file != ctx.file_id {
            continue;
        }
        if token.token.ty != ast::TokenType::Identifier {
            continue;
        }
        if token.span.start < expression_span.start || token.span.end > expression_span.end {
            continue;
        }

        last_identifier_span = Some(token.span);
    }

    last_identifier_span
}

/// Collect member access reference spans.
fn collect_member_reference_spans(
    session: &Session,
    ctx: &QueryContext<'_>,
    canonical_id: GlobalSymbolId,
    options: ReferenceCollectionOptions<'_>,
    namespace_aliases: &[GlobalSymbolId],
) -> Vec<Span> {
    // initialize the collected span list
    let mut spans = Vec::new();

    // scan member expressions for matches and namespace fallbacks
    let dir_tree = ctx.tree();
    for (expression_id, expression) in dir_tree.iter_nodes_of_type::<Expression>() {
        let Expression::Member { left, name, .. } = expression else {
            continue;
        };

        // resolve the member symbol through normal resolution first
        if let Some(member_symbol) =
            resolve_member_access_symbol(session, ctx, expression_id, *left, *name)
        {
            if !symbol_resolves_to_canonical(session, member_symbol, canonical_id) {
                continue;
            }

            let Some(span) = get_member_access_name_span(ctx, expression_id) else {
                continue;
            };

            // filter out spans outside the requested file
            if let Some(limit_file) = options.limit_to_file
                && span.file != limit_file
            {
                continue;
            }

            spans.push(span);
            continue;
        }

        // apply namespace import fallback when enabled
        if !options.include_namespace_members {
            continue;
        }

        // require a target name for namespace member matching
        let Some(target_name) = options.target_name else {
            continue;
        };

        // resolve the receiver symbol and require it to be a namespace alias
        let left_expr = dir_tree.get::<Expression>(*left);
        let Some(receiver_symbol) = left_expr.target_symbol() else {
            continue;
        };
        if !namespace_aliases.contains(&receiver_symbol) {
            continue;
        }

        // require the member name to match the requested target name
        let member_name = session.strings.get(*name).to_string();
        if member_name != target_name {
            continue;
        }

        let Some(span) = get_member_access_name_span(ctx, expression_id) else {
            continue;
        };

        // filter out spans outside the requested file
        if let Some(limit_file) = options.limit_to_file
            && span.file != limit_file
        {
            continue;
        }

        spans.push(span);
    }

    // return the collected spans
    spans
}

/// Collect dependency item reference spans.
fn collect_dependency_reference_spans(
    session: &Session,
    ctx: &QueryContext<'_>,
    canonical_id: GlobalSymbolId,
    options: ReferenceCollectionOptions<'_>,
) -> Vec<Span> {
    // collect matching dependency item ids first to avoid borrow issues
    let matching_dependency_ids: Vec<_> = {
        let dir_tree = ctx.tree();
        dir_tree
            .iter_nodes_of_type::<DependencyItem>()
            .filter_map(|(item_id, item)| {
                let target = item.target_symbol()?;
                symbol_resolves_to_canonical(session, target, canonical_id).then_some(item_id)
            })
            .collect()
    };

    // resolve spans for the matching dependency items
    let mut spans = Vec::new();
    for item_id in matching_dependency_ids {
        let span = if options.use_dependency_name_spans {
            dependency_item_name_span(session, ctx, item_id, options.target_name)
        } else {
            None
        }
        .or_else(|| get_dir_node_main_span(ctx.ast, ctx.dir, item_id.into()));

        let Some(span) = span else {
            continue;
        };

        // filter out spans outside the requested file
        if let Some(limit_file) = options.limit_to_file
            && span.file != limit_file
        {
            continue;
        }

        spans.push(span);
    }

    // return the collected spans
    spans
}

/// Resolve a dependency item's name span when a target name is provided.
fn dependency_item_name_span(
    session: &Session,
    ctx: &QueryContext<'_>,
    item_id: dir::LocalNodeId<DependencyItem>,
    target_name: Option<&str>,
) -> Option<Span> {
    // fall back to the main span when no name was provided
    let Some(target_name) = target_name else {
        return get_dir_node_main_span(ctx.ast, ctx.dir, item_id.into());
    };

    // resolve name + alias for matching
    let dir_tree = ctx.tree();
    let item = dir_tree.get::<DependencyItem>(item_id);
    let (name_id, alias_id) = match item {
        DependencyItem::Remote { name, alias, .. }
        | DependencyItem::Local { name, alias, .. }
        | DependencyItem::UnresolvedRemote { name, alias, .. }
        | DependencyItem::UnresolvedLocal { name, alias, .. } => {
            (name.map(|name| name.string()), *alias)
        }
        DependencyItem::Value { .. } => (None, None),
    };

    // resolve the ast node id for span lookup
    let ast_node_id = dir_tree.get_source(item_id.id);

    // match the remote/local item name first
    if let Some(name_id) = name_id
        && session.strings.get(name_id) == target_name
    {
        let span = ctx
            .ast
            .tree
            .get_side_span_by_id(ast_node_id, NodeSpanType::Type)?;
        return Some(Span::new(ctx.file_id, span.start, span.end));
    }

    // fall back to alias when present
    if let Some(alias_id) = alias_id
        && session.strings.get(alias_id) == target_name
    {
        let span = ctx
            .ast
            .tree
            .get_side_span_by_id(ast_node_id, NodeSpanType::Main)?;
        return Some(Span::new(ctx.file_id, span.start, span.end));
    }

    None
}

/// Collect namespace import aliases that target a module.
fn namespace_import_aliases_for_module(
    ctx: &QueryContext<'_>,
    module_id: ModuleId,
) -> Vec<GlobalSymbolId> {
    // scan dependency items for namespace imports to the target module
    let dir_tree = ctx.tree();
    dir_tree
        .iter_nodes_of_type::<DependencyItem>()
        .filter_map(|(_item_id, item)| {
            let DependencyItem::Remote {
                mode,
                kind,
                symbol,
                target_module,
                ..
            } = item
            else {
                return None;
            };

            // require namespace value imports with a concrete symbol
            if *mode != DependencyMode::Namespace || *kind != DependencyKind::Value {
                return None;
            }

            let local_symbol = symbol.as_ref()?;
            let target_module_id = target_module
                .for_kind(*kind)
                .and_then(|target| target.module_id())?;
            if target_module_id != module_id {
                return None;
            }

            Some(GlobalSymbolId::new(ctx.module_id, *local_symbol))
        })
        .collect()
}
