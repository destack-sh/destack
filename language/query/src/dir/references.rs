use std::collections::HashSet;

use destack_ast as ast;
use destack_dir::{
    self as dir, DependencyBinding, DependencyItem, DependencySpace, Expression, GlobalSymbolId,
    NodeType, Resolution,
};
use destack_source::{FileId, ModuleId, NodeSpanRegion, NodeSpanType, ProfileId, Span};
use destack_workspace::{Repository, Revision};

use super::import::is_dependency_alias_for_target;
use super::namespace::resolve_path_segment_symbol;
use super::nominal::resolve_member_access_symbol;
use super::{
    get_canonical_symbol, get_member_access_name_span, get_path_segment_span,
    resolve_expression_symbol, resolve_namespace_receiver_symbol, symbol_matches_reference_target,
};
use crate::ast::{get_node_tree_main_span, get_node_tree_span};
use crate::core::{AstQueryContext, DirQueryContext, query_context_for_profile};

/// Options for collecting symbol references.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ReferenceCollectionOptions<'a> {
    /// Whether to include direct expression references.
    pub include_expressions: bool,
    /// Whether to include member access references.
    pub include_members: bool,
    /// Whether to include dependency item references.
    pub include_dependencies: bool,
    /// Whether to include namespace receiver references.
    pub include_namespace_receivers: bool,
    /// Whether to skip dependency aliases that target the symbol.
    pub skip_dependency_aliases: bool,
    /// Whether to prefer name spans inside dependency items.
    pub use_dependency_name_spans: bool,
    /// An optional target name for name span resolution.
    pub target_name: Option<&'a str>,
    /// Whether expression-like references must keep the same visible name.
    pub require_target_name_match: bool,
    /// An optional file filter for collected spans.
    pub limit_to_file: Option<FileId>,
}

/// Build reference index target keys for one module.
pub(crate) fn build_reference_targets_for_module(
    repository: &Repository,
    revision: Revision,
    module_id: ModuleId,
    profile_id: ProfileId,
) -> Vec<GlobalSymbolId> {
    let Some(ctx) = query_context_for_profile(repository, revision, module_id, profile_id) else {
        return Vec::new();
    };

    let mut targets = HashSet::new();
    let dir = ctx.dir();
    let dir_tree = dir.tree();

    // expressions
    for (expression_id, expression) in dir_tree.iter_nodes_of_type::<Expression>() {
        if let Some(target_symbol) =
            resolve_expression_target_symbol(dir, expression_id, expression)
        {
            insert_reference_target_keys(repository, dir.revision(), &mut targets, target_symbol);
        }

        if let Expression::Member { left, .. } = expression
            && let Some(receiver_symbol) = resolve_namespace_receiver_symbol(dir, *left)
        {
            insert_reference_target_keys(repository, dir.revision(), &mut targets, receiver_symbol);
        }

        if let Expression::Member { .. } = expression
            && let Some(member_symbol) = resolve_member_access_symbol(dir, expression_id)
        {
            insert_reference_target_keys(repository, dir.revision(), &mut targets, member_symbol);
        }

        let resolution_id = dir
            .types()
            .node_resolution_id(expression_id.into_global_any(dir.module_id()));
        if let Some(resolution_id) = resolution_id {
            let resolution = dir.types().get_resolution(resolution_id);
            match resolution {
                Resolution::Static { candidate, .. } => {
                    insert_reference_target_keys(
                        repository,
                        dir.revision(),
                        &mut targets,
                        candidate.target_symbol,
                    );
                }
                Resolution::Dynamic { candidates, .. }
                | Resolution::Unresolved { candidates, .. } => {
                    for candidate in candidates {
                        insert_reference_target_keys(
                            repository,
                            dir.revision(),
                            &mut targets,
                            candidate.target_symbol,
                        );
                    }
                }
                Resolution::Builtin { .. } => {}
            }
        }

        match expression {
            Expression::UnresolvedPath { path, .. }
            | Expression::LocalReference { path, .. }
            | Expression::ModuleReference { path, .. }
            | Expression::GlobalReference { path, .. } => {
                for segment_index in 0..path.segments.len() {
                    let segment_index =
                        u16::try_from(segment_index).expect("path segment index overflow");

                    if let Some(segment_symbol) =
                        resolve_path_segment_symbol(dir, expression_id, segment_index)
                    {
                        insert_reference_target_keys(
                            repository,
                            dir.revision(),
                            &mut targets,
                            segment_symbol,
                        );
                    }
                }
            }
            _ => {}
        }
    }

    // dependency items
    for (_item_id, item) in dir_tree.iter_nodes_of_type::<DependencyItem>() {
        if let Some(local_symbol) = item.symbol() {
            let symbol_id = GlobalSymbolId::new(dir.module_id(), local_symbol);
            insert_reference_target_keys(repository, dir.revision(), &mut targets, symbol_id);
        }

        if let Some(target_symbol) = item.target_symbol() {
            insert_reference_target_keys(repository, dir.revision(), &mut targets, target_symbol);
        }
    }

    targets.into_iter().collect()
}

/// Insert the usable reference target keys for one observed symbol.
fn insert_reference_target_keys(
    repository: &Repository,
    revision: Revision,
    targets: &mut HashSet<GlobalSymbolId>,
    symbol_id: GlobalSymbolId,
) {
    targets.insert(symbol_id);
    targets.insert(get_canonical_symbol(repository, revision, symbol_id));
}

/// Collect symbol references within a query context.
pub(crate) fn collect_symbol_references_in_context(
    repository: &Repository,
    ast: AstQueryContext<'_>,
    dir: DirQueryContext<'_>,
    canonical_id: GlobalSymbolId,
    options: ReferenceCollectionOptions<'_>,
) -> Vec<Span> {
    // initialize the collected span list
    let mut spans = Vec::new();

    // compute namespace import aliases when namespace receiver matching is enabled
    let namespace_aliases = if options.include_namespace_receivers {
        namespace_import_aliases_for_module(dir, canonical_id.module_id)
    } else {
        Vec::new()
    };

    // collect direct expression references
    if options.include_expressions {
        let expression_spans =
            collect_expression_reference_spans(repository, ast, dir, canonical_id, options);
        spans.extend(expression_spans);
    }

    // collect member access references
    if options.include_members {
        let member_spans = collect_member_reference_spans(
            repository,
            ast,
            dir,
            canonical_id,
            options,
            &namespace_aliases,
        );
        spans.extend(member_spans);
    }

    // collect dependency item references
    if options.include_dependencies {
        let dependency_spans =
            collect_dependency_reference_spans(repository, ast, dir, canonical_id, options);
        spans.extend(dependency_spans);
    }

    // return the collected spans
    spans
}

/// Collect direct expression reference spans.
fn collect_expression_reference_spans(
    repository: &Repository,
    ast: AstQueryContext<'_>,
    dir: DirQueryContext<'_>,
    canonical_id: GlobalSymbolId,
    options: ReferenceCollectionOptions<'_>,
) -> Vec<Span> {
    // collect matching expression ids first to avoid borrow issues
    let matching_expression_ids: Vec<_> = {
        let dir_tree = dir.tree();
        dir_tree
            .iter_nodes_of_type::<Expression>()
            .filter_map(|(expression_id, expression)| {
                let resolved_target_symbol =
                    resolve_expression_target_symbol(dir, expression_id, expression);

                // skip member expressions when member references are enabled
                if options.include_members && matches!(expression, Expression::Member { .. }) {
                    return None;
                }
                // skip dependency aliases when requested
                let is_alias = if options.skip_dependency_aliases {
                    resolved_target_symbol
                        .map(|target_symbol| {
                            is_dependency_alias_for_target(
                                repository,
                                dir,
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
                    if symbol_matches_reference_target(
                        repository,
                        dir.revision(),
                        target_symbol,
                        canonical_id,
                    ) {
                        return Some(expression_id);
                    }

                    return None;
                }

                None
            })
            .collect()
    };

    // resolve spans for the matching expressions
    let dir_tree = dir.tree();
    let mut spans = Vec::new();
    for expression_id in matching_expression_ids {
        let span = resolve_expression_reference_span(ast, dir, dir_tree, expression_id);
        let Some(span) = span else {
            continue;
        };

        // keep rename style collection on the same visible name
        if !span_matches_target_name(repository, dir.revision(), span, options) {
            continue;
        }

        // filter out spans outside the requested file
        if options
            .limit_to_file
            .is_some_and(|limit_file| span.file != limit_file)
        {
            continue;
        }

        spans.push(span);
    }

    // capture plain path segments from multi segment path expressions
    for (expression_id, expression) in dir_tree.iter_nodes_of_type::<Expression>() {
        let path = match expression {
            Expression::UnresolvedPath { path, .. }
            | Expression::LocalReference { path, .. }
            | Expression::ModuleReference { path, .. }
            | Expression::GlobalReference { path, .. } => path,
            _ => continue,
        };
        if path.segments.len() < 2 {
            continue;
        }

        for segment_index in 0..path.segments.len() {
            let segment_index =
                u16::try_from(segment_index).expect("path reference segment index overflow");

            let Some(segment_symbol) =
                resolve_path_segment_symbol(dir, expression_id, segment_index)
            else {
                continue;
            };
            if !symbol_matches_reference_target(
                repository,
                dir.revision(),
                segment_symbol,
                canonical_id,
            ) {
                continue;
            }

            let Some(span) = get_path_segment_span(ast, dir, expression_id, segment_index) else {
                continue;
            };

            // keep rename style collection on the same visible name
            if !span_matches_target_name(repository, dir.revision(), span, options) {
                continue;
            }

            if options
                .limit_to_file
                .is_some_and(|limit_file| span.file != limit_file)
            {
                continue;
            }

            spans.push(span);
        }
    }

    // capture member receiver spans when the receiver matches the target symbol
    if options.include_members {
        let dir_tree = dir.tree();
        for (expression_id, expression) in dir_tree.iter_nodes_of_type::<Expression>() {
            let Expression::Member { left, .. } = expression else {
                continue;
            };
            if !member_receiver_matches_target(
                repository,
                dir,
                dir_tree,
                *left,
                canonical_id,
                options.target_name,
            ) {
                continue;
            }

            let span = resolve_member_receiver_reference_span(ast, dir, expression_id, *left);
            let Some(span) = span else {
                continue;
            };

            // keep rename style collection on the same visible name
            if !span_matches_target_name(repository, dir.revision(), span, options) {
                continue;
            }

            if let Some(limit_file) = options.limit_to_file
                && span.file != limit_file
            {
                continue;
            }

            spans.push(span);
        }
    }

    // return the matching spans
    spans
}

/// Check whether one collected span keeps the requested target name.
fn span_matches_target_name(
    repository: &Repository,
    revision: Revision,
    span: Span,
    options: ReferenceCollectionOptions<'_>,
) -> bool {
    if !options.require_target_name_match {
        return true;
    }

    let Some(target_name) = options.target_name else {
        return true;
    };

    let Some(file) = repository.file(revision, span.file).ok().flatten() else {
        return false;
    };
    let file_text = file.text();
    let start = span.start as usize;
    let end = span.end as usize;
    if start > end || end > file_text.len() {
        return false;
    }

    file_text
        .get(start..end)
        .is_some_and(|span_text| span_text == target_name)
}

/// Check whether a member receiver expression matches the canonical symbol target.
fn member_receiver_matches_target(
    repository: &Repository,
    dir: DirQueryContext<'_>,
    _dir_tree: &dir::Tree,
    receiver_expression_id: dir::LocalNodeId<Expression>,
    canonical_id: GlobalSymbolId,
    _target_name: Option<&str>,
) -> bool {
    let resolved_symbol = resolve_namespace_receiver_symbol(dir, receiver_expression_id);
    let Some(target_symbol) = resolved_symbol else {
        return false;
    };

    symbol_matches_reference_target(repository, dir.revision(), target_symbol, canonical_id)
}

/// Resolve the target symbol for an expression reference.
fn resolve_expression_target_symbol(
    dir: DirQueryContext<'_>,
    expression_id: dir::LocalNodeId<Expression>,
    expression: &Expression,
) -> Option<GlobalSymbolId> {
    if expression_is_member_receiver_expression(dir.tree(), expression_id) {
        return resolve_namespace_receiver_symbol(dir, expression_id);
    }

    if expression.target_symbol().is_some() {
        return expression.target_symbol();
    }

    resolve_expression_symbol(dir, expression_id)
}

/// Check whether an expression is used as the receiver of a member access.
fn expression_is_member_receiver_expression(
    dir_tree: &dir::Tree,
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

/// Resolve the best span for a reference expression.
fn resolve_expression_reference_span(
    ast: AstQueryContext<'_>,
    dir: DirQueryContext<'_>,
    dir_tree: &dir::Tree,
    expression_id: dir::LocalNodeId<Expression>,
) -> Option<Span> {
    let span = get_node_tree_main_span(ast, dir.tree(), expression_id.into());

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

    let Some(member_name_span) = get_member_access_name_span(ast, dir, parent_expression_id) else {
        return Some(span);
    };
    let receiver_end = member_name_span.start.saturating_sub(1);

    // clamp spans that include `.member` to only the receiver expression
    if span.file == member_name_span.file && span.start < receiver_end && span.end >= receiver_end {
        return Some(Span::new(span.file, span.start, receiver_end));
    }

    // derive receiver spans when direct mapping points at member names
    let member_span = get_node_tree_span(ast, dir.tree(), parent_expression_id.into());
    if member_span.file == member_name_span.file && member_span.start < receiver_end {
        return Some(Span::new(member_span.file, member_span.start, receiver_end));
    }

    Some(span)
}

/// Resolve the best span for a member receiver expression.
fn resolve_member_receiver_reference_span(
    ast: AstQueryContext<'_>,
    dir: DirQueryContext<'_>,
    member_expression_id: dir::LocalNodeId<Expression>,
    receiver_expression_id: dir::LocalNodeId<Expression>,
) -> Option<Span> {
    let receiver_expression_id = {
        let dir_tree = dir.tree();
        let mut receiver_expression_id = receiver_expression_id;
        loop {
            let receiver_expression = dir_tree.get::<Expression>(receiver_expression_id);
            let Expression::Parenthesized { expression } = receiver_expression else {
                break;
            };
            receiver_expression_id = *expression;
        }
        receiver_expression_id
    };

    let receiver_span = ast_expression_span_for_dir_expression(ast, dir, receiver_expression_id)
        .unwrap_or_else(|| get_node_tree_main_span(ast, dir.tree(), receiver_expression_id.into()));

    let Some(member_name_span) = get_member_access_name_span(ast, dir, member_expression_id) else {
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

    // derive receiver spans from member expression spans when needed
    let member_span = get_node_tree_span(ast, dir.tree(), member_expression_id.into());
    if member_span.file == member_name_span.file && member_span.start < receiver_end {
        return Some(Span::new(member_span.file, member_span.start, receiver_end));
    }

    Some(receiver_span)
}

/// Resolve the main AST span for a DIR expression source id.
fn ast_expression_span_for_dir_expression(
    ast: AstQueryContext<'_>,
    dir: DirQueryContext<'_>,
    expression_id: dir::LocalNodeId<Expression>,
) -> Option<Span> {
    let source_id = dir.tree().get_source(expression_id.id);

    let expression_id = ast::LocalNodeId::<ast::Expression>::new(source_id);
    ast_expression_main_span_without_parentheses(ast, expression_id)
}

/// Resolve an AST expression main span, unwrapping parenthesized expressions.
fn ast_expression_main_span_without_parentheses(
    ast: AstQueryContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> Option<Span> {
    let mut expression_id = expression_id;
    loop {
        let expression = ast.tree().get(expression_id);
        let ast::Expression::Parenthesized { expression } = expression else {
            break;
        };
        expression_id = *expression;
    }

    let expression = ast.tree().get(expression_id);
    if matches!(
        expression,
        ast::Expression::Identifier { .. } | ast::Expression::QualifiedReference { .. }
    ) {
        let expression_span = ast.tree().source_map.get(expression_id.id);
        if let Some(identifier_span) = last_identifier_span_in_expression(ast, expression_span) {
            return Some(identifier_span);
        }
    }

    let span = ast
        .tree()
        .source_map
        .get_main(expression_id.id)
        .unwrap_or_else(|| ast.tree().source_map.get(expression_id.id));
    Some(Span::new(ast.file_id(), span.start, span.end))
}

/// Resolve the last identifier token span inside an expression span.
fn last_identifier_span_in_expression(
    ast: AstQueryContext<'_>,
    expression_span: Span,
) -> Option<Span> {
    let mut last_identifier_span = None;
    for token in ast.tokens() {
        if token.span.file != ast.file_id() {
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
    repository: &Repository,
    ast: AstQueryContext<'_>,
    dir: DirQueryContext<'_>,
    canonical_id: GlobalSymbolId,
    options: ReferenceCollectionOptions<'_>,
    namespace_aliases: &[GlobalSymbolId],
) -> Vec<Span> {
    // initialize the collected span list
    let mut spans = Vec::new();

    // scan member expressions for matches and namespace receivers
    let dir_tree = dir.tree();
    for (expression_id, expression) in dir_tree.iter_nodes_of_type::<Expression>() {
        let Expression::Member { left, name, .. } = expression else {
            continue;
        };
        let Some(name) = *name else {
            continue;
        };

        // resolve the member symbol through normal resolution first
        if let Some(member_symbol) = resolve_member_access_symbol(dir, expression_id) {
            if !symbol_matches_reference_target(
                repository,
                dir.revision(),
                member_symbol,
                canonical_id,
            ) {
                continue;
            }

            let Some(span) = get_member_access_name_span(ast, dir, expression_id) else {
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

        // accept recorded dynamic candidate matches when one member access has no single target
        if member_resolution_matches_reference_target(repository, dir, expression_id, canonical_id)
        {
            let Some(span) = get_member_access_name_span(ast, dir, expression_id) else {
                continue;
            };

            if let Some(limit_file) = options.limit_to_file
                && span.file != limit_file
            {
                continue;
            }

            spans.push(span);
            continue;
        }

        // apply namespace import receiver matching when enabled
        if !options.include_namespace_receivers {
            continue;
        }

        // require a target name for namespace member matching
        let Some(target_name) = options.target_name else {
            continue;
        };

        // resolve the receiver symbol and require it to be a namespace alias
        let Some(receiver_symbol) = resolve_namespace_receiver_symbol(dir, *left) else {
            continue;
        };
        if !namespace_aliases.contains(&receiver_symbol) {
            continue;
        }

        // require the member name to match the requested target name
        let member_name = dir.strings().get(name).to_string();
        if member_name != target_name {
            continue;
        }

        let Some(span) = get_member_access_name_span(ast, dir, expression_id) else {
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

/// Check whether one member access resolution candidate set matches the target.
fn member_resolution_matches_reference_target(
    repository: &Repository,
    dir: DirQueryContext<'_>,
    expression_id: dir::LocalNodeId<Expression>,
    canonical_id: GlobalSymbolId,
) -> bool {
    let types = dir.types();
    let resolution_id = types.node_resolution_id(expression_id.into_global_any(dir.module_id()));
    let Some(resolution_id) = resolution_id else {
        return false;
    };

    let resolution = types.get_resolution(resolution_id);

    // only genuinely dynamic accesses should reach this path
    let Resolution::Dynamic { candidates, .. } = resolution else {
        return false;
    };

    candidates.iter().any(|candidate| {
        symbol_matches_reference_target(
            repository,
            dir.revision(),
            candidate.target_symbol,
            canonical_id,
        )
    })
}

/// Collect dependency item reference spans.
fn collect_dependency_reference_spans(
    repository: &Repository,
    ast: AstQueryContext<'_>,
    dir: DirQueryContext<'_>,
    canonical_id: GlobalSymbolId,
    options: ReferenceCollectionOptions<'_>,
) -> Vec<Span> {
    // collect matching dependency item ids first to avoid borrow issues
    let matching_dependency_ids: Vec<_> = {
        let dir_tree = dir.tree();
        dir_tree
            .iter_nodes_of_type::<DependencyItem>()
            .filter_map(|(item_id, item)| {
                let target = item.target_symbol()?;
                symbol_matches_reference_target(repository, dir.revision(), target, canonical_id)
                    .then_some(item_id)
            })
            .collect()
    };

    // resolve spans for the matching dependency items
    let mut spans = Vec::new();
    for item_id in matching_dependency_ids {
        let span = if options.use_dependency_name_spans {
            dependency_item_name_span(ast, dir, item_id, options.target_name)
        } else {
            None
        }
        .or_else(|| Some(get_node_tree_main_span(ast, dir.tree(), item_id.into())));

        let Some(span) = span else {
            continue;
        };

        // keep rename style collection on the same visible name
        if !span_matches_target_name(repository, dir.revision(), span, options) {
            continue;
        }

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
    ast: AstQueryContext<'_>,
    dir: DirQueryContext<'_>,
    item_id: dir::LocalNodeId<DependencyItem>,
    target_name: Option<&str>,
) -> Option<Span> {
    // fall back to the main span when no name was provided
    let Some(target_name) = target_name else {
        return Some(get_node_tree_main_span(ast, dir.tree(), item_id.into()));
    };

    // resolve name + alias for matching
    let dir_tree = dir.tree();
    let item = dir_tree.get::<DependencyItem>(item_id);
    let (name_id, alias_id) = match item {
        DependencyItem::Remote { name, alias, .. }
        | DependencyItem::Local { name, alias, .. }
        | DependencyItem::UnresolvedRemote { name, alias, .. }
        | DependencyItem::UnresolvedLocal { name, alias, .. } => {
            (name.map(|name| name.string()), *alias)
        }
        DependencyItem::Value { .. } => (None, None),
        DependencyItem::Error => return None,
    };

    // resolve the ast node id for span lookup
    let ast_node_id = dir_tree.get_source(item_id.id);

    // match the remote/local item name first
    if let Some(name_id) = name_id
        && dir.strings().get(name_id) == target_name
    {
        let span = ast
            .tree()
            .get_side_span_by_id(ast_node_id, NodeSpanType::Region(NodeSpanRegion::Type))?;
        return Some(Span::new(ast.file_id(), span.start, span.end));
    }

    // fall back to alias when present
    if let Some(alias_id) = alias_id
        && dir.strings().get(alias_id) == target_name
    {
        let span = ast
            .tree()
            .get_side_span_by_id(ast_node_id, NodeSpanType::Main)?;
        return Some(Span::new(ast.file_id(), span.start, span.end));
    }

    None
}

/// Collect namespace import aliases that target a module.
fn namespace_import_aliases_for_module(
    dir: DirQueryContext<'_>,
    module_id: ModuleId,
) -> Vec<GlobalSymbolId> {
    // scan dependency items for namespace imports to the target module
    let dir_tree = dir.tree();
    dir_tree
        .iter_nodes_of_type::<DependencyItem>()
        .filter_map(|(_item_id, item)| {
            let DependencyItem::Remote {
                binding,
                space,
                symbol,
                target_module,
                ..
            } = item
            else {
                return None;
            };

            // require namespace value imports with a concrete symbol
            if *binding != DependencyBinding::Namespace || *space != DependencySpace::Value {
                return None;
            }

            let local_symbol = symbol.as_ref()?;
            let target_module_id = target_module
                .for_space(*space)
                .and_then(|target| target.module_id())?;
            if target_module_id != module_id {
                return None;
            }

            Some(GlobalSymbolId::new(dir.module_id(), *local_symbol))
        })
        .collect()
}
